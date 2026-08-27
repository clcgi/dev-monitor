use dioxus::prelude::*;
use chrono::Local;
use crate::services::state::{Environment, ScriptStatus, AppState};
use crate::components::environment_selector::EnvironmentSelector;
use crate::components::log_viewer::LogViewer;
use crate::components::history::HistoryPanel;
use crate::components::workflow_stepper::WorkflowStepper;

#[derive(Props, Clone, PartialEq)]
pub struct DashboardProps {
    pub state: Signal<AppState>,
    pub on_env_select: EventHandler<Environment>,
    pub on_run: EventHandler<()>,
    pub on_stop: EventHandler<()>,
}

#[component]
pub fn Dashboard(props: DashboardProps) -> Element {
    let state = props.state.read();
    let script_name = state
        .selected_script
        .as_deref()
        .map(|s| s.replace("tools/", ""))
        .unwrap_or_else(|| "No script selected".to_string());

    let status = &state.script_status;
    let is_running = matches!(status, ScriptStatus::Running);

    let is_succeeded = matches!(status, ScriptStatus::Succeeded);
    let is_failed = matches!(status, ScriptStatus::Failed(_) | ScriptStatus::AppError(_));

    // `status-warning` and `dot-warning` were referenced here and defined
    // NOWHERE, so a Cancelled run showed uncoloured text beside an invisible
    // dot -- the one status a user most needs to distinguish from Failed.
    let (status_str, status_class, dot_class) = match status {
        ScriptStatus::Idle => ("Idle", "text-fg-faint", "bg-fg-faint"),
        ScriptStatus::Running => ("Running", "text-info", "bg-info animate-pulse"),
        ScriptStatus::Succeeded => ("Succeeded", "text-brand", "bg-brand"),
        ScriptStatus::Failed(_) => ("Failed", "text-danger", "bg-danger"),
        ScriptStatus::Cancelled => ("Cancelled", "text-warn", "bg-warn"),
        ScriptStatus::AppError(_) => ("App Error", "text-danger", "bg-danger"),
    };

    let duration_str = if let Some(start) = state.active_start_time {
        let end = state.active_end_time.unwrap_or_else(Local::now);
        let d = end.signed_duration_since(start).num_seconds();
        if d < 60 {
            format!("{}s", d)
        } else {
            format!("{}m {}s", d / 60, d % 60)
        }
    } else {
        "--".to_string()
    };
    
    let started_str = state.active_start_time.map(|t| t.format("%H:%M:%S").to_string()).unwrap_or_default();
    let has_started = state.active_start_time.is_some();
    let no_script = state.selected_script.is_none();
    let no_env = state.selected_env.is_none();
    
    let btn_text = if matches!(status, ScriptStatus::Succeeded | ScriptStatus::Failed(_) | ScriptStatus::Cancelled) {
        "Rerun Script"
    } else {
        "Run Script"
    };

    let failed_code = if let ScriptStatus::Failed(code) = status {
        format!(" ({})", code)
    } else {
        "".to_string()
    };

    rsx! {
        div { class: "flex min-h-0 flex-1 flex-col",

            if no_script {
                div { class: "flex flex-1 flex-col items-center justify-center gap-2 p-10 text-center",
                    div { class: "text-[15px] text-fg-soft", "Select a script" }
                    div { class: "text-xs text-fg-faint",
                        "Choose a script from the sidebar to monitor or run." }
                }
            } else {
                div { class: "flex h-full flex-col gap-4 p-4 sm:p-5",

                    // RESPONSIVE: this was two columns at every width, so on a
                    // narrow window the history card squeezed the controls into a
                    // sliver. It stacks below `lg` and splits 2:1 above it.
                    div { class: "flex flex-col gap-4 lg:flex-row lg:items-stretch lg:gap-6",
                        div { class: "flex flex-col gap-4 lg:flex-[2]",
                            div { class: "flex items-center gap-2",
                                div { class: "flex-1 truncate text-[15px] font-semibold text-fg",
                                    "{script_name}" }
                                div { class: "flex shrink-0 items-center text-[11px] uppercase \
                                              tracking-wider {status_class}",
                                    span { class: "mr-1.5 inline-block size-2 shrink-0 rounded-full {dot_class}" }
                                    "{status_str}{failed_code}"
                                }
                            }

                            if has_started {
                                div {
                                    class: "flex flex-wrap gap-6 rounded-lg border border-edge-soft \
                                            bg-elevated px-4 py-3",
                                    div { class: "flex flex-col gap-1",
                                        span { class: "text-[10px] uppercase tracking-wider text-fg-faint",
                                            "Started" }
                                        span { class: "font-mono text-xs text-fg", "{started_str}" }
                                    }
                                    div { class: "flex flex-col gap-1",
                                        span { class: "text-[10px] uppercase tracking-wider text-fg-faint",
                                            "Duration" }
                                        span { class: "font-mono text-xs text-fg", "{duration_str}" }
                                    }
                                }
                            }

                            div { class: "text-[11px] leading-relaxed text-fg-faint",
                                "Select the target environment before executing." }

                            EnvironmentSelector {
                                selected: state.selected_env,
                                on_select: move |e| props.on_env_select.call(e),
                            }

                            div { class: "flex flex-wrap items-center gap-3",
                                if is_running {
                                    button {
                                        class: "rounded-lg border border-danger bg-transparent px-3.5 \
                                                py-1.5 font-mono text-xs text-danger \
                                                hover:bg-danger-deep hover:text-white",
                                        onclick: move |_| props.on_stop.call(()),
                                        "Cancel Execution"
                                    }
                                } else {
                                    button {
                                        class: "rounded-lg border border-brand-deep bg-brand-deep px-3.5 \
                                                py-1.5 font-mono text-xs text-white \
                                                hover:border-brand hover:bg-brand \
                                                disabled:cursor-default disabled:opacity-50",
                                        disabled: no_env,
                                        onclick: move |_| props.on_run.call(()),
                                        "{btn_text}"
                                    }
                                }

                                if no_env {
                                    span { class: "text-[11px] text-warn",
                                        "Please select an environment" }
                                }
                            }
                        }

                        div { class: "flex flex-col gap-4 lg:flex-1",
                            HistoryPanel { history: state.history.clone() }
                        }
                    }

                    if has_started {
                        WorkflowStepper {
                            active_step: state.active_step.clone(),
                            step_history: state.step_history.clone(),
                            is_running,
                            is_failed,
                            is_succeeded,
                        }
                    }

                    details {
                        class: "overflow-hidden rounded-md border border-edge bg-surface",
                        summary {
                            class: "cursor-pointer bg-elevated px-3 py-2 text-[10px] uppercase \
                                    tracking-wider text-fg-faint hover:text-fg-soft",
                            "Technical Logs"
                        }
                        // The 400px height was inline and fixed; it now grows with
                        // the window instead of stranding the log pane at one size.
                        div { class: "flex h-64 flex-col p-3 lg:h-96",
                            LogViewer { logs: state.logs.clone() }
                        }
                    }
                }
            }
        }
    }
}
