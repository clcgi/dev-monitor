use super::kit::*;
use crate::monitoring::fleet_view::paginate;
use crate::monitoring::format as fmt;
use crate::monitoring::tokens::*;
use crate::monitoring::trace_view::*;
use dioxus::prelude::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Tab {
    Steps,
    Members,
    Logs,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Viz {
    List,
    Graph,
}

/// What the graph has selected.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Pick {
    Step(StepId),
    Zone(&'static str),
}

fn fact_value(h: &Header, label: &str) -> String {
    h.facts.iter().find(|f| f.label == label).map(|f| f.value.clone()).unwrap_or_default()
}

fn card() -> String {
    format!("border-radius:12px;background:{CARD};border:1px solid {BORDER}")
}

fn gradient(tone: Status) -> (&'static str, &'static str) {
    match tone {
        Status::Waiting => (GRAD_WAIT, "0 10px 24px rgba(184,92,40,.22)"),
        Status::Stalled | Status::Failed => (GRAD_BAD, "0 10px 24px rgba(163,18,32,.22)"),
        _ => (GRAD_DONE, "0 10px 24px rgba(27,107,107,.22)"),
    }
}

fn pill(bg: &str, fg: &str) -> String {
    format!("{}padding:5px 11px;border-radius:6px;background:{bg};color:{fg};white-space:nowrap;flex-shrink:0", sans(700, 11.5))
}

fn prov_bg(p: Option<Prov>) -> &'static str {
    match p {
        Some(Prov::Observed) => OK_BG,
        Some(Prov::Inferred) => INFO_BG,
        Some(Prov::Checked) => "#E4F2F2",
        None => CARD_MUTED,
    }
}

fn zone_icon(zone: &str) -> &'static str {
    match zone {
        "landing" => "tray",
        "raw" => "database",
        "quarantine" => "shield-warning",
        "rejected" => "prohibit",
        "pace" => "export",
        _ => "stack",
    }
}

fn segmented(on: bool) -> String {
    let (bg, fg, w, shadow) = if on { (CARD, TEXT, 700, "0 1px 3px rgba(26,26,26,.16)") } else { ("transparent", TEXT_BODY, 500, "none") };
    format!(
        "display:flex;align-items:center;gap:6px;padding:6px 13px;border:none;border-radius:6px;cursor:pointer;background:{bg};color:{fg};box-shadow:{shadow};white-space:nowrap;{}",
        sans(w, 12.5)
    )
}

