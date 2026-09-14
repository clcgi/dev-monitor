use super::kit::*;
use super::Screen;
use crate::monitoring::fleet_view::{paginate, Home};
use crate::monitoring::format as fmt;
use crate::monitoring::model::{Overview, Trace};
use crate::monitoring::tokens::*;
use dioxus::prelude::*;

#[component]
pub fn HomeScreen(home: Home, overview: Overview, refreshing: bool, on_trace: EventHandler<String>, on_go: EventHandler<Screen>) -> Element {
    let headline = format!("{}letter-spacing:.01em;color:{TEXT_STRONG}", sans(600, 18.0));
    let body = format!("{}line-height:1.75;color:{TEXT_BODY};margin-top:9px", sans(400, 13.0));
    let label = format!("{}letter-spacing:.16em;color:{DIM}", mono(400, 9.0));
    let value = format!("{}color:{TEXT_STRONG};overflow-wrap:anywhere;text-align:right", mono(500, 12.0));
    let footnote = format!("{}line-height:1.7;color:{DIM};max-width:420px", mono(400, 10.5));
    let first_parked = overview.parked.first().map(|d| d.document_id.clone());
    rsx! {
        div { style: "max-width:620px;margin:56px auto 0;display:flex;flex-direction:column;align-items:center;text-align:center;gap:18px",
            if refreshing { div { style: "width:100%", SweepBar { color: CYAN.to_string(), track: RULE.to_string() } } }
            div { style: "width:64px;height:64px;border:1px solid {BORDER};display:flex;align-items:center;justify-content:center;position:relative",
                Icon { name: "database", size: 28.0, color: GHOST.to_string() }
                div { style: "position:absolute;inset:-1px;border:1px solid {DASH};transform:rotate(45deg);opacity:.4" }
            }
            div {
                div { style: "{headline}", "{home.headline}" }
                div { style: "{body}", "{home.body}" }
            }
            div { style: "width:100%;border:1px solid {BORDER};background:{CARD};text-align:left",
                for (k, v) in home.facts.clone() {
                    div { key: "{k}", style: "display:flex;align-items:center;gap:12px;padding:9px 18px;border-bottom:1px solid {ROW_RULE}",
                        span { style: "{label};flex:1", {k.to_uppercase()} }
                        span { style: "{value}", "{v}" }
                    }
                }
            }
            for w in overview.warnings.clone() {
                div { style: "width:100%;text-align:left",
                    Banner { tone: crate::monitoring::trace_view::Tone::Warn, icon: "warning", title: "Partly unreadable".to_string(), body: w }
                }
            }
            div { style: "display:flex;gap:10px;flex-wrap:wrap;justify-content:center",
                if let Some(id) = first_parked {
                    ActionButton { label: "Trace the oldest parked".to_string(), icon: "hourglass-high", primary: true, onclick: move |_| on_trace.call(id.clone()) }
                }
                ActionButton { label: "Parked queue".to_string(), icon: "tray", primary: false, onclick: move |_| on_go.call(Screen::Queue) }
                ActionButton { label: "Stuck extractions".to_string(), icon: "pause-circle", primary: false, onclick: move |_| on_go.call(Screen::Stuck) }
            }
            if home.empty {
                div { style: "{footnote}", "Blank is the honest answer. The viewer will not synthesize a story from an empty environment." }
            }
        }
    }
}

#[component]
pub fn TracePrompt(overview: Option<Overview>, env_name: String, on_trace: EventHandler<String>, on_go: EventHandler<Screen>) -> Element {
    let title = format!("{}color:{TEXT_STRONG};margin-top:10px", sans(600, 16.0));
    let body = format!("{}line-height:1.7;color:{TEXT_BODY};margin-top:6px", sans(400, 12.5));
    let key = format!("{}color:{CYAN};overflow-wrap:anywhere;flex:1;min-width:0", mono(500, 12.0));
    let meta = format!("{}color:{DIM};flex-shrink:0", mono(400, 10.5));
    let parked: Vec<(String, String, String)> = overview
        .as_ref()
        .map(|o| {
            o.parked
                .iter()
                .take(6)
                .map(|d| (d.document_id.clone(), d.display_key().to_string(), fmt::or_dash(d.pending_attempts.map(|a| format!("{a} tries")))))
                .collect()
        })
        .unwrap_or_default();
    let extracting: Vec<(String, String, String)> = overview
        .as_ref()
        .map(|o| {
            o.stuck
                .iter()
                .take(6)
                .map(|s| (s.root.document_id.clone(), s.root.display_key().to_string(), format!("{} written", s.members_written)))
                .collect()
        })
        .unwrap_or_default();
    let curated: Vec<(String, String, String)> = overview
        .as_ref()
        .map(|o| o.curated.iter().take(6).map(|d| (d.document_id.clone(), d.display_key().to_string(), fmt::or_dash(Some(d.source_system.clone())))).collect())
        .unwrap_or_default();
    let curated_total = overview.as_ref().map(|o| format!("{} in curated", o.curated_rows)).unwrap_or_default();
    let pick = |rows: Vec<(String, String, String)>, empty: &'static str| {
        rsx! {
            if rows.is_empty() { EmptyRow { text: empty.to_string() } }
            for (i, (id, label, note)) in rows.into_iter().enumerate() {
                div {
                    key: "{i}-{id}",
                    class: "cdwm-row",
                    style: "display:flex;align-items:center;gap:11px;padding:9px 18px;border-bottom:1px solid {ROW_RULE};cursor:pointer",
                    onclick: move |_| on_trace.call(id.clone()),
                    span { style: "{key}", "{label}" }
                    span { style: "{meta}", "{note}" }
                }
            }
        }
    };
    rsx! {
        div { style: "max-width:760px;margin:40px auto 0;display:flex;flex-direction:column;gap:16px",
            div { style: "text-align:center",
                Icon { name: "crosshair", size: 34.0, color: GHOST.to_string() }
                div { style: "{title}", "Trace one document through {env_name}" }
                div { style: "{body}", "Type a document number, documentId, fileGuid, correlationId, pendingKey or blob path above, then press TRACE. Every panel is read live and read-only." }
            }
            if let Some(_) = overview {
                div { style: "display:grid;grid-template-columns:repeat(auto-fit,minmax(300px,1fr));gap:16px",
                    Panel { icon: "hourglass-high", icon_color: AMBER.to_string(), title: "Parked on metadata".to_string(), subtitle: "oldest first".to_string(),
                        {pick(parked, "Nothing is parked.")}
                    }
                    Panel { icon: "pause-circle", icon_color: CORAL.to_string(), title: "Extracting".to_string(), subtitle: "longest silent first".to_string(),
                        {pick(extracting, "No archive is mid-walk.")}
                    }
                    Panel { icon: "stack", icon_color: CYAN.to_string(), title: "Recently curated".to_string(), subtitle: curated_total,
                        {pick(curated, "Nothing has been placed in curated.")}
                    }
                }
            }
            div { style: "display:flex;gap:10px;justify-content:center",
                ActionButton { label: format!("{env_name} today"), icon: "database", primary: false, onclick: move |_| on_go.call(Screen::Home) }
            }
        }
    }
}

