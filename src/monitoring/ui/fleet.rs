use super::kit::*;
use super::Screen;
use crate::monitoring::fleet_view::{DeadRow, Kpi, QueueRow, RefRow, StuckRow, TimingRow};
use crate::monitoring::format as fmt;
use crate::monitoring::model::DeadLetters;
use crate::monitoring::tokens::*;
use crate::monitoring::trace_view::Tone;
use dioxus::prelude::*;

fn cell(weight: u16, size: f32, color: &str) -> String {
    format!("{}color:{color};overflow-wrap:anywhere;min-width:0", mono(weight, size))
}

fn pill(hot: bool) -> String {
    let (bd, fg) = if hot { (BAD_BD, CORAL) } else { (BORDER, DIM) };
    format!(
        "justify-self:start;{}letter-spacing:.1em;text-transform:uppercase;padding:2px 7px;border:1px solid {bd};color:{fg}",
        mono(500, 9.0)
    )
}

fn labels(names: &[&str]) -> Vec<(String, bool)> {
    names.iter().map(|n| (n.to_string(), false)).collect()
}

#[component]
fn KpiCard(kpi: Kpi) -> Element {
    let fg = tone_fg(kpi.tone);
    let label = format!("{}letter-spacing:.18em;color:{DIM}", mono(400, 9.0));
    let value = format!("{}color:{fg};letter-spacing:-.01em", mono(500, 26.0));
    let unit = format!("{}color:{DIM}", mono(400, 10.5));
    rsx! {
        div { style: "border:1px solid {BORDER};border-radius:14px;background:{CARD};padding:14px 16px;position:relative;min-width:0",
            div { style: "position:absolute;top:-1px;left:-1px;width:9px;height:9px;border-top:1px solid {fg};border-left:1px solid {fg}" }
            div { style: "{label}", "{kpi.label}" }
            div { style: "display:flex;align-items:baseline;gap:7px;margin-top:7px;flex-wrap:wrap",
                span { style: "{value}", "{kpi.value}" }
                span { style: "{unit}", "{kpi.unit}" }
            }
        }
    }
}

/// What a screen answers and why it exists: the mock's stub card, now over live data.
#[component]
fn Explainer(icon: &'static str, title: String, answers: String, why: String) -> Element {
    let head = format!("{}color:{TEXT_SOFT}", caps(600, 11.0, 0.16));
    let live = format!("{}letter-spacing:.14em;text-transform:uppercase;border:1px solid {OK_BD};color:{CYAN};padding:2px 7px;margin-left:auto", mono(500, 9.0));
    let label = format!("{}letter-spacing:.18em;color:{DIM}", mono(400, 9.0));
    let answers_style = format!("{}line-height:1.65;color:{TEXT_STRONG};margin-top:5px", sans(400, 14.0));
    let why_style = format!("{}line-height:1.7;color:{TEXT_SOFT};margin-top:5px", sans(400, 12.5));
    rsx! {
        div { style: "border:1px dashed {DASH};border-radius:12px;background:{CARD_MUTED}",
            div { style: "display:flex;align-items:center;gap:9px;padding:12px 18px;border-bottom:1px dashed {BORDER}",
                Icon { name: icon, size: 17.0, color: DIM.to_string() }
                span { style: "{head}", "{title}" }
                span { style: "{live}", "live · read-only" }
            }
            div { style: "padding:18px;display:flex;flex-direction:column;gap:16px",
                div {
                    div { style: "{label}", "WHAT IT ANSWERS" }
                    div { style: "{answers_style}", "{answers}" }
                }
                div {
                    div { style: "{label}", "WHY IT EARNS ITS PLACE" }
                    div { style: "{why_style}", "{why}" }
                }
            }
        }
    }
}

