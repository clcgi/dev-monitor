use dioxus::prelude::*;
use chrono::Local;
use crate::services::state::{Environment, ScriptStatus, AppState};
use crate::components::environment_selector::EnvironmentSelector;
use crate::components::log_viewer::LogViewer;
use crate::components::workflow_stepper::WorkflowStepper;
use crate::components::verdict_panel::VerdictPanel;
use crate::components::trace_panel::TracePanel;
use crate::components::arg_picker::ArgPicker;
use crate::components::choice_picker::ChoicePicker;

#[derive(Props, Clone, PartialEq)]
pub struct DashboardProps {
    pub state: Signal<AppState>,
    pub on_env_select: EventHandler<Environment>,
    pub on_run: EventHandler<()>,
    pub on_stop: EventHandler<()>,
    pub on_toggle_arg: EventHandler<String>,
    /// (flag, value) for a `CDW_CHOICE` flag.
    pub on_choose: EventHandler<(String, String)>,
    pub on_jump_to_run: EventHandler<String>,
    pub on_jump_to_step: EventHandler<crate::services::steps::StepId>,
    pub on_toggle_logs: EventHandler<()>,
    /// A once-a-second counter while a script runs.
    pub tick: u64,
    pub logs_open: bool,
    /// (nonce, line index).
    pub log_jump: Signal<Option<(u64, usize)>>,
    /// Which tab the bottom band shows.
    pub traces_tab: bool,
    pub on_tab: EventHandler<bool>,
}