#[component]
pub fn NoMatch(trace: Trace, env_name: String, oldest_parked: Option<String>, on_trace: EventHandler<String>, on_go: EventHandler<Screen>) -> Element {
    let mut audit_page = use_signal(|| 0usize);
    let audit_shown = paginate(&trace.audit, audit_page());
    let title = format!("{}color:{TEXT_STRONG};margin-top:10px;overflow-wrap:anywhere", sans(600, 16.0));
    let body = format!("{}line-height:1.7;color:{TEXT_BODY};margin-top:6px", sans(400, 12.5));
    let label = format!("padding:10px 18px;border-bottom:1px solid {RULE};{}letter-spacing:.18em;color:{DIM}", mono(400, 9.0));
    let row_label = format!("{}color:{TEXT_SOFT};flex:1", sans(400, 12.0));
    let result = format!("{}letter-spacing:.12em;text-transform:uppercase;color:{DIM}", mono(500, 9.0));
    let audit_at = format!("{}color:{DIM};width:130px;flex-shrink:0", mono(400, 10.5));
    let audit_text = format!("{}color:{TEXT_CELL};overflow-wrap:anywhere", mono(400, 11.0));
    let icon_for = |label: &str| {
        if label.starts_with("Cosmos catalog") {
            "rows"
        } else if label.starts_with("Cosmos audit") {
            "clock-counter-clockwise"
        } else {
            "archive-box"
        }
    };
    let scale = format!("{env_name} holds {} audit rows and {} catalog rows today", trace.audit_rows, trace.catalog_rows);
    let queue_is_primary = oldest_parked.is_none();
    rsx! {
        div { style: "max-width:660px;margin:48px auto 0;display:flex;flex-direction:column;gap:16px",
            div { style: "text-align:center",
                Icon { name: "magnifying-glass-minus", size: 34.0, color: GHOST.to_string() }
                div { style: "{title}", "No document matches “{trace.query}”" }
                div { style: "{body}",
                    if trace.audit.is_empty() {
                        "Nothing in the catalog, the audit trail or blob storage matches it."
                    } else {
                        "No catalog row matches it, but the audit trail does — the document was seen and never catalogued, or its row is gone."
                    }
                }
            }
            div { style: "border:1px solid {BORDER};background:{CARD}",
                div { style: "{label}", "WHAT WAS SEARCHED" }
                for s in trace.searched.clone() {
                    div { key: "{s.label}", style: "display:flex;align-items:center;gap:11px;padding:10px 18px;border-bottom:1px solid {ROW_RULE}",
                        Icon { name: icon_for(&s.label), size: 16.0, color: DIM.to_string() }
                        span { style: "{row_label}", "{s.label}" }
                        span { style: "{result}", "{s.result}" }
                    }
                }
                div { style: "display:flex;align-items:center;gap:11px;padding:10px 18px",
                    Icon { name: "database", size: 16.0, color: DIM.to_string() }
                    span { style: "{row_label}", "{scale}" }
                    span { style: "{result}", "expected" }
                }
            }
            if !trace.audit.is_empty() {
                Panel { icon: "clock-counter-clockwise", title: "Audit entries with no catalog row".to_string(),
                    for a in audit_shown.rows.clone() {
                        div { key: "{a.id}", style: "display:flex;gap:11px;padding:8px 18px;border-bottom:1px solid {ROW_RULE}",
                            span { style: "{audit_at}", {fmt::parse(&a.occurred_at).map(fmt::day_clock).unwrap_or_default()} }
                            span { style: "{audit_text}", "{a.event_type} documentId={a.document_id} {a.detail}" }
                        }
                    }
                    Pager { page: audit_shown.page, pages: audit_shown.pages, total: audit_shown.total, on_page: move |p| audit_page.set(p) }
                }
            }
            div { style: "display:flex;gap:10px;justify-content:center;flex-wrap:wrap",
                if let Some(id) = oldest_parked {
                    ActionButton { label: "Trace the oldest parked".to_string(), icon: "hourglass-high", primary: true, onclick: move |_| on_trace.call(id.clone()) }
                }
                ActionButton { label: "Browse the queue".to_string(), icon: "tray", primary: queue_is_primary, onclick: move |_| on_go.call(Screen::Queue) }
            }
        }
    }
}
