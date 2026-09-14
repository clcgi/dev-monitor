use super::format::{self as f, DASH};
use super::model::{AuditRow, BlobCheck, Doc, ExpectedEntry, LogRow, Trace};
use chrono::{DateTime, Duration, Utc};
use serde_json::Value;
use std::collections::{BTreeMap, HashMap};

pub const METADATA_UNRESOLVED: &str = "METADATA_UNRESOLVED";

/// cdw.domain.file_roles.FileIndicator: curated placement parks any other value (reason NO_ROLE).
pub const READABLE_INDICATORS: [&str; 4] = ["RO", "RW", "RP", "RA"];

pub const STALL_AFTER_MINUTES: i64 = 10;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Done,
    Waiting,
    Stalled,
    Failed,
    Skipped,
    NotBuilt,
    Unknown,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Status::Done => "cleared",
            Status::Waiting => "waiting",
            Status::Stalled => "stalled",
            Status::Failed => "failed",
            Status::Skipped => "skipped",
            Status::NotBuilt => "not built",
            Status::Unknown => "unknown",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Prov {
    Observed,
    Inferred,
    Checked,
}

impl Prov {
    pub fn label(self) -> &'static str {
        match self {
            Prov::Observed => "observed",
            Prov::Inferred => "inferred",
            Prov::Checked => "checked",
        }
    }

    pub fn tip(self) -> &'static str {
        match self {
            Prov::Observed => "Observed — an audit entry was actually written",
            Prov::Inferred => "Inferred — deduced from a catalog field; no audit event exists",
            Prov::Checked => "Checked — the platform was queried just now",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tone {
    Info,
    Ok,
    Warn,
    Bad,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BoxTone {
    Here,
    Present,
    Ghost,
    Na,
    Bad,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EdgeTone {
    Done,
    Inferred,
    Waiting,
    Stalled,
    Na,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LogKind {
    Audit,
    Info,
    Warn,
    Error,
    None,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub enum StepId {
    Received,
    Validated,
    Admitted,
    Reference,
    Promoted,
    Dispatched,
    Extracted,
    Curated,
}

/// Log groups. Steps map onto them so selecting a step opens its logs.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, PartialOrd, Ord)]
pub enum GroupId {
    Admitted,
    Reference,
    Promotion,
    Extraction,
    Lifecycle,
    Other,
}

impl StepId {
    pub fn group(self) -> GroupId {
        match self {
            StepId::Received | StepId::Validated | StepId::Admitted => GroupId::Admitted,
            StepId::Reference => GroupId::Reference,
            StepId::Promoted | StepId::Dispatched | StepId::Curated => GroupId::Promotion,
            StepId::Extracted => GroupId::Extraction,
        }
    }
}

impl GroupId {
    /// The step a log group stands for, so opening one selects the other.
    pub fn step(self) -> Option<StepId> {
        match self {
            GroupId::Admitted => Some(StepId::Admitted),
            GroupId::Reference => Some(StepId::Reference),
            GroupId::Promotion => Some(StepId::Promoted),
            GroupId::Extraction => Some(StepId::Extracted),
            GroupId::Lifecycle | GroupId::Other => None,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Fact {
    pub label: String,
    pub value: String,
}

fn fact(label: &str, value: impl Into<String>) -> Fact {
    Fact { label: label.into(), value: value.into() }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Step {
    pub id: StepId,
    pub label: &'static str,
    pub status: Status,
    pub prov: Option<Prov>,
    pub at: String,
    pub detail: String,
    pub why: String,
    pub meta: Vec<Fact>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct MapBox {
    pub tone: BoxTone,
    pub title: &'static str,
    pub tag: String,
    pub sub: String,
    pub meta: String,
    pub here: bool,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Edge {
    pub tone: EdgeTone,
    pub label: &'static str,
    pub prov: Option<Prov>,
    pub note: String,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Branch {
    pub zone: &'static str,
    /// The document actually went this way.
    pub taken: bool,
    pub built: bool,
    pub note: String,
}

#[derive(Clone, PartialEq, Debug)]
pub struct TransitMap {
    pub source: String,
    pub e1: Edge,
    pub landing: MapBox,
    pub e2: Edge,
    pub raw: MapBox,
    pub e3: Edge,
    pub end: MapBox,
    pub rejected: Branch,
    pub quarantine: Branch,
    pub curated: Branch,
}

#[derive(Clone, PartialEq, Debug)]
pub struct ZoneRow {
    pub zone: String,
    pub present: bool,
    pub built: bool,
    pub tag: String,
    pub path: String,
    pub note: String,
    pub size: String,
}

#[derive(Clone, PartialEq, Debug)]
pub struct MetaRow {
    pub label: String,
    pub declared: String,
    pub resolved: String,
    pub flag: bool,
    pub pending: bool,
}

#[derive(Clone, PartialEq, Debug)]
pub struct TreeRow {
    pub rail: String,
    pub depth: u32,
    pub container: bool,
    pub reached: bool,
    pub parent: String,
    pub name: String,
    pub note: String,
    pub tag: String,
    pub size: String,
    pub document_id: String,
    /// Refused by the archive policy: never written, and the walk ends here.
    pub rejected: bool,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Tree {
    pub banner: String,
    pub note: String,
    pub tone: Tone,
    pub rows: Vec<TreeRow>,
    pub footer: String,
}

#[derive(Clone, PartialEq, Debug)]
pub struct LogEntry {
    pub at: String,
    pub kind: LogKind,
    pub text: String,
}

#[derive(Clone, PartialEq, Debug)]
pub struct LogGroup {
    pub id: GroupId,
    pub label: &'static str,
    pub as_of: String,
    pub lag: String,
    /// No lag to report: nothing ran, or nothing was recorded.
    pub quiet: bool,
    pub entries: Vec<LogEntry>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Gap {
    pub short: String,
    pub why: String,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Header {
    pub key: String,
    /// `file · source · rev · size`, under the document key.
    pub sub: String,
    pub state: String,
    pub tone: Status,
    pub matched_on: String,
    pub facts: Vec<Fact>,
    pub as_of: String,
    pub lifecycle: String,
    pub life: Tone,
}

#[derive(Clone, PartialEq, Debug)]
pub struct Candidate {
    pub key: String,
    pub document_id: String,
    /// Unique per arrival: a document that arrived twice shares its documentId across rows.
    pub file_guid: String,
    pub revision: String,
    pub state: String,
    pub registered: String,
}

/// The status card beside the document card: its state, in a sentence.
#[derive(Clone, PartialEq, Debug)]
pub struct Hero {
    pub icon: &'static str,
    pub kicker: String,
    pub title: String,
    pub body: String,
    pub tone: Status,
}

/// "Right now": the one figure that matters for the state the document is in.
#[derive(Clone, PartialEq, Debug)]
pub struct Live {
    pub state: String,
    pub big: String,
    pub big_note: String,
    pub note: String,
    pub tone: Status,
}

#[derive(Clone, PartialEq, Debug)]
pub struct TraceView {
    pub header: Header,
    pub steps: Vec<Step>,
    pub map: TransitMap,
    pub zone_at: Option<&'static str>,
    pub zones: Vec<ZoneRow>,
    pub meta: Vec<MetaRow>,
    pub meta_note: String,
    pub meta_tone: Tone,
    pub tree: Option<Tree>,
    pub log_note: String,
    pub logs_available: bool,
    pub groups: Vec<LogGroup>,
    pub gaps: Vec<Gap>,
    pub candidates: Vec<Candidate>,
    /// The step opened when the trace first loads.
    pub focus: Option<StepId>,
    pub hero: Hero,
    pub live: Live,
}

pub const ZONES: [&str; 5] = ["landing", "raw", "quarantine", "rejected", "curated"];

struct Ctx<'a> {
    t: &'a Trace,
    root: &'a Doc,
    now: DateTime<Utc>,
    checked: DateTime<Utc>,
}

impl<'a> Ctx<'a> {
    fn last_event(&self, name: &str) -> Option<&'a AuditRow> {
        self.t
            .audit
            .iter()
            .filter(|a| a.event_type == name && a.document_id == self.root.document_id)
            .last()
    }

    fn event_count(&self, name: &str) -> usize {
        self.t
            .audit
            .iter()
            .filter(|a| a.event_type == name && a.document_id == self.root.document_id)
            .count()
    }

    fn blob(&self, zone: &str) -> Option<&'a BlobCheck> {
        self.t.blobs.iter().find(|b| b.zone == zone)
    }

    fn present(&self, zone: &str) -> bool {
        self.blob(zone).and_then(|b| b.exists) == Some(true)
    }

    fn state(&self) -> &str {
        &self.root.state
    }

    fn pending(&self) -> bool {
        self.state() == "PendingMetadata"
    }

    fn quarantine_reason(&self) -> Option<String> {
        self.last_event("UploadQuarantined")
            .and_then(|a| detail_str(&a.detail, "reason"))
            .or_else(|| self.root.quarantine_reason.clone())
    }

    /// Quarantined because metadata never resolved: an expiry, not a refusal.
    fn expired_pending(&self) -> bool {
        self.quarantine_reason().as_deref() == Some(METADATA_UNRESOLVED)
    }

    fn has_prerequisite(&self) -> bool {
        self.t.source.as_ref().is_some_and(|s| s.reference_set.is_some())
            || self.root.pending_since.is_some()
    }

    fn pending_max_days(&self) -> Option<i64> {
        self.t.source.as_ref().and_then(|s| s.pending_max_days)
    }

    fn landing_deletes_at(&self) -> Option<DateTime<Utc>> {
        let arrived = self
            .blob("landing")
            .and_then(|b| f::parse_opt(&b.last_modified))
            .or_else(|| f::parse_opt(&self.root.uploaded_at))?;
        Some(arrived + Duration::days(self.t.env.landing_retention_days.max(0)))
    }

    /// The curated address the catalog records, if any (it writes an empty one on every document).
    fn curated_address(&self) -> Option<&'a str> {
        self.root.curated.as_ref().map(|c| c.path.as_str()).filter(|p| !p.is_empty())
    }

    /// (members placed in curated, members with bytes in raw)
    fn members_curated(&self) -> (usize, usize) {
        let has = |loc: &Option<super::model::Location>| loc.as_ref().is_some_and(|l| !l.path.is_empty());
        let curated = self.t.family.iter().filter(|m| has(&m.curated)).count();
        let files = self.t.family.iter().filter(|m| has(&m.raw)).count();
        (curated, files)
    }

    fn written_members(&self) -> Vec<&'a Doc> {
        self.t
            .family
            .iter()
            .filter(|m| m.raw.as_ref().is_some_and(|r| !r.path.is_empty()))
            .collect()
    }

    fn expected_files(&self) -> Option<usize> {
        let e = self.t.expected.as_ref()?;
        if e.entries.is_empty() && e.error.is_some() {
            return None;
        }
        Some(e.entries.iter().filter(|x| !x.container && x.rejected.is_none()).count())
    }

    fn last_progress(&self) -> Option<DateTime<Utc>> {
        self.t
            .family
            .iter()
            .filter_map(|m| f::parse_opt(&m.written_at))
            .max()
            .or_else(|| f::parse_opt(&self.root.written_at))
    }

    fn extraction_state(&self) -> Option<&str> {
        self.root.archive_extraction_state.as_deref()
    }

    fn extraction_stalled(&self) -> bool {
        if self.extraction_state() != Some("Extracting") {
            return false;
        }
        if f::parse_opt(&self.root.archive_extraction_lease_expires_at).is_some_and(|l| l > self.now) {
            return false;
        }
        self.last_progress()
            .map_or(true, |p| self.now - p > Duration::minutes(STALL_AFTER_MINUTES))
    }

    fn size(&self) -> Option<u64> {
        self.root.actual_size_bytes.or(self.root.size_bytes)
    }
}

fn detail_str(detail: &Value, key: &str) -> Option<String> {
    match detail.get(key)? {
        Value::Null => None,
        Value::String(s) => Some(s.clone()),
        other => Some(other.to_string()),
    }
}

fn at_of(value: &Option<String>) -> String {
    f::parse_opt(value).map(f::clock).unwrap_or_else(|| DASH.into())
}

fn event_at(a: &AuditRow) -> String {
    f::parse(&a.occurred_at).map(f::clock).unwrap_or_else(|| DASH.into())
}

pub fn build(t: &Trace, now: DateTime<Utc>) -> Option<TraceView> {
    let root = t.root.as_ref()?;
    let checked = f::parse(&t.checked_at).unwrap_or(now);
    let cx = Ctx { t, root, now, checked };

    let steps = steps(&cx);
    let map = transit_map(&cx, &steps);
    let zone_at = zone_at(&cx);
    let (meta, meta_note, meta_tone) = meta(&cx);
    let tree = tree(&cx);
    let (log_note, groups) = log_groups(&cx);
    let gaps = gaps(&cx, &steps, tree.as_ref());
    let focus = focus(&steps, t.is_archive);
    let header = header(&cx, zone_at);
    let hero = hero(&cx, &steps, focus, &header);
    let live = live(&cx, &header);

    Some(TraceView {
        focus,
        hero,
        live,
        header,
        steps,
        map,
        zone_at,
        zones: zones(&cx, zone_at),
        meta,
        meta_note,
        meta_tone,
        tree,
        log_note,
        logs_available: t.logs.available,
        groups,
        gaps,
        candidates: t
            .candidates
            .iter()
            .map(|c| Candidate {
                key: c.display_key().to_string(),
                document_id: c.document_id.clone(),
                file_guid: c.file_guid.clone(),
                revision: c.revision_id.clone(),
                state: c.state.clone(),
                registered: f::parse_opt(&c.registered_at).map(f::day_minute).unwrap_or_else(|| DASH.into()),
            })
            .collect(),
    })
}

fn focus(steps: &[Step], is_archive: bool) -> Option<StepId> {
    steps
        .iter()
        .find(|s| matches!(s.status, Status::Stalled | Status::Failed))
        .or_else(|| steps.iter().find(|s| s.status == Status::Waiting))
        .map(|s| s.id)
        .or(Some(if is_archive { StepId::Extracted } else { StepId::Reference }))
}

fn capitalise(s: &str) -> String {
    let mut chars = s.chars();
    chars.next().map(|c| c.to_uppercase().collect::<String>() + chars.as_str()).unwrap_or_default()
}

fn hero(cx: &Ctx, steps: &[Step], focus: Option<StepId>, header: &Header) -> Hero {
    let (icon, kicker) = match header.tone {
        Status::Waiting if cx.pending() => ("hourglass-high", "waiting, not failing"),
        Status::Waiting => ("clock", "in progress"),
        Status::Stalled => ("warning", "stalled"),
        Status::Failed => ("warning-circle", "stopped"),
        _ if steps.iter().any(|s| s.status == Status::Done && s.prov == Some(Prov::Inferred)) => ("seal-check", "complete · some hops inferred"),
        _ => ("seal-check", "every step recorded"),
    };
    if cx.pending() {
        let n = cx.root.pending_attempts.unwrap_or(0);
        return Hero {
            icon,
            kicker: kicker.into(),
            title: "Waiting on the next reference refresh".into(),
            body: format!(
                "The document arrived intact. Its routing metadata has not reached the platform yet — {n} lookup{} so far, and no caller action is needed.",
                if n == 1 { "" } else { "s" }
            ),
            tone: header.tone,
        };
    }
    let (title, body) = match focus.map(|id| find(steps, id)) {
        Some(s) => (capitalise(&s.detail), s.why),
        None => (header.state.clone(), header.lifecycle.clone()),
    };
    Hero { icon, kicker: kicker.into(), title, body, tone: header.tone }
}

fn live(cx: &Ctx, header: &Header) -> Live {
    let r = cx.root;
    let since = |t: Option<DateTime<Utc>>| t.map(|t| f::span(cx.now - t));
    let (big, big_note) = if cx.pending() {
        (since(f::parse_opt(&r.pending_since)), "waiting")
    } else if cx.extraction_state() == Some("Extracting") && cx.extraction_stalled() {
        (since(cx.last_progress()), "since last progress")
    } else if cx.extraction_state() == Some("Extracting") {
        let total = cx.expected_files().map(|t| format!(" / {t}")).unwrap_or_default();
        (Some(format!("{}{total}", cx.written_members().len())), "members written")
    } else {
        let start = f::parse_opt(&r.uploaded_at).or_else(|| f::parse_opt(&r.registered_at));
        let end = [&r.archive_extracted_at, &r.dispatched_at, &r.promoted_at].into_iter().filter_map(f::parse_opt).max();
        match (header.tone, start, end) {
            (Status::Done, Some(s), Some(e)) => (Some(f::span(e - s)), "end to end"),
            _ => (since(f::parse_opt(&r.written_at)), "since the last change"),
        }
    };
    Live {
        state: match cx.extraction_state() {
            Some(x) => format!("{} · {x}", r.state),
            None => r.state.clone(),
        },
        big: big.unwrap_or_else(|| DASH.into()),
        big_note: big_note.into(),
        note: header.lifecycle.clone(),
        tone: header.tone,
    }
}

fn header(cx: &Ctx, zone_at: Option<&str>) -> Header {
    let r = cx.root;
    let stalled = cx.extraction_stalled();
    let (state, tone) = match cx.extraction_state() {
        Some("Extracting") if stalled => ("Extracting".to_string(), Status::Stalled),
        Some("Extracting") => ("Extracting".to_string(), Status::Waiting),
        Some("Extracted") => ("Extracted".to_string(), Status::Done),
        Some(s @ "ExtractionFailed") => (s.to_string(), Status::Failed),
        _ => (
            r.state.clone(),
            match r.state.as_str() {
                "Registered" | "AwaitingUpload" | "Uploaded" | "PendingMetadata" => Status::Waiting,
                "Quarantined" | "Rejected" | "Failed" | "Expired" => Status::Failed,
                "" => Status::Unknown,
                _ => Status::Done,
            },
        ),
    };
    let received = f::parse_opt(&r.uploaded_at).or_else(|| f::parse_opt(&r.registered_at));
    let (lifecycle, life) = lifecycle(cx, zone_at, stalled);
    Header {
        key: r.display_key().to_string(),
        sub: [
            Some(r.file_name.clone()).filter(|n| !n.is_empty()),
            Some(r.source_system.clone()).filter(|s| !s.is_empty()),
            Some(format!("rev {}", f::or_dash(Some(r.revision_id.clone())))),
            cx.size().map(f::bytes),
        ]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>()
        .join(" · "),
        state,
        tone,
        matched_on: cx.t.matched_on.clone().unwrap_or_else(|| "query".into()),
        facts: vec![
            fact("DOCUMENTID", r.document_id.clone()),
            fact("REVISION", f::or_dash(Some(r.revision_id.clone()))),
            fact("SOURCE", f::or_dash(Some(r.source_system.clone()))),
            fact("RECEIVED", received.map(f::full).unwrap_or_else(|| DASH.into())),
            fact("SIZE", cx.size().map(f::bytes).unwrap_or_else(|| DASH.into())),
        ],
        as_of: f::full(cx.checked),
        lifecycle,
        life,
    }
}

fn lifecycle(cx: &Ctx, zone_at: Option<&str>, stalled: bool) -> (String, Tone) {
    let landing_tail = || match (cx.present("landing"), cx.landing_deletes_at()) {
        (true, Some(at)) => format!(
            " The landing copy is still present and will be deleted on {}.",
            f::date(at)
        ),
        _ => String::new(),
    };
    let written = cx.written_members().len();
    let total = cx.expected_files().map(|n| n.to_string()).unwrap_or_else(|| "an unknown number of".into());
    match (cx.state(), cx.extraction_state()) {
        ("Quarantined", _) if cx.expired_pending() => (
            format!(
                "In quarantine. Its metadata never resolved{}, so the sweeper expired it. Quarantine is terminal.",
                cx.pending_max_days().map(|d| format!(" within {d} days")).unwrap_or_default()
            ),
            Tone::Bad,
        ),
        ("Quarantined", _) => (
            format!(
                "In quarantine — refused by a rule{}. Quarantine is terminal, not a later stage.",
                cx.quarantine_reason().map(|r| format!(": {r}")).unwrap_or_default()
            ),
            Tone::Bad,
        ),
        ("Rejected", _) => (
            format!(
                "Rejected at the validation gate{}. Rejected is terminal.",
                cx.last_event("UploadRejected")
                    .and_then(|a| detail_str(&a.detail, "reason"))
                    .map(|r| format!(": {r}"))
                    .unwrap_or_default()
            ),
            Tone::Bad,
        ),
        ("Expired", _) => (
            "Expired: it was registered, but its bytes never arrived before the upload session ran out.".into(),
            Tone::Bad,
        ),
        ("Failed", _) => ("Failed. The catalog records no further progress for this document.".into(), Tone::Bad),
        ("Registered" | "AwaitingUpload", _) => (
            "Registered — its bytes have not arrived in landing yet.".into(),
            Tone::Warn,
        ),
        ("Uploaded", _) => ("In landing; the gate has not decided on it yet.".into(), Tone::Warn),
        ("PendingMetadata", _) => {
            let deletes = cx.landing_deletes_at();
            let expires = f::parse_opt(&cx.root.pending_since).zip(cx.pending_max_days()).map(|(s, d)| s + Duration::days(d));
            let mut text = match (zone_at, deletes) {
                (Some("landing"), Some(at)) => format!(
                    "Still in landing. Landing is transient — the lifecycle rule deletes this object on {}, {}.",
                    f::date(at),
                    f::until(cx.now, at)
                ),
                (Some("landing"), None) => "Still in landing. Landing is transient.".into(),
                _ => "Parked on metadata, but no landing object was found at the recorded address.".into(),
            };
            if let Some(at) = expires {
                text.push_str(&format!(
                    " Unresolved, the sweeper quarantines it on {} ({}).",
                    f::date(at),
                    f::until(cx.now, at)
                ));
            } else {
                text.push_str(" No pending budget applies to this source, so only the landing lifecycle ends the wait.");
            }
            let tone = if zone_at == Some("landing") { Tone::Warn } else { Tone::Bad };
            (text, tone)
        }
        (_, Some("Extracting")) if stalled => (
            format!(
                "In raw, stuck on Extracting. {written} of {total} members written{} and nothing has progressed since; no failure was reported.",
                cx.last_progress().map(|p| format!(", the last {} ago", f::span(cx.now - p))).unwrap_or_default()
            ),
            Tone::Bad,
        ),
        (_, Some("Extracting")) => (
            format!("In raw and extracting — {written} of {total} members written so far."),
            Tone::Warn,
        ),
        (_, Some("ExtractionFailed")) => (
            format!(
                "In raw, but extraction failed after {written} members. The root is quarantined for consumption; members written before the breach are inert."
            ),
            Tone::Bad,
        ),
        (_, Some("Extracted")) => (
            format!("In raw, with {written} members written back into raw beside it.{}", landing_tail()),
            Tone::Ok,
        ),
        _ if zone_at == Some("raw") => (
            format!("In raw, the source of truth for content.{}", landing_tail()),
            Tone::Ok,
        ),
        _ => (
            format!("State {}, but no object was found in any zone it should occupy.", cx.state()),
            Tone::Bad,
        ),
    }
}

fn mime_agrees(r: &Doc) -> Option<bool> {
    if r.sniffed_mime.is_empty() || r.mime_type.is_empty() {
        return None;
    }
    Some(r.sniffed_mime.eq_ignore_ascii_case(&r.mime_type))
}

fn steps(cx: &Ctx) -> Vec<Step> {
    vec![
        step_received(cx),
        step_validated(cx),
        step_admitted(cx),
        step_reference(cx),
        step_promoted(cx),
        step_dispatched(cx),
        step_extracted(cx),
        step_curated(cx),
    ]
}

fn step(id: StepId, label: &'static str, status: Status, prov: Option<Prov>) -> Step {
    Step { id, label, status, prov, at: DASH.into(), detail: String::new(), why: String::new(), meta: vec![] }
}

fn step_received(cx: &Ctx) -> Step {
    let r = cx.root;
    let mut s = step(StepId::Received, "Received from source", Status::Unknown, None);
    let size = cx.size().map(f::bytes).unwrap_or_else(|| "size unknown".into());
    if let Some(a) = cx.last_event("DocumentUploadedSmall") {
        s.status = Status::Done;
        s.prov = Some(Prov::Observed);
        s.at = event_at(a);
        s.detail = format!("DocumentUploadedSmall · {size} · {}", r.source_system);
        s.why = "The upload event was written to the audit trail, so this hop is recorded rather than deduced.".into();
    } else if r.uploaded_at.is_some() {
        s.status = Status::Done;
        s.prov = Some(Prov::Inferred);
        s.at = at_of(&r.uploaded_at);
        s.detail = format!("deduced from catalog.uploadedAt · {size} · {}", r.source_system);
        s.why = "No upload event exists for this document — a session upload writes the blob directly and is only seen when the gate reconciles it. The time comes from the catalog.".into();
        s.meta.push(fact("INFERRED FROM", "catalog.uploadedAt"));
    } else if r.state == "Expired" {
        s.status = Status::Failed;
        s.prov = Some(if cx.last_event("DocumentExpired").is_some() { Prov::Observed } else { Prov::Checked });
        s.at = cx.last_event("DocumentExpired").map(event_at).unwrap_or_else(|| DASH.into());
        s.detail = "the upload session expired before any bytes arrived".into();
        s.why = "Registration succeeded, but nothing was ever written to landing for it.".into();
    } else if let Some(a) = cx.last_event("DocumentRegistered") {
        s.status = Status::Waiting;
        s.prov = Some(Prov::Observed);
        s.at = event_at(a);
        s.detail = "registered — bytes not uploaded yet".into();
        s.why = "The caller pre-registered the document; the upload itself has not reached landing.".into();
    } else {
        s.detail = "no upload event and no catalog upload time".into();
        s.why = "Nothing on record says when, or whether, the bytes arrived.".into();
    }
    s
}

fn step_validated(cx: &Ctx) -> Step {
    let r = cx.root;
    let mut s = step(StepId::Validated, "Validation gate", Status::Unknown, Some(Prov::Checked));
    if let Some(a) = cx.last_event("UploadRejected") {
        s.status = Status::Failed;
        s.prov = Some(Prov::Observed);
        s.at = event_at(a);
        s.detail = format!("rejected — {}", detail_str(&a.detail, "reason").unwrap_or_else(|| "no reason recorded".into()));
        s.why = "The gate refused the bytes and moved them to rejected. Rejected is terminal.".into();
        return s;
    }
    if let Some(a) = cx.last_event("UploadQuarantined").filter(|_| !cx.expired_pending()) {
        s.status = Status::Failed;
        s.prov = Some(Prov::Observed);
        s.at = event_at(a);
        s.detail = format!("quarantined — {}", detail_str(&a.detail, "reason").unwrap_or_else(|| "no reason recorded".into()));
        s.why = "A rule refused the document and the gate moved it to quarantine.".into();
        return s;
    }
    if matches!(r.state.as_str(), "Registered" | "AwaitingUpload" | "Expired") {
        s.status = if r.state == "Expired" { Status::Skipped } else { Status::Waiting };
        s.prov = None;
        s.detail = "nothing uploaded to validate".into();
        s.why = "The gate runs when bytes land. None have.".into();
        return s;
    }
    if r.state == "Uploaded" && r.sniffed_mime.is_empty() {
        s.status = Status::Waiting;
        s.detail = "in landing, not yet sniffed".into();
        s.why = "The gate has not recorded a sniffed type yet.".into();
        return s;
    }
    s.status = Status::Done;
    s.at = at_of(&r.uploaded_at);
    match mime_agrees(r) {
        Some(true) => {
            s.detail = "passed — declared MIME matches sniffed MIME".into();
            s.why = format!(
                "Both the declared and the sniffed type are {}. A disagreement here is the class of bug that shipped once.",
                r.sniffed_mime
            );
        }
        Some(false) => {
            s.detail = "passed — but declared and sniffed MIME disagree".into();
            s.why = "The caller's declared type and the type sniffed from the bytes differ, and the gate admitted it anyway. Routing reads the sniffed type, so this is worth reading beside everything that follows.".into();
            s.meta = vec![fact("DECLARED", r.mime_type.clone()), fact("SNIFFED", r.sniffed_mime.clone())];
        }
        None => {
            s.detail = "passed — no declared type to compare".into();
            s.why = format!(
                "Sniffed as {}; the caller declared no MIME type, so there was nothing to disagree with.",
                f::or_dash(Some(r.sniffed_mime.clone()))
            );
        }
    }
    s
}

fn step_admitted(cx: &Ctx) -> Step {
    let r = cx.root;
    let mut s = step(StepId::Admitted, "Admitted to landing", Status::Unknown, None);
    let landing = cx.blob("landing");
    let path = r.landing.as_ref().map(|l| l.path.clone()).filter(|p| !p.is_empty());
    if path.is_none() && r.uploaded_at.is_none() {
        s.status = if matches!(r.state.as_str(), "Registered" | "AwaitingUpload") { Status::Waiting } else { Status::Unknown };
        s.detail = "no landing address recorded".into();
        s.why = "The catalog names no landing object for this document.".into();
        return s;
    }
    s.status = Status::Done;
    s.prov = Some(if cx.last_event("DocumentUploadedSmall").is_some() { Prov::Observed } else { Prov::Checked });
    s.at = at_of(&r.uploaded_at);
    s.detail = match landing.and_then(|b| b.exists) {
        Some(true) => "zone landing — as the caller wrote it".into(),
        Some(false) if r.promoted_at.is_some() => "zone landing — since removed after promotion".into(),
        Some(false) => "zone landing — the object is no longer there".into(),
        None => "zone landing — presence could not be checked".into(),
    };
    s.why = format!(
        "landing holds the caller's bytes untouched. It is transient: objects are deleted {} days after arrival.",
        cx.t.env.landing_retention_days
    );
    if landing.and_then(|b| b.exists) == Some(false) && r.promoted_at.is_none() && !matches!(r.state.as_str(), "Quarantined" | "Rejected") {
        s.status = Status::Failed;
        s.why.push_str(" The catalog was never promoted, yet the landing object is gone — the lifecycle rule deletes with no catalog write.");
    }
    s
}

fn step_reference(cx: &Ctx) -> Step {
    let r = cx.root;
    let mut s = step(StepId::Reference, "Reference resolved", Status::Unknown, Some(Prov::Checked));
    let generation = cx.t.reference.as_ref().and_then(|p| p.generation);
    let attempts = r.pending_attempts.unwrap_or(0);
    if !cx.has_prerequisite() {
        s.status = Status::Skipped;
        s.detail = format!("no metadata prerequisite for {}", f::or_dash(Some(r.source_system.clone())));
        s.why = "This source declares no reference set, so nothing is looked up and nothing waits.".into();
        return s;
    }
    if cx.pending() {
        s.status = Status::Waiting;
        s.at = cx.last_event("DocumentPendingMetadata").map(event_at).unwrap_or_else(|| at_of(&r.pending_since));
        s.detail = format!("{attempts} lookup{} since arrival, none resolved", if attempts == 1 { "" } else { "s" });
        s.why = "The document arrived intact. The business metadata its routing depends on has not yet reached the platform's reference data. It is re-checked after every reference refresh and needs no caller action.".into();
        if let Some(key) = &r.pending_key {
            s.meta.push(fact("PENDINGKEY", key.clone()));
        }
        s.meta.push(fact("ATTEMPTS", attempts.to_string()));
        if let Some(g) = generation {
            s.meta.push(fact("GENERATION", g.to_string()));
        }
        if let Some(reason) = &r.pending_reason {
            s.meta.push(fact("REASON", reason.clone()));
        }
        return s;
    }
    if cx.expired_pending() {
        s.status = Status::Failed;
        s.prov = Some(Prov::Observed);
        s.at = cx.last_event("UploadQuarantined").map(event_at).unwrap_or_else(|| DASH.into());
        s.detail = match cx.pending_max_days() {
            Some(d) => format!("never resolved · expired after {d} days"),
            None => "never resolved · expired".into(),
        };
        s.why = "The reference lookup kept failing until the pending budget ran out, and the sweeper moved the document to quarantine.".into();
        s.meta.push(fact("ATTEMPTS", attempts.to_string()));
        return s;
    }
    let project = r.attribute("reference.sbmProjectNumber");
    if let Some(a) = cx.last_event("PendingMetadataResolved") {
        s.status = Status::Done;
        s.prov = Some(Prov::Observed);
        s.at = event_at(a);
        s.detail = format!("resolved after {attempts} attempts{}", project.as_ref().map(|p| format!(" · {p}")).unwrap_or_default());
        s.why = "The document parked on metadata, then a later reference generation held its row.".into();
    } else if r.promoted_at.is_some() || r.reference_data_as_of.is_some() {
        s.status = Status::Done;
        s.at = at_of(&r.promoted_at);
        s.detail = format!(
            "resolved{}{}",
            generation.map(|g| format!(" · generation {g}")).unwrap_or_default(),
            project.as_ref().map(|p| format!(" · {p}")).unwrap_or_default()
        );
        s.why = "The register held a row for this document, so routing attributes were available when the gate needed them.".into();
    } else if matches!(r.state.as_str(), "Quarantined" | "Rejected") {
        s.status = Status::Skipped;
        s.prov = None;
        s.detail = "not reached".into();
        s.why = "The document stopped at the gate, before any lookup.".into();
        return s;
    } else {
        s.status = Status::Waiting;
        s.prov = None;
        s.detail = "not looked up yet".into();
        s.why = "The lookup runs when the gate admits the bytes.".into();
        return s;
    }
    if let Some(p) = project {
        s.meta.push(fact("SBMPROJECTNUMBER", p));
    }
    if let Some(g) = generation {
        s.meta.push(fact("GENERATION", g.to_string()));
    }
    s
}

fn step_promoted(cx: &Ctx) -> Step {
    let r = cx.root;
    let mut s = step(StepId::Promoted, "Promoted landing → raw", Status::Waiting, Some(Prov::Inferred));
    s.meta.push(fact("INFERRED FROM", "catalog.promotedAt"));
    if let Some(a) = cx.last_event("DocumentPromotionStalled") {
        s.status = Status::Stalled;
        s.prov = Some(Prov::Observed);
        s.at = event_at(a);
        s.detail = "the sweeper found the promotion stalled".into();
        s.why = "promotedAt was set but the move did not complete; the sweeper recorded the stall.".into();
        return s;
    }
    if r.promoted_at.is_some() {
        s.status = Status::Done;
        s.at = at_of(&r.promoted_at);
        s.detail = "deduced from catalog.promotedAt — no audit event exists".into();
        s.why = "Nothing records the promotion itself. The timestamp comes from the catalog, so this hop is shown as inferred even though it plainly happened.".into();
        return s;
    }
    if matches!(r.state.as_str(), "Quarantined" | "Rejected" | "Expired" | "Failed") {
        s.status = Status::Skipped;
        s.prov = None;
        s.meta.clear();
        s.detail = "not taken".into();
        s.why = "The document left the main line before promotion.".into();
        return s;
    }
    s.detail = if cx.pending() { "waiting on reference".into() } else { "nothing to infer yet".into() };
    s.why = "Promotion waits on the gate and the reference lookup. This hop never writes an audit event, so even once it happens it can only be deduced from catalog.promotedAt.".into();
    s
}

fn step_dispatched(cx: &Ctx) -> Step {
    let r = cx.root;
    let mut s = step(StepId::Dispatched, "Dispatched for evaluation", Status::Waiting, None);
    if let Some(a) = cx.last_event("DocumentDispatchFailed") {
        s.status = Status::Failed;
        s.prov = Some(Prov::Observed);
        s.at = event_at(a);
        s.detail = "the evaluate notification could not be sent".into();
        s.why = "The document is in raw but its processing-rules notification failed. Nothing downstream will pick it up until it is re-dispatched.".into();
        return s;
    }
    if let Some(a) = cx.last_event("DocumentDispatched") {
        s.status = Status::Done;
        s.prov = Some(Prov::Observed);
        s.at = event_at(a);
        s.detail = format!("DocumentDispatched · {}", detail_str(&a.detail, "queue").unwrap_or_else(|| "evaluate".into()));
        s.why = "The processing-rules notification was sent and recorded.".into();
        return s;
    }
    if r.dispatched_at.is_some() {
        s.status = Status::Done;
        s.prov = Some(Prov::Inferred);
        s.at = at_of(&r.dispatched_at);
        s.detail = "deduced from catalog.dispatchedAt".into();
        s.why = "The catalog records a dispatch time but no audit entry was found for it.".into();
        s.meta.push(fact("INFERRED FROM", "catalog.dispatchedAt"));
        return s;
    }
    if r.promoted_at.is_none() {
        s.status = if matches!(r.state.as_str(), "Quarantined" | "Rejected" | "Expired") { Status::Skipped } else { Status::Waiting };
        s.detail = "waits on promotion".into();
        s.why = "Dispatch follows promotion.".into();
    } else {
        s.detail = "promoted, not dispatched yet".into();
        s.why = "The document is in raw; its evaluate notification has not been recorded.".into();
    }
    s
}

fn step_extracted(cx: &Ctx) -> Step {
    let r = cx.root;
    let mut s = step(StepId::Extracted, "Archive extracted", Status::Skipped, Some(Prov::Checked));
    if !cx.t.is_archive {
        s.detail = "not an archive".into();
        s.why = format!(
            "Sniffed as {}, so no extraction is attempted. Skipped is a different fact from failed.",
            f::or_dash(Some(r.sniffed_mime.clone()))
        );
        return s;
    }
    let written = cx.written_members().len();
    let total = cx.expected_files();
    let deepest = deepest_reached(cx);
    let depth_total = cx.t.expected.as_ref().and_then(|e| e.entries.iter().map(|x| x.depth).max());
    if let Some(a) = cx.last_event("ArchiveExtractionFailed") {
        s.status = Status::Failed;
        s.prov = Some(Prov::Observed);
        s.at = event_at(a);
        s.detail = format!("ArchiveExtractionFailed · {}", detail_str(&a.detail, "reason").unwrap_or_default());
        s.why = "The walk hit a budget or policy breach and quarantined the root. Members written before it stay inert.".into();
        s.meta.push(fact("MEMBERS WRITTEN", detail_str(&a.detail, "membersWritten").unwrap_or_else(|| written.to_string())));
        return s;
    }
    match cx.extraction_state() {
        Some("Extracted") => {
            let observed = cx.last_event("ArchiveExtracted");
            s.status = Status::Done;
            s.prov = Some(if observed.is_some() { Prov::Observed } else { Prov::Inferred });
            s.at = observed.map(event_at).unwrap_or_else(|| at_of(&r.archive_extracted_at));
            let count = r.archive_member_count.unwrap_or(written as u64);
            let depths = deepest.map(|d| d + 1).unwrap_or(1);
            s.detail = format!("ArchiveExtracted · {count} members at {depths} depth{}", if depths == 1 { "" } else { "s" });
            s.why = format!(
                "The walk reached depth {} and wrote every member back into raw. Depth is the thing to check: nested extraction was silently broken until 2026-08-14.",
                deepest.unwrap_or(0)
            );
            s.meta.push(fact("MEMBERS", count.to_string()));
            s.meta.push(fact("DEEPEST", deepest.unwrap_or(0).to_string()));
        }
        Some("Extracting") => {
            let stalled = cx.extraction_stalled();
            s.status = if stalled { Status::Stalled } else { Status::Waiting };
            s.prov = Some(Prov::Inferred);
            let last = cx.last_progress();
            s.at = last.map(f::clock).unwrap_or_else(|| DASH.into());
            let of = total.map(|t| format!("{written} of {t}")).unwrap_or_else(|| format!("{written} of ?"));
            s.detail = if stalled {
                format!("{of} members written, then no progress for {}", last.map(|p| f::span(cx.now - p)).unwrap_or_else(|| "an unknown time".into()))
            } else {
                format!("walking — {of} members written so far")
            };
            s.why = if stalled {
                "The walk wrote members and stopped. No ArchiveExtracted event was written and no failure was reported — the Job replica ended mid-walk. The stall is inferred from the absence of progress, which is the only signal there is.".into()
            } else {
                "The walk is writing members; progress is read from each member's last write.".into()
            };
            s.meta.push(fact("WRITTEN", of));
            s.meta.push(fact(
                "DEEPEST",
                match (deepest, depth_total) {
                    (Some(d), Some(t)) => format!("{d} of {t}"),
                    (Some(d), None) => d.to_string(),
                    (None, Some(t)) => format!("none of {t}"),
                    (None, None) => DASH.into(),
                },
            ));
            if let Some(p) = last {
                s.meta.push(fact("LAST PROGRESS", format!("{} ago", f::span(cx.now - p))));
            }
            if let Some(replica) = replica(cx) {
                s.meta.push(fact("REPLICA", replica));
            }
        }
        Some("ExtractionFailed") => {
            s.status = Status::Failed;
            s.prov = Some(Prov::Checked);
            s.at = at_of(&r.archive_extracted_at);
            s.detail = format!("extraction failed after {written} members");
            s.why = "The catalog marks the root ExtractionFailed, but no failure event was found to say why.".into();
        }
        _ if r.promoted_at.is_none() => {
            s.status = if matches!(r.state.as_str(), "Quarantined" | "Rejected" | "Expired") { Status::Skipped } else { Status::Waiting };
            s.detail = "waits on promotion".into();
            s.why = "An archive is extracted from raw, so the walk cannot start before promotion.".into();
        }
        _ => {
            s.status = Status::Waiting;
            s.detail = "promoted, extraction not started".into();
            s.why = "The root is in raw but the extraction Job has not marked it Extracting yet.".into();
        }
    }
    s
}

fn step_curated(cx: &Ctx) -> Step {
    let r = cx.root;
    let mut s = step(StepId::Curated, "Written to curated", Status::Waiting, Some(Prov::Checked));
    let built = cx.blob("curated").is_some_and(|b| b.container_exists) || cx.t.env.has_container("curated");
    if !built {
        s.status = Status::NotBuilt;
        s.prov = None;
        s.detail = "the zone does not exist".into();
        s.why = "No curated container exists in this storage account. This is not an empty result; the zone is not built here.".into();
        return s;
    }
    if cx.present("curated") {
        s.status = Status::Done;
        s.detail = "published copy present".into();
        s.why = "A consumer-facing copy exists at the curated address the catalog records.".into();
        if let Some(c) = &r.curated {
            s.meta.push(fact("PATH", format!("{}/{}", c.container, c.path)));
        }
        return s;
    }
    // The catalog writes an empty `curated` object on every document; only a path is an address.
    let curated_path = r.curated.as_ref().is_some_and(|c| !c.path.is_empty());
    let placement_keys = r.attribute("reference.sbmProjectNumber").is_some()
        && (r.attribute("reference.uniqueDocNumber").is_some() || !r.business_key.is_empty());
    if cx.t.is_archive {
        let (placed, files) = cx.members_curated();
        s.status = if cx.curated_address().is_some() { Status::Done } else { Status::Skipped };
        s.detail = match cx.curated_address() {
            Some(_) => format!("curated address assigned · {placed} of {files} members in curated"),
            None => "archive roots are not offered".into(),
        };
        s.why = "An archive root gets a curated address so its place in the tree is visible, but never its bytes: it is retained, not offered to a consumer. Its members are placed in curated one by one.".into();
        if let Some(path) = cx.curated_address() {
            s.meta.push(fact("ADDRESS", path.to_string()));
        }
        s.meta.push(fact("MEMBERS IN CURATED", format!("{placed} of {files}")));
    } else if matches!(r.state.as_str(), "Quarantined" | "Rejected" | "Expired") {
        s.status = Status::Skipped;
        s.detail = "not taken".into();
        s.why = "Only documents that reach raw are curated.".into();
    } else if curated_path {
        s.status = Status::Failed;
        s.detail = "the catalog names a curated address with no object".into();
        s.why = "catalog.curated.path is set, but nothing exists at that path.".into();
    } else if r.dispatched_at.is_some() && !READABLE_INDICATORS.contains(&r.file_indicator.as_str()) {
        s.status = Status::Stalled;
        s.prov = Some(Prov::Inferred);
        s.detail = format!("parked — file indicator {:?} cannot be read", r.file_indicator);
        s.why = "Curated placement reads the file indicator to choose the role and the Official/Native half, and accepts only RO, RW, RP or RA exactly. Any other value parks the document (CuratedSkipped reason=NO_ROLE); it stays intact in raw until the indicator is corrected and the document is re-evaluated.".into();
        s.meta.push(fact("FILE INDICATOR", f::or_dash(Some(r.file_indicator.clone()))));
    } else if r.dispatched_at.is_some() && !placement_keys {
        s.status = Status::Stalled;
        s.prov = Some(Prov::Inferred);
        s.detail = "parked — no project or document number".into();
        s.why = "The curated path is built from the register's project and document numbers. Without both, placement parks the document (CuratedSkipped reason=NO_PLACEMENT_KEYS).".into();
    } else {
        s.detail = "not written yet".into();
        s.why = "Curation follows promotion and the processing rules.".into();
    }
    s
}

fn replica(cx: &Ctx) -> Option<String> {
    cx.t.logs.entries.iter().rev().find_map(|e| e.replica.clone())
}

fn containers_in_family(cx: &Ctx) -> Vec<String> {
    cx.t
        .family
        .iter()
        .filter(|m| m.raw.as_ref().map_or(true, |r| r.path.is_empty()))
        .filter_map(|m| m.path_from_root.clone())
        .collect()
}

fn depth_of(path: &str, containers: &[String]) -> u32 {
    containers.iter().filter(|c| path.starts_with(&format!("{c}/"))).count() as u32
}

fn deepest_reached(cx: &Ctx) -> Option<u32> {
    let containers = containers_in_family(cx);
    cx.written_members()
        .iter()
        .filter_map(|m| m.path_from_root.as_deref())
        .map(|p| depth_of(p, &containers))
        .max()
}

fn edge(step: &Step, label: &'static str, note: String) -> Edge {
    let tone = match (step.status, step.prov) {
        (Status::Done, Some(Prov::Inferred)) => EdgeTone::Inferred,
        (Status::Done, _) => EdgeTone::Done,
        (Status::Waiting, _) => EdgeTone::Waiting,
        (Status::Stalled | Status::Failed, _) => EdgeTone::Stalled,
        _ => EdgeTone::Na,
    };
    Edge { tone, label, prov: step.prov, note }
}

fn find(steps: &[Step], id: StepId) -> Step {
    steps.iter().find(|s| s.id == id).cloned().unwrap_or_else(|| step(id, "", Status::Unknown, None))
}

fn transit_map(cx: &Ctx, steps: &[Step]) -> TransitMap {
    let received = find(steps, StepId::Received);
    let promoted = find(steps, StepId::Promoted);
    let extracted = find(steps, StepId::Extracted);
    let zone_at = zone_at(cx);
    let retention = cx.t.env.landing_retention_days;

    let size_of = |zone: &str| cx.blob(zone).and_then(|b| b.size).map(f::bytes);
    let zone_box = |zone: &'static str, sub_present: String, sub_absent: String, extra: String| {
        let check = cx.blob(zone);
        match check.and_then(|b| b.exists) {
            Some(true) => MapBox {
                tone: if zone_at == Some(zone) { BoxTone::Here } else { BoxTone::Present },
                title: zone,
                tag: "object exists".into(),
                sub: sub_present,
                meta: [size_of(zone), Some(extra).filter(|e| !e.is_empty())].into_iter().flatten().collect::<Vec<_>>().join(" · "),
                here: zone_at == Some(zone),
            },
            Some(false) => MapBox { tone: BoxTone::Ghost, title: zone, tag: "no object".into(), sub: sub_absent, meta: DASH.into(), here: false },
            None if check.and_then(|b| b.error.as_ref()).is_some() => MapBox {
                tone: BoxTone::Ghost, title: zone, tag: "unknown".into(), sub: "presence could not be checked".into(), meta: DASH.into(), here: false,
            },
            None => MapBox { tone: BoxTone::Ghost, title: zone, tag: "no address".into(), sub: sub_absent, meta: DASH.into(), here: false },
        }
    };

    let deletes = cx.landing_deletes_at().map(|d| format!("deletes {}", d.format("%m-%d"))).unwrap_or_default();
    let landing = zone_box(
        "landing",
        if zone_at == Some("landing") {
            format!("as the caller wrote it · transient, {retention} days")
        } else {
            format!("still inside the {retention}-day window")
        },
        if cx.root.promoted_at.is_some() { "removed after promotion".into() } else { "as the caller wrote it".into() },
        deletes,
    );
    let raw = zone_box(
        "raw",
        if cx.t.is_archive && !cx.written_members().is_empty() {
            "root is here; members beside it".into()
        } else {
            "source of truth for content".into()
        },
        "source of truth for content".into(),
        String::new(),
    );

    let written = cx.written_members().len();
    let end = if !cx.t.is_archive {
        MapBox { tone: BoxTone::Na, title: "members", tag: "not an archive".into(), sub: "no extraction attempted".into(), meta: DASH.into(), here: false }
    } else {
        let total = cx.expected_files();
        let bytes: u64 = cx.written_members().iter().filter_map(|m| m.actual_size_bytes.or(m.size_bytes)).sum();
        let deepest = deepest_reached(cx).unwrap_or(0);
        let depth_total = cx.t.expected.as_ref().and_then(|e| e.entries.iter().map(|x| x.depth).max());
        match (extracted.status, total) {
            (Status::Stalled | Status::Failed, t) => MapBox {
                tone: BoxTone::Bad,
                title: "members",
                tag: t.map(|t| format!("{written} of {t} written")).unwrap_or_else(|| format!("{written} written")),
                sub: match (t, depth_total) {
                    (Some(t), Some(d)) => format!("{} not reached · deepest level {deepest} of {d}", t.saturating_sub(written)),
                    _ => "the rest were not reached".into(),
                },
                meta: f::bytes(bytes),
                here: false,
            },
            (Status::Done, _) => MapBox {
                tone: BoxTone::Present,
                title: "members",
                tag: format!("{written} written"),
                sub: format!("{written} files at {} depth{}, one revision", deepest + 1, if deepest == 0 { "" } else { "s" }),
                meta: f::bytes(bytes),
                here: false,
            },
            _ => MapBox {
                tone: BoxTone::Ghost,
                title: "members",
                tag: if written > 0 { format!("{written} written so far") } else { "not extracted yet".into() },
                sub: total.map(|t| format!("{t} expected")).unwrap_or_else(|| "member count unknown".into()),
                meta: DASH.into(),
                here: false,
            },
        }
    };

    let e2_note = match (promoted.status, promoted.prov) {
        (Status::Done, _) => "catalog.promotedAt".into(),
        (Status::Waiting, _) if cx.pending() => "waiting on reference".into(),
        (Status::Skipped, _) => "not taken".into(),
        _ => promoted.detail.clone(),
    };
    let e3_note = match extracted.status {
        Status::Skipped => "skipped".into(),
        Status::Done => format!("depth {} reached", deepest_reached(cx).unwrap_or(0)),
        Status::Stalled => format!(
            "no progress · {}",
            cx.last_progress().map(|p| f::span(cx.now - p)).unwrap_or_else(|| "?".into())
        ),
        Status::Failed => "failed".into(),
        _ => "not started".into(),
    };

    let taken = |zone: &str, event: &str| cx.present(zone) || cx.last_event(event).is_some();
    let curated_built = cx.blob("curated").is_some_and(|b| b.container_exists) || cx.t.env.has_container("curated");
    TransitMap {
        source: f::or_dash(Some(cx.root.source_system.clone())),
        e1: edge(&received, "received", received.at.clone()),
        landing,
        e2: edge(&promoted, "promoted", e2_note),
        raw,
        e3: edge(&extracted, "extracted", e3_note),
        end,
        rejected: Branch {
            zone: "rejected",
            taken: taken("rejected", "UploadRejected"),
            built: true,
            note: if taken("rejected", "UploadRejected") { "failed the gate · taken".into() } else { "failed the gate · not taken".into() },
        },
        quarantine: Branch {
            zone: "quarantine",
            taken: taken("quarantine", "UploadQuarantined"),
            built: true,
            note: if taken("quarantine", "UploadQuarantined") { "refused or expired · taken".into() } else { "refused by a rule · not taken".into() },
        },
        curated: Branch {
            zone: "curated",
            taken: cx.present("curated"),
            built: curated_built,
            note: if !curated_built {
                "does not exist yet".into()
            } else if cx.present("curated") {
                "published copy present".into()
            } else {
                "no copy for this document".into()
            },
        },
    }
}

/// Where the bytes of truth are right now.
fn zone_at(cx: &Ctx) -> Option<&'static str> {
    ["quarantine", "rejected", "raw", "landing", "curated"].into_iter().find(|z| cx.present(z))
}

fn zones(cx: &Ctx, zone_at: Option<&'static str>) -> Vec<ZoneRow> {
    let mut order: Vec<&str> = ZONES.to_vec();
    if let Some(at) = zone_at {
        order.retain(|z| *z != at);
        order.insert(0, at);
    }
    order.into_iter().map(|z| zone_row(cx, z)).collect()
}

fn zone_row(cx: &Ctx, zone: &str) -> ZoneRow {
    let r = cx.root;
    let check = cx.blob(zone);
    let built = check.map_or(cx.t.env.has_container(zone), |b| b.container_exists);
    let present = check.and_then(|b| b.exists) == Some(true);
    let mut path = check
        .and_then(|b| b.path.as_ref().map(|p| format!("{}/{p}", b.container)))
        .filter(|_| present)
        .unwrap_or_else(|| DASH.into());
    let size = if present { check.and_then(|b| b.size).map(f::bytes).unwrap_or_else(|| DASH.into()) } else { DASH.into() };
    let error = check.and_then(|b| b.error.clone());
    let (tag, note) = if !built {
        ("zone not built".to_string(), "The container does not exist in this storage account.".to_string())
    } else if let Some(e) = error {
        ("unknown".into(), format!("Could not check: {e}"))
    } else {
        let tag = if present { "object exists" } else { "no object" }.to_string();
        let note = match (zone, present) {
            ("landing", true) => format!(
                "As the caller wrote it.{}",
                cx.landing_deletes_at()
                    .map(|d| format!(" Deleted on {} by the {}-day lifecycle rule.", f::date(d), cx.t.env.landing_retention_days))
                    .unwrap_or_default()
            ),
            ("landing", false) if r.promoted_at.is_some() => "Gone: promoted to raw, and landing is transient.".into(),
            ("landing", false) => "Nothing at the recorded landing address.".into(),
            ("raw", true) if cx.t.is_archive => "The source of truth for content. Members sit beside it under the same revision.".into(),
            ("raw", true) => "The source of truth for content.".into(),
            ("raw", false) if cx.pending() => "Not promoted: the reference lookup has not resolved. raw is the source of truth for content once it exists.".into(),
            ("raw", false) if r.promoted_at.is_some() => "catalog.promotedAt is set, yet nothing is at the raw address.".into(),
            ("raw", false) => "Not promoted.".into(),
            ("quarantine", true) => format!("Refused or expired: {}.", cx.quarantine_reason().unwrap_or_else(|| "no reason recorded".into())),
            ("quarantine", false) => "Not refused by any rule. A terminal sibling of raw, not a later stage.".into(),
            ("rejected", true) => "Failed the validation gate.".into(),
            ("rejected", false) if r.uploaded_at.is_some() => "Passed the validation gate, so nothing was written here.".into(),
            ("rejected", false) => "Nothing uploaded yet, so nothing to reject.".into(),
            ("curated", true) => "The consumer-facing copy.".into(),
            ("curated", false) if cx.t.is_archive => "Archive roots are not offered to consumers; their members are.".into(),
            _ => "No curated copy for this document.".into(),
        };
        (tag, note)
    };
    let (tag, note) = match (zone, cx.t.is_archive, cx.curated_address()) {
        ("curated", true, Some(address)) if !present => {
            let (placed, files) = cx.members_curated();
            path = format!("curated/{address}");
            (
                "address only".to_string(),
                format!("Archive roots get a curated address but never bytes; their members are placed one by one — {placed} of {files} are in curated."),
            )
        }
        _ => (tag, note),
    };
    ZoneRow { zone: zone.into(), present, built, tag, path, note, size }
}

fn meta(cx: &Ctx) -> (Vec<MetaRow>, String, Tone) {
    let r = cx.root;
    let pending = cx.pending();
    let dash = || DASH.to_string();
    let looked_up = cx.has_prerequisite();
    let unresolved = |label: &str| if pending { label.to_string() } else if looked_up { "null".into() } else { "not looked up".into() };

    let declared_number = if r.business_key.is_empty() { r.source_document_id.clone() } else { r.business_key.clone() };
    let resolved_number = r.attribute("reference.uniqueDocNumber");
    let number_flag = resolved_number.as_ref().is_some_and(|n| !declared_number.is_empty() && !n.eq_ignore_ascii_case(&declared_number));
    let mime_flag = mime_agrees(r) == Some(false);
    let size_flag = matches!((r.size_bytes, r.actual_size_bytes), (Some(a), Some(b)) if a != b && a > 0);

    let generation = cx.t.reference.as_ref().and_then(|p| {
        p.generation.map(|g| {
            format!("{g}{}", f::parse_opt(&p.built_at).map(|b| format!(" · {}", f::day_minute(b))).unwrap_or_default())
        })
    });

    let mut rows = vec![
        MetaRow {
            label: "Document number".into(),
            declared: f::or_dash(Some(declared_number.clone())),
            pending: pending && resolved_number.is_none(),
            resolved: resolved_number.clone().unwrap_or_else(|| unresolved("no row")),
            flag: number_flag,
        },
        MetaRow {
            label: "SBM project number".into(),
            declared: dash(),
            pending: pending && r.attribute("reference.sbmProjectNumber").is_none(),
            resolved: r.attribute("reference.sbmProjectNumber").unwrap_or_else(|| unresolved("pending")),
            flag: false,
        },
        MetaRow {
            label: "Document type".into(),
            declared: f::or_dash(Some(r.document_type.clone())),
            resolved: f::or_dash(Some(r.document_type.clone())),
            flag: false,
            pending: false,
        },
        MetaRow {
            label: "Declared MIME".into(),
            declared: f::or_dash(Some(r.mime_type.clone())),
            resolved: f::or_dash(Some(r.sniffed_mime.clone())),
            flag: mime_flag,
            pending: false,
        },
        MetaRow {
            label: "Source system".into(),
            declared: f::or_dash(Some(r.source_system.clone())),
            resolved: f::or_dash(Some(r.source_system.clone())),
            flag: false,
            pending: false,
        },
        MetaRow {
            label: "Size".into(),
            declared: r.size_bytes.map(f::bytes).unwrap_or_else(dash),
            resolved: r.actual_size_bytes.or(r.size_bytes).map(f::bytes).unwrap_or_else(dash),
            flag: size_flag,
            pending: false,
        },
        MetaRow {
            label: "Reference gen".into(),
            declared: dash(),
            resolved: generation.unwrap_or_else(|| if looked_up { "no pointer".into() } else { dash() }),
            flag: false,
            pending: false,
        },
    ];
    // Every other register attribute, so nothing resolved is hidden.
    for (key, _) in r.attributes.iter().filter(|(k, _)| {
        k.starts_with("reference.") && !matches!(k.as_str(), "reference.uniqueDocNumber" | "reference.sbmProjectNumber")
    }) {
        rows.push(MetaRow {
            label: key.trim_start_matches("reference.").to_string(),
            declared: dash(),
            resolved: r.attribute(key).unwrap_or_else(|| "null".into()),
            flag: false,
            pending: false,
        });
    }

    let (note, tone) = if mime_flag {
        (
            format!(
                "declaredMime ≠ sniffedMime ({} against {}). Routing reads the sniffed type — the shape of the bug that once made every plain ZIP sniff as a Word document.",
                r.mime_type, r.sniffed_mime
            ),
            Tone::Bad,
        )
    } else if number_flag {
        ("The register's document number differs from the one the caller declared.".into(), Tone::Bad)
    } else if size_flag {
        ("The declared size and the bytes that arrived differ.".into(), Tone::Bad)
    } else if pending {
        (
            "Declared and sniffed types agree, and the register holds no row for this document number yet — the pending attributes are what promotion is waiting on.".into(),
            Tone::Info,
        )
    } else if !looked_up {
        ("This source has no metadata prerequisite, so nothing was resolved against the register.".into(), Tone::Info)
    } else {
        (
            "Everything the caller declared is corroborated by the register, and the sniffed type matches the declared one. This is what agreement looks like.".into(),
            Tone::Ok,
        )
    };
    (rows, note, tone)
}

fn rail(depth: u32, last: bool) -> String {
    let mut out = "│  ".repeat(depth as usize);
    out.push_str(if last { "└─ " } else { "├─ " });
    out
}

fn tree(cx: &Ctx) -> Option<Tree> {
    if !cx.t.is_archive {
        return None;
    }
    let by_path: HashMap<&str, &Doc> = cx
        .t
        .family
        .iter()
        .filter_map(|m| m.path_from_root.as_deref().map(|p| (p, m)))
        .collect();
    let containers = containers_in_family(cx);

    let mut entries: Vec<ExpectedEntry> = cx.t.expected.as_ref().map(|e| e.entries.clone()).unwrap_or_default();
    let known: std::collections::HashSet<String> = entries.iter().map(|e| e.path.clone()).collect();
    let mut extra: Vec<ExpectedEntry> = cx
        .t
        .family
        .iter()
        .filter_map(|m| {
            let p = m.path_from_root.clone()?;
            (!known.contains(&p)).then(|| ExpectedEntry {
                rejected: None,
                depth: depth_of(&p, &containers),
                container: m.raw.as_ref().map_or(true, |r| r.path.is_empty()),
                size: m.actual_size_bytes.or(m.size_bytes),
                document_id: m.document_id.clone(),
                path: p,
            })
        })
        .collect();
    extra.sort_by(|a, b| a.path.cmp(&b.path));
    entries.extend(extra);
    if entries.is_empty() && cx.written_members().is_empty() {
        let (banner, note) = match cx.t.expected.as_ref().and_then(|e| e.error.clone()) {
            Some(e) => ("MEMBERS UNKNOWN".to_string(), e),
            None => ("NO MEMBERS YET".to_string(), "Nothing has been extracted, and the archive listing is not available.".to_string()),
        };
        return Some(Tree { banner, note, tone: Tone::Warn, rows: vec![], footer: footer(cx) });
    }

    let last = entries.len().saturating_sub(1);
    let rows: Vec<TreeRow> = entries
        .iter()
        .enumerate()
        .map(|(i, e)| {
            let doc = by_path.get(e.path.as_str());
            let reached = if e.container {
                doc.is_some()
            } else {
                doc.is_some_and(|d| d.raw.as_ref().is_some_and(|r| !r.path.is_empty()))
            };
            let (parent, name) = match e.path.rsplit_once('/') {
                Some((p, n)) => (format!("{p}/"), n.to_string()),
                None => (String::new(), e.path.clone()),
            };
            let size = doc
                .and_then(|d| d.actual_size_bytes.or(d.size_bytes).filter(|s| *s > 0))
                .or(e.size)
                .filter(|_| reached);
            TreeRow {
                rail: rail(e.depth, i == last),
                depth: e.depth,
                container: e.container,
                reached,
                parent,
                name,
                note: match (e.container, reached) {
                    (true, true) => "nested archive — a container, not a leaf".into(),
                    (true, false) => "nested archive — never opened".into(),
                    (false, true) => "member · own documentId, same revision".into(),
                    (false, false) => "expected — the walk never reached this".into(),
                },
                tag: match (e.container, reached) {
                    (true, true) => "container",
                    (true, false) => "not opened",
                    (false, true) => "raw",
                    (false, false) => "not reached",
                }
                .into(),
                size: size.map(f::bytes).unwrap_or_else(|| DASH.into()),
                document_id: doc.map(|d| d.document_id.clone()).unwrap_or_else(|| e.document_id.clone()),
                rejected: false,
            }
        })
        .collect();
    let mut rows = rows;
    for (row, e) in rows.iter_mut().zip(&entries) {
        if let Some(reason) = &e.rejected {
            row.rejected = true;
            row.tag = "refused".into();
            row.note = format!("refused by the archive policy ({reason}) — the job quarantines the root here");
        }
    }

    let files: Vec<&TreeRow> = rows.iter().filter(|r| !r.container && !r.rejected).collect();
    let total = files.len();
    let written = files.iter().filter(|r| r.reached).count();
    let depths = entries.iter().map(|e| e.depth).max().map(|d| d + 1).unwrap_or(1);
    let listing_error = cx.t.expected.as_ref().and_then(|e| e.error.clone());
    let (banner, note, tone) = if written < total {
        (
            format!("{written} OF {total} WRITTEN · {} NOT REACHED", total - written),
            "Member identity is a pure function of (documentId, revisionId, pathFromRoot), so the missing rows are knowable before the walk runs. A half-extracted tree must not look like a small complete one.".to_string(),
            Tone::Bad,
        )
    } else if let Some(e) = listing_error {
        (format!("{written} MEMBERS WRITTEN · LISTING INCOMPLETE"), e, Tone::Warn)
    } else {
        (
            format!("{total} MEMBERS · {depths} DEPTH{} · ALL WRITTEN", if depths == 1 { "" } else { "S" }),
            "Depth is the point: nested extraction was silently broken from inception until 2026-08-14, and a flat list of filenames would have hidden it.".to_string(),
            Tone::Ok,
        )
    };
    Some(Tree { banner, note, tone, rows, footer: footer(cx) })
}

fn footer(cx: &Ctx) -> String {
    format!(
        "One archive, several documents. Every member derives its own documentId from (root {}, revision {}, pathFromRoot) and shares the root's revision.",
        cx.root.document_id,
        f::or_dash(Some(cx.root.revision_id.clone()))
    )
}

fn group_of_event(a: &AuditRow) -> GroupId {
    match a.event_type.as_str() {
        "DocumentRegistered" | "DocumentUploadedSmall" | "DuplicateArrival" | "UploadRejected" => GroupId::Admitted,
        "UploadQuarantined" if detail_str(&a.detail, "reason").as_deref() == Some(METADATA_UNRESOLVED) => GroupId::Reference,
        "UploadQuarantined" => GroupId::Admitted,
        "DocumentPendingMetadata" | "PendingMetadataResolved" => GroupId::Reference,
        "DocumentDispatched" | "DocumentDispatchFailed" | "DocumentPromotionStalled" => GroupId::Promotion,
        "ArchiveExtracted" | "ArchiveExtractionFailed" | "PdfSplitByTag" => GroupId::Extraction,
        "DocumentExpired" | "LandingResidueDetected" => GroupId::Lifecycle,
        _ => GroupId::Other,
    }
}

fn group_of_log(row: &LogRow) -> GroupId {
    if row.source == "job" {
        return GroupId::Extraction;
    }
    let c = row.category.to_ascii_lowercase();
    if ["reconcile", "upload", "preregister", "commit"].iter().any(|k| c.contains(k)) {
        GroupId::Admitted
    } else if ["reevaluate", "reference"].iter().any(|k| c.contains(k)) {
        GroupId::Reference
    } else if ["processing_rules", "evaluate", "promote", "dispatch"].iter().any(|k| c.contains(k)) {
        GroupId::Promotion
    } else if c.contains("extract") {
        GroupId::Extraction
    } else if c.contains("sweep") {
        GroupId::Lifecycle
    } else {
        GroupId::Other
    }
}

fn audit_text(a: &AuditRow) -> String {
    let mut text = a.event_type.clone();
    if a.document_id.len() > 0 {
        text.push_str(&format!(" documentId={}", a.document_id));
    }
    if let Value::Object(map) = &a.detail {
        let sorted: BTreeMap<_, _> = map.iter().collect();
        for (k, v) in sorted {
            let v = match v {
                Value::String(s) => s.clone(),
                Value::Null => continue,
                other => other.to_string(),
            };
            text.push_str(&format!(" {k}={v}"));
        }
    }
    text
}

fn log_kind(level: &str) -> LogKind {
    match level.to_ascii_lowercase().as_str() {
        "error" | "critical" => LogKind::Error,
        "warning" | "warn" => LogKind::Warn,
        _ => LogKind::Info,
    }
}

fn log_groups(cx: &Ctx) -> (String, Vec<LogGroup>) {
    let logs = &cx.t.logs;
    let lag = logs.lag_seconds.map(|s| Duration::milliseconds((s * 1000.0) as i64));
    let log_checked = f::parse_opt(&logs.checked_at).unwrap_or(cx.checked);
    let note = match (logs.available, lag) {
        (true, Some(l)) => format!(
            "App Insights is {} behind. State and timeline above are current; these log entries are not, and each group carries its own as of.",
            f::span(l)
        ),
        (true, None) => "App Insights reported no recent ingestion to measure its lag against, so how current these entries are is unknown. State and timeline above are current.".into(),
        (false, _) => format!(
            "Logs could not be read{}. State, zones and audit entries are still current; only log lines are missing.",
            logs.reason.as_ref().map(|r| format!(": {r}")).unwrap_or_default()
        ),
    };

    let mut buckets: BTreeMap<GroupId, Vec<(String, LogEntry)>> = BTreeMap::new();
    for a in &cx.t.audit {
        let at = f::parse(&a.occurred_at);
        buckets.entry(group_of_event(a)).or_default().push((
            a.occurred_at.clone(),
            LogEntry { at: at.map(f::day_clock).unwrap_or_else(|| DASH.into()), kind: LogKind::Audit, text: audit_text(a) },
        ));
    }
    for row in &logs.entries {
        let at = f::parse(&row.at);
        let mut text = row.message.clone();
        if let Some(replica) = &row.replica {
            text = format!("[{replica}] {text}");
        } else if !row.category.is_empty() {
            text = format!("[{}] {text}", row.category);
        }
        buckets.entry(group_of_log(row)).or_default().push((
            row.at.clone(),
            LogEntry { at: at.map(f::day_clock).unwrap_or_else(|| DASH.into()), kind: log_kind(&row.level), text },
        ));
    }

    let as_of = if logs.available {
        f::day_clock(log_checked - lag.unwrap_or_else(Duration::zero))
    } else {
        DASH.to_string()
    };
    let lag_label = match (logs.available, lag) {
        (true, Some(l)) => f::span(l),
        (true, None) => "lag unknown".into(),
        (false, _) => "logs unavailable".into(),
    };

    let plan: [(GroupId, &'static str); 6] = [
        (GroupId::Admitted, "Received + admitted"),
        (GroupId::Reference, "Reference lookup"),
        (GroupId::Promotion, "Promotion + dispatch"),
        (GroupId::Extraction, "Extraction"),
        (GroupId::Lifecycle, "Lifecycle sweep"),
        (GroupId::Other, "Other"),
    ];
    let mut groups = vec![];
    for (id, label) in plan {
        let mut rows = buckets.remove(&id).unwrap_or_default();
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        let entries: Vec<LogEntry> = rows.into_iter().map(|(_, e)| e).collect();
        if entries.is_empty() {
            let (text, lag_text) = match id {
                GroupId::Promotion => (
                    "No log or audit entry exists for this step. Promotion is never recorded — it is read from the catalog.",
                    "no entries",
                ),
                GroupId::Extraction if !cx.t.is_archive => ("Skipped — not an archive.", "not run"),
                GroupId::Extraction => ("No extraction entry was found for this archive.", "no entries"),
                GroupId::Reference if !cx.has_prerequisite() => ("Skipped — this source has no metadata prerequisite.", "not run"),
                GroupId::Reference => ("No reference entry was found for this document.", "no entries"),
                GroupId::Admitted => ("No upload or gate entry was found for this document.", "no entries"),
                GroupId::Lifecycle | GroupId::Other => continue,
            };
            groups.push(LogGroup {
                id,
                label,
                as_of: DASH.into(),
                lag: lag_text.into(),
                quiet: true,
                entries: vec![LogEntry { at: DASH.into(), kind: LogKind::None, text: text.into() }],
            });
        } else {
            groups.push(LogGroup { id, label, as_of: as_of.clone(), lag: lag_label.clone(), quiet: !logs.available, entries });
        }
    }
    (note, groups)
}

fn gaps(cx: &Ctx, steps: &[Step], tree: Option<&Tree>) -> Vec<Gap> {
    let r = cx.root;
    let mut out = vec![];
    let mut push = |short: String, why: String| out.push(Gap { short, why });

    let extracted = find(steps, StepId::Extracted);
    if extracted.status == Status::Stalled {
        push(
            "Why the walk stopped is not recorded anywhere".into(),
            "There is no failure entry. The absence of progress in the catalog is the only signal, so the stall is inferred and the cause is unknown to this view.".into(),
        );
    }
    if let Some(t) = tree {
        let missing = t.rows.iter().filter(|r| !r.reached && !r.container).count();
        if missing > 0 {
            push(
                format!("The {missing} unwritten member{} expected, not observed", if missing == 1 { " is" } else { "s are" }),
                "Member identity is a pure function of (documentId, revisionId, pathFromRoot), so the rows can be drawn before the walk runs. They are shown as expected and not reached, never as facts.".into(),
            );
        }
    }
    if let Some(err) = cx.t.expected.as_ref().and_then(|e| e.error.clone()) {
        push("The archive's full member list could not be read".into(), err);
    }
    if extracted.status == Status::Stalled && replica(cx).is_none() {
        push(
            "Which Job replica ran the walk is unknown".into(),
            "No console log line from the extraction Job names this document, so the replica cannot be identified.".into(),
        );
    }
    if r.promoted_at.is_some() || cx.pending() {
        push(
            "Promotion is inferred: no DocumentPromoted audit event exists".into(),
            "The landing → raw hop is deduced from catalog.promotedAt. This view marks it inferred rather than drawing it like a recorded hop.".into(),
        );
    }
    let attempts = r.pending_attempts.unwrap_or(0) as usize;
    let recorded = cx.event_count("DocumentPendingMetadata") + cx.event_count("PendingMetadataResolved");
    if attempts > recorded && attempts > 1 {
        push(
            format!("{} of {attempts} reference lookups are counted, not recorded", attempts - recorded),
            "The catalog keeps an attempt counter, but a re-check that still finds no row writes no audit entry, so the earlier timings are unknown.".into(),
        );
    }
    if find(steps, StepId::Received).prov == Some(Prov::Inferred) {
        push(
            "The upload itself left no audit entry".into(),
            "A session upload writes straight to landing; the arrival time comes from catalog.uploadedAt.".into(),
        );
    }
    if find(steps, StepId::Curated).status == Status::NotBuilt {
        push(
            "curated cannot be reported on — the zone does not exist".into(),
            "Absence here means the zone is not built in this environment, not that this document is missing from it.".into(),
        );
    }
    for b in cx.t.blobs.iter().filter(|b| b.error.is_some()) {
        push(
            format!("Whether {} holds an object is unknown", b.zone),
            b.error.clone().unwrap_or_default(),
        );
    }
    if !cx.t.logs.available {
        push(
            "Log lines could not be read".into(),
            cx.t.logs.reason.clone().unwrap_or_else(|| "Neither App Insights nor Log Analytics could be located for this environment.".into()),
        );
    }
    if cx.t.is_archive && !cx.t.family.is_empty() {
        push(
            "Member identity is read from the catalog, not the audit trail".into(),
            "Each member carries its own documentId and fileGuid that no event reports, so member identity is a catalog fact rather than an observed one.".into(),
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitoring::model::{Env, Expected, Location, Logs, RefPointer, SourceCfg};
    use serde_json::json;

    const NOW: &str = "2026-08-14T09:04:11Z";

    fn now() -> DateTime<Utc> {
        f::parse(NOW).unwrap()
    }

    fn env() -> Env {
        Env {
            name: "dev".into(),
            landing_retention_days: 8,
            pending_max_days: 5,
            containers: ["landing", "raw", "quarantine", "rejected"].iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        }
    }

    fn blob(zone: &str, path: Option<&str>, exists: bool, size: u64, modified: &str) -> BlobCheck {
        BlobCheck {
            zone: zone.into(),
            container: zone.into(),
            container_exists: zone != "curated",
            path: path.map(String::from),
            exists: path.map(|_| exists),
            size: exists.then_some(size),
            last_modified: exists.then(|| modified.to_string()),
            error: None,
        }
    }

    fn audit(event: &str, doc: &str, at: &str, detail: Value) -> AuditRow {
        AuditRow { id: format!("{doc}:{event}:{at}"), document_id: doc.into(), event_type: event.into(), occurred_at: at.into(), correlation_id: "c1".into(), detail }
    }

    fn parked() -> Trace {
        let root = Doc {
            document_id: "01M00E8JM7QK4V2C1H9YB3".into(),
            revision_id: "00".into(),
            business_key: "001.01000.000100-fb001".into(),
            source_system: "NEO".into(),
            state: "PendingMetadata".into(),
            mime_type: "application/pdf".into(),
            sniffed_mime: "application/pdf".into(),
            size_bytes: Some(184_320),
            uploaded_at: Some("2026-08-11T04:12:07Z".into()),
            pending_since: Some("2026-08-11T04:12:09Z".into()),
            pending_attempts: Some(3),
            pending_key: Some("NEO/001.01000.000100".into()),
            file_guid: "g-root".into(),
            landing: Some(Location { container: "landing".into(), path: "NEO/2026/08/01M00E8JM7QK4V2C1H9YB3/00/g-root.pdf".into() }),
            ..Default::default()
        };
        Trace {
            env: env(),
            checked_at: NOW.into(),
            query: "001.01000.000100-fb001".into(),
            matched_on: Some("businessKey".into()),
            root: Some(root),
            audit: vec![
                audit("DocumentUploadedSmall", "01M00E8JM7QK4V2C1H9YB3", "2026-08-11T04:12:07Z", json!({"sizeBytes": 184320})),
                audit("DocumentPendingMetadata", "01M00E8JM7QK4V2C1H9YB3", "2026-08-11T04:12:09Z", json!({"attempt": 1, "referenceKey": "NEO/001.01000.000100"})),
            ],
            blobs: vec![
                blob("landing", Some("NEO/2026/08/01M00E8JM7QK4V2C1H9YB3/00/g-root.pdf"), true, 184_320, "2026-08-11T04:12:07Z"),
                blob("raw", None, false, 0, ""),
                blob("quarantine", Some("01M00E8JM7QK4V2C1H9YB3/00/g-root"), false, 0, ""),
                blob("rejected", Some("01M00E8JM7QK4V2C1H9YB3/00/g-root"), false, 0, ""),
                blob("curated", None, false, 0, ""),
            ],
            logs: Logs { available: true, lag_seconds: Some(109.0), checked_at: Some(NOW.into()), ..Default::default() },
            source: Some(SourceCfg { source_system: "NEO".into(), reference_set: Some("mdr".into()), pending_max_days: Some(5) }),
            reference: Some(RefPointer { reference_set_id: "mdr".into(), generation: Some(2), row_count: Some(10), built_at: Some("2026-08-14T02:04:00Z".into()), ..Default::default() }),
            ..Default::default()
        }
    }

    fn by_id(v: &TraceView, id: StepId) -> &Step {
        v.steps.iter().find(|s| s.id == id).unwrap()
    }

    #[test]
    fn a_parked_document_waits_in_landing_and_says_when_it_is_deleted() {
        let v = build(&parked(), now()).unwrap();
        assert_eq!(v.zone_at, Some("landing"));
        assert_eq!(v.focus, Some(StepId::Reference));
        assert_eq!((v.hero.kicker.as_str(), v.live.big.as_str(), v.live.big_note.as_str()), ("waiting, not failing", "3d 04h", "waiting"));
        assert_eq!(v.header.sub, "NEO · rev 00 · 184,320 B");
        assert_eq!(v.header.tone, Status::Waiting);
        assert_eq!(v.header.life, Tone::Warn);
        assert!(v.header.lifecycle.contains("2026-08-19"), "{}", v.header.lifecycle);
        assert!(v.header.lifecycle.contains("quarantines it on 2026-08-16"), "{}", v.header.lifecycle);
        assert_eq!(by_id(&v, StepId::Reference).status, Status::Waiting);
        assert_eq!(by_id(&v, StepId::Promoted).detail, "waiting on reference");
        assert_eq!(v.map.e2.tone, EdgeTone::Waiting);
        assert!(v.map.landing.here);
        assert_eq!(v.map.landing.meta, "184,320 B · deletes 08-19");
    }

    #[test]
    fn promotion_is_never_drawn_as_observed() {
        let mut t = parked();
        let root = t.root.as_mut().unwrap();
        root.state = "Dispatched".into();
        root.pending_since = None;
        root.promoted_at = Some("2026-08-11T04:12:14Z".into());
        root.raw = Some(Location { container: "raw".into(), path: "NEO/P/D/00/Official/g-root.pdf".into() });
        t.blobs[1] = blob("raw", Some("NEO/P/D/00/Official/g-root.pdf"), true, 184_320, "2026-08-11T04:12:14Z");
        let v = build(&t, now()).unwrap();
        let promoted = by_id(&v, StepId::Promoted);
        assert_eq!((promoted.status, promoted.prov), (Status::Done, Some(Prov::Inferred)));
        assert_eq!(v.map.e2.tone, EdgeTone::Inferred);
        assert_eq!(v.zone_at, Some("raw"));
        assert!(v.gaps.iter().any(|g| g.short.starts_with("Promotion is inferred")));
    }

    #[test]
    fn a_mime_disagreement_is_flagged_on_the_gate_and_in_declared_vs_resolved() {
        let mut t = parked();
        t.root.as_mut().unwrap().sniffed_mime = "application/vnd.openxmlformats-officedocument.wordprocessingml.document".into();
        let v = build(&t, now()).unwrap();
        assert!(by_id(&v, StepId::Validated).detail.contains("disagree"));
        let row = v.meta.iter().find(|m| m.label == "Declared MIME").unwrap();
        assert!(row.flag);
        assert_eq!(v.meta_tone, Tone::Bad);
    }

    #[test]
    fn uncounted_lookups_become_a_gap() {
        let v = build(&parked(), now()).unwrap();
        assert!(v.gaps.iter().any(|g| g.short == "2 of 3 reference lookups are counted, not recorded"), "{:?}", v.gaps);
    }

    #[test]
    fn curated_missing_from_the_account_is_not_built_rather_than_absent() {
        let v = build(&parked(), now()).unwrap();
        assert_eq!(by_id(&v, StepId::Curated).status, Status::NotBuilt);
        let curated = v.zones.iter().find(|z| z.zone == "curated").unwrap();
        assert_eq!(curated.tag, "zone not built");
    }

    fn half_extracted() -> Trace {
        let mut t = parked();
        let root = t.root.as_mut().unwrap();
        root.document_id = "ROOT".into();
        root.state = "Dispatched".into();
        root.pending_since = None;
        root.pending_attempts = None;
        root.mime_type = "application/zip".into();
        root.sniffed_mime = "application/zip".into();
        root.promoted_at = Some("2026-08-14T08:19:47Z".into());
        root.archive_extraction_state = Some("Extracting".into());
        root.raw = Some(Location { container: "raw".into(), path: "PACE/P/D/00/Official/g-root.zip".into() });
        t.audit.clear();
        t.is_archive = true;
        t.blobs[1] = blob("raw", Some("PACE/P/D/00/Official/g-root.zip"), true, 1_432, "2026-08-14T08:19:47Z");
        t.family = vec![Doc {
            document_id: "M1".into(),
            file_guid: "g-m1".into(),
            root_file_guid: "g-root".into(),
            path_from_root: Some("summary.pdf".into()),
            raw: Some(Location { container: "raw".into(), path: "PACE/P/D/00/Official/g-m1.pdf".into() }),
            size_bytes: Some(604),
            written_at: Some("2026-08-14T08:21:07Z".into()),
            ..Default::default()
        }];
        let e = |path: &str, depth: u32, container: bool| ExpectedEntry { path: path.into(), depth, container, size: Some(10), document_id: format!("id-{path}"), rejected: None };
        t.expected = Some(Expected {
            entries: vec![
                e("summary.pdf", 0, false),
                e("inner.zip", 0, true),
                e("inner.zip/notes.txt", 1, false),
                e("inner.zip/deeper.zip", 1, true),
                e("inner.zip/deeper.zip/detail.pdf", 2, false),
            ],
            error: None,
        });
        t
    }

    #[test]
    fn a_walk_silent_past_the_threshold_is_stalled_with_unreached_rows_drawn() {
        let v = build(&half_extracted(), now()).unwrap();
        let extracted = by_id(&v, StepId::Extracted);
        assert_eq!(extracted.status, Status::Stalled);
        assert_eq!(v.focus, Some(StepId::Extracted));
        assert_eq!((v.hero.kicker.as_str(), v.live.big.as_str(), v.live.big_note.as_str()), ("stalled", "43m 04s", "since last progress"));
        assert!(extracted.detail.starts_with("1 of 3 members written, then no progress for 43m"), "{}", extracted.detail);
        assert_eq!(v.header.tone, Status::Stalled);
        assert_eq!(v.map.end.tone, BoxTone::Bad);
        assert_eq!(v.map.end.tag, "1 of 3 written");
        let tree = v.tree.unwrap();
        assert_eq!(tree.banner, "1 OF 3 WRITTEN · 2 NOT REACHED");
        let rails: Vec<&str> = tree.rows.iter().map(|r| r.rail.as_str()).collect();
        assert_eq!(rails, ["├─ ", "├─ ", "│  ├─ ", "│  ├─ ", "│  │  └─ "]);
        assert_eq!(tree.rows[1].tag, "not opened");
        assert_eq!(tree.rows[4].document_id, "id-inner.zip/deeper.zip/detail.pdf");
        assert!(v.gaps.iter().any(|g| g.short.starts_with("Why the walk stopped")));
    }

    #[test]
    fn a_live_lease_keeps_a_quiet_walk_out_of_stalled() {
        let mut t = half_extracted();
        t.root.as_mut().unwrap().archive_extraction_lease_expires_at = Some("2026-08-14T09:30:00Z".into());
        let v = build(&t, now()).unwrap();
        assert_eq!(by_id(&v, StepId::Extracted).status, Status::Waiting);
    }

    #[test]
    fn expired_pending_quarantine_is_an_expiry_not_a_gate_refusal() {
        let mut t = parked();
        t.root.as_mut().unwrap().state = "Quarantined".into();
        t.audit.push(audit("UploadQuarantined", "01M00E8JM7QK4V2C1H9YB3", "2026-08-16T04:12:09Z", json!({"reason": METADATA_UNRESOLVED})));
        let v = build(&t, now()).unwrap();
        assert_eq!(by_id(&v, StepId::Validated).status, Status::Done);
        assert_eq!(by_id(&v, StepId::Reference).status, Status::Failed);
        assert!(v.header.lifecycle.contains("never resolved within 5 days"));
        let reference = v.groups.iter().find(|g| g.id == GroupId::Reference).unwrap();
        assert!(reference.entries.iter().any(|e| e.text.starts_with("UploadQuarantined")));
    }

    #[test]
    fn unavailable_logs_say_why_and_still_group_audit_entries() {
        let mut t = parked();
        t.logs = Logs { available: false, reason: Some("the Azure CLI login has expired".into()), ..Default::default() };
        let v = build(&t, now()).unwrap();
        assert!(v.log_note.contains("login has expired"));
        let admitted = v.groups.iter().find(|g| g.id == GroupId::Admitted).unwrap();
        assert_eq!(admitted.entries[0].kind, LogKind::Audit);
        assert_eq!(admitted.lag, "logs unavailable");
        assert!(v.gaps.iter().any(|g| g.short == "Log lines could not be read"));
    }

    #[test]
    fn curated_state_follows_the_path_and_the_platform_s_parking_rules() {
        let mut t = parked();
        t.env.containers.push("curated".into());
        let root = t.root.as_mut().unwrap();
        root.state = "Dispatched".into();
        root.pending_since = None;
        root.promoted_at = Some("2026-08-11T04:12:14Z".into());
        root.dispatched_at = Some("2026-08-11T04:12:15Z".into());
        root.curated = Some(Location { container: String::new(), path: String::new() });
        root.attributes.insert("reference.sbmProjectNumber".into(), json!("SM1000"));
        root.attributes.insert("reference.uniqueDocNumber".into(), json!("001-x"));

        root.file_indicator = "RW".into();
        let not_yet = build(&t, now()).unwrap();
        assert_eq!(by_id(&not_yet, StepId::Curated).status, Status::Waiting, "an empty curated object is not a failure");

        t.root.as_mut().unwrap().file_indicator = "rw".into();
        let parked_on_indicator = build(&t, now()).unwrap();
        let step = by_id(&parked_on_indicator, StepId::Curated);
        assert_eq!(step.status, Status::Stalled);
        assert!(step.detail.contains("\"rw\""), "{}", step.detail);

        let root = t.root.as_mut().unwrap();
        root.file_indicator = "RO".into();
        root.curated = Some(Location { container: "curated".into(), path: "NEO/SM1000/001-x/00/Official/g.pdf".into() });
        t.blobs[4] = BlobCheck { zone: "curated".into(), container: "curated".into(), container_exists: true, path: Some("NEO/SM1000/001-x/00/Official/g.pdf".into()), exists: Some(true), size: Some(1), last_modified: None, error: None };
        let placed = build(&t, now()).unwrap();
        assert_eq!(by_id(&placed, StepId::Curated).status, Status::Done);
    }

    #[test]
    fn a_curated_archive_root_shows_an_address_and_its_curated_members_not_a_missing_object() {
        let mut t = half_extracted();
        t.env.containers.push("curated".into());
        t.root.as_mut().unwrap().curated = Some(Location { container: "curated".into(), path: "PACE/P/D/00/Native/g-root.zip".into() });
        t.family[0].curated = Some(Location { container: "curated".into(), path: "PACE/P/D/00/Native/g-m1.pdf".into() });
        let mut second = t.family[0].clone();
        second.file_guid = "g-m2".into();
        second.path_from_root = Some("inner.zip/notes.txt".into());
        second.curated = None;
        t.family.push(second);
        let v = build(&t, now()).unwrap();
        let curated = v.zones.iter().find(|z| z.zone == "curated").unwrap();
        assert_eq!(curated.tag, "address only");
        assert!(curated.note.contains("1 of 2"), "{}", curated.note);
        assert_eq!(curated.path, "curated/PACE/P/D/00/Native/g-root.zip");
        let step = by_id(&v, StepId::Curated);
        assert_eq!(step.status, Status::Done);
        assert!(step.detail.contains("1 of 2 members"), "{}", step.detail);
    }

    #[test]
    fn no_root_means_no_trace() {
        let mut t = parked();
        t.root = None;
        assert!(build(&t, now()).is_none());
    }
}
