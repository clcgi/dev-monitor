use super::kit::{mono, sans, Icon};
use super::Screen;
use crate::monitoring::tokens::*;
use crate::monitoring::trace_view::Status;
use crate::services::state::Environment;
use dioxus::prelude::*;

const LOGO: &str = include_str!("../../../assets/sbm-logo.svg");

#[derive(Clone, PartialEq, Debug)]
pub struct NavItem {
    pub screen: Screen,
    pub icon: &'static str,
    pub label: String,
    pub badge: Option<String>,
    /// A badge that is a warning rather than a count of zero or more.
    pub hot: bool,
}

/// A document traced in this session, for the rail's "Documents" list.
#[derive(Clone, PartialEq, Debug)]
pub struct RecentDoc {
    pub document_id: String,
    pub key: String,
    pub tone: Status,
}

#[component]
pub fn Aside(
    nav: Vec<NavItem>,
    active: Screen,
    env: Environment,
    stage_note: String,
    catalog: String,
    audit: String,
    docs: Vec<RecentDoc>,
    current: String,
    on_nav: EventHandler<Screen>,
    on_env: EventHandler<Environment>,
    on_trace: EventHandler<String>,
    on_developer: EventHandler<()>,
) -> Element {
    let section = format!("{}color:{DIM};padding:0 22px 9px", sans(600, 12.0));
    let note = format!("{}line-height:1.6;color:{DIM}", sans(500, 11.5));
    let counts = format!("display:flex;justify-content:space-between;{}color:{DIM}", mono(500, 10.5));
    let env_name = env.as_str().to_uppercase();
    let developer = format!(
        "display:flex;align-items:center;gap:8px;height:34px;padding:0 12px;border:1px solid {DASH};border-radius:8px;background:{CARD};color:{TEXT_SOFT};cursor:pointer;{}",
        sans(600, 12.5)
    );
    rsx! {
        aside { class: "cdwm-rail", style: "flex-shrink:0;background:{CARD};border-right:1px solid {BORDER};display:flex;flex-direction:column;min-height:0;overflow-y:auto;padding:24px 0 22px",
            div { style: "padding:0 22px 22px",
                div { class: "cdwm-logo", title: "SBM Offshore", dangerous_inner_html: LOGO }
                div { style: "{sans(600, 13.0)}color:{TEXT_SOFT};margin-top:15px", "Central Document Warehouse" }
                div { style: "{sans(500, 12.0)}color:{DIM};margin-top:2px", {format!("Document trace · {env_name} · read only · v{}", env!("CARGO_PKG_VERSION"))} }
            }

            div { style: "{section}", "Menu" }
            nav { style: "display:flex;flex-direction:column;gap:2px;padding:0 12px",
                for item in nav {
                    {nav_button(item, active, on_nav)}
                }
            }

            div { style: "height:1px;background:{BORDER};margin:20px 22px 18px" }

            div { style: "{section}", "Documents" }
            div { style: "display:flex;flex-direction:column;gap:2px;padding:0 12px",
                if docs.is_empty() {
                    div { style: "{note};padding:0 10px", "Documents you trace appear here." }
                }
                for d in docs {
                    {doc_button(d, &current, on_trace)}
                }
            }

            div { style: "margin-top:auto;padding:22px 22px 0;display:flex;flex-direction:column;gap:10px",
                div { style: "display:flex;gap:3px;background:#E9ECEE;border-radius:8px;padding:3px",
                    for candidate in [Environment::Sandbox, Environment::Dev, Environment::Stg] {
                        {env_button(candidate, env, on_env)}
                    }
                }
                div { style: "{note}", "Catalog and blob checks are immediate; App Insights lags minutes. {stage_note}" }
                div { style: "{counts}",
                    span { "catalog {catalog}" }
                    span { "audit {audit}" }
                }
                button {
                    r#type: "button",
                    title: "Open the Developer Monitor (script runner). Runs keep going while you are here.",
                    style: "{developer}",
                    onclick: move |_| on_developer.call(()),
                    Icon { name: "terminal-window", size: 16.0, color: TEXT_SOFT.to_string() }
                    "Developer Monitor"
                }
            }
        }
    }
}