#[component]
pub fn QueueScreen(kpis: Vec<Kpi>, rows: Vec<QueueRow>, current: String, refreshing: bool, on_trace: EventHandler<String>) -> Element {
    let columns = "2.1fr .7fr 1.2fr .9fr .7fr 2fr 1.2fr";
    let head = vec![
        ("DOCUMENT NUMBER".to_string(), false),
        ("SRC".to_string(), false),
        ("ARRIVED".to_string(), false),
        ("WAITING".to_string(), false),
        ("TRIES".to_string(), true),
        ("PENDINGKEY".to_string(), false),
        ("LIFECYCLE".to_string(), false),
    ];
    rsx! {
        div { style: "display:flex;flex-direction:column;gap:16px;max-width:1620px",
            Banner {
                tone: Tone::Warn,
                icon: "hourglass-high",
                title: "Waiting, not failing".to_string(),
                body: "Every document here arrived intact. Its business metadata has not yet reached the platform's reference data, which refreshes nightly from the lakehouse. The lookup re-runs after every refresh and needs no caller action — until the pending budget or the landing lifecycle runs out.".to_string(),
            }
            if refreshing { SweepBar { color: AMBER.to_string(), track: RULE.to_string() } }
            div { style: "display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:12px",
                for k in kpis { KpiCard { key: "{k.label}", kpi: k } }
            }
            Panel { icon: "tray", title: "The parked queue".to_string(), subtitle: "oldest first".to_string(), right: format!("{} rows", rows.len()),
                div { style: "overflow-x:auto",
                    GridHead { columns: columns.to_string(), min_width: 1060, labels: head }
                    if rows.is_empty() {
                        EmptyRow { text: "Nothing is parked. Every document with a metadata prerequisite has resolved.".to_string() }
                    }
                    for r in rows {
                        {
                            let id = r.file_guid.clone();
                            let selected = !current.is_empty() && r.document_id == current;
                            let row_bg = if selected { SELECTED_BG } else { "transparent" };
                            rsx! {
                                div {
                                    key: "{r.document_id}-{r.file_guid}",
                                    class: if selected { "cdwm-row cdwm-selected" } else { "cdwm-row" },
                                    title: "Trace {r.document_id}",
                                    style: "display:grid;grid-template-columns:{columns};gap:9px;padding:9px 18px;border-bottom:1px solid {ROW_RULE};align-items:center;cursor:pointer;background:{row_bg};min-width:1060px",
                                    onclick: move |_| on_trace.call(id.clone()),
                                    span { style: "{cell(500, 12.0, BLUE)}", "{r.key}" }
                                    span { style: "{cell(400, 11.5, DIM)}", "{r.src}" }
                                    span { style: "{cell(400, 11.0, TEXT_BODY)}", "{r.arrived}" }
                                    span { style: "{cell(500, 12.0, TEXT_STRONG)}", "{r.waiting}" }
                                    span { style: "{cell(400, 11.5, DIM)};text-align:right;padding-right:16px", "{r.attempts}" }
                                    span { style: "{cell(400, 11.0, DIM)}", "{r.pending_key}" }
                                    span { style: "{pill(r.hot)}", "{r.lifecycle}" }
                                }
                            }
                        }
                    }
                    Footnote { text: "Read from the catalog just now. pendingKey is shown because it is the only thing an operator can hand to the lakehouse team. Lifecycle names whichever comes first: the sweeper's quarantine or the landing delete, which writes nothing.".to_string() }
                }
            }
        }
    }
}

#[component]
pub fn StuckScreen(rows: Vec<StuckRow>, refreshing: bool, on_trace: EventHandler<String>) -> Element {
    let columns = "2fr .5fr 1fr 1.2fr 1.1fr 1.3fr";
    let walking = format!("{}letter-spacing:.1em;text-transform:uppercase;padding:1px 6px;border:1px solid {WARN_BD};color:{AMBER};margin-left:8px", mono(500, 8.5));
    rsx! {
        div { style: "display:flex;flex-direction:column;gap:16px;max-width:1620px",
            Banner {
                tone: Tone::Bad,
                icon: "pause-circle",
                title: "Roots on Extracting that stopped progressing".to_string(),
                body: format!("A Job replica that vanishes mid-walk writes no failure event — the signal is the absence of progress, so this view is built on each member's last write rather than on state alone. A walk silent for more than {} minutes with no live lease is stalled.", crate::monitoring::trace_view::STALL_AFTER_MINUTES),
            }
            if refreshing { SweepBar { color: CORAL.to_string(), track: RULE.to_string() } }
            Panel { icon: "pause-circle", icon_color: CORAL.to_string(), title: "Stuck extractions".to_string(), subtitle: "ordered by time since the last member was written".to_string(),
                right: format!("{} stalled / {} extracting", rows.iter().filter(|r| r.stalled).count(), rows.len()),
                div { style: "overflow-x:auto",
                    GridHead { columns: columns.to_string(), min_width: 1000, labels: labels(&["ROOT DOCUMENTID", "REV", "WRITTEN", "DEEPEST REACHED", "LAST PROGRESS", "JOB REPLICA"]) }
                    if rows.is_empty() {
                        EmptyRow { text: "No archive root is on Extracting. Nothing is mid-walk.".to_string() }
                    }
                    for r in rows {
                        {
                            let id = r.file_guid.clone();
                            let bar_width = r.pct.map(|p| format!("{p}%")).unwrap_or_else(|| "0%".into());
                            let written_fg = if r.stalled { CORAL } else { AMBER };
                            let track = if r.stalled { "#F4DEE0" } else { "#F0E4C8" };
                            rsx! {
                                div {
                                    key: "{r.document_id}-{r.file_guid}",
                                    class: "cdwm-row",
                                    title: "Trace {r.document_id}",
                                    style: "display:grid;grid-template-columns:{columns};gap:9px;padding:11px 18px;border-bottom:1px solid {ROW_RULE};align-items:center;cursor:pointer;min-width:1000px",
                                    onclick: move |_| on_trace.call(id.clone()),
                                    span { style: "{cell(500, 12.0, BLUE)}",
                                        "{r.document_id}"
                                        if !r.stalled { span { style: "{walking}", "walking" } }
                                    }
                                    span { style: "{cell(400, 11.5, DIM)}", "{r.rev}" }
                                    span { style: "display:inline-flex;align-items:center;gap:8px",
                                        span { style: "{cell(500, 12.0, written_fg)}", "{r.written}" }
                                        span { style: "width:44px;height:3px;background:{track};flex-shrink:0;display:block",
                                            span { style: "display:block;height:3px;width:{bar_width};min-width:2px;background:{written_fg}" }
                                        }
                                    }
                                    span { style: "{cell(400, 11.5, DIM)}", "{r.depth}" }
                                    span { style: "{cell(500, 12.0, TEXT_STRONG)}", "{r.last}" }
                                    span { style: "{cell(400, 11.0, DIM)}", "{r.replica}" }
                                }
                            }
                        }
                    }
                    Footnote { text: "Read-only. Requeue and replay are each their own decision about mutating a live environment, and are out of scope. The expected member count comes from reading the archive's own listing; \"?\" means it could not be read.".to_string() }
                }
            }
        }
    }
}

