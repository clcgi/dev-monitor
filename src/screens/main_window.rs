use dioxus::prelude::*;
use chrono::Local;
use futures_util::StreamExt;
use crate::components::sidebar::Sidebar;
use crate::components::dashboard::Dashboard;
use crate::services::state::{AppState, Environment, ScriptStatus, LogMsg, StreamType, HistoryEntry, Verdict};
use crate::services::process::ScriptRunner;

#[derive(Props, Clone, PartialEq)]
pub struct MainWindowProps {
    pub show_auth_reminder: Signal<bool>,
    /// None follows the OS; Some(true) light, Some(false) dark.
    pub theme_preference: Option<bool>,
    pub system_is_light: bool,
    pub on_theme_change: EventHandler<Option<bool>>,
}

pub enum ProcessCommand {
    Run { script: String, env: Environment, args: Vec<String> },
    Kill, // To abort the running process
}

#[component]
pub fn MainWindow(mut props: MainWindowProps) -> Element {
    let mut state = use_signal(AppState::new);
    // Open by default: the log pane is the reason this app exists, and a
    // collapsed one on first launch reads as a run that produced nothing.
    let mut logs_open = use_signal(|| true);
    let mut history_open = use_signal(|| false);
    // (nonce, line index). The nonce is what makes a second jump to the SAME
    // line fire again -- the prop would otherwise be unchanged and the
    // effect would not run.
    let mut log_jump = use_signal(|| Option::<(u64, usize)>::None);
    let mut jump_nonce = use_signal(|| 0u64);

    // A ONE-SECOND HEARTBEAT WHILE RUNNING, and it is not cosmetic. Nothing else
    // re-renders during a wait: the app repaints when a log line arrives, and
    // the two waits that matter most -- a KEDA cold start, a 10-second poll --
    // produce no output at all. Without this the elapsed readings freeze at
    // whatever the last line left them, at the exact moment a user most needs to
    // know time is still passing.
    let mut tick = use_signal(|| 0u64);
    use_coroutine(move |_rx: UnboundedReceiver<()>| async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            // Only while something is running, so an idle app is not repainting
            // once a second forever.
            if state.read().running_script.is_some() {
                let n = *tick.read();
                tick.set(n.wrapping_add(1));
            }
        }
    });
    
    // We store logs directly in state now, but we'll use a signal for preflight checks
    
    
    // Coroutine to handle script execution
    let process_coroutine = use_coroutine(move |mut rx: UnboundedReceiver<ProcessCommand>| async move {
        let mut child_proc: Option<tokio::process::Child> = None;
        
        loop {
            tokio::select! {
                cmd = rx.next() => {
                    match cmd {
                        Some(ProcessCommand::Run { script, env, args }) => {
                            // `script` is captured for the whole run and every
                            // write below is addressed to it. NOT to whatever is
                            // selected: the user is free to look at another
                            // script while this one runs, and its output must
                            // still land in its own log.
                            {
                                let mut s = state.write();
                                s.running_script = Some(script.clone());
                                let entry = s.entry(&script);
                                entry.status = ScriptStatus::Running;
                                entry.start_time = Some(Local::now());
                                entry.end_time = None;
                                entry.logs.clear();
                                entry.verdicts.clear();
                                entry.active_step = None;
                                entry.step_history.clear();
                                entry.step_started = None;
                            }
                            // The log this pointed into has just been cleared,
                            // so the index now names a different line -- or none.
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
                                                match markers::parse(&log_msg.content) {
                                                    // A new pass over the chain. simulate_upload.py
                                                    // runs it twice to compare ingress routes, and
                                                    // without this reset the stepper walks forward
                                                    // and then appears to jump BACKWARDS -- which
                                                    // reads as a fault rather than a second pass.
                                                    Some(Marker::Run(_)) => {
                                                        let mut s = state.write();
                                                        let e = s.entry(&script);
                                                        e.active_step = None;
                                                        e.step_started = None;
                                                        e.step_history.clear();
                                                    }
                                                    Some(Marker::Step(step)) => {
                                                        let mut s = state.write();
                                                        let e = s.entry(&script);
                                                        // Restamped only on a CHANGE of stage. A
                                                        // script that re-emits the same marker --
                                                        // verify_ingestion prints one per evidenced
                                                        // stage, and a poll loop may repeat one --
                                                        // must not keep resetting the clock, or the
                                                        // elapsed reading never leaves zero and says
                                                        // nothing.
                                                        if e.active_step.as_ref() != Some(&step) {
                                                            e.step_started = Some(Local::now());
                                                        }
                                                        e.active_step = Some(step.clone());
                                                        complete_up_to(&mut e.step_history, &step);
                                                    }
                                                    // Completion does NOT move the cursor. A step
                                                    // that finished is not the step now running,
                                                    // and conflating the two is how a hung step
                                                    // looks identical to a slow one.
                                                    Some(Marker::StepDone(step)) => {
                                                        let mut s = state.write();
                                                        let e = s.entry(&script);
                                                        complete_up_to(&mut e.step_history, &step);
                                                        if !e.step_history.contains(&step) {
                                                            e.step_history.push(step);
                                                        }
                                                    }
                                                    // A verdict the script reached about ITSELF.
                                                    // Kept alongside the exit code rather than
                                                    // replacing it: flow_all runs six flows through
                                                    // one exit code, so the code says the suite
                                                    // failed and only these say which flow did.
                                                    Some(Marker::Result { ok, label }) => {
                                                        state.write().entry(&script).verdicts.push(Verdict { label, ok });
                                                    }
                                                    // A line that LOOKS like a marker but names
                                                    // nothing known is surfaced, never dropped. A
                                                    // typo in a script is otherwise indistinguishable
                                                    // from a step that simply never ran -- which is
                                                    // precisely the silent failure this parser exists
                                                    // to end.
                                                    None if markers::is_unrecognised(&log_msg.content) => {
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
                                                state.write().entry(&script).logs.push(log_msg);
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
                                                        e.step_started = None;
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
                                                    // The stage clock stops with the process. A
                                                    // finished run that kept counting would read
                                                    // as one still waiting.
                                                    e.step_started = None;
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
                            // Addressed to whatever is RUNNING, which need not be
                            // what is selected -- the user may have moved on.
                            let mut s = state.write();
                            if let Some(path) = s.running_script.take() {
                                let e = s.entry(&path);
                                e.status = ScriptStatus::Cancelled;
                                e.end_time = Some(Local::now());
                                e.step_started = None;
                            }
                        },
                        None => break,
                    }
                }
            }
        }
    });

    rsx! {
        div { class: "flex h-screen flex-col",
            
            // Topbar
            // Converted to utilities. RESPONSIVE: the subtitle is dropped below
            // `sm` and the repo name truncates rather than pushing the picker
            // off-screen -- the window is resizable and had no breakpoint at all
            // before (the whole stylesheet contained zero @media queries).
            header {
                class: "flex shrink-0 items-center gap-3 border-b border-edge bg-surface \
                        px-3 py-2.5 sm:gap-4 sm:px-4",
                span { class: "shrink-0 font-semibold tracking-tight text-fg", "DEV Monitor" }
                div { class: "flex min-w-0 flex-col leading-tight",
                    span { class: "truncate text-xs text-fg-soft", "CentralDocumentWarehouse" }
                    span { class: "hidden truncate text-[11px] text-fg-faint sm:block",
                        "Monitoring DEV ingestion pipeline" }
                }
                // Pushes the controls to the trailing edge at every width.
                div { class: "ml-auto" }
                // The palette picker is deliberately NOT mounted. The four
                // palettes and `components::theme_picker` are kept intact --
                // see DEFAULT_THEME in main.rs -- so restoring it is one line
                // here rather than four CSS blocks and a component.
                // History moved off the run view entirely: it is a record
                // ACROSS runs and was competing for height with the stepper and
                // the log pane, both of which describe the run in front of you.
                button {
                    r#type: "button",
                    title: "Execution history",
                    class: "flex items-center gap-2 rounded-md border border-edge bg-elevated \
                            px-2.5 py-1.5 text-fg-soft transition-colors hover:bg-active \
                            hover:text-fg focus:outline-none",
                    onclick: move |_| { let open = *history_open.read(); history_open.set(!open); },
                    i { class: "ph ph-clock-counter-clockwise text-base" }
                    span { class: "hidden text-xs sm:inline", "History" }
                    if !state.read().history.is_empty() {
                        span {
                            class: "rounded bg-active px-1.5 text-[10px] text-fg-faint",
                            "{state.read().history.len()}"
                        }
                    }
                }
                crate::components::theme_toggle::ThemeToggle {
                    preference: props.theme_preference,
                    system_is_light: props.system_is_light,
                    on_change: move |pref| props.on_theme_change.call(pref),
                }
            }
            
            crate::components::history::HistoryPanel {
                history: state.read().history.clone(),
                open: *history_open.read(),
                on_close: move |_| history_open.set(false),
            }

            // Body
            // min-h-0 is what lets the log pane scroll instead of pushing
            // the window taller: a flex child defaults to min-height:auto.
            div { class: "flex min-h-0 flex-1",
                Sidebar {
                    selected_script: state.read().selected_script.clone(),
                    // Which script is running, so the sidebar can say so on the
                    // row itself. Without it the only running indicator is on
                    // the main page, and the sidebar -- the thing a user looks
                    // at to pick the next script -- gives no sign a run is in
                    // progress.
                    running_script: state.read().running_script.clone(),
                    on_select: move |meta: crate::services::scripts::ScriptMeta| {
                        // NOTHING IS CLEARED HERE ANY MORE, and that is the fix.
                        // This used to wipe the logs, stepper and verdicts --
                        // which belonged to a run that may still have been going,
                        // whose output then kept arriving into the newly selected
                        // script's view. Selecting is now purely a change of what
                        // is displayed.
                        let mut s = state.write();
                        // First touch seeds the entry, so its saved flags exist
                        // before the picker reads them.
                        let defaults: Vec<String> = meta
                            .args
                            .iter()
                            .filter(|a| a.default_on)
                            .map(|a| a.flag.clone())
                            .collect();
                        let fresh = !s.scripts.contains_key(&meta.path);
                        let entry = s.entry(&meta.path);
                        if fresh {
                            entry.enabled_args = defaults;
                        }
                        s.selected_script = Some(meta.path.clone());
                        s.selected_meta = Some(meta);
                        drop(s);
                        // The jump index points into the log of whatever was
                        // shown before, which is a different list now.
                        log_jump.set(None);
                    },
                }

                Dashboard {
                    state: state.clone(),
                    on_env_select: move |env| {
                        state.write().selected_env = Some(env);
                    },
                    // Read so this component re-renders on the heartbeat, which
                    // is what advances the elapsed readings during a silent wait.
                    tick: *tick.read(),
                    logs_open: *logs_open.read(),
                    log_jump,
                    on_toggle_logs: move |_| {
                        let open = *logs_open.read();
                        logs_open.set(!open);
                    },
                    on_jump_to_run: move |label: String| {
                        // The `[CDW_RUN: label]` line the flow prints at its own
                        // start, so the jump lands at the BEGINNING of that run
                        // rather than on its verdict at the end -- the verdict is
                        // what was just clicked, and its context is above it.
                        //
                        // The LAST match wins: re-running a suite appends a
                        // second copy of every run to the same log, and the one
                        // a fresh verdict refers to is the most recent.
                        let needle = format!("[CDW_RUN: {label}]");
                        let found = state
                            .read()
                            .current()
                            .logs
                            .iter()
                            .rposition(|l| l.content.contains(&needle));
                        if let Some(index) = found {
                            // Opened, because a jump into a collapsed pane
                            // silently does nothing.
                            logs_open.set(true);
                            let nonce = *jump_nonce.read() + 1;
                            jump_nonce.set(nonce);
                            log_jump.set(Some((nonce, index)));
                        } else {
                            // Cloned out of the read borrow BEFORE writing: an
                            // `if let` over `state.read()` holds the borrow for
                            // the whole block, and the write inside it panics at
                            // runtime rather than failing to compile in a way
                            // that names the cause.
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
                    on_run: move |_| {
                        let s = state.read();
                        if let (Some(script), Some(env)) = (&s.selected_script, s.selected_env) {
                            process_coroutine.send(ProcessCommand::Run {
                                script: script.clone(),
                                env,
                                args: s.current().enabled_args.clone(),
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
                    class: "fixed inset-0 z-[100] flex items-center justify-center bg-black/55 p-4",
                    div {
                        // w-full + max-w-md instead of a fixed 460px: the dialog
                        // no longer overflows a narrow window.
                        class: "flex max-h-[84vh] w-full max-w-md flex-col overflow-hidden \
                                rounded-xl border border-edge bg-surface",
                        div {
                            class: "flex shrink-0 items-center justify-between border-b \
                                    border-edge-soft px-4 py-3 font-semibold text-fg",
                            span { "Azure Authentication Required" }
                            button {
                                class: "text-xl leading-none text-fg-faint hover:text-fg",
                                onclick: move |_| props.show_auth_reminder.set(false),
                                "×"
                            }
                        }
                        div { class: "flex flex-col gap-2.5 overflow-y-auto p-4 text-sm text-fg-soft",
                            p { "You must be authenticated with Azure to interact with the environment." }
                            p { "Please ensure you have run:" }
                            p {
                                class: "rounded-md bg-app p-2.5 font-mono text-[11px] \
                                        leading-relaxed text-fg-soft",
                                "az login"
                            }
                            button {
                                class: "mt-2 w-full rounded-lg border border-brand-deep \
                                        bg-brand-deep px-3.5 py-1.5 font-mono text-xs text-white \
                                        hover:border-brand hover:bg-brand",
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


/// Mark every linear step BEFORE `step` as completed.
///
/// A run that starts partway along the chain -- `run_against_dev.py` writes
/// straight to `raw` and skips landing entirely -- would otherwise leave every
/// earlier step showing as pending forever, which reads as "those stages
/// failed" rather than "this tool does not do those stages".
///
/// Exception zones have no position on the chain, so they complete nothing.
fn complete_up_to(
    history: &mut Vec<crate::services::state::WorkflowStep>,
    step: &crate::services::state::WorkflowStep,
) {
    use crate::services::state::WorkflowStep;
    let Some(idx) = step.linear_index() else {
        return;
    };
    for earlier in WorkflowStep::LINEAR.iter().take(idx) {
        if !history.contains(earlier) {
            history.push(earlier.clone());
        }
    }
}
