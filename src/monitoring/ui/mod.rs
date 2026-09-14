mod fleet;
mod home;
pub mod kit;
mod shell;
mod trace;

use crate::monitoring::fleet_view;
use crate::monitoring::model::{DeadLetters, Overview, Probe, Timing, Trace};
use crate::monitoring::probe::{self, Request};
use crate::monitoring::tokens::{BG, KEYFRAMES, SANS, TEXT};
use crate::monitoring::trace_view::{self, GroupId, StepId};
use crate::services::state::Environment;
use chrono::Utc;
use dioxus::prelude::*;
use kit::{StateCard, StateKind};
use serde::de::DeserializeOwned;
use shell::NavItem;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen {
    Trace,
    Queue,
    Stuck,
    Dead,
    Refs,
    Timing,
    Home,
}

/// One probe request's lifecycle.
#[derive(Clone, PartialEq, Debug)]
pub struct Fetch<T> {
    pub pending: bool,
    pub result: Option<Probe<T>>,
    seq: u64,
    task: Option<Task>,
}

impl<T> Default for Fetch<T> {
    fn default() -> Self {
        Fetch { pending: false, result: None, seq: 0, task: None }
    }
}

fn fetch<T: DeserializeOwned + 'static>(mut target: Signal<Fetch<T>>, env: Environment, request: Request) {
    let seq = {
        let mut w = target.write();
        // Cancelling drops the superseded probe's future; kill_on_drop ends its process.
        if let Some(previous) = w.task.take() {
            previous.cancel();
        }
        w.seq += 1;
        w.pending = true;
        w.seq
    };
    let task = spawn(async move {
        let result = probe::run::<T>(env.as_str(), request).await;
        let mut w = target.write();
        if w.seq == seq {
            w.pending = false;
            w.result = Some(result);
            w.task = None;
        }
    });
    target.write().task = Some(task);
}

fn reset<T: 'static>(mut target: Signal<Fetch<T>>) {
    let mut w = target.write();
    if let Some(previous) = w.task.take() {
        previous.cancel();
    }
    w.seq += 1;
    w.pending = false;
    w.result = None;
}

/// The data, or the state card that stands in for it.
fn gate<T: Clone>(f: &Fetch<T>, what: &str, env: &str, retry: Callback<()>) -> Result<(T, bool), Element> {
    let card = |kind: StateKind, message: String| {
        rsx! {
            StateCard { kind, what: what.to_string(), env: env.to_string(), message, on_retry: move |_| retry.call(()) }
        }
    };
    match &f.result {
        Some(Probe::Ok { data }) => Ok((data.clone(), f.pending)),
        None => Err(card(StateKind::Loading, String::new())),
        // A retry in flight: its answer, not the failure it replaces, is what comes next.
        Some(_) if f.pending => Err(card(StateKind::Loading, String::new())),
        Some(Probe::Auth { message }) => Err(card(StateKind::Auth, message.clone())),
        Some(Probe::Unavailable { message }) => Err(card(StateKind::Unavailable, message.clone())),
        Some(Probe::Error { message }) => Err(card(StateKind::Error, message.clone())),
    }
}

