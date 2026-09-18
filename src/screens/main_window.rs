use dioxus::prelude::*;
use chrono::Local;
use futures_util::StreamExt;
use crate::components::sidebar::Sidebar;
use crate::components::dashboard::Dashboard;
use crate::services::state::{AppState, Environment, ScriptStatus, LogMsg, StreamType, HistoryEntry, Verdict};
use crate::services::marker_syntax::MarkerSyntax;
use crate::services::steps::{StepCatalog, StepId};
use crate::services::process::ScriptRunner;

#[derive(Props, Clone, PartialEq)]
pub struct MainWindowProps {
    pub show_auth_reminder: Signal<bool>,
    /// Set once a newer release is known, whether or not the banner is up.
    pub pending_update: Option<crate::services::updates::Update>,
    pub on_show_update: EventHandler<crate::services::updates::Update>,
    /// None follows the OS; Some(true) light, Some(false) dark.
    pub theme_preference: Option<bool>,
    pub system_is_light: bool,
    pub on_theme_change: EventHandler<Option<bool>>,
    /// Back to CDW Monitoring. This window stays mounted, so a run keeps going.
    pub on_open_monitoring: EventHandler<()>,
}

pub enum ProcessCommand {
    Run { script: String, env: Environment, args: Vec<String> },
    Kill, // To abort the running process
}