#[component]
pub fn RefsScreen(rows: Vec<RefRow>, refreshing: bool, on_go: EventHandler<Screen>) -> Element {
    let columns = "1.4fr .8fr 1.4fr .9fr .8fr 1fr 2.2fr";
    rsx! {
        div { style: "display:flex;flex-direction:column;gap:16px;max-width:1620px",
            Explainer {
                icon: "arrows-clockwise",
                title: "Reference freshness".to_string(),
                answers: "The age, generation and row count of each reference set's current.json — the only commit point for reference data.".to_string(),
                why: "A nightly build that silently did not run has no other signal, and every parked document depends on it.".to_string(),
            }
            if refreshing { SweepBar { color: CYAN.to_string(), track: RULE.to_string() } }
            Panel { icon: "arrows-clockwise", title: "Reference pointers".to_string(), subtitle: "one row per referenceSets document".to_string(),
                div { style: "overflow-x:auto",
                    GridHead { columns: columns.to_string(), min_width: 1000, labels: labels(&["REFERENCE SET", "GENERATION", "BUILT AT", "AGE", "ROWS", "DOCUMENTS WAITING", "NOTE"]) }
                    if rows.is_empty() {
                        EmptyRow { text: "No reference sets are configured in this environment, so nothing waits on reference data.".to_string() }
                    }
                    for r in rows {
                        {
                            let fg = tone_fg(r.tone);
                            let waiting_style = cell(400, 11.5, if r.waiting > 0 { AMBER } else { DIM });
                            let note_style = format!("{}color:{TEXT_BODY};overflow-wrap:anywhere", sans(400, 11.0));
                            rsx! {
                                div { key: "{r.set_id}", style: "display:grid;grid-template-columns:{columns};gap:9px;padding:10px 18px;border-bottom:1px solid {ROW_RULE};align-items:center;min-width:1000px",
                                    span { style: "{cell(500, 12.0, TEXT_STRONG)}", "{r.set_id}" }
                                    span { style: "{cell(500, 12.0, fg)}", "{r.generation}" }
                                    span { style: "{cell(400, 11.0, TEXT_BODY)}", "{r.built_at}" }
                                    span { style: "{cell(500, 12.0, fg)}", "{r.age}" }
                                    span { style: "{cell(400, 11.5, DIM)}", "{r.rows}" }
                                    span { style: "{waiting_style}", "{r.waiting}" }
                                    span { style: "{note_style}", "{r.note}" }
                                }
                            }
                        }
                    }
                    Footnote { text: "Read from the reference container's {set}/current.json. A generation becomes visible only when this pointer flips, so its builtAt is when the platform could first resolve against it.".to_string() }
                }
            }
            div { style: "display:flex;gap:10px",
                ActionButton { label: "Parked queue".to_string(), icon: "tray", primary: true, onclick: move |_| on_go.call(Screen::Queue) }
                ActionButton { label: "Stuck extractions".to_string(), icon: "pause-circle", primary: false, onclick: move |_| on_go.call(Screen::Stuck) }
            }
        }
    }
}

