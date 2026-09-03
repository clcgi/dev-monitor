use dioxus::prelude::*;
use chrono::Local;
use crate::services::state::{Environment, ScriptStatus, AppState};
use crate::components::environment_selector::EnvironmentSelector;
use crate::components::log_viewer::LogViewer;
use crate::components::workflow_stepper::WorkflowStepper;
use crate::components::verdict_panel::VerdictPanel;
use crate::components::arg_picker::ArgPicker;

#[derive(Props, Clone, PartialEq)]
pub struct DashboardProps {
    pub state: Signal<AppState>,
    pub on_env_select: EventHandler<Environment>,
    pub on_run: EventHandler<()>,
    pub on_stop: EventHandler<()>,
    pub on_toggle_arg: EventHandler<String>,
    pub on_jump_to_run: EventHandler<String>,
    pub on_toggle_logs: EventHandler<()>,
    /// A once-a-second counter while a script runs.
    pub tick: u64,
    pub logs_open: bool,
    /// (nonce, line index).
    pub log_jump: Signal<Option<(u64, usize)>>,
}

#[component]
pub fn Dashboard(props: DashboardProps) -> Element {
    let state = props.state.read();
    // Everything about the run belongs to the SELECTED script, not to the app.
    let run = state.current();
    let script_name = state
        .selected_script
        .as_deref()
        .map(|s| s.replace("tools/", ""))
        .unwrap_or_else(|| "No script selected".to_string());

    let status = &run.status;
    let is_running = matches!(status, ScriptStatus::Running);

    let is_succeeded = matches!(status, ScriptStatus::Succeeded);
    let is_failed = matches!(status, ScriptStatus::Failed(_) | ScriptStatus::AppError(_));

    let (status_str, status_class, dot_class) = match status {
        ScriptStatus::Idle => ("Idle", "text-fg-faint", "bg-fg-faint"),
        ScriptStatus::Running => ("Running", "text-accent", "bg-accent animate-pulse"),
        ScriptStatus::Succeeded => ("Succeeded", "text-accent", "bg-accent"),
        ScriptStatus::Failed(_) => ("Failed", "text-danger", "bg-danger"),
        ScriptStatus::Cancelled => ("Cancelled", "text-warn", "bg-warn"),
        ScriptStatus::AppError(_) => ("App Error", "text-danger", "bg-danger"),
    };

    let duration_str = if let Some(start) = run.start_time {
        let end = run.end_time.unwrap_or_else(Local::now);
        let d = end.signed_duration_since(start).num_seconds();
        if d < 60 {
            format!("{}s", d)
        } else {
            format!("{}m {}s", d / 60, d % 60)
        }
    } else {
        "--".to_string()
    };
    
    let started_str = run.start_time.map(|t| t.format("%H:%M:%S").to_string()).unwrap_or_default();
    let has_started = run.has_started();
    let no_script = state.selected_script.is_none();
    let no_env = state.selected_env.is_none();
    // Something else is mid-run.
    let busy_with = state
        .running_script
        .clone()
        .filter(|p| state.selected_script.as_ref() != Some(p));
    
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
                div { class: "flex flex-1 flex-col items-center justify-center gap-3 p-10 text-center",
                    div { class: "text-body-strong text-fg-muted", "Select a script" }
                    div { class: "text-caption text-fg-faint",
                        "Choose a script from the sidebar to monitor or run." }
                }
            } else {
                div { class: "flex h-full min-h-0 flex-col gap-4 p-5 sm:p-6 overflow-y-auto",

                    // HERO AREA (Apple style: massive typography, crisp status pill)
                    div { class: "flex shrink-0 flex-col gap-3",
                        div { class: "flex items-center justify-between gap-4",
                            div { class: "flex-1 truncate text-display-lg text-fg",
                                "{script_name}" }
                            div { class: "flex shrink-0 items-center gap-2 rounded-full px-3 py-1 text-caption-strong                                           {status_class} bg-black/5 dark:bg-white/5 border border-border-soft",
                                span { class: "inline-block size-2 shrink-0 rounded-full {dot_class}" }
                                "{status_str}{failed_code}"
                            }
                        }
                        if let Some(meta) = state.selected_meta.as_ref() {
                            div { class: "flex flex-wrap items-center gap-2 text-body text-fg-muted",
                                span {
                                    class: "rounded-md border border-border-soft px-2 py-0.5 text-caption-strong tracking-wide bg-card",
                                    "{meta.category}"
                                }
                                if !meta.summary.is_empty() {
                                    span { class: "min-w-0 flex-1 leading-relaxed text-neutral-500", "{meta.summary}" }
                                }
                            }
                        }
                    }

                    // MAIN CONTROLS BAND
                    div { class: "flex shrink-0 flex-col gap-5",

                        // UTILITY CARD for controls
                        div {
                            class: "flex shrink-0 flex-col gap-4 rounded-2xl border border-border-soft                                     bg-card p-5 shadow-sm",
                            div { class: "flex flex-col gap-4 sm:flex-row sm:items-center",
                                div { class: "min-w-0 flex-1",
                                    EnvironmentSelector {
                                        selected: state.selected_env,
                                        on_select: move |e| props.on_env_select.call(e),
                                    }
                                }
                                div { class: "flex shrink-0 items-center gap-4",
                                    if no_env {
                                        span { class: "text-caption text-warn", "Select an environment" }
                                    }
                                    if let Some(other) = busy_with.as_ref() {
                                        span { class: "text-caption text-warn",
                                            "{other.replace(\"tools/\", \"\")} is running" }
                                    }
                                    if is_running {
                                        button {
                                            class: "rounded-full border border-danger/30 bg-danger/10 px-6                                                     py-2 text-button-utility text-danger                                                     hover:bg-danger hover:text-white transition-all scale-100 active:scale-95",
                                            onclick: move |_| props.on_stop.call(()),
                                            "Cancel"
                                        }
                                    } else {
                                        button {
                                            class: "rounded-full bg-accent px-6                                                     py-2 text-button-utility text-white shadow-sm                                                     hover:opacity-90 transition-all scale-100 active:scale-95                                                     disabled:cursor-default disabled:opacity-50 disabled:scale-100",
                                            disabled: no_env || busy_with.is_some(),
                                            onclick: move |_| props.on_run.call(()),
                                            "{btn_text}"
                                        }
                                    }
                                }
                            }

                            // ArgPicker matches the minimal style inside the card
                            ArgPicker {
                                args: state.selected_meta.as_ref().map(|m| m.args.clone()).unwrap_or_default(),
                                enabled: run.enabled_args.clone(),
                                disabled: is_running,
                                on_toggle: move |flag: String| props.on_toggle_arg.call(flag),
                            }
                        }

                        if has_started {
                            div {
                                class: "flex shrink-0 flex-wrap gap-6 rounded-2xl border                                         border-border-soft bg-card px-4 py-3 shadow-sm",
                                div { class: "flex flex-col gap-1",
                                    span { class: "text-caption-strong text-fg-faint uppercase tracking-wider",
                                        "Started" }
                                    span { class: "font-mono text-[15px] text-fg", "{started_str}" }
                                }
                                div { class: "flex flex-col gap-1",
                                    span { class: "text-caption-strong text-fg-faint uppercase tracking-wider",
                                        "Duration" }
                                    span { class: "font-mono text-[15px] text-fg", "{duration_str}" }
                                }
                            }
                        }

                        if has_started && !state.selected_meta.as_ref().is_some_and(|m| m.has_no_steps()) {
                            WorkflowStepper {
                                steps: state.selected_meta.as_ref().and_then(|m| m.steps().map(|s| s.to_vec())),
                                step_elapsed_s: run.step_started.map(|t| {
                                    Local::now().signed_duration_since(t).num_seconds().max(0) as u64
                                }),
                                active_step: run.active_step.clone(),
                                step_history: run.step_history.clone(),
                                is_running,
                                is_failed,
                                is_succeeded,
                                catalog: state.catalog.clone(),
                            }
                        }

                        VerdictPanel {
                            verdicts: run.verdicts.clone(),
                            on_jump: move |label: String| props.on_jump_to_run.call(label),
                        }
                    }

                    // TECHNICAL LOGS (Dark Terminal Tile)
                    div {
                        class: if props.logs_open {
                            "flex min-h-[16rem] flex-1 flex-col overflow-hidden rounded-2xl                              border border-border-soft bg-[#1E1E1E] shadow-inner mt-2"
                        } else {
                            "flex shrink-0 flex-col overflow-hidden rounded-2xl border                              border-border-soft bg-card shadow-sm mt-2"
                        },
                        button {
                            r#type: "button",
                            class: if props.logs_open {
                                "flex shrink-0 items-center gap-2 bg-black/40 px-4 py-3                                  text-caption-strong text-white/70 hover:text-white transition-colors"
                            } else {
                                "flex shrink-0 items-center gap-2 bg-transparent px-4 py-3                                  text-caption-strong text-fg-muted hover:text-fg transition-colors"
                            },
                            onclick: move |_| props.on_toggle_logs.call(()),
                            i {
                                class: if props.logs_open { "ph ph-caret-down text-lg" } else { "ph ph-caret-right text-lg" },
                            }
                            span { "Technical Logs" }
                            span { class: "ml-auto opacity-60 text-xs font-mono", "{run.logs.len()} lines" }
                        }
                        if props.logs_open {
                            div { class: "flex min-h-0 flex-1 flex-col p-4",
                                LogViewer {
                                    logs: run.logs.clone(),
                                    jump: props.log_jump,
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
