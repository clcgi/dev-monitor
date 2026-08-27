use dioxus::prelude::*;
use chrono::Local;
use futures_util::StreamExt;
use crate::components::sidebar::Sidebar;
use crate::components::dashboard::Dashboard;
use crate::services::state::{AppState, Environment, ScriptStatus, LogMsg, StreamType, HistoryEntry};
use crate::services::process::ScriptRunner;

#[derive(Props, Clone, PartialEq)]
pub struct MainWindowProps {
    pub show_auth_reminder: Signal<bool>,
}

pub enum ProcessCommand {
    Run { script: String, env: Environment },
    Kill, // To abort the running process
}

#[component]
pub fn MainWindow(mut props: MainWindowProps) -> Element {
    let mut state = use_signal(AppState::new);
    
    // We store logs directly in state now, but we'll use a signal for preflight checks
    
    
    // Coroutine to handle script execution
    let process_coroutine = use_coroutine(move |mut rx: UnboundedReceiver<ProcessCommand>| async move {
        let mut child_proc: Option<tokio::process::Child> = None;
        
        loop {
            tokio::select! {
                cmd = rx.next() => {
                    match cmd {
                        Some(ProcessCommand::Run { script, env }) => {
                            state.write().script_status = ScriptStatus::Running;
                            state.write().active_start_time = Some(Local::now());
                            state.write().active_end_time = None;
                            state.write().logs.clear();
                            
                            state.write().logs.push(LogMsg {
                                timestamp: Local::now(),
                                stream: StreamType::System,
                                content: format!("$ bash -c \"export CDW_ENV={}; source deploy/00-variables.sh && ... {}\"", env.as_str(), script),
                            });
                            
                            match ScriptRunner::spawn(&script, env.as_str()) {
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
                                                        s.active_step = None;
                                                        s.step_history.clear();
                                                    }
                                                    Some(Marker::Step(step)) => {
                                                        let mut s = state.write();
                                                        s.active_step = Some(step.clone());
                                                        complete_up_to(&mut s.step_history, &step);
                                                    }
                                                    // Completion does NOT move the cursor. A step
                                                    // that finished is not the step now running,
                                                    // and conflating the two is how a hung step
                                                    // looks identical to a slow one.
                                                    Some(Marker::StepDone(step)) => {
                                                        let mut s = state.write();
                                                        complete_up_to(&mut s.step_history, &step);
                                                        if !s.step_history.contains(&step) {
                                                            s.step_history.push(step);
                                                        }
                                                    }
                                                    // A line that LOOKS like a marker but names
                                                    // nothing known is surfaced, never dropped. A
                                                    // typo in a script is otherwise indistinguishable
                                                    // from a step that simply never ran -- which is
                                                    // precisely the silent failure this parser exists
                                                    // to end.
                                                    None if markers::is_unrecognised(&log_msg.content) => {
                                                        state.write().logs.push(LogMsg {
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
                                                state.write().logs.push(log_msg);
                                            }
                                            kill = rx.next() => {
                                                if let Some(ProcessCommand::Kill) = kill {
                                                    if let Some(mut p) = child_proc.take() {
                                                        let _ = p.kill().await;
                                                        state.write().logs.push(LogMsg {
                                                            timestamp: Local::now(),
                                                            stream: StreamType::System,
                                                            content: "Process killed by user.".to_string(),
                                                        });
                                                        state.write().script_status = ScriptStatus::Cancelled;
                                                        state.write().active_end_time = Some(Local::now());
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
                                                state.write().active_end_time = Some(Local::now());
                                                let end_time = Local::now();
                                                let mut exit_code = -1;
                                                
                                                if let Ok(exit_status) = status {
                                                    exit_code = exit_status.code().unwrap_or(-1);
                                                    if exit_status.success() {
                                                        state.write().script_status = ScriptStatus::Succeeded;
                                                    } else {
                                                        state.write().script_status = ScriptStatus::Failed(exit_code);
                                                    }
                                                } else {
                                                    state.write().script_status = ScriptStatus::AppError("Failed to wait on process".to_string());
                                                }
                                                
                                                state.write().logs.push(LogMsg {
                                                    timestamp: Local::now(),
                                                    stream: StreamType::System,
                                                    content: format!("Process exited with code {}", exit_code),
                                                });
                                                
                                                // Save to history
                                                let entry = HistoryEntry {
                                                    script_name: script.clone(),
                                                    environment: env,
                                                    start_time: state.read().active_start_time.unwrap_or(end_time),
                                                    end_time,
                                                    status: state.read().script_status.clone(),
                                                };
                                                state.write().history.push(entry);
                                                
                                                child_proc = None;
                                                break;
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    state.write().logs.push(LogMsg {
                                        timestamp: Local::now(),
                                        stream: StreamType::System,
                                        content: format!("Failed to spawn process: {}", e),
                                    });
                                    state.write().script_status = ScriptStatus::AppError(e.to_string());
                                }
                            }
                        },
                        Some(ProcessCommand::Kill) => {
                            if let Some(mut p) = child_proc.take() {
                                let _ = p.kill().await;
                            }
                            state.write().script_status = ScriptStatus::Cancelled;
                            state.write().active_end_time = Some(Local::now());
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
                // Pushes the picker to the trailing edge at every width.
                div { class: "ml-auto" }
                crate::components::theme_picker::ThemePicker {}
            }
            
            // Body
            // min-h-0 is what lets the log pane scroll instead of pushing
            // the window taller: a flex child defaults to min-height:auto.
            div { class: "flex min-h-0 flex-1",
                Sidebar {
                    selected_script: state.read().selected_script.clone(),
                    on_select: move |script: String| {
                        let mut s = state.write();
                        s.selected_script = Some(script.clone());
                        s.script_status = ScriptStatus::Idle;
                        s.logs.clear();
                        s.active_start_time = None;
                        s.active_end_time = None;
                        
                    },
                }
                
                Dashboard {
                    state: state.clone(),
                    on_env_select: move |env| {
                        state.write().selected_env = Some(env);
                    },
                    on_run: move |_| {
                        let s = state.read();
                        if let (Some(script), Some(env)) = (&s.selected_script, s.selected_env) {
                            process_coroutine.send(ProcessCommand::Run { 
                                script: script.clone(), 
                                env 
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