#[component]
pub fn DeadScreen(rows: Vec<DeadRow>, data: DeadLetters, refreshing: bool, on_trace: EventHandler<String>, on_go: EventHandler<Screen>) -> Element {
    let columns = "2fr 1.1fr 1.2fr 1.2fr 1.8fr";
    let queue_kpis: Vec<Kpi> = data
        .queues
        .iter()
        .map(|q| Kpi {
            label: "DEAD-LETTERED",
            value: q.dead_letter_count.map(|n| n.to_string()).unwrap_or_else(|| fmt::DASH.into()),
            unit: format!(
                "{}{}",
                q.name,
                q.active_count.map(|a| format!(" · {a} active")).unwrap_or_default()
            ),
            tone: if q.dead_letter_count.unwrap_or(0) > 0 { Tone::Bad } else if q.error.is_some() { Tone::Warn } else { Tone::Ok },
        })
        .collect();
    let errors: Vec<String> = data.queues.iter().filter_map(|q| q.error.as_ref().map(|e| format!("{}: {e}", q.name))).collect();
    let eg_columns = "3fr 1fr 1.2fr";
    rsx! {
        div { style: "display:flex;flex-direction:column;gap:16px;max-width:1620px",
            Explainer {
                icon: "trash",
                title: "Dead-letter watch".to_string(),
                answers: "Documents whose admission or processing notification was dead-lettered, so the document may still sit in landing where the lifecycle rule will delete it.".to_string(),
                why: "Invisible in every other view: the catalog looks ordinary, the blob exists, and nothing reports the parked notification. The document simply disappears when landing expires.".to_string(),
            }
            if refreshing { SweepBar { color: CORAL.to_string(), track: RULE.to_string() } }
            if !queue_kpis.is_empty() {
                div { style: "display:grid;grid-template-columns:repeat(auto-fit,minmax(200px,1fr));gap:12px",
                    for (i, k) in queue_kpis.into_iter().enumerate() { KpiCard { key: "{i}", kpi: k } }
                }
            }
            for e in errors {
                Banner { tone: Tone::Warn, icon: "plugs", title: "Partly unreadable".to_string(), body: e }
            }
            Panel { icon: "trash", icon_color: CORAL.to_string(), title: "Dead-lettered notifications".to_string(), subtitle: "oldest first".to_string(), right: format!("{} peeked", rows.len()),
                div { style: "overflow-x:auto",
                    GridHead { columns: columns.to_string(), min_width: 980, labels: labels(&["DOCUMENT NUMBER", "PARKED AT", "NOTIFICATION QUEUE", "LANDING DELETES", "RETRY STATE"]) }
                    if rows.is_empty() {
                        EmptyRow { text: "No dead-lettered message could be peeked on any notification queue.".to_string() }
                    }
                    for (i, r) in rows.into_iter().enumerate() {
                        {
                            let id = r.document_id.clone();
                            let clickable = id.is_some();
                            let cursor = if clickable { "pointer" } else { "default" };
                            let key_style = cell(500, 12.0, if clickable { BLUE } else { TEXT_STRONG });
                            rsx! {
                                div {
                                    key: "{i}",
                                    class: if clickable { "cdwm-row" } else { "" },
                                    style: "display:grid;grid-template-columns:{columns};gap:9px;padding:9px 18px;border-bottom:1px solid {ROW_RULE};align-items:center;cursor:{cursor};min-width:980px",
                                    onclick: move |_| if let Some(id) = id.clone() { on_trace.call(id) },
                                    span { style: "{key_style}", "{r.key}" }
                                    span { style: "{cell(400, 11.0, TEXT_BODY)}", "{r.parked_at}" }
                                    span { style: "{cell(400, 11.5, DIM)}", "{r.queue}" }
                                    span { style: "{pill(r.hot)}", "{r.landing_deletes}" }
                                    span { style: "{cell(400, 11.0, DIM)}", "{r.retry}" }
                                }
                            }
                        }
                    }
                    Footnote { text: "Peeked, never received: nothing here settles, requeues or replays a message.".to_string() }
                }
            }
            Panel { icon: "archive-box", title: "Event Grid dead letters".to_string(), subtitle: "deliveries that never reached a queue".to_string(), right: format!("{} blobs", data.event_grid.len()),
                div { style: "overflow-x:auto",
                    GridHead { columns: eg_columns.to_string(), min_width: 760, labels: labels(&["BLOB", "SIZE", "WRITTEN"]) }
                    if data.event_grid.is_empty() {
                        EmptyRow { text: "The Event Grid dead-letter container holds nothing (or does not exist here).".to_string() }
                    }
                    for b in data.event_grid.clone() {
                        div { key: "{b.path:?}", style: "display:grid;grid-template-columns:{eg_columns};gap:9px;padding:9px 18px;border-bottom:1px solid {ROW_RULE};align-items:center;min-width:760px",
                            span { style: "{cell(400, 11.0, TEXT_STRONG)}", {b.path.clone().unwrap_or_default()} }
                            span { style: "{cell(400, 11.0, DIM)}", {b.size.map(fmt::bytes).unwrap_or_default()} }
                            span { style: "{cell(400, 11.0, DIM)}", {fmt::parse_opt(&b.last_modified).map(fmt::full).unwrap_or_default()} }
                        }
                    }
                }
            }
            div { style: "display:flex;gap:10px",
                ActionButton { label: "Parked queue".to_string(), icon: "tray", primary: true, onclick: move |_| on_go.call(Screen::Queue) }
                ActionButton { label: "Stuck extractions".to_string(), icon: "pause-circle", primary: false, onclick: move |_| on_go.call(Screen::Stuck) }
            }
        }
    }
}