fn nav_button(item: NavItem, active: Screen, on_nav: EventHandler<Screen>) -> Element {
    let on = item.screen == active;
    let (bg, edge, fg, icon_fg, weight) = if on { (SELECTED_BG, ORANGE, TEXT, ORANGE_DEEP, 700) } else { ("transparent", "transparent", TEXT_SOFT, DIM, 500) };
    let style = format!(
        "display:flex;align-items:center;gap:12px;width:100%;padding:11px 12px;border:none;border-radius:9px;border-left:3px solid {edge};cursor:pointer;text-align:left;background:{bg}"
    );
    let label = format!("flex:1;min-width:0;{}color:{fg}", sans(weight, 14.0));
    let badge_bg = match (item.hot, item.screen) {
        (false, _) => "#C7CCD0",
        (true, Screen::Queue) => ORANGE_DEEP,
        (true, Screen::Refs) => AMBER,
        (true, _) => CORAL,
    };
    let badge_style = format!("{}color:#fff;background:{badge_bg};border-radius:20px;padding:2px 8px;flex-shrink:0", sans(700, 11.0));
    let screen = item.screen;
    rsx! {
        button { r#type: "button", style: "{style}", onclick: move |_| on_nav.call(screen),
            Icon { name: item.icon, size: 20.0, color: icon_fg.to_string() }
            span { style: "{label}", "{item.label}" }
            if let Some(b) = item.badge { span { style: "{badge_style}", "{b}" } }
        }
    }
}

fn doc_button(d: RecentDoc, current: &str, on_trace: EventHandler<String>) -> Element {
    let on = d.document_id == current;
    let dot = match d.tone {
        Status::Waiting => ORANGE_MID,
        Status::Stalled | Status::Failed => BAD_BD,
        Status::Done => OK_BD,
        _ => "#C7CCD0",
    };
    let (bg, fg, weight, fill) = if on { (SELECTED_BG, TEXT, 600, dot) } else { ("transparent", TEXT_BODY, 400, "transparent") };
    let key_style = format!("flex:1;min-width:0;{}color:{fg};overflow:hidden;text-overflow:ellipsis;white-space:nowrap", mono(weight, 12.0));
    let id = d.document_id.clone();
    rsx! {
        button {
            key: "{d.document_id}",
            r#type: "button",
            title: "Trace {d.document_id}",
            style: "display:flex;align-items:center;gap:11px;width:100%;padding:10px 12px;border:none;border-radius:9px;cursor:pointer;text-align:left;background:{bg}",
            onclick: move |_| on_trace.call(id.clone()),
            span { style: "width:10px;height:10px;border-radius:50%;border:2.5px solid {dot};flex-shrink:0;background:{fill}" }
            span { style: "{key_style}", "{d.key}" }
        }
    }
}

