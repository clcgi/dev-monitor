use super::format::{self as f, DASH};
use super::model::{BlobCheck, DeadLetters, Doc, Overview, StuckRoot, Timing};
use super::trace_view::{Tone, STALL_AFTER_MINUTES};
use chrono::{DateTime, Duration, Utc};
use std::collections::HashMap;

pub const HOT_WITHIN_HOURS: i64 = 48;

/// A pointer older than a nightly build plus half a night has missed a build.
pub const STALE_REFERENCE_HOURS: i64 = 36;

#[derive(Clone, PartialEq, Debug)]
pub struct Kpi {
    pub label: &'static str,
    pub value: String,
    pub unit: String,
    pub tone: Tone,
}

/// `2d`, `14h`, `35m`: the lifecycle column's short countdown.
fn countdown(d: Duration) -> String {
    let d = if d < Duration::zero() { -d } else { d };
    if d >= Duration::days(1) {
        format!("{}d", d.num_days())
    } else if d >= Duration::hours(1) {
        format!("{}h", d.num_hours())
    } else {
        format!("{}m", d.num_minutes().max(1))
    }
}

fn pending_max_days(o: &Overview, doc: &Doc) -> Option<i64> {
    o.sources.iter().find(|s| s.source_system == doc.source_system).and_then(|s| s.pending_max_days)
}

fn landing_deletes_at(doc: &Doc, retention_days: i64) -> Option<DateTime<Utc>> {
    f::parse_opt(&doc.uploaded_at).map(|u| u + Duration::days(retention_days.max(0)))
}

#[derive(Clone, PartialEq, Debug)]
pub struct QueueRow {
    pub file_guid: String,
    pub key: String,
    pub document_id: String,
    pub src: String,
    pub arrived: String,
    pub waiting: String,
    pub attempts: String,
    pub pending_key: String,
    pub lifecycle: String,
    pub hot: bool,
    pub arrived_at: Option<i64>,
    pub waiting_secs: Option<i64>,
    pub attempt_count: Option<i64>,
    pub lifecycle_secs: Option<i64>,
}

fn next_lifecycle(o: &Overview, doc: &Doc, now: DateTime<Utc>) -> Option<(String, bool, i64)> {
    let deletes = landing_deletes_at(doc, o.env.landing_retention_days).map(|t| ("deletes", t));
    let expires = f::parse_opt(&doc.pending_since)
        .zip(pending_max_days(o, doc))
        .map(|(s, d)| ("quarantines", s + Duration::days(d)));
    let (verb, at) = [deletes, expires].into_iter().flatten().min_by_key(|(_, t)| *t)?;
    let left = at - now;
    let hot = left < Duration::hours(HOT_WITHIN_HOURS);
    Some(if left < Duration::zero() {
        (format!("{verb} overdue {}", countdown(left)), true, left.num_seconds())
    } else {
        (format!("{verb} {}", countdown(left)), hot, left.num_seconds())
    })
}

pub fn queue(o: &Overview, now: DateTime<Utc>) -> (Vec<Kpi>, Vec<QueueRow>) {
    let mut parked: Vec<&Doc> = o.parked.iter().collect();
    parked.sort_by_key(|d| f::parse_opt(&d.pending_since).or_else(|| f::parse_opt(&d.uploaded_at)));

    let rows: Vec<QueueRow> = parked
        .iter()
        .map(|d| {
            let since = f::parse_opt(&d.pending_since);
            let (lifecycle, hot, lifecycle_secs) = match next_lifecycle(o, d, now) {
                Some((text, hot, secs)) => (text, hot, Some(secs)),
                None => (DASH.into(), false, None),
            };
            QueueRow {
                file_guid: d.file_guid.clone(),
                key: d.display_key().to_string(),
                document_id: d.document_id.clone(),
                src: f::or_dash(Some(d.source_system.clone())),
                arrived: f::parse_opt(&d.uploaded_at).or(since).map(f::day_minute).unwrap_or_else(|| DASH.into()),
                waiting: since.map(|s| f::span(now - s)).unwrap_or_else(|| DASH.into()),
                attempts: d.pending_attempts.map(|a| a.to_string()).unwrap_or_else(|| DASH.into()),
                pending_key: f::or_dash(d.pending_key.clone()),
                lifecycle,
                hot,
                arrived_at: f::parse_opt(&d.uploaded_at).or(since).map(|t| t.timestamp()),
                waiting_secs: since.map(|s| (now - s).num_seconds()),
                attempt_count: d.pending_attempts.map(i64::from),
                lifecycle_secs,
            }
        })
        .collect();

    let oldest = parked.first().and_then(|d| f::parse_opt(&d.pending_since)).map(|s| f::span(now - s));
    let deleting = parked
        .iter()
        .filter(|d| landing_deletes_at(d, o.env.landing_retention_days).is_some_and(|t| t - now < Duration::hours(HOT_WITHIN_HOURS)))
        .count();
    let pointer = reference_for_parked(o);
    let reference = match pointer {
        Some(p) if p.error.is_none() && p.generation.is_some() => {
            let age = f::parse_opt(&p.built_at).map(|b| now - b);
            Kpi {
                label: "REFERENCE GENERATION",
                value: p.generation.unwrap_or_default().to_string(),
                unit: age.map(|a| format!("{} old", f::span(a))).unwrap_or_else(|| "built at unknown".into()),
                tone: if age.is_some_and(|a| a > Duration::hours(STALE_REFERENCE_HOURS)) { Tone::Warn } else { Tone::Ok },
            }
        }
        Some(_) => Kpi { label: "REFERENCE GENERATION", value: DASH.into(), unit: "pointer unreadable".into(), tone: Tone::Bad },
        None => Kpi { label: "REFERENCE GENERATION", value: DASH.into(), unit: "no reference set".into(), tone: Tone::Info },
    };
    let kpis = vec![
        Kpi { label: "PARKED DOCUMENTS", value: parked.len().to_string(), unit: String::new(), tone: Tone::Warn },
        Kpi { label: "OLDEST WAIT", value: oldest.unwrap_or_else(|| DASH.into()), unit: String::new(), tone: Tone::Warn },
        Kpi {
            label: "LANDING DELETES < 48H",
            value: deleting.to_string(),
            unit: "docs".into(),
            tone: if deleting > 0 { Tone::Bad } else { Tone::Info },
        },
        reference,
    ];
    (kpis, rows)
}