#[component]
pub fn TimingScreen(rows: Vec<TimingRow>, documents: usize, truncated: bool, refreshing: bool, on_go: EventHandler<Screen>) -> Element {
    let columns = "1.8fr .9fr .9fr .8fr 1fr 1fr";
    rsx! {
        div { style: "display:flex;flex-direction:column;gap:16px;max-width:1620px",
            Explainer {
                icon: "timer",
                title: "Step timing".to_string(),
                answers: "Where documents wait, per step — p50 and p95 dwell time, not merely whether the step passed.".to_string(),
                why: "Pass/fail is already visible in the checklist. How long documents sit in a step is not, and that is what operators feel.".to_string(),
            }
            if refreshing { SweepBar { color: CYAN.to_string(), track: RULE.to_string() } }
            Panel { icon: "timer", title: "Dwell by step".to_string(), subtitle: "last 30 days, from catalog timestamps".to_string(),
                div { style: "overflow-x:auto",
                    GridHead {
                        columns: columns.to_string(),
                        min_width: 900,
                        labels: vec![
                            ("STEP".to_string(), false),
                            ("P50 DWELL".to_string(), false),
                            ("P95 DWELL".to_string(), false),
                            ("SAMPLES".to_string(), true),
                            ("DOCUMENTS IN STEP".to_string(), true),
                            ("OLDEST IN STEP".to_string(), false),
                        ],
                    }
                    for r in rows {
                        {
                            let step_style = format!("{}color:{TEXT_STRONG}", sans(500, 12.5));
                            let in_step_style = format!("{};text-align:right;padding-right:16px", cell(500, 12.0, if r.in_step > 0 { AMBER } else { DIM }));
                            rsx! {
                                div { key: "{r.step}", style: "display:grid;grid-template-columns:{columns};gap:9px;padding:10px 18px;border-bottom:1px solid {ROW_RULE};align-items:center;min-width:900px",
                                    span { style: "{step_style}", "{r.step}" }
                                    span { style: "{cell(500, 12.0, TEXT_STRONG)}", "{r.p50}" }
                                    span { style: "{cell(500, 12.0, AMBER)}", "{r.p95}" }
                                    span { style: "{cell(400, 11.5, DIM)};text-align:right;padding-right:16px", "{r.samples}" }
                                    span { style: "{in_step_style}", "{r.in_step}" }
                                    span { style: "{cell(400, 11.5, DIM)}", "{r.oldest}" }
                                }
                            }
                        }
                    }
                    Footnote {
                        text: format!(
                            "{documents} documents registered in the last 30 days{}. Archive members are excluded: they copy their root's timestamps, so they would repeat its dwell once per file. Percentiles are nearest-rank — an observed dwell, never an interpolated one.",
                            if truncated { " (the most recent 5,000 — the rest were not read)" } else { "" }
                        ),
                    }
                }
            }
            div { style: "display:flex;gap:10px",
                ActionButton { label: "Parked queue".to_string(), icon: "tray", primary: true, onclick: move |_| on_go.call(Screen::Queue) }
                ActionButton { label: "Stuck extractions".to_string(), icon: "pause-circle", primary: false, onclick: move |_| on_go.call(Screen::Stuck) }
            }
        }
    }
}