fn env_button(candidate: Environment, current: Environment, on_env: EventHandler<Environment>) -> Element {
    let on = candidate == current;
    let (bg, fg, weight, shadow) = if on { (CARD, TEXT, 700, "0 1px 3px rgba(26,26,26,.16)") } else { ("transparent", TEXT_BODY, 500, "none") };
    let style = format!("flex:1;padding:5px 0;border:none;border-radius:6px;cursor:pointer;background:{bg};color:{fg};box-shadow:{shadow};{}", sans(weight, 12.0));
    let label = match candidate {
        Environment::Sandbox => "SBX",
        Environment::Dev => "DEV",
        Environment::Stg => "STG",
    };
    rsx! {
        button { r#type: "button", title: "Monitor {candidate}", style: "{style}", onclick: move |_| on_env.call(candidate), "{label}" }
    }
}

#[component]
pub fn Header(
    breadcrumb: String,
    title: String,
    query: String,
    busy: bool,
    back: Option<String>,
    on_back: EventHandler<()>,
    on_submit: EventHandler<String>,
    on_refresh: EventHandler<()>,
) -> Element {
    // The draft lives here, so typing re-renders the search box and not every screen.
    let mut draft = use_signal(|| query.clone());
    use_effect(use_reactive((&query,), move |(query,)| draft.set(query)));
    let crumb = format!("{}color:{DIM};white-space:nowrap;overflow:hidden;text-overflow:ellipsis", sans(600, 12.0));
    let h1 = format!("margin:2px 0 0;{}color:{TEXT};white-space:nowrap;overflow:hidden;text-overflow:ellipsis", sans(700, 20.0));
    let input = format!("flex:1;min-width:0;background:transparent;border:none;outline:none;color:{TEXT};{}padding:0", mono(400, 12.5));
    let trace_btn = format!(
        "display:inline-flex;align-items:center;gap:7px;height:38px;padding:0 16px;border:1px solid {ORANGE};border-radius:9px;background:{ORANGE};color:#fff;cursor:pointer;{}",
        sans(700, 13.0)
    );
    let refresh_spin = if busy { "animation:cdwm-spin 1s linear infinite;" } else { "" };
    let back_style = format!(
        "display:inline-flex;align-items:center;justify-content:center;width:34px;height:34px;padding:0;border:1px solid {DASH};border-radius:9px;background:{CARD};cursor:pointer;flex-shrink:0"
    );
    rsx! {
        header { style: "flex-shrink:0;border-bottom:1px solid {BORDER};background:{CARD}",
            div { class: "cdwm-header", style: "display:flex;align-items:center;gap:12px 14px;flex-wrap:wrap",
                if let Some(label) = back {
                    button { r#type: "button", title: "Back to {label}", "aria-label": "Back to {label}", style: "{back_style}", onclick: move |_| on_back.call(()),
                        Icon { name: "arrow-left", size: 17.0, color: TEXT_SOFT.to_string() }
                    }
                }
                div { style: "min-width:0;overflow:hidden;flex:1 1 220px",
                    div { style: "{crumb}", "{breadcrumb}" }
                    h1 { style: "{h1}", "{title}" }
                }
                div { style: "display:flex;align-items:center;gap:10px;flex:1 1 420px;min-width:0;max-width:620px",
                div { style: "display:flex;align-items:center;height:38px;border:1px solid {DASH};border-radius:9px;background:{CARD};min-width:0;flex:1 1 200px",
                    span { style: "padding:0 10px 0 12px;display:inline-flex", Icon { name: "magnifying-glass", size: 17.0, color: DIM.to_string() } }
                    input {
                        style: "{input}",
                        value: "{draft}",
                        placeholder: "document number · documentId · fileGuid · blob path",
                        spellcheck: "false",
                        oninput: move |e| draft.set(e.value()),
                        onkeydown: move |e| if e.key() == Key::Enter { on_submit.call(draft()) },
                    }
                }
                button { r#type: "button", class: "cdwm-trace", style: "{trace_btn}", onclick: move |_| on_submit.call(draft()),
                    Icon { name: "magnifying-glass", size: 17.0, color: "#fff".to_string() }
                    "Trace"
                }
                button {
                    r#type: "button",
                    title: "Read everything on this screen again",
                    style: "display:inline-flex;align-items:center;justify-content:center;width:38px;height:38px;border:1px solid {DASH};border-radius:9px;background:{CARD};cursor:pointer",
                    onclick: move |_| on_refresh.call(()),
                    span { style: "display:inline-flex;{refresh_spin}", Icon { name: "arrow-clockwise", size: 17.0, color: TEXT_SOFT.to_string() } }
                }
                }
            }
        }
    }
}
