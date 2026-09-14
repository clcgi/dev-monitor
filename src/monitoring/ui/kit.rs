use crate::monitoring::fleet_view::{Sort, PAGE_SIZE};
use crate::monitoring::tokens::*;
use crate::monitoring::trace_view::{Prov, Status, Tone};
use dioxus::prelude::*;

pub fn mono(weight: u16, size: f32) -> String {
    format!("font-family:{MONO};font-weight:{weight};font-size:{size}px;")
}

pub fn sans(weight: u16, size: f32) -> String {
    format!("font-family:{SANS};font-weight:{weight};font-size:{size}px;")
}

/// Letter-spaced caps, the mock's label voice.
pub fn caps(weight: u16, size: f32, spacing: f32) -> String {
    format!("{}letter-spacing:{spacing}em;text-transform:uppercase;", sans(weight, size))
}

pub struct StatusLook {
    pub fg: &'static str,
    pub bd: &'static str,
    pub bg: &'static str,
    pub icon: &'static str,
}

pub fn status_look(s: Status) -> StatusLook {
    match s {
        Status::Done => StatusLook { fg: CYAN, bd: OK_BD, bg: OK_TINT, icon: "check-circle" },
        Status::Waiting => StatusLook { fg: AMBER, bd: WARN_BD, bg: WARN_BG, icon: "clock" },
        Status::Stalled => StatusLook { fg: CORAL, bd: BAD_BD, bg: BAD_BG, icon: "pause-circle" },
        Status::Failed => StatusLook { fg: CORAL, bd: BAD_BD, bg: BAD_BG, icon: "x" },
        Status::Skipped => StatusLook { fg: DIM, bd: BORDER, bg: CARD_SOFT, icon: "minus" },
        Status::NotBuilt => StatusLook { fg: TEXT_BODY, bd: DASH, bg: CARD_MUTED, icon: "lock" },
        Status::Unknown => StatusLook { fg: DIM, bd: BORDER, bg: CARD_SOFT, icon: "question" },
    }
}

pub fn prov_look(p: Prov) -> (&'static str, &'static str) {
    match p {
        Prov::Observed => (CYAN, "eye"),
        Prov::Inferred => (VIOLET, "sparkle"),
        Prov::Checked => (ICE, "crosshair"),
    }
}

/// (background, foreground, icon)
pub fn tone_look(t: Tone) -> (&'static str, &'static str, &'static str) {
    match t {
        Tone::Warn => (WARN_BG, AMBER, "clock"),
        Tone::Ok => (OK_BG, CYAN, "check-circle"),
        Tone::Bad => (BAD_BG, CORAL, "pause-circle"),
        Tone::Info => (INFO_BG, ICE, "info"),
    }
}

pub fn tone_fg(t: Tone) -> &'static str {
    match t {
        Tone::Warn => AMBER,
        Tone::Ok => CYAN,
        Tone::Bad => CORAL,
        Tone::Info => DIM,
    }
}

#[component]
pub fn Icon(name: &'static str, size: f32, color: String) -> Element {
    rsx! {
        i {
            class: "ph ph-{name}",
            style: "font-size:{size}px;color:{color};flex-shrink:0;line-height:1;display:inline-block",
        }
    }
}

#[component]
pub fn PulseDot(color: String, size: f32, pulse: bool, #[props(default = 2.2)] period: f32) -> Element {
    let animation = if pulse { format!("cdwm-pulse {period}s ease-in-out infinite") } else { "none".to_string() };
    rsx! {
        span { style: "width:{size}px;height:{size}px;border-radius:50%;background:{color};flex-shrink:0;display:inline-block;animation:{animation}" }
    }
}

#[component]
pub fn Panel(
    icon: &'static str,
    title: String,
    subtitle: Option<String>,
    right: Option<String>,
    #[props(default = CYAN.to_string())] icon_color: String,
    children: Element,
) -> Element {
    let title_style = format!("{}color:{TEXT_STRONG}", caps(600, 11.0, 0.16));
    let sub_style = format!("{}color:{DIM};margin-left:8px", sans(400, 10.5));
    let right_style = format!("{}color:{DIM};margin-left:auto", mono(400, 10.0));
    rsx! {
        div { style: "border:1px solid {BORDER};border-radius:12px;overflow:hidden;background:{CARD};min-width:0",
            div { style: "display:flex;align-items:center;gap:9px;padding:11px 18px;border-bottom:1px solid {RULE};flex-wrap:wrap",
                Icon { name: icon, size: 16.0, color: icon_color }
                span { style: "{title_style}", "{title}" }
                if let Some(s) = subtitle { span { style: "{sub_style}", "{s}" } }
                if let Some(r) = right { span { style: "{right_style}", "{r}" } }
            }
            {children}
        }
    }
}