#[component]
pub fn TraceScreen(
    view: TraceView,
    query: String,
    checked: String,
    refreshing: bool,
    open_step: Option<StepId>,
    open_group: Option<GroupId>,
    on_step: EventHandler<StepId>,
    on_group: EventHandler<GroupId>,
    on_trace: EventHandler<String>,
) -> Element {
    let mut tab = use_signal(|| Tab::Steps);
    let mut viz = use_signal(|| Viz::List);
    let mut pick = use_signal(|| Option::<Pick>::None);
    let mut member_page = use_signal(|| 0usize);
    let mut log_page = use_signal(|| 0usize);
    // A different document: indexes and zones picked on the last one mean nothing here.
    let document = fact_value(&view.header, "DOCUMENTID");
    use_effect(use_reactive((&document,), move |_| {
        pick.set(None);
        member_page.set(0);
    }));
    let checked_clock = fmt::parse(&checked).map(fmt::clock).unwrap_or_default();

    let done = view.steps.iter().filter(|s| s.status == Status::Done).count();
    let pct = (done * 100 + view.steps.len() / 2) / view.steps.len().max(1);
    let bar = match view.header.tone {
        Status::Stalled | Status::Failed => "linear-gradient(90deg,#DC3545,#A31220)",
        Status::Waiting => "linear-gradient(90deg,#F36F27,#B85C28)",
        _ => "linear-gradient(90deg,#4CAF50,#17703C)",
    };
    let selected = pick().or(open_step.map(Pick::Step)).or(view.focus.map(Pick::Step));
    let tab_now = tab();
    let viz_now = viz();

    let title = format!("{}line-height:1.2;color:{TEXT};overflow-wrap:anywhere", sans(700, 26.0));
    let sub = format!("{}color:{DIM};margin-top:5px;overflow-wrap:anywhere", sans(500, 14.0));
    let progress = format!("{}color:{TEXT_BODY};white-space:nowrap", sans(600, 13.0));
    let section = format!("{}color:{TEXT}", sans(700, 18.0));
    let tab_title = match tab_now {
        Tab::Steps => "Steps",
        Tab::Members => "Members",
        Tab::Logs => "Logs by step",
    };
    let tab_button = |on: bool| {
        let (edge, fg, w) = if on { (ORANGE, TEXT, 700) } else { ("transparent", TEXT_BODY, 500) };
        format!("padding:0 2px 11px;border:none;border-bottom:2.5px solid {edge};background:transparent;color:{fg};cursor:pointer;white-space:nowrap;{}", sans(w, 14.0))
    };
    let current_tab = format!("{} · rev {}", view.header.key, fmt::or_dash(Some(fact_value(&view.header, "REVISION"))));

    rsx! {
        div { class: "cdwm-trace-layout",
            div { style: "min-width:0",
                div { style: "display:flex;align-items:center;gap:20px;flex-wrap:wrap",
                    div { style: "min-width:0",
                        div { style: "{title}", "{view.header.key}" }
                        div { style: "{sub}", "{view.header.sub} · matched on {view.header.matched_on} · checked {checked_clock}" }
                    }
                    span { style: "flex:1" }
                    div { style: "display:flex;align-items:center;gap:13px",
                        span { style: "{progress}", "{done} of {view.steps.len()} steps cleared" }
                        div { style: "width:132px;height:8px;border-radius:20px;background:{BORDER};overflow:hidden",
                            div { style: "height:100%;width:{pct}%;border-radius:20px;background:{bar}" }
                        }
                    }
                }
                if refreshing { div { style: "margin-top:12px", SweepBar { color: ORANGE.to_string(), track: BORDER.to_string() } } }

                {hero_cards(&view)}

                div { style: "display:flex;align-items:center;gap:16px;margin-top:28px;flex-wrap:wrap",
                    div { style: "{section}", "{tab_title}" }
                    span { style: "flex:1" }
                    if !view.candidates.is_empty() {
                        div { style: "display:flex;gap:3px;background:#E9ECEE;border-radius:8px;padding:3px;flex-wrap:wrap",
                            button { r#type: "button", style: "{segmented(true)}", title: "The document traced for “{query}”", "{current_tab}" }
                            for c in view.candidates.clone() {
                                {
                                    // By fileGuid: re-tracing a repeated documentId would land on the same row again.
                                    let id = if c.file_guid.is_empty() { c.document_id.clone() } else { c.file_guid.clone() };
                                    rsx! {
                                        button {
                                            key: "{c.document_id}-{c.file_guid}",
                                            r#type: "button",
                                            title: "{c.state} · registered {c.registered}",
                                            style: "{segmented(false)}",
                                            onclick: move |_| on_trace.call(id.clone()),
                                            "{c.key} · rev {c.revision} · {c.registered}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                div { style: "display:flex;align-items:flex-end;gap:26px;margin-top:16px;border-bottom:1px solid {BORDER};flex-wrap:wrap",
                    button { r#type: "button", style: "{tab_button(tab_now == Tab::Steps)}", onclick: move |_| tab.set(Tab::Steps), "Steps" }
                    button { r#type: "button", style: "{tab_button(tab_now == Tab::Members)}", onclick: move |_| tab.set(Tab::Members), "Members" }
                    button { r#type: "button", style: "{tab_button(tab_now == Tab::Logs)}", onclick: move |_| tab.set(Tab::Logs), "Logs" }
                    span { style: "flex:1" }
                    if tab_now == Tab::Steps {
                        div { style: "display:flex;gap:3px;background:#E9ECEE;border-radius:8px;padding:3px;margin-bottom:8px",
                            button { r#type: "button", title: "The checklist, one row per step", style: "{segmented(viz_now == Viz::List)}", onclick: move |_| viz.set(Viz::List),
                                Icon { name: "list", size: 16.0, color: TEXT_BODY.to_string() } "List"
                            }
                            button { r#type: "button", title: "Dependency network", style: "{segmented(viz_now == Viz::Graph)}", onclick: move |_| viz.set(Viz::Graph),
                                Icon { name: "tree-structure", size: 16.0, color: TEXT_BODY.to_string() } "Graph"
                            }
                        }
                    }
                }

                match (tab_now, viz_now) {
                    (Tab::Steps, Viz::List) => rsx! {
                        div { style: "display:flex;flex-direction:column;gap:8px;margin-top:16px",
                            for s in view.steps.clone() {
                                {step_card(s, open_step, move |id: StepId| { pick.set(Some(Pick::Step(id))); on_step.call(id) })}
                            }
                        }
                    },
                    (Tab::Steps, Viz::Graph) => rsx! {
                        {graph(&view, selected, move |p| {
                            pick.set(Some(p));
                            if let Pick::Step(id) = p {
                                if open_step != Some(id) { on_step.call(id) }
                            }
                        })}
                        {picked_card(&view, selected, &checked_clock)}
                    },
                    (Tab::Members, _) => members(&view, on_trace, member_page(), EventHandler::new(move |p| member_page.set(p))),
                    (Tab::Logs, _) => logs(
                        &view,
                        open_group,
                        EventHandler::new(move |g| {
                            log_page.set(0);
                            on_group.call(g)
                        }),
                        log_page(),
                        EventHandler::new(move |p| log_page.set(p)),
                    ),
                }
            }

            aside { class: "cdwm-trace-side", style: "min-width:0;{card()};padding:24px 20px 30px",
                {right_now(&view, &checked_clock)}
                {where_bytes(&view)}
                {declared(&view)}
                {unknowns(&view)}
            }
        }
    }
}

fn hero_cards(view: &TraceView) -> Element {
    let h = &view.header;
    let hero = &view.hero;
    let (grad, shadow) = gradient(hero.tone);
    let kicker = format!("{}letter-spacing:.09em;color:rgba(255,255,255,.95);text-transform:uppercase", sans(700, 11.5));
    let tile = "width:38px;height:38px;border-radius:10px;background:rgba(255,255,255,.24);display:flex;align-items:center;justify-content:center";
    let bubble = |pos: &str, size: u32, alpha: &str| format!("position:absolute;{pos};width:{size}px;height:{size}px;border-radius:50%;background:rgba(255,255,255,{alpha})");
    let doc_id = fact_value(h, "DOCUMENTID");
    let source = fact_value(h, "SOURCE");
    let rev = fact_value(h, "REVISION");
    let key_style = format!("position:relative;{}line-height:1.35;color:#fff;margin-top:18px;overflow-wrap:anywhere", mono(600, 19.0));
    let id_style = format!("position:relative;{}color:rgba(255,255,255,.85);margin-top:8px;overflow-wrap:anywhere", mono(500, 12.0));
    let hero_title = format!("position:relative;{}line-height:1.4;color:#fff;margin-top:18px", sans(700, 19.0));
    let (bubble_a, bubble_b, bubble_c) = (
        bubble("right:-38px;top:-38px", 148, ".13"),
        bubble("right:16px;bottom:-50px", 100, ".09"),
        bubble("right:-42px;bottom:-42px", 156, ".12"),
    );
    let hero_body = format!("position:relative;{}line-height:1.6;color:rgba(255,255,255,.95);margin-top:8px", sans(500, 13.0));
    rsx! {
        div { style: "display:grid;grid-template-columns:repeat(auto-fit,minmax(276px,1fr));gap:16px;margin-top:20px",
            div { style: "position:relative;border-radius:14px;padding:20px 22px 22px;overflow:hidden;background:{GRAD_DOC};box-shadow:0 10px 24px rgba(184,92,40,.22)",
                div { style: "{bubble_a}" }
                div { style: "{bubble_b}" }
                div { style: "position:relative;display:flex;align-items:center;justify-content:space-between;gap:12px",
                    span { style: "{tile}", Icon { name: "file-text", size: 20.0, color: "#fff".to_string() } }
                    span { style: "{kicker}", "{source} · rev {rev}" }
                }
                div { style: "{key_style}", "{h.key}" }
                div { style: "{id_style}", "{doc_id}" }
            }
            div { style: "position:relative;border-radius:14px;padding:20px 22px 22px;overflow:hidden;background:{grad};box-shadow:{shadow}",
                div { style: "{bubble_c}" }
                div { style: "position:relative;display:flex;align-items:center;justify-content:space-between;gap:12px",
                    span { style: "{tile}", Icon { name: hero.icon, size: 20.0, color: "#fff".to_string() } }
                    span { style: "{kicker}", "{hero.kicker}" }
                }
                div { style: "{hero_title}", "{hero.title}" }
                div { style: "{hero_body}", "{hero.body}" }
            }
        }
    }
}

fn step_card(s: Step, open_step: Option<StepId>, on_toggle: impl FnMut(StepId) + 'static) -> Element {
    let look = status_look(s.status);
    let open = open_step == Some(s.id);
    let (border, shadow) = if open { ("#E5C6AE", "0 5px 14px rgba(184,92,40,.12)") } else { (BORDER, "none") };
    let label_fg = if matches!(s.status, Status::NotBuilt | Status::Skipped) { TEXT_BODY } else { TEXT };
    let label = format!("{}color:{label_fg}", sans(700, 14.5));
    let sub = format!("{}color:{DIM};margin-top:2px;overflow-wrap:anywhere", sans(500, 12.5));
    let (prov_fg, prov_icon) = s.prov.map(prov_look).unwrap_or((TEXT_BODY, "minus"));
    let prov_label = s.prov.map(|p| p.label()).unwrap_or("n/a");
    let tip = s.prov.map(|p| p.tip()).unwrap_or("Not applicable to this step");
    let id = s.id;
    let mut on_toggle = on_toggle;
    let at = if s.at == fmt::DASH { "no timestamp".to_string() } else { s.at.clone() };
    rsx! {
        div {
            key: "{s.label}",
            style: "border-radius:12px;background:{CARD};border:1px solid {border};border-left:3px solid {look.bd};padding:14px 17px;cursor:pointer;box-shadow:{shadow}",
            onclick: move |_| on_toggle(id),
            div { style: "display:flex;align-items:center;gap:14px;flex-wrap:wrap",
                span { style: "width:40px;height:40px;border-radius:10px;background:{look.bg};display:flex;align-items:center;justify-content:center;flex-shrink:0",
                    Icon { name: look.icon, size: 21.0, color: look.fg.to_string() }
                }
                div { style: "flex:1;min-width:150px",
                    div { style: "{label}", "{s.label}" }
                    div { style: "{sub}", "{s.detail} · {at}" }
                }
                span { title: "{tip}", style: "display:inline-flex;align-items:center;gap:6px;padding:5px 11px;border-radius:6px;background:{prov_bg(s.prov)};flex-shrink:0;cursor:help",
                    Icon { name: prov_icon, size: 15.0, color: prov_fg.to_string() }
                    span { style: "{sans(600, 11.5)}color:{prov_fg};white-space:nowrap", "{prov_label}" }
                }
                span { style: "{pill(look.bg, look.fg)}", "{s.status.label()}" }
            }
            if open {
                div { style: "padding:14px 0 2px 54px",
                    p { style: "margin:0;{sans(500, 14.0)}line-height:1.7;color:{TEXT_SOFT};max-width:92ch", "{s.why}" }
                    {meta_grid(&s.meta)}
                }
            }
        }
    }
}

fn meta_grid(meta: &[Fact]) -> Element {
    if meta.is_empty() {
        return rsx! {};
    }
    let label = format!("{}letter-spacing:.06em;color:{DIM};text-transform:uppercase", sans(600, 11.0));
    let value = format!("{}color:{TEXT};margin-top:4px;overflow-wrap:anywhere", mono(500, 12.5));
    rsx! {
        div { style: "display:grid;grid-template-columns:repeat(auto-fit,minmax(158px,1fr));gap:12px;margin-top:15px;padding:14px 16px;border-radius:10px;background:{CARD_MUTED}",
            for m in meta.iter().cloned() {
                div { key: "{m.label}",
                    div { style: "{label}", "{m.label}" }
                    div { style: "{value}", "{m.value}" }
                }
            }
        }
    }
}

const NW: f64 = 134.0;
const NH: f64 = 92.0;
const GAP_X: f64 = 50.0;
const GAP_Y: f64 = 72.0;

/// (stroke, width, dash) of the edge INTO a step, from that step's own facts.
fn edge_into(s: &Step) -> (&'static str, f64, &'static str) {
    match (s.status, s.prov) {
        (Status::Waiting, _) => (ORANGE_MID, 2.4, "6 5"),
        (Status::Stalled | Status::Failed, _) => (BAD_BD, 2.4, "5 6"),
        (Status::Done, Some(Prov::Inferred)) => (BLUE, 2.4, "7 5"),
        (Status::Done, _) => (OK_BD, 2.4, "0"),
        _ => ("#C7CCD0", 1.4, "3 6"),
    }
}

fn graph(view: &TraceView, selected: Option<Pick>, on_pick: impl FnMut(Pick) + Copy + 'static) -> Element {
    let col = |i: usize| i as f64 * (NW + GAP_X);
    let width = col(view.steps.len().saturating_sub(1)) + NW + 8.0;
    let height = NH + GAP_Y + 72.0 + 8.0;
    let head = format!("{}color:{TEXT}", sans(700, 14.0));
    let note = format!("{}color:{DIM}", sans(500, 12.5));
    let arrow = |x: f64, y: f64, dx: f64, dy: f64| {
        let len = (dx * dx + dy * dy).sqrt().max(1.0);
        let (ux, uy) = (dx / len, dy / len);
        format!("M{},{} L{x},{y} L{},{}", x - 7.0 * ux - 4.0 * uy, y - 7.0 * uy + 4.0 * ux, x - 7.0 * ux + 4.0 * uy, y - 7.0 * uy - 4.0 * ux)
    };
    let mut heads: Vec<(String, &'static str)> = vec![];
    let mut edges: Vec<(String, &'static str, f64, &'static str)> = view
        .steps
        .windows(2)
        .enumerate()
        .map(|(i, pair)| {
            let (stroke, w, dash) = edge_into(&pair[1]);
            let (x1, x2, y) = (col(i) + NW, col(i + 1), NH / 2.0);
            heads.push((arrow(x2, y, 1.0, 0.0), stroke));
            (format!("M{x1},{y} L{x2},{y}"), stroke, w, dash)
        })
        .collect();
    // Terminal siblings hang below the step that would send a document there.
    let branches = [("rejected", 1usize), ("quarantine", 3usize)];
    for (_, from) in branches {
        let nx = col(from) + (NW + GAP_X) / 2.0;
        let (x1, x2, y2) = (col(from) + NW / 2.0, nx + NW / 2.0, NH + GAP_Y);
        heads.push((arrow(x2, y2, x2 - x1, y2 - NH), "#C7CCD0"));
        edges.push((format!("M{x1},{NH} L{x2},{y2}"), "#C7CCD0", 1.4, "3 6"));
    }
    let legend = [
        (format!("2.5px solid {OK_BD}"), "recorded"),
        (format!("2.5px dashed {BLUE}"), "deduced — no audit event"),
        (format!("2.5px dashed {ORANGE_MID}"), "waiting"),
        (format!("2.5px dashed {BAD_BD}"), "stopped"),
        ("2px dotted #C7CCD0".to_string(), "not taken / not built"),
    ];
    rsx! {
        div { style: "margin-top:16px;{card()};overflow:hidden",
            div { style: "display:flex;align-items:center;gap:14px;padding:14px 18px;border-bottom:1px solid {ROW_RULE};flex-wrap:wrap",
                span { style: "{head}", "Dependency network" }
                span { style: "{note}", "the solid path is what this document actually took · terminal siblings hang below" }
                span { style: "flex:1" }
                span { style: "{note}", "scroll sideways" }
            }
            div { style: "overflow-x:auto;padding:24px 18px 20px",
                div { style: "position:relative;width:{width}px;height:{height}px",
                    svg { width: "{width}", height: "{height}", style: "position:absolute;inset:0;overflow:visible",
                        for (i, (d, stroke, w, dash)) in edges.into_iter().enumerate() {
                            path { key: "{i}", d: "{d}", fill: "none", stroke: "{stroke}", stroke_width: "{w}", stroke_dasharray: "{dash}" }
                        }
                        for (i, (d, stroke)) in heads.into_iter().enumerate() {
                            path { key: "head-{i}", d: "{d}", fill: "none", stroke: "{stroke}", stroke_width: "1.8" }
                        }
                    }
                    for (i, s) in view.steps.iter().cloned().enumerate() {
                        {
                            let look = status_look(s.status);
                            let on = selected == Some(Pick::Step(s.id));
                            let (bg, border, shadow) = if on { (SELECTED_BG, ORANGE_DEEP, "0 6px 16px rgba(184,92,40,.18)") } else { (CARD, BORDER, "0 1px 2px rgba(26,26,26,.05)") };
                            let style_border = if s.status == Status::NotBuilt { "dashed" } else { "solid" };
                            let (prov_fg, prov_icon) = s.prov.map(prov_look).unwrap_or((DIM, "minus"));
                            let id = s.id;
                            let mut on_pick = on_pick;
                            let label_fg = if matches!(s.status, Status::NotBuilt | Status::Skipped) { TEXT_BODY } else { TEXT };
                            let left = col(i);
                            let ordinal = format!("{:02}", i + 1);
                            rsx! {
                                div {
                                    key: "{s.label}",
                                    style: "position:absolute;left:{left}px;top:0;width:{NW}px;min-height:{NH}px;padding:11px 12px;border-radius:10px;background:{bg};border:1px {style_border} {border};border-left:3px solid {look.bd};cursor:pointer;display:flex;flex-direction:column;gap:5px;box-shadow:{shadow}",
                                    onclick: move |_| on_pick(Pick::Step(id)),
                                    div { style: "display:flex;align-items:center;gap:7px",
                                        Icon { name: look.icon, size: 16.0, color: look.fg.to_string() }
                                        span { style: "{mono(600, 10.5)}color:{DIM}", "{ordinal}" }
                                        span { style: "flex:1" }
                                        Icon { name: prov_icon, size: 14.0, color: prov_fg.to_string() }
                                    }
                                    div { style: "{sans(700, 12.5)}line-height:1.28;color:{label_fg}", "{s.label}" }
                                    div { style: "{sans(600, 10.5)}letter-spacing:.05em;text-transform:uppercase;color:{look.fg}", "{s.status.label()}" }
                                    div { style: "{mono(500, 10.5)}color:{DIM}", "{s.at}" }
                                }
                            }
                        }
                    }
                    for (zone, from) in branches {
                        {
                            let row = view.zones.iter().find(|z| z.zone == zone).cloned();
                            let tag = row.as_ref().map(|z| z.tag.clone()).unwrap_or_default();
                            let taken = row.as_ref().is_some_and(|z| z.present);
                            let on = selected == Some(Pick::Zone(zone));
                            let (bg, border) = if on { (SELECTED_BG, ORANGE_DEEP) } else { (CARD_SOFT, DASH) };
                            let nx = col(from) + (NW + GAP_X) / 2.0;
                            let ny = NH + GAP_Y;
                            let mut on_pick = on_pick;
                            rsx! {
                                div {
                                    key: "{zone}",
                                    style: "position:absolute;left:{nx}px;top:{ny}px;width:{NW}px;min-height:72px;padding:11px 12px;border-radius:10px;background:{bg};border:1px dashed {border};border-left:3px solid {DASH};cursor:pointer;display:flex;flex-direction:column;gap:5px",
                                    onclick: move |_| on_pick(Pick::Zone(zone)),
                                    div { style: "display:flex;align-items:center;gap:7px",
                                        Icon { name: zone_icon(zone), size: 16.0, color: TEXT_BODY.to_string() }
                                        span { style: "{mono(600, 10.5)}color:{DIM}", "—" }
                                        span { style: "flex:1" }
                                        Icon { name: "crosshair", size: 14.0, color: TEAL.to_string() }
                                    }
                                    div { style: "{sans(700, 12.5)}color:{TEXT_SOFT}", "{zone}" }
                                    div { style: "{sans(600, 10.5)}letter-spacing:.05em;text-transform:uppercase;color:{TEXT_BODY}", "{tag}" }
                                    div { style: "{mono(500, 10.5)}color:{DIM}", if taken { "taken" } else { "not taken" } }
                                }
                            }
                        }
                    }
                }
            }
            div { style: "display:flex;gap:20px;padding:12px 18px;border-top:1px solid {ROW_RULE};background:{CARD_SOFT};flex-wrap:wrap",
                for (line, label) in legend {
                    span { key: "{label}", style: "display:inline-flex;align-items:center;gap:8px;white-space:nowrap",
                        span { style: "width:22px;border-top:{line};flex-shrink:0" }
                        span { style: "{sans(500, 12.0)}color:{TEXT_BODY}", "{label}" }
                    }
                }
            }
        }
    }
}

/// The selection under the graph.
fn picked_card(view: &TraceView, selected: Option<Pick>, checked: &str) -> Element {
    struct Card {
        label: String,
        icon: &'static str,
        tile_bg: &'static str,
        tile_fg: &'static str,
        edge: &'static str,
        status: String,
        pill: (&'static str, &'static str),
        prov: Option<Prov>,
        at: String,
        body: String,
        meta: Vec<Fact>,
    }
    let fact = |label: &str, value: String| Fact { label: label.into(), value };
    let c = match selected {
        Some(Pick::Zone(zone)) => {
            let Some(z) = view.zones.iter().find(|z| z.zone == zone) else { return rsx! {} };
            Card {
                label: z.zone.clone(),
                icon: zone_icon(zone),
                tile_bg: CARD_MUTED,
                tile_fg: TEXT_BODY,
                edge: DASH,
                status: z.tag.clone(),
                pill: if z.present { (OK_BG, CYAN) } else { (CARD_MUTED, TEXT_BODY) },
                prov: Some(Prov::Checked),
                at: checked.to_string(),
                body: z.note.clone(),
                meta: vec![fact("OBJECT", if z.present { "present" } else { "absent" }.into()), fact("DETAIL", z.path.clone())],
            }
        }
        Some(Pick::Step(id)) => {
            let Some(s) = view.steps.iter().find(|s| s.id == id) else { return rsx! {} };
            let look = status_look(s.status);
            Card {
                label: s.label.to_string(),
                icon: look.icon,
                tile_bg: look.bg,
                tile_fg: look.fg,
                edge: look.bd,
                status: s.status.label().into(),
                pill: (look.bg, look.fg),
                prov: s.prov,
                at: s.at.clone(),
                body: s.why.clone(),
                meta: s.meta.clone(),
            }
        }
        None => return rsx! {},
    };
    let (prov_fg, prov_icon) = c.prov.map(prov_look).unwrap_or((TEXT_BODY, "minus"));
    let prov_label = c.prov.map(|p| p.label()).unwrap_or("not applicable");
    rsx! {
        div { style: "margin-top:12px;{card()};border-left:3px solid {c.edge};padding:16px 18px",
            div { style: "display:flex;align-items:center;gap:11px;flex-wrap:wrap",
                span { style: "width:36px;height:36px;border-radius:10px;background:{c.tile_bg};display:flex;align-items:center;justify-content:center;flex-shrink:0",
                    Icon { name: c.icon, size: 19.0, color: c.tile_fg.to_string() }
                }
                span { style: "{sans(700, 15.0)}color:{TEXT};overflow-wrap:anywhere", "{c.label}" }
                span { style: "{pill(c.pill.0, c.pill.1)}", "{c.status}" }
                span { style: "display:inline-flex;align-items:center;gap:6px;padding:4px 11px;border-radius:6px;background:{prov_bg(c.prov)}",
                    Icon { name: prov_icon, size: 15.0, color: prov_fg.to_string() }
                    span { style: "{sans(600, 11.5)}color:{prov_fg};white-space:nowrap", "{prov_label}" }
                }
                span { style: "flex:1" }
                span { style: "{mono(500, 12.0)}color:{DIM};white-space:nowrap", "{c.at}" }
            }
            p { style: "margin:12px 0 0;{sans(500, 14.0)}line-height:1.7;color:{TEXT_SOFT};max-width:96ch", "{c.body}" }
            {meta_grid(&c.meta)}
        }
    }
}

fn members(view: &TraceView, on_trace: EventHandler<String>, page: usize, on_page: EventHandler<usize>) -> Element {
    let Some(tree) = view.tree.clone() else {
        let why = view.steps.iter().find(|s| s.id == StepId::Extracted).map(|s| s.why.clone()).unwrap_or_default();
        return rsx! {
            div { style: "margin-top:16px;{card()};padding:36px 24px;text-align:center",
                span { style: "width:52px;height:52px;border-radius:13px;background:{CARD_MUTED};display:inline-flex;align-items:center;justify-content:center",
                    Icon { name: "file-text", size: 26.0, color: DIM.to_string() }
                }
                div { style: "{sans(700, 16.0)}color:{TEXT};margin-top:14px", "Not an archive" }
                div { style: "{sans(500, 13.5)}line-height:1.7;color:{TEXT_BODY};margin:7px auto 0;max-width:52ch", "{why}" }
            }
        };
    };
    let (note_bg, note_fg, note_icon) = match tree.tone {
        Tone::Ok => (OK_BG, CYAN, "check-circle"),
        Tone::Bad => (BAD_BG, CORAL, "warning-circle"),
        _ => (WARN_BG, AMBER, "warning"),
    };
    let shown = paginate(&tree.rows, page);
    rsx! {
        div { style: "margin-top:16px;display:flex;flex-direction:column;gap:8px",
            div { style: "{sans(700, 12.5)}letter-spacing:.04em;color:{note_fg}", "{tree.banner}" }
            for (i, r) in shown.rows.clone().into_iter().enumerate() {
                {
                    let (tile_bg, tile_fg, edge) = match (r.reached, r.container) {
                        (true, true) => (INFO_BG, BLUE, BLUE),
                        (true, false) => (OK_BG, CYAN, OK_BD),
                        _ => (BAD_BG, CORAL, BAD_BD),
                    };
                    let tag = match (r.reached, r.container, r.rejected) {
                        (_, _, true) => "refused",
                        (true, true, false) => "opened",
                        (false, true, false) => "not opened",
                        (true, false, false) => "in raw",
                        (false, false, false) => "not reached",
                    };
                    let icon = if r.rejected { "prohibit" } else if r.container { "file-zip" } else { "file-text" };
                    let clickable = r.reached && !r.document_id.is_empty();
                    let cursor = if clickable { "pointer" } else { "default" };
                    let id = r.document_id.clone();
                    let name_fg = if r.reached { TEXT } else { TEXT_BODY };
                    let kind = if r.container { " · nested archive" } else { "" };
                    let indent = r.depth.min(4) * 22;
                    let size_style = format!("{}color:{};min-width:74px;text-align:right;flex-shrink:0", mono(600, 12.0), if r.reached { TEXT_SOFT } else { DIM });
                    rsx! {
                        div {
                            key: "{i}-{r.name}",
                            title: if clickable { format!("Trace member {}", r.document_id) } else { r.note.clone() },
                            style: "display:flex;align-items:center;gap:14px;flex-wrap:wrap;border-radius:12px;background:{CARD};border:1px solid {BORDER};border-left:3px solid {edge};padding:13px 17px;margin-left:{indent}px;cursor:{cursor}",
                            onclick: move |_| if clickable { on_trace.call(id.clone()) },
                            span { style: "width:38px;height:38px;border-radius:10px;background:{tile_bg};display:flex;align-items:center;justify-content:center;flex-shrink:0",
                                Icon { name: icon, size: 20.0, color: tile_fg.to_string() }
                            }
                            div { style: "flex:1;min-width:140px",
                                div { style: "{mono(600, 13.5)}color:{name_fg};overflow-wrap:anywhere", "{r.name}" }
                                div { style: "{sans(500, 12.0)}color:{DIM};margin-top:2px", "depth {r.depth}{kind} · {r.note}" }
                            }
                            span { style: "{pill(tile_bg, tile_fg)}", "{tag}" }
                            span { style: "{size_style}", "{r.size}" }
                        }
                    }
                }
            }
            Pager { page: shown.page, pages: shown.pages, total: shown.total, on_page: move |p| on_page.call(p) }
            div { style: "display:flex;gap:12px;padding:15px 18px;border-radius:12px;background:{note_bg};border-left:3px solid {note_fg};margin-top:4px",
                Icon { name: note_icon, size: 20.0, color: note_fg.to_string() }
                span { style: "{sans(500, 13.5)}line-height:1.7;color:{TEXT_SOFT}", "{tree.note} {tree.footer}" }
            }
        }
    }
}

fn logs(view: &TraceView, open_group: Option<GroupId>, on_group: EventHandler<GroupId>, page: usize, on_page: EventHandler<usize>) -> Element {
    let (bg, fg) = if view.logs_available { (WARN_BG, AMBER) } else { (BAD_BG, CORAL) };
    rsx! {
        div { style: "margin-top:16px;display:flex;flex-direction:column;gap:10px",
            div { style: "display:flex;gap:12px;padding:15px 18px;border-radius:12px;background:{bg};border-left:3px solid {fg}",
                Icon { name: "clock", size: 20.0, color: fg.to_string() }
                span { style: "{sans(500, 13.5)}line-height:1.7;color:{TEXT_SOFT}", "{view.log_note}" }
            }
            for g in view.groups.clone() {
                {
                    let open = open_group == Some(g.id);
                    let chevron = if open { "transform:rotate(180deg);" } else { "" };
                    let (lag_bg, lag_fg) = if g.quiet { (CARD_MUTED, TEXT_BODY) } else { (WARN_BG, AMBER) };
                    let id = g.id;
                    rsx! {
                        div { key: "{g.label}", style: "{card()};overflow:hidden",
                            div { style: "display:flex;align-items:center;gap:12px;padding:14px 17px;cursor:pointer;flex-wrap:wrap", onclick: move |_| on_group.call(id),
                                span { style: "display:inline-flex;{chevron}transition:transform .15s", Icon { name: "caret-down", size: 20.0, color: DIM.to_string() } }
                                span { style: "{sans(700, 14.0)}color:{TEXT};flex:1;min-width:90px", "{g.label}" }
                                span { style: "{sans(600, 11.5)}padding:4px 11px;border-radius:6px;background:{lag_bg};color:{lag_fg};white-space:nowrap", "{g.lag}" }
                            }
                            if open {
                                div { style: "padding:0 17px 14px 49px",
                                    div { style: "{mono(500, 11.5)}color:{DIM};padding-bottom:8px", "as of {g.as_of}" }
                                    for (i, e) in paginate(&g.entries, page).rows.into_iter().enumerate() {
                                        {
                                            let text_fg = match e.kind {
                                                LogKind::None => DIM,
                                                LogKind::Warn => AMBER,
                                                LogKind::Error => CORAL,
                                                LogKind::Audit => CYAN,
                                                LogKind::Info => TEXT_SOFT,
                                            };
                                            rsx! {
                                                div { key: "{i}", style: "display:grid;grid-template-columns:128px 1fr;gap:12px;padding:8px 0;border-top:1px solid {ROW_RULE}",
                                                    span { style: "{mono(500, 11.5)}color:{DIM}", "{e.at}" }
                                                    span { style: "{mono(400, 12.0)}line-height:1.7;color:{text_fg};overflow-wrap:anywhere", "{e.text}" }
                                                }
                                            }
                                        }
                                    }
                                    {
                                        let shown = paginate(&g.entries, page);
                                        rsx! { Pager { page: shown.page, pages: shown.pages, total: shown.total, on_page: move |p| on_page.call(p) } }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn side_head(title: &str, sub: &str) -> Element {
    rsx! {
        div { style: "{sans(700, 16.0)}color:{TEXT}", "{title}" }
        div { style: "{sans(500, 12.5)}color:{DIM};margin-top:3px", "{sub}" }
    }
}

fn right_now(view: &TraceView, checked: &str) -> Element {
    let live = &view.live;
    let (grad, shadow) = gradient(live.tone);
    let pulse = if live.tone == Status::Waiting { "animation:cdwm-fade 2s ease-in-out infinite;" } else { "" };
    rsx! {
        div {
            div { style: "display:flex;align-items:center;gap:10px;flex-wrap:wrap",
                span { style: "{sans(700, 17.0)}color:{TEXT}", "Right now" }
                span { style: "flex:1" }
                span { style: "{mono(500, 11.5)}color:{DIM}", "{checked}" }
            }
            div { style: "border-radius:12px;padding:16px 18px;margin-top:13px;background:{grad};box-shadow:{shadow}",
                div { style: "display:flex;align-items:center;gap:11px",
                    span { style: "width:9px;height:9px;border-radius:50%;background:#fff;flex-shrink:0;{pulse}" }
                    span { style: "{sans(700, 12.5)}letter-spacing:.05em;color:#fff;text-transform:uppercase", "{live.state}" }
                }
                div { style: "display:flex;align-items:flex-end;gap:12px;margin-top:13px;flex-wrap:wrap",
                    span { style: "{mono(600, 26.0)}line-height:1;color:#fff", "{live.big}" }
                    span { style: "{sans(600, 12.5)}color:rgba(255,255,255,.95);padding-bottom:3px", "{live.big_note}" }
                }
                div { style: "{sans(500, 13.0)}line-height:1.6;color:rgba(255,255,255,.96);margin-top:10px", "{live.note}" }
            }
        }
    }
}

fn where_bytes(view: &TraceView) -> Element {
    let mut zones = view.zones.clone();
    zones.sort_by_key(|z| ZONES.iter().position(|k| *k == z.zone));
    rsx! {
        div {
            {side_head("Where the bytes are", "absence is recorded with a reason")}
            div { style: "display:flex;flex-direction:column;gap:2px;margin-top:12px",
                for z in zones {
                    {
                        let here = view.zone_at == Some(z.zone.as_str());
                        let (bg, tile_bg, tile_fg, pill_bg, pill_fg) = if here {
                            (SELECTED_BG, "#F5E6D8", ORANGE_DEEP, "#F5E6D8", ORANGE_DEEP)
                        } else if !z.built {
                            ("transparent", CARD_SOFT, DIM, CARD_MUTED, TEXT_BODY)
                        } else if z.present {
                            ("transparent", OK_BG, CYAN, OK_BG, CYAN)
                        } else {
                            ("transparent", CARD_MUTED, DIM, CARD_MUTED, TEXT_BODY)
                        };
                        let meta = if z.present { format!("{} · {}", z.size, z.path) } else { z.note.clone() };
                        let name_fg = if z.built { TEXT } else { DIM };
                        rsx! {
                            div { key: "{z.zone}", title: "{z.note}", style: "display:flex;align-items:center;gap:12px;padding:10px 11px;border-radius:8px;background:{bg}",
                                span { style: "width:32px;height:32px;border-radius:9px;background:{tile_bg};display:flex;align-items:center;justify-content:center;flex-shrink:0",
                                    Icon { name: zone_icon(&z.zone), size: 17.0, color: tile_fg.to_string() }
                                }
                                div { style: "flex:1;min-width:0",
                                    div { style: "{mono(600, 12.5)}color:{name_fg}", "{z.zone}" }
                                    div { style: "{sans(500, 11.5)}color:{DIM};margin-top:1px;overflow-wrap:anywhere", "{meta}" }
                                }
                                span { style: "{sans(700, 10.5)}padding:4px 9px;border-radius:5px;background:{pill_bg};color:{pill_fg};white-space:nowrap;flex-shrink:0", "{z.tag}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn declared(view: &TraceView) -> Element {
    let (note_bg, note_fg, note_icon) = match view.meta_tone {
        Tone::Bad => (BAD_BG, CORAL, "warning-circle"),
        Tone::Ok => (OK_BG, CYAN, "check-circle"),
        _ => (INFO_BG, BLUE, "info"),
    };
    rsx! {
        div {
            {side_head("Declared vs resolved", "disagreements are the interesting part")}
            div { style: "display:flex;flex-direction:column;margin-top:12px",
                for (i, m) in view.meta.clone().into_iter().enumerate() {
                    {
                        let resolved_fg = if m.flag { CORAL } else if m.pending { AMBER } else { TEXT };
                        rsx! {
                            div { key: "{i}-{m.label}", style: "padding:10px 0;border-bottom:1px solid {ROW_RULE}",
                                div { style: "{sans(600, 12.0)}color:{DIM}", "{m.label}" }
                                div { style: "display:flex;align-items:center;gap:8px;margin-top:4px;flex-wrap:wrap",
                                    span { style: "{mono(500, 11.5)}color:{TEXT_SOFT};overflow-wrap:anywhere", "{m.declared}" }
                                    Icon { name: "arrow-right", size: 15.0, color: DIM.to_string() }
                                    span { style: "display:inline-flex;align-items:center;gap:5px;min-width:0",
                                        if m.flag { Icon { name: "warning-circle", size: 15.0, color: CORAL.to_string() } }
                                        span { style: "{mono(600, 11.5)}color:{resolved_fg};overflow-wrap:anywhere", "{m.resolved}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            div { style: "display:flex;gap:11px;padding:14px 16px;border-radius:10px;background:{note_bg};border-left:3px solid {note_fg};margin-top:13px",
                Icon { name: note_icon, size: 19.0, color: note_fg.to_string() }
                span { style: "{sans(500, 13.0)}line-height:1.7;color:{TEXT_SOFT}", "{view.meta_note}" }
            }
        }
    }
}

fn unknowns(view: &TraceView) -> Element {
    rsx! {
        div {
            div { style: "display:flex;align-items:center;gap:9px",
                span { style: "{sans(700, 16.0)}color:{TEXT}", "Unknowns" }
                span { style: "{sans(700, 11.5)}padding:3px 9px;border-radius:5px;background:{BAD_BG};color:{CORAL}", "{view.gaps.len()}" }
            }
            div { style: "{sans(500, 12.5)}color:{DIM};margin-top:3px", "what this view could not determine" }
            div { style: "display:flex;flex-direction:column;gap:10px;margin-top:12px",
                if view.gaps.is_empty() {
                    div { style: "{sans(500, 12.5)}color:{TEXT_BODY}", "Nothing: every fact here was read or recorded." }
                }
                for (i, g) in view.gaps.iter().cloned().enumerate() {
                    div { key: "{i}", style: "padding:13px 15px;border-radius:10px;background:{CARD_SOFT};border:1px solid #E5E7EA",
                        div { style: "{sans(700, 13.0)}line-height:1.5;color:{TEXT}", "{g.short}" }
                        div { style: "{sans(500, 12.5)}line-height:1.65;color:{TEXT_BODY};margin-top:5px", "{g.why}" }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitoring::model::{Doc, Env, Trace};
    use dioxus::dioxus_core::{ElementId, Mutation};
    use std::cell::RefCell;

    thread_local! {
        static OPENED: RefCell<Vec<Option<StepId>>> = const { RefCell::new(Vec::new()) };
        static VIEW: RefCell<Option<TraceView>> = const { RefCell::new(None) };
    }

    fn fixture_view() -> TraceView {
        if let Some(view) = VIEW.with(|v| v.borrow().clone()) {
            return view;
        }
        let root = Doc { document_id: "D1".into(), revision_id: "00".into(), state: "Dispatched".into(), ..Default::default() };
        let trace = Trace { env: Env { name: "dev".into(), ..Default::default() }, checked_at: "2026-08-14T09:00:00Z".into(), root: Some(root), ..Default::default() };
        crate::monitoring::trace_view::build(&trace, chrono::Utc::now()).expect("a root")
    }

    /// Wires on_step exactly as MonitoringApp does, and records every state it reaches.
    #[component]
    fn Harness() -> Element {
        let mut open_step = use_signal(|| Option::<StepId>::None);
        rsx! {
            TraceScreen {
                view: fixture_view(),
                query: "D1".to_string(),
                checked: "2026-08-14T09:00:00Z".to_string(),
                refreshing: false,
                open_step: open_step(),
                open_group: None,
                on_step: move |id: StepId| {
                    let open = open_step() == Some(id);
                    open_step.set(if open { None } else { Some(id) });
                    OPENED.with(|o| o.borrow_mut().push(*open_step.peek()));
                },
                on_group: move |_| {},
                on_trace: move |_| {},
            }
        }
    }

    fn click(dom: &VirtualDom, id: ElementId) {
        let data = dioxus::html::PlatformEventData::new(Box::new(dioxus::html::SerializedMouseData::default()));
        dom.runtime().handle_event("click", dioxus::dioxus_core::Event::new(std::rc::Rc::new(data) as std::rc::Rc<dyn std::any::Any>, true), id);
    }

    fn listeners(mutations: dioxus::dioxus_core::Mutations) -> Vec<ElementId> {
        mutations
            .edits
            .into_iter()
            .filter_map(|m| match m {
                Mutation::NewEventListener { name, id } if name == "click" => Some(id),
                _ => None,
            })
            .collect()
    }

    fn last_opened() -> Option<Option<StepId>> {
        OPENED.with(|o| o.borrow().last().cloned())
    }

    /// The same check over a recorded real answer: CDW_PROBE_SAMPLES/curated-root-trace.json. Opt-in.
    #[test]
    #[ignore]
    fn every_step_card_toggles_on_a_recorded_trace() {
        let dir = std::path::PathBuf::from(std::env::var("CDW_PROBE_SAMPLES").expect("CDW_PROBE_SAMPLES"));
        let answer = std::fs::read_to_string(dir.join("curated-root-trace.json")).expect("curated-root-trace.json");
        let crate::monitoring::model::Probe::Ok { data } = crate::monitoring::probe::parse::<Trace>(&answer, "") else { panic!("not ok") };
        VIEW.with(|v| *v.borrow_mut() = crate::monitoring::trace_view::build(&data, chrono::Utc::now()));
        every_step_card_toggles_its_own_step();
    }

    #[test]
    fn every_step_card_toggles_its_own_step() {
        dioxus::html::set_event_converter(Box::new(dioxus::html::SerializedHtmlEventConverter));
        let count = listeners(VirtualDom::new(Harness).rebuild_to_vec()).len();
        let mut opened = vec![];
        // A fresh page per listener: a tab button would otherwise unmount the cards clicked after it.
        for index in 0..count {
            OPENED.with(|o| o.borrow_mut().clear());
            let mut dom = VirtualDom::new(Harness);
            let id = listeners(dom.rebuild_to_vec())[index];
            click(&dom, id);
            dom.render_immediate_to_vec();
            let Some(Some(step)) = last_opened() else { continue };
            click(&dom, id);
            dom.render_immediate_to_vec();
            assert_eq!(last_opened(), Some(None), "a second click on {step:?} closes it");
            opened.push(step);
        }
        opened.sort();
        let steps = fixture_view().steps.len();
        assert_eq!(opened.len(), steps, "one working card per step");
        opened.dedup();
        assert_eq!(opened.len(), steps, "each card opens its own step");
    }
}