#[component]
pub fn Dashboard(props: DashboardProps) -> Element {
    let state = props.state.read();
    // Everything about the run belongs to the SELECTED script, not to the app.
    let run = state.current();
    let trace_line_count: usize = run.traces.values().map(|v| v.len()).sum();
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
                            // TIME SITS WITH STATUS, not in a band of its own.
                            // "Running" and "for how long" are one fact read in
                            // one glance, and they were two rows apart with the
                            // controls between them -- so the eye that found the
                            // pill had to travel past a button to learn the age
                            // of the run. Rendered as fine print beside the
                            // pill: it is a subtitle to the status, not a
                            // headline of its own.
                            div { class: "flex shrink-0 items-center gap-3",
                                if has_started {
                                    span { class: "font-mono text-caption text-fg-faint tabular-nums",
                                        "{started_str}"
                                        if !duration_str.is_empty() {
                                            span { class: "text-fg-muted", "  ·  {duration_str}" }
                                        }
                                    }
                                }
                                div { class: "flex shrink-0 items-center gap-2 rounded-full px-3 py-1 text-caption-strong                                           {status_class} bg-black/5 dark:bg-white/5 border border-border-soft",
                                    span { class: "inline-block size-2 shrink-0 rounded-full {dot_class}" }
                                    "{status_str}{failed_code}"
                                }
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

                    // ONE PANEL, not four. Duration, the stepper and the verdicts
                    // are not separate concerns -- they are the same run, and as
                    // four bordered cards they read as four unrelated widgets
                    // stacked by accident. They extend the control panel now,
                    // separated by rules rather than by borders.
                    div {
                        class: "flex shrink-0 flex-col rounded-2xl border border-border-soft \
                                bg-card shadow-sm",

                        div { class: "flex flex-col gap-4 p-5",
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
                                            class: "rounded-full border border-danger/30 bg-danger/10 px-6 py-2 text-button-utility text-danger hover:bg-danger hover:text-white transition-all scale-100 active:scale-95",
                                            onclick: move |_| props.on_stop.call(()),
                                            "Cancel"
                                        }
                                    } else {
                                        button {
                                            class: "rounded-full bg-accent px-6 py-2 text-button-utility text-white shadow-sm hover:opacity-90 transition-all scale-100 active:scale-95 disabled:cursor-default disabled:opacity-50 disabled:scale-100",
                                            disabled: no_env || busy_with.is_some(),
                                            onclick: move |_| props.on_run.call(()),
                                            "{btn_text}"
                                        }
                                    }
                                }
                            }

                            // Above the toggles: the case is WHAT runs, the
                            // toggles only modify how, and reading them in the
                            // other order invites launching the default case.
                            ChoicePicker {
                                choices: state.selected_meta.as_ref().map(|m| m.choices.clone()).unwrap_or_default(),
                                chosen: run.chosen.clone(),
                                disabled: is_running,
                                on_choose: move |pair: (String, String)| props.on_choose.call(pair),
                            }

                            ArgPicker {
                                args: state.selected_meta.as_ref().map(|m| m.args.clone()).unwrap_or_default(),
                                enabled: run.enabled_args.clone(),
                                disabled: is_running,
                                on_toggle: move |flag: String| props.on_toggle_arg.call(flag),
                            }
                        }

                        // Everything below appears only once a run exists, so an
                        // idle panel is the controls and nothing else.
                        // The Started / Duration band used to sit here, at 15px
                        // mono under uppercase labels -- the largest thing in
                        // the panel after the title, for two values nobody
                        // reads twice. It is now fine print beside the status
                        // pill, which is where the eye already goes, and the
                        // panel is one horizontal band shorter.

                        if has_started && !state.selected_meta.as_ref().is_some_and(|m| m.has_no_steps()) {
                            div { class: "border-t border-border-soft",
                                WorkflowStepper {
                                    steps: state.selected_meta.as_ref().and_then(|m| m.steps().map(|s| s.to_vec())),
                                    step_elapsed_s: run.step_started.map(|t| {
                                        Local::now().signed_duration_since(t).num_seconds().max(0) as u64
                                    }),
                                    step_seconds: run.step_seconds.clone(),
                                    active_step: run.active_step.clone(),
                                    step_history: run.step_history.clone(),
                                    is_running,
                                    is_failed,
                                    is_succeeded,
                                    catalog: state.catalog.clone(),
                                    on_jump: move |id| props.on_jump_to_step.call(id),
                                }
                            }
                        }

                        if !run.verdicts.is_empty() {
                            div { class: "border-t border-border-soft",
                                VerdictPanel {
                                    verdicts: run.verdicts.clone(),
                                    on_jump: move |label: String| props.on_jump_to_run.call(label),
                                }
                            }
                        }
                    }

                    // The bottom band: the run's own output, and the platform's.
                    // Two tabs rather than two panels, because they are read one
                    // at a time and the window has room for one of them at a
                    // usable height.
                    div {
                        class: if props.logs_open {
                            "flex min-h-[16rem] flex-1 flex-col overflow-hidden rounded-2xl \
                             border border-border-soft bg-[#1E1E1E] shadow-inner mt-2"
                        } else {
                            "flex shrink-0 flex-col overflow-hidden rounded-2xl border \
                             border-border-soft bg-card shadow-sm mt-2"
                        },
                        div {
                            class: if props.logs_open {
                                "flex shrink-0 items-center bg-black/40"
                            } else {
                                "flex shrink-0 items-center bg-transparent"
                            },
                            button {
                                r#type: "button",
                                class: if props.logs_open {
                                    "flex shrink-0 items-center gap-2 px-4 py-3 text-caption-strong text-white/70 hover:text-white transition-colors"
                                } else {
                                    "flex shrink-0 items-center gap-2 px-4 py-3 text-caption-strong text-fg-muted hover:text-fg transition-colors"
                                },
                                onclick: move |_| props.on_toggle_logs.call(()),
                                i {
                                    class: if props.logs_open { "ph ph-caret-down text-lg" } else { "ph ph-caret-right text-lg" },
                                }
                                span { "Output" }
                            }

                            if props.logs_open {
                                div { class: "flex items-center gap-1",
                                    button {
                                        r#type: "button",
                                        class: if !props.traces_tab {
                                            "rounded-md bg-white/10 px-3 py-1 text-caption-strong text-white"
                                        } else {
                                            "rounded-md px-3 py-1 text-caption-strong text-white/50 hover:text-white"
                                        },
                                        onclick: move |_| props.on_tab.call(false),
                                        "Technical Logs"
                                    }
                                    button {
                                        r#type: "button",
                                        class: if props.traces_tab {
                                            "flex items-center gap-1.5 rounded-md bg-white/10 px-3 py-1 text-caption-strong text-white"
                                        } else {
                                            "flex items-center gap-1.5 rounded-md px-3 py-1 text-caption-strong text-white/50 hover:text-white"
                                        },
                                        onclick: move |_| props.on_tab.call(true),
                                        "Platform traces"
                                        // The count is the affordance: without it
                                        // nothing says the tab has anything in it.
                                        if !run.traces.is_empty() {
                                            span { class: "rounded-full bg-white/20 px-1.5 text-[10px]",
                                                "{run.traces.len()}" }
                                        }
                                    }
                                }
                            }

                            // `text-caption`, not `text-xs`: the theme replaces
                            // Tailwind's scale, so this was the one label in the
                            // band sized off-system.
                            span { class: "ml-auto px-4 font-mono text-caption tabular-nums opacity-60",
                                if props.logs_open && props.traces_tab {
                                    "{trace_line_count} lines"
                                } else {
                                    "{run.logs.len()} lines"
                                }
                            }
                        }
                        if props.logs_open {
                            if props.traces_tab {
                                TracePanel {
                                    traces: run.traces.clone(),
                                    in_trace: run.in_trace.clone(),
                                    is_running,
                                }
                            } else {
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
}