#[component]
pub fn MonitoringApp(on_open_developer: EventHandler<()>) -> Element {
    let mut screen = use_signal(|| Screen::Trace);
    let mut env = use_signal(|| Environment::Dev);
    let mut query = use_signal(String::new);
    // The query the trace on screen answers, which the search box may have moved past.
    let mut traced = use_signal(String::new);
    let overview = use_signal(Fetch::<Overview>::default);
    let trace = use_signal(Fetch::<Trace>::default);
    let dead = use_signal(Fetch::<DeadLetters>::default);
    let timing = use_signal(Fetch::<Timing>::default);
    let mut open_step = use_signal(|| Option::<StepId>::None);
    let mut open_group = use_signal(|| Option::<GroupId>::None);
    // The query whose default step has been opened: a refresh must not undo a choice.
    let mut focused = use_signal(String::new);
    // Documents traced this session, newest first, for the rail.
    let mut recent = use_signal(Vec::<shell::RecentDoc>::new);
    // Relative times ("41 min ago") must move without a refetch.
    let mut clock = use_signal(Utc::now);

    use_coroutine(move |_rx: UnboundedReceiver<()>| async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(30)).await;
            clock.set(Utc::now());
        }
    });

    use_effect(move || {
        let e = env();
        reset(trace);
        reset(dead);
        reset(timing);
        fetch(overview, e, Request::Overview);
        let q = traced.peek().clone();
        if !q.is_empty() {
            fetch(trace, e, Request::Trace(q));
        }
        match *screen.peek() {
            Screen::Dead => fetch(dead, e, Request::DeadLetters),
            Screen::Timing => fetch(timing, e, Request::Timing),
            _ => {}
        }
    });

    // A new trace opens on the step that matters, as the mock does.
    use_effect(move || {
        let fetched = trace.read();
        let Some(Probe::Ok { data }) = &fetched.result else { return };
        if *focused.peek() == data.query {
            return;
        }
        let Some(view) = trace_view::build(data, Utc::now()) else { return };
        open_step.set(view.focus);
        open_group.set(view.focus.map(|s| s.group()));
        focused.set(data.query.clone());
        let document_id = data.root.as_ref().map(|r| r.document_id.clone()).unwrap_or_default();
        let mut list = recent.write();
        list.retain(|d| d.document_id != document_id);
        list.insert(0, shell::RecentDoc { document_id, key: view.header.key.clone(), tone: view.header.tone });
        list.truncate(8);
    });

    let run_trace = use_callback(move |q: String| {
        let q = q.trim().to_string();
        if q.is_empty() {
            return;
        }
        query.set(q.clone());
        traced.set(q.clone());
        screen.set(Screen::Trace);
        open_step.set(None);
        open_group.set(None);
        focused.set(String::new());
        fetch(trace, env(), Request::Trace(q));
    });

    let go = use_callback(move |s: Screen| {
        screen.set(s);
        let e = env();
        match s {
            Screen::Dead if dead.peek().result.is_none() && !dead.peek().pending => fetch(dead, e, Request::DeadLetters),
            Screen::Timing if timing.peek().result.is_none() && !timing.peek().pending => fetch(timing, e, Request::Timing),
            _ => {}
        }
    });

    let refresh = use_callback(move |_: ()| {
        let e = env();
        fetch(overview, e, Request::Overview);
        match screen() {
            Screen::Trace => {
                let q = traced();
                if !q.is_empty() {
                    fetch(trace, e, Request::Trace(q));
                }
            }
            Screen::Dead => fetch(dead, e, Request::DeadLetters),
            Screen::Timing => fetch(timing, e, Request::Timing),
            _ => {}
        }
    });

    let now = clock();
    let env_now = env();
    let env_name = env_now.as_str().to_uppercase();
    let ov = overview.read().clone();
    let ov_data = match &ov.result {
        Some(Probe::Ok { data }) => Some(data.clone()),
        _ => None,
    };
    let active = screen();

    let stalled = ov_data.as_ref().map(|o| o.stuck.iter().filter(|s| fleet_view::is_stalled(s, now)).count());
    let stale_ref = ov_data.as_ref().is_some_and(|o| fleet_view::references(o, now).iter().any(|r| r.tone != trace_view::Tone::Ok));
    // The queues' own counts when every queue reported one; otherwise what could be peeked.
    let dead_count = match &dead.read().result {
        Some(Probe::Ok { data }) => {
            let counted: Option<u64> = data.queues.iter().map(|q| q.dead_letter_count).sum();
            Some(counted.unwrap_or(data.messages.len() as u64) + data.event_grid.len() as u64)
        }
        _ => None,
    };
    let nav = vec![
        NavItem { screen: Screen::Trace, icon: "magnifying-glass", label: "Document trace".into(), badge: None, hot: false },
        NavItem {
            screen: Screen::Queue,
            icon: "tray",
            label: "Parked queue".into(),
            badge: ov_data.as_ref().map(|o| o.parked.len().to_string()),
            hot: ov_data.as_ref().is_some_and(|o| !o.parked.is_empty()),
        },
        NavItem {
            screen: Screen::Stuck,
            icon: "pause-circle",
            label: "Stuck extractions".into(),
            badge: stalled.map(|n| n.to_string()),
            hot: stalled.is_some_and(|n| n > 0),
        },
        NavItem {
            screen: Screen::Dead,
            icon: "trash",
            label: "Dead-letter watch".into(),
            badge: dead_count.map(|n| n.to_string()),
            hot: dead_count.is_some_and(|n| n > 0),
        },
        NavItem {
            screen: Screen::Refs,
            icon: "arrows-clockwise",
            label: "Reference freshness".into(),
            badge: stale_ref.then(|| "OLD".to_string()),
            hot: stale_ref,
        },
        NavItem { screen: Screen::Timing, icon: "timer", label: "Step timing".into(), badge: None, hot: false },
        NavItem { screen: Screen::Home, icon: "database", label: format!("{env_name} today"), badge: None, hot: false },
    ];

    let any_pending = ov.pending || trace.read().pending || dead.read().pending || timing.read().pending;
    let stage_note = if any_pending {
        format!("Reading {env_name} through the Azure CLI, read-only…")
    } else if let Some(o) = &ov_data {
        format!(
            "Live, read-only. Checked {}. State is current when read; logs lag.",
            crate::monitoring::format::parse(&o.checked_at).map(crate::monitoring::format::clock).unwrap_or_default()
        )
    } else {
        "Live, read-only. Nothing read yet.".to_string()
    };
    let rows = |n: u64| format!("{n} ROW{}", if n == 1 { "" } else { "S" });
    let catalog = ov_data.as_ref().map(|o| rows(o.catalog_rows)).unwrap_or_else(|| "—".into());
    let audit = ov_data.as_ref().map(|o| rows(o.audit_rows)).unwrap_or_else(|| "—".into());

    let trace_fetch = trace.read().clone();
    let traced_now = traced();
    let trace_data = match &trace_fetch.result {
        Some(Probe::Ok { data }) if data.query == traced_now => Some(data.clone()),
        _ => None,
    };
    // Built once per answer and clock tick, not on every render.
    let built = use_memo(move || {
        let fetched = trace.read();
        let now = clock();
        match &fetched.result {
            Some(Probe::Ok { data }) if data.query == traced() => trace_view::build(data, now),
            _ => None,
        }
    });
    let trace_view = built();
    let no_match = trace_data.as_ref().is_some_and(|t| t.root.is_none());

    let title = match active {
        Screen::Trace if no_match => "No match".to_string(),
        Screen::Trace => "Document trace".to_string(),
        Screen::Queue => "Parked queue".to_string(),
        Screen::Stuck => "Stuck extractions".to_string(),
        Screen::Dead => "Dead-letter watch".to_string(),
        Screen::Refs => "Reference freshness".to_string(),
        Screen::Timing => "Step timing".to_string(),
        Screen::Home => format!("{env_name} as it is today"),
    };

    let current_doc = trace_data.as_ref().and_then(|t| t.root.as_ref()).map(|r| r.document_id.clone()).unwrap_or_default();
    let oldest_parked = ov_data.as_ref().and_then(|o| fleet_view::queue(o, now).1.first().map(|r| r.document_id.clone()));

    let on_trace = move |q: String| run_trace.call(q);
    let on_go = move |s: Screen| go.call(s);

    let body: Element = match active {
        Screen::Trace if traced_now.is_empty() => rsx! {
            home::TracePrompt { overview: ov_data.clone(), env_name: env_name.clone(), on_trace, on_go }
        },
        Screen::Trace => {
            // An answer to an older query is not this query's answer.
            let shown = Fetch { pending: trace_fetch.pending, result: trace_data.clone().map(|data| Probe::Ok { data }).or_else(|| {
                trace_fetch.result.clone().filter(|r| !matches!(r, Probe::Ok { .. }))
            }), seq: 0, task: None };
            match gate(&shown, "the trace", &env_name, refresh) {
                Err(card) => card,
                Ok((data, pending)) => match trace_view {
                    Some(view) => rsx! {
                        trace::TraceScreen {
                            view,
                            query: data.query.clone(),
                            checked: data.checked_at.clone(),
                            refreshing: pending,
                            open_step: open_step(),
                            open_group: open_group(),
                            on_step: move |id: StepId| {
                                let open = open_step() == Some(id);
                                open_step.set(if open { None } else { Some(id) });
                                open_group.set(if open { None } else { Some(id.group()) });
                            },
                            on_group: move |g: GroupId| {
                                let open = open_group() == Some(g);
                                open_group.set(if open { None } else { Some(g) });
                                open_step.set(if open { None } else { g.step() });
                            },
                            on_trace,
                        }
                    },
                    None => rsx! { home::NoMatch { trace: data, env_name: env_name.clone(), oldest_parked: oldest_parked.clone(), on_trace, on_go } },
                },
            }
        }
        Screen::Queue => match gate(&ov, "the parked queue", &env_name, refresh) {
            Err(card) => card,
            Ok((o, pending)) => {
                let (kpis, rows) = fleet_view::queue(&o, now);
                rsx! { fleet::QueueScreen { kpis, rows, current: current_doc.clone(), refreshing: pending, on_trace } }
            }
        },
        Screen::Stuck => match gate(&ov, "stuck extractions", &env_name, refresh) {
            Err(card) => card,
            Ok((o, pending)) => rsx! { fleet::StuckScreen { rows: fleet_view::stuck(&o, now), refreshing: pending, on_trace } },
        },
        Screen::Refs => match gate(&ov, "reference pointers", &env_name, refresh) {
            Err(card) => card,
            Ok((o, pending)) => rsx! { fleet::RefsScreen { rows: fleet_view::references(&o, now), refreshing: pending, on_go } },
        },
        Screen::Dead => match gate(&dead.read().clone(), "dead-letter queues", &env_name, refresh) {
            Err(card) => card,
            Ok((d, pending)) => rsx! {
                fleet::DeadScreen { rows: fleet_view::dead_letters(&d, now), data: d.clone(), refreshing: pending, on_trace, on_go }
            },
        },
        Screen::Timing => match gate(&timing.read().clone(), "step timing", &env_name, refresh) {
            Err(card) => card,
            Ok((t, pending)) => rsx! {
                fleet::TimingScreen { rows: fleet_view::timing(&t, now), documents: t.docs.len(), truncated: t.truncated, refreshing: pending, on_go }
            },
        },
        Screen::Home => match gate(&ov, "the environment", &env_name, refresh) {
            Err(card) => card,
            Ok((o, pending)) => rsx! {
                home::HomeScreen { home: fleet_view::home(&o, now), overview: o.clone(), refreshing: pending, on_trace, on_go }
            },
        },
    };

    rsx! {
        style { {KEYFRAMES} }
        div {
            class: "cdwm-root",
            style: "display:flex;height:100vh;min-height:0;background:{BG};color:{TEXT};font-family:{SANS};-webkit-font-smoothing:antialiased",
            shell::Aside {
                nav,
                active: if active == Screen::Trace || no_match { Screen::Trace } else { active },
                env: env_now,
                stage_note,
                catalog,
                audit,
                docs: recent(),
                current: current_doc.clone(),
                on_trace,
                on_nav: on_go,
                on_env: move |e: Environment| env.set(e),
                on_developer: move |_| on_open_developer.call(()),
            }
            div { style: "flex:1;min-width:0;display:flex;flex-direction:column;background:{BG}",
                shell::Header {
                    breadcrumb: format!("Central Document Warehouse · live · {env_name} · read only"),
                    title,
                    query: query(),
                    busy: any_pending,
                    on_submit: move |q: String| run_trace.call(q),
                    on_refresh: move |_| refresh.call(()),
                }
                div { style: "flex:1;overflow:auto;padding:26px 28px 40px", {body} }
            }
        }
    }
}