#[component]
pub fn Banner(tone: Tone, icon: &'static str, title: String, body: String) -> Element {
    let (bg, fg, _) = tone_look(tone);
    let border = if tone == Tone::Bad { BAD_BD } else { BORDER };
    let title_fg = if tone == Tone::Bad { BAD_INK } else { fg };
    let title_style = format!("{}color:{title_fg}", caps(600, 11.0, 0.14));
    let body_style = format!("{}line-height:1.7;color:{TEXT_SOFT};margin-top:3px", sans(400, 11.5));
    rsx! {
        div { style: "display:flex;gap:11px;padding:13px 16px;border-radius:12px;border:1px solid {border};border-left:3px solid {fg};background:{bg}",
            Icon { name: icon, size: 18.0, color: fg.to_string() }
            div { style: "min-width:0",
                div { style: "{title_style}", "{title}" }
                div { style: "{body_style}", "{body}" }
            }
        }
    }
}

#[component]
pub fn ActionButton(label: String, icon: &'static str, primary: bool, onclick: EventHandler<()>) -> Element {
    let (bd, bg, fg) = if primary { (ORANGE_DEEP, SELECTED_BG, ORANGE_DEEP) } else { (DASH, CARD, TEXT_SOFT) };
    let style = format!(
        "display:inline-flex;align-items:center;gap:7px;height:36px;padding:0 15px;border-radius:8px;border:1px solid {bd};background:{bg};color:{fg};cursor:pointer;{}",
        caps(600, 10.5, 0.1)
    );
    rsx! {
        button { r#type: "button", style: "{style}", onclick: move |_| onclick.call(()),
            Icon { name: icon, size: 16.0, color: fg.to_string() }
            "{label}"
        }
    }
}