#[component]
pub fn MainWindow(mut props: MainWindowProps) -> Element {
    let mut state = use_signal(AppState::new);
    // Open by default: a collapsed log pane reads as a run that produced nothing.
    let mut logs_open = use_signal(|| true);
    let mut history_open = use_signal(|| false);
    let mut settings_open = use_signal(|| false);
    let mut traces_tab = use_signal(|| false);
    // (nonce, line index).
    let mut log_jump = use_signal(|| Option::<(u64, usize)>::None);
    let mut jump_nonce = use_signal(|| 0u64);

    // A ONE-SECOND HEARTBEAT WHILE RUNNING, and it is not cosmetic.
    let mut tick = use_signal(|| 0u64);
    use_coroutine(move |_rx: UnboundedReceiver<()>| async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            // Only while running, so an idle app does not repaint once a second forever.
            if state.read().running_script.is_some() {
                let n = *tick.read();
                tick.set(n.wrapping_add(1));
            }
        }
    });
    
    // We store logs directly in state now, but we'll use a signal for preflight.
    
    // Coroutine to handle script execution
    let process_coroutine = use_coroutine(move |mut rx: UnboundedReceiver<ProcessCommand>| async move {
        let mut child_proc: Option<tokio::process::Child> = None;
        
        loop {
            tokio::select! {
                cmd = rx.next() => {
                    match cmd {
                        Some(ProcessCommand::Run { script, env, args }) => {
                            let catalog = state.read().catalog.clone();
                            let syntax = state.read().syntax.clone();
                            // `script` is captured for the whole run and every write below is addressed.
                            {
                                let mut s = state.write();
                                s.running_script = Some(script.clone());
                                let entry = s.entry(&script);
                                entry.status = ScriptStatus::Running;
                                entry.start_time = Some(Local::now());
                                entry.end_time = None;
                                entry.logs.clear();
                                entry.verdicts.clear();
                                entry.restart_steps();
                                entry.traces.clear();
                                entry.in_trace = None;
                            }
                            // The log it pointed into was just cleared, so the index names a different line.
                            log_jump.set(None);
                            
                            state.write().entry(&script).logs.push(LogMsg {
                                timestamp: Local::now(),
                                stream: StreamType::System,
                                content: format!(
                                    "$ bash -c \"export CDW_ENV={}; source deploy/00-variables.sh && ... {}{}\"",
                                    env.as_str(),
                                    script,
                                    args.iter().map(|a| format!(" {a}")).collect::<String>(),
                                ),
                            });
                            
                            match ScriptRunner::spawn(&script, env.as_str(), &args) {
                                Ok((child, mut rx_logs)) => {
                                    child_proc = Some(child);
                                    
                                    loop {
                                        tokio::select! {
                                            Some(log_msg) = rx_logs.recv() => {
                                                use crate::services::markers::{self, Marker};
                                                match markers::parse(&log_msg.content, &catalog, &syntax) {
                                                    // A new pass over the chain.
                                                    Some(Marker::Run(_)) => {
                                                        state.write().entry(&script).restart_steps();
                                                    }
                                                    Some(Marker::Step(step)) => {
                                                        // Restamped only on a CHANGE of stage, and the
                                                        // stage it leaves is banked there.
                                                        state.write().entry(&script).begin_step(step, Local::now());
                                                    }
                                                    // Completion does NOT move the cursor.
                                                    Some(Marker::StepDone(step)) => {
                                                        state.write().entry(&script).finish_step(step);
                                                    }
                                                    // A verdict the script reached about ITSELF. Kept alongside the exit code.
                                                    Some(Marker::TraceBegin(component)) => {
                                                        state.write().entry(&script).in_trace = Some(component);
                                                    }
                                                    Some(Marker::TraceEnd(_)) => {
                                                        state.write().entry(&script).in_trace = None;
                                                    }
                                                    Some(Marker::Result { ok, label }) => {
                                                        state.write().entry(&script).verdicts.push(Verdict { label, ok });
                                                    }
                                                    // A line that LOOKS like a marker but names nothing known is surfaced, never.
                                                    None if markers::is_unrecognised(&log_msg.content, &catalog, &syntax) => {
                                                        state.write().entry(&script).logs.push(LogMsg {
                                                            timestamp: chrono::Local::now(),
                                                            stream: StreamType::System,
                                                            content: format!(
                                                                "unrecognised CDW marker, ignored: {}",
                                                                log_msg.content.trim()
                                                            ),
                                                        });
                                                    }
                                                    None => {}
                                                }
                                                {
                                                    let mut s = state.write();
                                                    let e = s.entry(&script);
                                                    if let Some(component) = e.in_trace.clone() {
                                                        e.traces.entry(component).or_default().push(log_msg.clone());
                                                    }
                                                    e.logs.push(log_msg);
                                                }
                                            }
                                            kill = rx.next() => {
                                                if let Some(ProcessCommand::Kill) = kill {
                                                    if let Some(mut p) = child_proc.take() {
                                                        let _ = p.kill().await;
                                                        let mut s = state.write();
                                                        s.running_script = None;
                                                        let e = s.entry(&script);
                                                        e.logs.push(LogMsg {
                                                            timestamp: Local::now(),
                                                            stream: StreamType::System,
                                                            content: "Process killed by user.".to_string(),
                                                        });
                                                        e.status = ScriptStatus::Cancelled;
                                                        e.end_time = Some(Local::now());
                                                        e.stop_step_clock(Local::now());
                                                    }
                                                    break;
                                                }
                                            }
                                            status = async {
                                                if let Some(p) = &mut child_proc {
                                                    p.wait().await
                                                } else {
                                                    std::future::pending::<std::io::Result<std::process::ExitStatus>>().await
                                                }
                                            } => {
                                                let end_time = Local::now();
                                                let mut exit_code = -1;
                                                let final_status = match &status {
                                                    Ok(exit_status) => {
                                                        exit_code = exit_status.code().unwrap_or(-1);
                                                        if exit_status.success() {
                                                            ScriptStatus::Succeeded
                                                        } else {
                                                            ScriptStatus::Failed(exit_code)
                                                        }
                                                    }
                                                    Err(_) => ScriptStatus::AppError(
                                                        "Failed to wait on process".to_string(),
                                                    ),
                                                };

                                                let mut s = state.write();
                                                s.running_script = None;
                                                let started = {
                                                    let e = s.entry(&script);
                                                    e.end_time = Some(end_time);
                                                    // The stage clock stops with the process, and
                                                    // the stage it stopped on keeps the seconds it
                                                    // actually spent there.
                                                    e.stop_step_clock(end_time);
                                                    e.status = final_status.clone();
                                                    e.logs.push(LogMsg {
                                                        timestamp: Local::now(),
                                                        stream: StreamType::System,
                                                        content: format!("Process exited with code {}", exit_code),
                                                    });
                                                    e.start_time.unwrap_or(end_time)
                                                };
                                                s.history.push(HistoryEntry {
                                                    script_name: script.clone(),
                                                    environment: env,
                                                    start_time: started,
                                                    end_time,
                                                    status: final_status,
                                                });
                                                drop(s);
                                                
                                                child_proc = None;
                                                break;
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    let mut s = state.write();
                                    s.running_script = None;
                                    let entry = s.entry(&script);
                                    entry.logs.push(LogMsg {
                                        timestamp: Local::now(),
                                        stream: StreamType::System,
                                        content: format!("Failed to spawn process: {}", e),
                                    });
                                    entry.status = ScriptStatus::AppError(e.to_string());
                                }
                            }
                        },
                        Some(ProcessCommand::Kill) => {
                            if let Some(mut p) = child_proc.take() {
                                let _ = p.kill().await;
                            }
                            let mut s = state.write();
                            if let Some(path) = s.running_script.take() {
                                let e = s.entry(&path);
                                e.status = ScriptStatus::Cancelled;
                                e.end_time = Some(Local::now());
                                e.stop_step_clock(Local::now());
                            }
                        },
                        None => break,
                    }
                }
            }
        }
    });

    rsx! {
        div { class: "flex h-screen flex-col bg-app",
            
            header {
                class: "flex h-[44px] shrink-0 items-center justify-between bg-nav px-4 text-white",
                div { class: "flex items-center gap-2",
                    /* i { class: "ph-fill ph-apple-logo text-lg" } */
                    span { class: "text-nav-link tracking-widest uppercase", "DEV Monitor" }
                }
                div { class: "flex items-center gap-4",
                    button {
                        r#type: "button",
                        title: "CDW Monitoring",
                        class: "text-nav-link text-white/80 hover:text-white transition-colors flex items-center gap-1",
                        onclick: move |_| props.on_open_monitoring.call(()),
                        i { class: "ph ph-crosshair text-sm" }
                        span { class: "hidden sm:inline", "Monitoring" }
                    }
                    button {
                        r#type: "button",
                        title: "Execution history",
                        class: "text-nav-link text-white/80 hover:text-white transition-colors flex items-center gap-1",
                        onclick: move |_| { let open = *history_open.read(); history_open.set(!open); },
                        i { class: "ph ph-clock-counter-clockwise text-sm" }
                        span { class: "hidden sm:inline", "History" }
                        if !state.read().history.is_empty() {
                            span {
                                class: "ml-1 rounded-full bg-white/20 px-1.5 py-0.5 text-[9px]",
                                "{state.read().history.len()}"
                            }
                        }
                    }
                    
                    if let Some(u) = props.pending_update.clone() {
                        button {
                            r#type: "button",
                            title: "Version {u.version} is available",
                            class: "text-nav-link text-accent hover:text-accent/80 flex items-center gap-1",
                            onclick: move |_| props.on_show_update.call(u.clone()),
                            i { class: "ph-fill ph-arrow-circle-up text-sm" }
                            span { class: "hidden sm:inline", "Update" }
                        }
                    }

                    crate::components::theme_toggle::ThemeToggle {
                        preference: props.theme_preference,
                        system_is_light: props.system_is_light,
                        on_change: move |pref| props.on_theme_change.call(pref),
                    }

                    button {
                        r#type: "button",
                        title: "Settings",
                        class: "text-nav-link text-white/80 hover:text-white transition-colors \
                                flex items-center gap-1",
                        onclick: move |_| settings_open.set(true),
                        i { class: "ph ph-gear text-sm" }
                    }
                }
            }
            
            if *settings_open.read() {
                crate::components::settings_modal::SettingsModal {
                    catalog: state.read().catalog.clone(),
                    syntax: state.read().syntax.clone(),
                    on_close: move |_| settings_open.set(false),
                    on_save: move |(c, sy): (StepCatalog, MarkerSyntax)| {
                        if let Err(e) = crate::services::step_config::save(&c) {
                            eprintln!("could not save steps.json: {e}");
                        }
                        if let Err(e) = crate::services::step_config::save_syntax(&sy) {
                            eprintln!("could not save markers.json: {e}");
                        }
                        let mut st = state.write();
                        st.catalog = c;
                        st.syntax = sy;
                        drop(st);
                        settings_open.set(false);
                    },
                }
            }

            crate::components::history::HistoryPanel {
                history: state.read().history.clone(),
                open: *history_open.read(),
                on_close: move |_| history_open.set(false),
            }

            div { class: "flex min-h-0 flex-1",
                Sidebar {
                    selected_script: state.read().selected_script.clone(),
                    running_script: state.read().running_script.clone(),
                    statuses: state.read().scripts.iter()
                        .map(|(k, v)| (k.clone(), v.status.clone())).collect(),
                    catalog: state.read().catalog.clone(),
                    syntax: state.read().syntax.clone(),
                    on_select: move |meta: crate::services::scripts::ScriptMeta| {
                        let mut s = state.write();
                        let defaults: Vec<String> = meta
                            .args
                            .iter()
                            .filter(|a| a.default_on)
                            .map(|a| a.flag.clone())
                            .collect();
                        // The first value, so a script whose choice is required
                        // can be run without touching the dropdown -- and only
                        // on first selection, so a deliberate choice is not
                        // reset by clicking away and back.
                        //
                        // BUILT UP IN DECLARATION ORDER, not independently: a
                        // choice that narrows by another must default WITHIN
                        // that other's default, or the pair starts on a
                        // combination the file does not hold. Which makes the
                        // order the script declares its choices in meaningful
                        // -- `--indicator` before `--case`.
                        let mut first_choices: std::collections::HashMap<String, String> =
                            std::collections::HashMap::new();
                        for choice in meta.choices.iter() {
                            if let Some(value) = choice.values_for(&first_choices).first() {
                                first_choices.insert(choice.flag.clone(), value.clone());
                            }
                        }
                        let fresh = !s.scripts.contains_key(&meta.path);
                        let entry = s.entry(&meta.path);
                        if fresh {
                            entry.enabled_args = defaults;
                            entry.chosen = first_choices;
                        }
                        s.selected_script = Some(meta.path.clone());
                        s.selected_meta = Some(meta);
                        drop(s);
                        log_jump.set(None);
                    },
                }

                Dashboard {
                    state: state.clone(),
                    on_env_select: move |env| {
                        state.write().selected_env = Some(env);
                    },
                    tick: *tick.read(),
                    logs_open: *logs_open.read(),
                    traces_tab: *traces_tab.read(),
                    on_tab: move |t: bool| traces_tab.set(t),
                    log_jump,
                    on_toggle_logs: move |_| {
                        let open = *logs_open.read();
                        logs_open.set(!open);
                    },
                    on_jump_to_step: move |id: StepId| {
                        let found = {
                            let s = state.read();
                            let (catalog, syntax) = (s.catalog.clone(), s.syntax.clone());
                            s.current().logs.iter().position(|l| {
                                matches!(
                                    crate::services::markers::parse(&l.content, &catalog, &syntax),
                                    Some(crate::services::markers::Marker::Step(ref found)) if *found == id
                                )
                            })
                        };
                        if let Some(index) = found {
                            logs_open.set(true);
                            traces_tab.set(false);
                            let nonce = *jump_nonce.read() + 1;
                            jump_nonce.set(nonce);
                            log_jump.set(Some((nonce, index)));
                        }
                    },
                    on_jump_to_run: move |label: String| {
                        let needle = format!("[CDW_RUN: {label}]");
                        let found = state
                            .read()
                            .current()
                            .logs
                            .iter()
                            .rposition(|l| l.content.contains(&needle));
                        if let Some(index) = found {
                            logs_open.set(true);
                            let nonce = *jump_nonce.read() + 1;
                            jump_nonce.set(nonce);
                            log_jump.set(Some((nonce, index)));
                        } else {
                            let selected = state.read().selected_script.clone();
                            let Some(path) = selected else { return };
                            state.write().entry(&path).logs.push(LogMsg {
                                timestamp: Local::now(),
                                stream: StreamType::System,
                                content: format!(
                                    "no `[CDW_RUN: {label}]` line in this log -- nothing to jump to"
                                ),
                            });
                        }
                    },
                    on_toggle_arg: move |flag: String| {
                        let mut s = state.write();
                        let Some(path) = s.selected_script.clone() else { return };
                        let e = s.entry(&path);
                        if let Some(i) = e.enabled_args.iter().position(|f| *f == flag) {
                            e.enabled_args.remove(i);
                        } else {
                            e.enabled_args.push(flag);
                        }
                    },
                    on_choose: move |(flag, value): (String, String)| {
                        let mut s = state.write();
                        let Some(path) = s.selected_script.clone() else { return };
                        let choices = s
                            .selected_meta
                            .as_ref()
                            .map(|m| m.choices.clone())
                            .unwrap_or_default();
                        let entry = s.entry(&path);
                        entry.chosen.insert(flag.clone(), value);
                        // ANYTHING THAT NARROWS BY THIS FLAG IS NOW SUSPECT.
                        // Switching --indicator from RO to RW leaves --case
                        // holding a document that has no RW copy; left alone it
                        // stays on screen, looks chosen, and is refused by the
                        // script after Run. Moved to the first value the new
                        // selection does offer, or cleared when it offers none
                        // -- command_args already omits an empty choice.
                        for choice in choices.iter().filter(|c| c.depends_on == flag) {
                            let allowed = choice.values_for(&entry.chosen);
                            let current =
                                entry.chosen.get(&choice.flag).cloned().unwrap_or_default();
                            if allowed.contains(&current) {
                                continue;
                            }
                            match allowed.first() {
                                Some(first) => entry.chosen.insert(choice.flag.clone(), first.clone()),
                                None => entry.chosen.remove(&choice.flag),
                            };
                        }
                    },
                    on_run: move |_| {
                        let s = state.read();
                        if let (Some(script), Some(env)) = (&s.selected_script, s.selected_env) {
                            process_coroutine.send(ProcessCommand::Run {
                                script: script.clone(),
                                env,
                                args: s.current().command_args(),
                            });
                        }
                    },
                    on_stop: move |_| {
                        process_coroutine.send(ProcessCommand::Kill);
                    },
                }
            }
            
            // Auth reminder modal overlay
            if *props.show_auth_reminder.read() {
                div {
                    class: "fixed inset-0 z-[100] flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm",
                    div {
                        class: "flex max-h-[84vh] w-full max-w-md flex-col overflow-hidden rounded-2xl border border-border-soft bg-card shadow-2xl",
                        div {
                            class: "flex shrink-0 items-center justify-between border-b border-border-soft px-5 py-4",
                            span { class: "text-body-strong text-fg", "Azure Authentication Required" }
                            button {
                                class: "text-xl leading-none text-fg-faint hover:text-fg transition-colors",
                                onclick: move |_| props.show_auth_reminder.set(false),
                                "×"
                            }
                        }
                        div { class: "flex flex-col gap-3 overflow-y-auto p-5 text-body text-fg-muted",
                            p { "You must be authenticated with Azure to interact with the environment." }
                            p { "Please ensure you have run:" }
                            div {
                                class: "rounded-lg bg-black/5 dark:bg-black/40 p-4 font-mono text-[13px] text-fg shadow-inner border border-border-soft/50",
                                "az login"
                            }
                            button {
                                class: "mt-4 w-full rounded-full bg-accent px-5 py-3 text-button-utility text-white hover:scale-95 transition-transform",
                                onclick: move |_| props.show_auth_reminder.set(false),
                                "I have authenticated"
                            }
                        }
                    }
                }
            }
        }
    }
}