/// The set most parked documents wait on; the first set when none are parked.
fn reference_for_parked(o: &Overview) -> Option<&super::model::RefPointer> {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for d in &o.parked {
        if let Some(set) = o.sources.iter().find(|s| s.source_system == d.source_system).and_then(|s| s.reference_set.as_deref()) {
            *counts.entry(set).or_default() += 1;
        }
    }
    let busiest = counts.into_iter().max_by_key(|(set, n)| (*n, std::cmp::Reverse(*set))).map(|(s, _)| s);
    match busiest {
        Some(set) => o.references.iter().find(|p| p.reference_set_id == set),
        None => o.references.first(),
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct StuckRow {
    pub file_guid: String,
    pub document_id: String,
    pub key: String,
    pub rev: String,
    pub written: String,
    /// 0..=100, of the files the archive lists; None when the listing is unknown.
    pub pct: Option<u32>,
    pub depth: String,
    pub last: String,
    pub replica: String,
    pub stalled: bool,
    pub last_secs: Option<i64>,
    pub deepest_n: Option<i64>,
}

pub fn is_stalled(s: &StuckRoot, now: DateTime<Utc>) -> bool {
    if f::parse_opt(&s.root.archive_extraction_lease_expires_at).is_some_and(|l| l > now) {
        return false;
    }
    f::parse_opt(&s.last_progress).map_or(true, |p| now - p > Duration::minutes(STALL_AFTER_MINUTES))
}

pub fn stuck(o: &Overview, now: DateTime<Utc>) -> Vec<StuckRow> {
    let mut roots: Vec<&StuckRoot> = o.stuck.iter().collect();
    roots.sort_by_key(|s| f::parse_opt(&s.last_progress));
    roots
        .into_iter()
        .map(|s| {
            let listed = s.expected.as_ref().filter(|e| !e.entries.is_empty());
            let total = listed.map(|e| e.entries.iter().filter(|x| !x.container && x.rejected.is_none()).count());
            let max_depth = listed.and_then(|e| e.entries.iter().map(|x| x.depth).max());
            StuckRow {
                file_guid: s.root.file_guid.clone(),
                document_id: s.root.document_id.clone(),
                key: s.root.display_key().to_string(),
                rev: f::or_dash(Some(s.root.revision_id.clone())),
                written: match total {
                    Some(t) => format!("{} / {t}", s.members_written),
                    None => format!("{} / ?", s.members_written),
                },
                pct: total.map(|t| if t == 0 { 0 } else { (s.members_written as usize * 100 / t).min(100) as u32 }),
                depth: match (s.deepest, max_depth) {
                    (Some(d), Some(m)) => format!("level {d} of {m}"),
                    (Some(d), None) => format!("level {d}"),
                    (None, Some(m)) => format!("none of {m}"),
                    (None, None) => DASH.into(),
                },
                last: f::parse_opt(&s.last_progress).map(|p| format!("{} ago", f::span(now - p))).unwrap_or_else(|| DASH.into()),
                replica: s.replica.clone().unwrap_or_else(|| "unknown".into()),
                stalled: is_stalled(s, now),
                last_secs: f::parse_opt(&s.last_progress).map(|p| (now - p).num_seconds()),
                deepest_n: s.deepest.map(i64::from),
            }
        })
        .collect()
}

#[derive(Clone, PartialEq, Debug)]
pub struct RefRow {
    pub set_id: String,
    pub generation: String,
    pub built_at: String,
    pub age: String,
    pub rows: String,
    pub waiting: usize,
    pub tone: Tone,
    pub note: String,
    pub generation_n: Option<i64>,
    pub age_secs: Option<i64>,
    pub row_count: Option<i64>,
}

pub fn references(o: &Overview, now: DateTime<Utc>) -> Vec<RefRow> {
    o.references
        .iter()
        .map(|p| {
            let sources: Vec<&str> = o
                .sources
                .iter()
                .filter(|s| s.reference_set.as_deref() == Some(p.reference_set_id.as_str()))
                .map(|s| s.source_system.as_str())
                .collect();
            let waiting = o.parked.iter().filter(|d| sources.contains(&d.source_system.as_str())).count();
            let built = f::parse_opt(&p.built_at);
            let age = built.map(|b| now - b);
            let stale = age.is_some_and(|a| a > Duration::hours(STALE_REFERENCE_HOURS));
            let (tone, note) = match (&p.error, stale) {
                (Some(e), _) => (Tone::Bad, e.clone()),
                (None, true) => (Tone::Warn, format!("Older than {STALE_REFERENCE_HOURS}h: a nightly build has likely not run.")),
                (None, false) => (Tone::Ok, format!("Read by {}.", if sources.is_empty() { "no source".into() } else { sources.join(", ") })),
            };
            RefRow {
                set_id: p.reference_set_id.clone(),
                generation: p.generation.map(|g| g.to_string()).unwrap_or_else(|| DASH.into()),
                built_at: built.map(f::full).unwrap_or_else(|| DASH.into()),
                age: age.map(f::span).unwrap_or_else(|| DASH.into()),
                rows: p.row_count.map(|r| r.to_string()).unwrap_or_else(|| DASH.into()),
                waiting,
                tone,
                note,
                generation_n: p.generation,
                age_secs: age.map(|a| a.num_seconds()),
                row_count: p.row_count.map(|r| r as i64),
            }
        })
        .collect()
}

#[derive(Clone, PartialEq, Debug)]
pub struct DeadRow {
    pub key: String,
    pub document_id: Option<String>,
    pub parked_at: String,
    pub queue: String,
    pub landing_deletes: String,
    pub retry: String,
    pub hot: bool,
    pub parked_ts: Option<i64>,
    pub deletes_secs: Option<i64>,
    pub deliveries: Option<i64>,
    pub sequence: Option<i64>,
}

pub fn dead_letters(d: &DeadLetters, now: DateTime<Utc>) -> Vec<DeadRow> {
    let mut messages: Vec<_> = d.messages.iter().collect();
    messages.sort_by_key(|m| f::parse_opt(&m.enqueued_at));
    messages
        .into_iter()
        .map(|m| {
            let doc = m.document_id.as_ref().and_then(|id| d.docs.iter().find(|x| &x.document_id == id));
            let deletes = doc.and_then(|x| landing_deletes_at(x, d.env.landing_retention_days));
            let promoted = doc.is_some_and(|x| x.promoted_at.is_some());
            DeadRow {
                key: doc
                    .map(|x| x.display_key().to_string())
                    .or_else(|| m.document_id.clone())
                    .or_else(|| m.subject.clone())
                    .unwrap_or_else(|| format!("message #{}", m.sequence.unwrap_or_default())),
                document_id: m.document_id.clone(),
                parked_at: f::parse_opt(&m.enqueued_at).map(f::day_minute).unwrap_or_else(|| DASH.into()),
                queue: m.queue.clone(),
                landing_deletes: match (promoted, deletes) {
                    (true, _) => "promoted · safe".into(),
                    (false, Some(t)) if t < now => format!("deleted {} ago", f::span(now - t)),
                    (false, Some(t)) => format!("deletes {}", countdown(t - now)),
                    (false, None) => DASH.into(),
                },
                retry: format!(
                    "{} deliver{}{}",
                    m.delivery_count.unwrap_or_default(),
                    if m.delivery_count == Some(1) { "y" } else { "ies" },
                    m.reason.as_ref().map(|r| format!(" · {r}")).unwrap_or_default()
                ),
                hot: !promoted && deletes.is_some_and(|t| t - now < Duration::hours(HOT_WITHIN_HOURS)),
                parked_ts: f::parse_opt(&m.enqueued_at).map(|t| t.timestamp()),
                deletes_secs: deletes.map(|t| (t - now).num_seconds()),
                deliveries: m.delivery_count.map(i64::from),
                sequence: m.sequence,
            }
        })
        .collect()
}

#[derive(Clone, PartialEq, Debug)]
pub struct TimingRow {
    pub step: &'static str,
    pub p50: String,
    pub p95: String,
    pub samples: usize,
    pub in_step: usize,
    pub oldest: String,
    pub p50_secs: Option<i64>,
    pub p95_secs: Option<i64>,
    pub oldest_secs: Option<i64>,
}

/// Nearest-rank: an observed dwell, never an interpolated one.
pub fn percentile(sorted: &[Duration], p: f64) -> Option<Duration> {
    if sorted.is_empty() {
        return None;
    }
    let rank = ((p * sorted.len() as f64).ceil() as usize).clamp(1, sorted.len());
    Some(sorted[rank - 1])
}

type Pair = fn(&Doc) -> (Option<String>, Option<String>);
type InStep = fn(&Doc) -> Option<Option<String>>;

pub fn timing(t: &Timing, now: DateTime<Utc>) -> Vec<TimingRow> {
    // Members copy their root's timestamps, so counting them would repeat the root's dwell per file.
    let docs: Vec<&Doc> = t.docs.iter().filter(|d| !d.is_member()).collect();
    let plan: [(&'static str, Pair, InStep); 5] = [
        (
            "Registered → uploaded",
            |d| (d.registered_at.clone(), d.uploaded_at.clone()),
            |d| matches!(d.state.as_str(), "Registered" | "AwaitingUpload").then(|| d.registered_at.clone()),
        ),
        (
            "Gate: uploaded → promoted",
            |d| if d.pending_since.is_some() { (None, None) } else { (d.uploaded_at.clone(), d.promoted_at.clone()) },
            |d| (d.state == "Uploaded").then(|| d.uploaded_at.clone()),
        ),
        (
            "Waiting on reference",
            |d| (d.pending_since.clone(), d.promoted_at.clone()),
            |d| (d.state == "PendingMetadata").then(|| d.pending_since.clone()),
        ),
        (
            "Promoted → dispatched",
            |d| (d.promoted_at.clone(), d.dispatched_at.clone()),
            |d| (d.state == "Promoted" && d.archive_extraction_state.is_none()).then(|| d.promoted_at.clone()),
        ),
        (
            "Archive extraction",
            |d| (d.promoted_at.clone(), d.archive_extracted_at.clone()),
            |d| (d.archive_extraction_state.as_deref() == Some("Extracting")).then(|| d.promoted_at.clone()),
        ),
    ];
    plan.iter()
        .map(|(step, pair, in_step)| {
            let mut dwell: Vec<Duration> = docs
                .iter()
                .filter_map(|d| {
                    let (a, b) = pair(d);
                    Some(f::parse_opt(&b)? - f::parse_opt(&a)?)
                })
                .filter(|d| *d >= Duration::zero())
                .collect();
            dwell.sort();
            let waiting: Vec<Option<DateTime<Utc>>> = docs.iter().filter_map(|d| in_step(d)).map(|s| f::parse_opt(&s)).collect();
            let oldest = waiting.iter().flatten().min().map(|s| f::span(now - *s));
            let oldest_secs = waiting.iter().flatten().min().map(|s| (now - *s).num_seconds());
            TimingRow {
                step,
                p50: percentile(&dwell, 0.50).map(f::span).unwrap_or_else(|| DASH.into()),
                p95: percentile(&dwell, 0.95).map(f::span).unwrap_or_else(|| DASH.into()),
                samples: dwell.len(),
                in_step: waiting.len(),
                oldest: oldest.unwrap_or_else(|| DASH.into()),
                p50_secs: percentile(&dwell, 0.50).map(|d| d.num_seconds()),
                p95_secs: percentile(&dwell, 0.95).map(|d| d.num_seconds()),
                oldest_secs,
            }
        })
        .collect()
}

#[derive(Clone, PartialEq, Debug)]
pub struct Home {
    pub empty: bool,
    pub headline: String,
    pub body: String,
    pub facts: Vec<(String, String)>,
}

fn plural(n: u64, word: &str) -> String {
    format!("{n} {word}{}", if n == 1 { "" } else { "s" })
}

pub fn home(o: &Overview, now: DateTime<Utc>) -> Home {
    let env = if o.env.name.is_empty() { "This environment".to_string() } else { o.env.name.to_uppercase() };
    let as_of = f::parse(&o.checked_at).map(f::full).unwrap_or_else(|| DASH.into());
    let stalled = o.stuck.iter().filter(|s| is_stalled(s, now)).count();
    let facts = vec![
        ("Catalog rows".to_string(), o.catalog_rows.to_string()),
        ("Audit rows".to_string(), o.audit_rows.to_string()),
        ("Parked on metadata".to_string(), o.parked.len().to_string()),
        ("Extractions stalled".to_string(), format!("{stalled} of {}", o.stuck.len())),
        ("Curated documents".to_string(), o.curated_rows.to_string()),
        ("Dispatched, not curated".to_string(), o.dispatched_not_curated.to_string()),
        ("Delivered to PACE".to_string(), o.pace_delivered.len().to_string()),
        ("Reference sets".to_string(), o.references.len().to_string()),
        ("Cosmos".to_string(), format!("{}/{}", o.env.cosmos_account, o.env.cosmos_db)),
        ("Storage".to_string(), o.env.store_account.clone()),
    ];
    if o.catalog_rows == 0 {
        Home {
            empty: true,
            headline: format!("{env} has nothing to trace yet"),
            body: format!(
                "{} and 0 catalog rows as of {as_of}. This is real platform state, not a fault in the viewer.",
                plural(o.audit_rows, "audit row")
            ),
            facts,
        }
    } else {
        Home {
            empty: false,
            headline: format!("{env} as it is today"),
            body: format!(
                "{} and {} as of {as_of}. Every figure below was read just now, read-only.",
                plural(o.catalog_rows, "catalog row"),
                plural(o.audit_rows, "audit row")
            ),
            facts,
        }
    }
}

// ------------------------------------------------------------------ sorting and pages

#[derive(Clone, PartialEq, Debug)]
pub enum SortKey {
    Num(i64),
    Text(String),
    Missing,
}

pub trait Sortable {
    fn sort_key(&self, column: usize) -> SortKey;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Sort {
    pub column: usize,
    pub ascending: bool,
}

pub const PAGE_SIZE: usize = 50;

/// The same column flips direction; another column starts ascending.
pub fn toggle(current: Option<Sort>, column: usize) -> Sort {
    match current {
        Some(s) if s.column == column => Sort { column, ascending: !s.ascending },
        _ => Sort { column, ascending: true },
    }
}

/// Stable, and unknown values stay last in either direction.
pub fn sorted<T: Sortable + Clone>(rows: &[T], sort: Option<Sort>) -> Vec<T> {
    use std::cmp::Ordering;
    let mut out = rows.to_vec();
    if let Some(Sort { column, ascending }) = sort {
        out.sort_by(|a, b| match (a.sort_key(column), b.sort_key(column)) {
            (SortKey::Missing, SortKey::Missing) => Ordering::Equal,
            (SortKey::Missing, _) => Ordering::Greater,
            (_, SortKey::Missing) => Ordering::Less,
            (x, y) => {
                let order = match (x, y) {
                    (SortKey::Num(x), SortKey::Num(y)) => x.cmp(&y),
                    (SortKey::Text(x), SortKey::Text(y)) => x.to_lowercase().cmp(&y.to_lowercase()),
                    (SortKey::Num(_), _) => Ordering::Less,
                    _ => Ordering::Greater,
                };
                if ascending { order } else { order.reverse() }
            }
        });
    }
    out
}

#[derive(Clone, PartialEq, Debug)]
pub struct Page<T> {
    pub rows: Vec<T>,
    pub page: usize,
    pub pages: usize,
    pub total: usize,
}

/// A page past the end (after a refresh shrank the data) lands on the last page.
pub fn paginate<T: Clone>(rows: &[T], page: usize) -> Page<T> {
    let total = rows.len();
    let pages = total.div_ceil(PAGE_SIZE).max(1);
    let page = page.min(pages - 1);
    Page { rows: rows.iter().skip(page * PAGE_SIZE).take(PAGE_SIZE).cloned().collect(), page, pages, total }
}

fn text(s: &str) -> SortKey {
    if s.is_empty() || s == DASH { SortKey::Missing } else { SortKey::Text(s.to_string()) }
}

fn num(n: Option<i64>) -> SortKey {
    n.map(SortKey::Num).unwrap_or(SortKey::Missing)
}

impl Sortable for QueueRow {
    fn sort_key(&self, column: usize) -> SortKey {
        match column {
            0 => text(&self.key),
            1 => text(&self.src),
            2 => num(self.arrived_at),
            3 => num(self.waiting_secs),
            4 => num(self.attempt_count),
            5 => text(&self.pending_key),
            6 => num(self.lifecycle_secs),
            _ => SortKey::Missing,
        }
    }
}

impl Sortable for StuckRow {
    fn sort_key(&self, column: usize) -> SortKey {
        match column {
            0 => text(&self.document_id),
            1 => self.rev.parse::<i64>().map(SortKey::Num).unwrap_or_else(|_| text(&self.rev)),
            2 => num(self.pct.map(i64::from)),
            3 => num(self.deepest_n),
            4 => num(self.last_secs),
            5 => text(&self.replica),
            _ => SortKey::Missing,
        }
    }
}

impl Sortable for RefRow {
    fn sort_key(&self, column: usize) -> SortKey {
        match column {
            0 => text(&self.set_id),
            1 => num(self.generation_n),
            2 => num(self.age_secs.map(|a| -a)),
            3 => num(self.age_secs),
            4 => num(self.row_count),
            5 => SortKey::Num(self.waiting as i64),
            _ => SortKey::Missing,
        }
    }
}

impl Sortable for DeadRow {
    fn sort_key(&self, column: usize) -> SortKey {
        match column {
            0 => text(&self.key),
            1 => num(self.parked_ts),
            2 => text(&self.queue),
            3 => num(self.deletes_secs),
            4 => num(self.deliveries),
            _ => SortKey::Missing,
        }
    }
}

impl Sortable for TimingRow {
    fn sort_key(&self, column: usize) -> SortKey {
        match column {
            0 => text(self.step),
            1 => num(self.p50_secs),
            2 => num(self.p95_secs),
            3 => SortKey::Num(self.samples as i64),
            4 => SortKey::Num(self.in_step as i64),
            5 => num(self.oldest_secs),
            _ => SortKey::Missing,
        }
    }
}

impl Sortable for BlobCheck {
    fn sort_key(&self, column: usize) -> SortKey {
        match column {
            0 => self.path.as_deref().map(text).unwrap_or(SortKey::Missing),
            1 => num(self.size.map(|s| s as i64)),
            2 => num(f::parse_opt(&self.last_modified).map(|t| t.timestamp())),
            _ => SortKey::Missing,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitoring::model::{DeadMessage, Env, Expected, ExpectedEntry, RefPointer, SourceCfg};

    fn t(s: &str) -> DateTime<Utc> {
        f::parse(s).unwrap()
    }

    fn overview() -> Overview {
        let doc = |key: &str, since: &str, uploaded: &str| Doc {
            document_id: format!("id-{key}"),
            business_key: key.into(),
            source_system: "NEO".into(),
            state: "PendingMetadata".into(),
            pending_since: Some(since.into()),
            uploaded_at: Some(uploaded.into()),
            pending_attempts: Some(2),
            pending_key: Some(format!("NEO/{key}")),
            ..Default::default()
        };
        Overview {
            env: Env { name: "dev".into(), landing_retention_days: 8, pending_max_days: 5, ..Default::default() },
            checked_at: "2026-08-14T09:00:00Z".into(),
            catalog_rows: 12,
            audit_rows: 40,
            parked: vec![
                doc("young", "2026-08-14T04:41:00Z", "2026-08-14T04:41:00Z"),
                doc("old", "2026-08-08T01:04:00Z", "2026-08-08T01:04:00Z"),
            ],
            sources: vec![SourceCfg { source_system: "NEO".into(), reference_set: Some("mdr".into()), pending_max_days: Some(7) }],
            references: vec![RefPointer { reference_set_id: "mdr".into(), generation: Some(2), row_count: Some(9), built_at: Some("2026-08-14T02:00:00Z".into()), ..Default::default() }],
            ..Default::default()
        }
    }

    #[test]
    fn the_queue_is_oldest_first_and_names_the_nearer_lifecycle_event() {
        let now = t("2026-08-14T09:00:00Z");
        let (kpis, rows) = queue(&overview(), now);
        assert_eq!(rows[0].key, "old");
        // The 7-day expiry (08-15 01:04) comes before the 8-day landing delete (08-16 01:04).
        assert_eq!(rows[0].lifecycle, "quarantines 16h");
        assert!(rows[0].hot);
        assert_eq!(rows[1].lifecycle, "quarantines 6d");
        assert!(!rows[1].hot);
        assert_eq!(kpis[0].value, "2");
        assert_eq!(kpis[1].value, "6d 07h");
        // The old document's landing copy deletes in 40h, inside the window.
        assert_eq!(kpis[2].value, "1");
        assert_eq!((kpis[3].value.as_str(), kpis[3].unit.as_str()), ("2", "7h 00m old"));
    }

    #[test]
    fn a_source_without_a_pending_budget_only_counts_down_to_the_landing_delete() {
        let mut o = overview();
        o.sources[0].reference_set = None;
        o.sources[0].pending_max_days = None;
        let (_, rows) = queue(&o, t("2026-08-14T09:00:00Z"));
        assert_eq!(rows[0].lifecycle, "deletes 1d");
        assert!(rows[0].hot);
    }

    #[test]
    fn an_overdue_event_is_hot_and_says_overdue() {
        let now = t("2026-08-20T09:00:00Z");
        let (_, rows) = queue(&overview(), now);
        assert_eq!(rows[0].lifecycle, "quarantines overdue 5d");
        assert!(rows[0].hot);
    }

    #[test]
    fn stuck_rows_count_files_only_and_order_by_silence() {
        let now = t("2026-08-14T09:04:00Z");
        let entry = |p: &str, depth: u32, container: bool| ExpectedEntry { path: p.into(), depth, container, ..Default::default() };
        let root = |id: &str, last: &str, written: u64| StuckRoot {
            root: Doc { document_id: id.into(), revision_id: "00".into(), ..Default::default() },
            members_written: written,
            last_progress: Some(last.into()),
            deepest: Some(0),
            expected: Some(Expected { entries: vec![entry("a.pdf", 0, false), entry("b.zip", 0, true), entry("b.zip/c.pdf", 1, false)], error: None }),
            replica: None,
        };
        let mut o = overview();
        o.stuck = vec![root("recent", "2026-08-14T09:00:00Z", 1), root("silent", "2026-08-14T06:46:00Z", 1)];
        let rows = stuck(&o, now);
        assert_eq!(rows[0].document_id, "silent");
        assert_eq!(rows[0].written, "1 / 2");
        assert_eq!(rows[0].pct, Some(50));
        assert_eq!(rows[0].depth, "level 0 of 1");
        assert_eq!(rows[0].last, "2h 18m ago");
        assert!(rows[0].stalled);
        assert!(!rows[1].stalled, "4 minutes of quiet is not a stall");
        assert_eq!(rows[1].replica, "unknown");
    }

    #[derive(Clone, Debug, PartialEq)]
    struct Row(Option<i64>);

    impl Sortable for Row {
        fn sort_key(&self, _: usize) -> SortKey {
            num(self.0)
        }
    }

    #[test]
    fn sorting_flips_direction_and_keeps_unknowns_last() {
        let rows = vec![Row(Some(2)), Row(None), Row(Some(1)), Row(Some(3))];
        let asc = toggle(None, 0);
        assert_eq!(sorted(&rows, Some(asc)), vec![Row(Some(1)), Row(Some(2)), Row(Some(3)), Row(None)]);
        let desc = toggle(Some(asc), 0);
        assert!(!desc.ascending);
        assert_eq!(sorted(&rows, Some(desc)), vec![Row(Some(3)), Row(Some(2)), Row(Some(1)), Row(None)]);
        assert!(toggle(Some(desc), 1).ascending, "a new column starts ascending");
        assert_eq!(sorted(&rows, None), rows, "no sort keeps the derived order");
    }

    #[test]
    fn pages_hold_fifty_rows_and_clamp_past_the_end() {
        let rows: Vec<usize> = (0..120).collect();
        let third = paginate(&rows, 2);
        assert_eq!((third.rows.len(), third.page, third.pages, third.total, third.rows[0]), (20, 2, 3, 120, 100));
        assert_eq!(paginate(&rows, 9).page, 2);
        assert_eq!(paginate::<usize>(&[], 0).pages, 1);
        assert_eq!(paginate(&rows[..50], 0).pages, 1);
    }

    #[test]
    fn the_parked_queue_sorts_by_waiting_time() {
        let (_, rows) = queue(&overview(), t("2026-08-14T09:00:00Z"));
        let longest_first = sorted(&rows, Some(Sort { column: 3, ascending: false }));
        assert_eq!(longest_first[0].key, "old");
        let shortest_first = sorted(&rows, Some(Sort { column: 3, ascending: true }));
        assert_eq!(shortest_first[0].key, "young");
    }

    #[derive(Clone, Debug, PartialEq)]
    struct Label(&'static str);

    impl Sortable for Label {
        fn sort_key(&self, _: usize) -> SortKey {
            text(self.0)
        }
    }

    #[test]
    fn text_sorts_case_insensitively_and_numbers_before_text() {
        let rows = vec![Label("beta"), Label("Alpha"), Label("—"), Label("gamma")];
        let asc = sorted(&rows, Some(Sort { column: 0, ascending: true }));
        assert_eq!(asc, vec![Label("Alpha"), Label("beta"), Label("gamma"), Label("—")]);
        struct Mixed(SortKey);
        impl Clone for Mixed { fn clone(&self) -> Self { Mixed(self.0.clone()) } }
        impl Sortable for Mixed { fn sort_key(&self, _: usize) -> SortKey { self.0.clone() } }
        let mixed = sorted(&[Mixed(SortKey::Text("C".into())), Mixed(SortKey::Num(9))], Some(Sort { column: 0, ascending: true }));
        assert_eq!(mixed[0].0, SortKey::Num(9));
    }

    #[test]
    fn built_at_and_age_sort_in_opposite_directions() {
        let mut o = overview();
        o.references.push(RefPointer { reference_set_id: "old".into(), generation: Some(1), built_at: Some("2026-08-01T00:00:00Z".into()), ..Default::default() });
        let rows = references(&o, t("2026-08-14T09:00:00Z"));
        let by_built = sorted(&rows, Some(Sort { column: 2, ascending: true }));
        let by_age = sorted(&rows, Some(Sort { column: 3, ascending: true }));
        assert_eq!(by_built[0].set_id, "old", "oldest build first");
        assert_eq!(by_age[0].set_id, "mdr", "youngest first");
    }

    #[test]
    fn deepest_level_sorts_as_a_number_not_as_text() {
        let root = |id: &str, deepest: u32| StuckRoot {
            root: Doc { document_id: id.into(), revision_id: "10".into(), ..Default::default() },
            deepest: Some(deepest),
            last_progress: Some("2026-08-14T08:00:00Z".into()),
            ..Default::default()
        };
        let mut o = overview();
        o.stuck = vec![root("ten", 10), root("two", 2)];
        let rows = stuck(&o, t("2026-08-14T09:00:00Z"));
        assert_eq!(sorted(&rows, Some(Sort { column: 3, ascending: true }))[0].document_id, "two");
        assert_eq!(rows[0].sort_key(1), SortKey::Num(10), "a numeric revision sorts as a number");
    }

    #[test]
    fn nearest_rank_percentiles() {
        let d: Vec<Duration> = (1..=20).map(Duration::seconds).collect();
        assert_eq!(percentile(&d, 0.50), Some(Duration::seconds(10)));
        assert_eq!(percentile(&d, 0.95), Some(Duration::seconds(19)));
        assert_eq!(percentile(&[], 0.5), None);
    }

    #[test]
    fn timing_ignores_members_and_counts_documents_in_step() {
        let now = t("2026-08-14T09:00:00Z");
        let promoted = |id: &str, root_guid: &str, mins: i64| Doc {
            document_id: id.into(),
            file_guid: id.into(),
            root_file_guid: root_guid.into(),
            state: "Dispatched".into(),
            uploaded_at: Some("2026-08-14T08:00:00Z".into()),
            promoted_at: Some(f::full(t("2026-08-14T08:00:00Z") + Duration::minutes(mins)).replace(' ', "T")),
            ..Default::default()
        };
        let parked = Doc { state: "PendingMetadata".into(), pending_since: Some("2026-08-13T09:00:00Z".into()), ..Default::default() };
        let timing_input = Timing {
            docs: vec![promoted("a", "", 2), promoted("b", "", 4), promoted("m", "root", 500), parked],
            ..Default::default()
        };
        let rows = timing(&timing_input, now);
        let gate = rows.iter().find(|r| r.step == "Gate: uploaded → promoted").unwrap();
        assert_eq!((gate.samples, gate.p50.as_str(), gate.p95.as_str()), (2, "2m 00s", "4m 00s"));
        let reference = rows.iter().find(|r| r.step == "Waiting on reference").unwrap();
        assert_eq!((reference.in_step, reference.oldest.as_str()), (1, "1d 00h"));
    }

    #[test]
    fn dead_letters_resolve_documents_and_flag_landing_deletes() {
        let now = t("2026-08-14T09:00:00Z");
        let d = DeadLetters {
            env: Env { landing_retention_days: 8, ..Default::default() },
            messages: vec![DeadMessage {
                queue: "landing-created".into(),
                enqueued_at: Some("2026-08-07T10:00:00Z".into()),
                delivery_count: Some(10),
                reason: Some("MaxDeliveryCountExceeded".into()),
                document_id: Some("D1".into()),
                ..Default::default()
            }],
            docs: vec![Doc { document_id: "D1".into(), business_key: "001-x".into(), uploaded_at: Some("2026-08-07T09:59:00Z".into()), ..Default::default() }],
            ..Default::default()
        };
        let rows = dead_letters(&d, now);
        assert_eq!(rows[0].key, "001-x");
        assert_eq!(rows[0].landing_deletes, "deletes 1d");
        assert!(rows[0].hot);
        assert_eq!(rows[0].retry, "10 deliveries · MaxDeliveryCountExceeded");
    }

    #[test]
    fn an_empty_catalog_is_reported_as_nothing_to_trace() {
        let mut o = overview();
        o.catalog_rows = 0;
        o.audit_rows = 1;
        let h = home(&o, t("2026-08-14T09:00:00Z"));
        assert!(h.empty);
        assert_eq!(h.headline, "DEV has nothing to trace yet");
        assert!(h.body.starts_with("1 audit row and 0 catalog rows as of 2026-08-14 09:00:00Z"));
    }

    #[test]
    fn a_stale_pointer_warns_and_counts_who_waits_on_it() {
        let rows = references(&overview(), t("2026-08-16T09:00:00Z"));
        assert_eq!(rows[0].waiting, 2);
        assert_eq!(rows[0].tone, Tone::Warn);
    }
}