/// Table header row over a CSS grid, shared by every fleet table.
#[component]
pub fn GridHead(
    columns: String,
    min_width: u32,
    labels: Vec<(String, bool)>,
    sort: Option<Sort>,
    on_sort: EventHandler<usize>,
    #[props(default)] fixed: Vec<usize>,
) -> Element {
    rsx! {
        div { style: "display:grid;grid-template-columns:{columns};gap:9px;padding:9px 18px;border-bottom:1px solid {BORDER};background:{CARD_SOFT};min-width:{min_width}px",
            for (i, (label, right)) in labels.into_iter().enumerate() {
                {
                    let active = sort.filter(|s| s.column == i);
                    let color = if active.is_some() { ORANGE_DEEP } else { DIM };
                    let align = if right { "justify-content:flex-end;padding-right:16px;" } else { "" };
                    let style = format!(
                        "display:flex;align-items:center;gap:4px;min-width:0;background:none;border:none;padding:0;{align}{}letter-spacing:.16em;color:{color}",
                        mono(if active.is_some() { 700 } else { 400 }, 9.0)
                    );
                    let arrow = active.map(|s| if s.ascending { "caret-up" } else { "caret-down" }).unwrap_or("caret-up-down");
                    let arrow_color = if active.is_some() { color } else { GHOST };
                    let aria = active.map(|s| if s.ascending { "ascending" } else { "descending" }).unwrap_or("none");
                    let direction = active.map(|s| if s.ascending { " (ascending)" } else { " (descending)" }).unwrap_or_default();
                    if fixed.contains(&i) {
                        rsx! { span { key: "{i}", style: "{style}", "{label}" } }
                    } else {
                        rsx! {
                            button { key: "{i}", r#type: "button", "aria-sort": "{aria}", title: "Sort by {label}{direction}", style: "{style};cursor:pointer", onclick: move |_| on_sort.call(i),
                                "{label}"
                                Icon { name: arrow, size: 11.0, color: arrow_color.to_string() }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Previous / next over pages of PAGE_SIZE rows; nothing when everything fits on one page.
#[component]
pub fn Pager(page: usize, pages: usize, total: usize, on_page: EventHandler<usize>) -> Element {
    if total <= PAGE_SIZE {
        return rsx! {};
    }
    let range = format!("{}–{} of {total}", page * PAGE_SIZE + 1, ((page + 1) * PAGE_SIZE).min(total));
    let position = format!("Page {} of {pages}", page + 1);
    let (has_prev, has_next) = (page > 0, page + 1 < pages);
    let button = |enabled: bool| {
        format!(
            "display:inline-flex;align-items:center;gap:6px;height:30px;padding:0 12px;border:1px solid {DASH};border-radius:8px;background:{CARD};color:{};cursor:{};{}",
            if enabled { TEXT_SOFT } else { GHOST },
            if enabled { "pointer" } else { "default" },
            sans(600, 12.0)
        )
    };
    let (prev_style, next_style) = (button(has_prev), button(has_next));
    let text = format!("{}color:{TEXT_BODY}", sans(500, 12.5));
    rsx! {
        div { style: "display:flex;align-items:center;gap:12px;padding:10px 18px;border-top:1px solid {ROW_RULE};flex-wrap:wrap",
            span { style: "{text}", "{range}" }
            span { style: "flex:1" }
            span { style: "{text}", "{position}" }
            button { r#type: "button", disabled: !has_prev, style: "{prev_style}", onclick: move |_| if has_prev { on_page.call(page - 1) },
                Icon { name: "caret-left", size: 13.0, color: TEXT_SOFT.to_string() }
                "Previous"
            }
            button { r#type: "button", disabled: !has_next, style: "{next_style}", onclick: move |_| if has_next { on_page.call(page + 1) },
                "Next"
                Icon { name: "caret-right", size: 13.0, color: TEXT_SOFT.to_string() }
            }
        }
    }
}

#[component]
pub fn Footnote(text: String) -> Element {
    let style = format!("padding:11px 18px;background:{CARD_MUTED};{}line-height:1.6;color:{DIM}", sans(400, 10.5));
    rsx! { div { style: "{style}", "{text}" } }
}

#[component]
pub fn EmptyRow(text: String) -> Element {
    let style = format!("padding:22px 18px;border-bottom:1px solid {ROW_RULE};{}color:{TEXT_BODY};text-align:center", sans(400, 12.0));
    rsx! { div { style: "{style}", "{text}" } }
}

/// A thin moving bar: the page is refreshing but still shows the last answer.
#[component]
pub fn SweepBar(color: String, track: String) -> Element {
    rsx! {
        div { style: "position:relative;height:3px;background:{track};overflow:hidden",
            div { style: "position:absolute;inset:0;width:25%;background:linear-gradient(90deg,transparent,{color},transparent);animation:cdwm-sweep 2.6s linear infinite" }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StateKind {
    Loading,
    Auth,
    Unavailable,
    Error,
}

/// The four ways a screen can fail to have data, each with its own fix.
#[component]
pub fn StateCard(kind: StateKind, what: String, env: String, message: String, on_retry: EventHandler<()>) -> Element {
    let (icon, color, title, bg, bd) = match kind {
        StateKind::Loading => ("spinner-gap", CYAN, format!("Reading {what} from {env}…"), CARD, BORDER),
        StateKind::Auth => ("sign-in", AMBER, "Azure sign-in required".to_string(), WARN_BG, WARN_BD),
        StateKind::Unavailable => ("plugs", AMBER, format!("{what} is unavailable"), WARN_BG, WARN_BD),
        StateKind::Error => ("warning-circle", CORAL, format!("Could not read {what}"), BAD_BG, BAD_BD),
    };
    let spin = if kind == StateKind::Loading { "animation:cdwm-spin 1s linear infinite;" } else { "" };
    let title_style = format!("{}color:{TEXT_STRONG};margin-top:12px", sans(600, 16.0));
    let body_style = format!("{}line-height:1.7;color:{TEXT_BODY};margin-top:6px;overflow-wrap:anywhere", sans(400, 12.5));
    let code_style = format!("{}color:{TEXT};background:{CARD};border:1px solid {BORDER};padding:10px 12px;text-align:left;white-space:pre-wrap", mono(400, 12.0));
    let note_style = format!("{}line-height:1.7;color:{DIM}", mono(400, 10.5));
    rsx! {
        div { style: "max-width:660px;margin:48px auto 0;display:flex;flex-direction:column;gap:14px",
            div { style: "text-align:center;border-radius:12px;border:1px solid {bd};background:{bg};padding:26px 24px",
                span { style: "display:inline-block;{spin}", Icon { name: icon, size: 34.0, color: color.to_string() } }
                div { style: "{title_style}", "{title}" }
                if kind == StateKind::Loading {
                    div { style: "{body_style}", "Read-only. Cosmos, blob storage and the log stores are queried with your Azure CLI identity." }
                    div { style: "margin-top:14px", SweepBar { color: CYAN.to_string(), track: RULE.to_string() } }
                } else {
                    div { style: "{body_style}", "{message}" }
                }
            }
            if kind == StateKind::Auth {
                div { style: "display:flex;flex-direction:column;gap:8px",
                    div { style: "{note_style}", "SIGN IN WITH THE AZURE CLI, THEN RETRY" }
                    div { style: "{code_style}", "az login\naz account set --subscription <the {env} subscription>" }
                    div { style: "{note_style}", "The subscription and every resource name come from deploy/00-variables.sh for CDW_ENV={env}; nothing here is hard-coded." }
                }
            }
            if kind != StateKind::Loading {
                div { style: "display:flex;justify-content:center",
                    ActionButton { label: "Retry".to_string(), icon: "arrow-clockwise", primary: true, onclick: move |_| on_retry.call(()) }
                }
            }
        }
    }
}
