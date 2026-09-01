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
    /// A once-a-second counter while a script runs. Never read for its
    /// value -- it exists so this component re-renders during a wait that
    /// produces no log lines, which is what keeps the elapsed times moving.
    pub tick: u64,
    pub logs_open: bool,
    /// (nonce, line index). A Signal rather than a value so the log viewer's
    /// effect has something reactive to depend on -- see LogViewerProps::jump.
    pub log_jump: Signal<Option<(u64, usize)>>,
}

#[component]
pub fn Dashboard(props: DashboardProps) -> Element {
    let state = props.state.read();
    // Everything about the run belongs to the SELECTED script, not to the
    // app: another script may be running right now, and its logs are its
    // own. Cloned once so the rest of the render reads a consistent
    // snapshot rather than re-borrowing the signal per field.
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
    // Something else is mid-run. Only ONE process is spawned at a time, and a
    // Run sent now sits unread in the channel until the current one exits --
    // the button appeared to do nothing, and then the wrong script started
    // minutes later. Say so instead.
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
                div { class: "flex flex-1 flex-col items-center justify-center gap-2 p-10 text-center",
                    div { class: "text-[15px] text-fg-soft", "Select a script" }
                    div { class: "text-xs text-fg-faint",
                        "Choose a script from the sidebar to monitor or run." }
                }
            } else {
                // THE HEIGHT CONTRACT, and it is the whole fix for the cropped
                // log pane. Every ancestor from here down carries `min-h-0`,
                // because a flex child's default `min-height:auto` refuses to
                // shrink below its content -- so the controls above pushed the
                // log pane off the bottom of a short window with nothing able to
                // scroll. Now the controls scroll within a capped band and the
                // log pane takes the rest.
                div { class: "flex h-full min-h-0 flex-col gap-3 p-4 sm:p-5",

                    // The header stays put; only the controls below it scroll.
                    div { class: "flex shrink-0 flex-col gap-1",
                        div { class: "flex items-center gap-2",
                            div { class: "flex-1 truncate text-[15px] font-semibold text-fg",
                                "{script_name}" }
                            div { class: "flex shrink-0 items-center text-[11px] uppercase \
                                          tracking-wider {status_class}",
                                span { class: "mr-1.5 inline-block size-2 shrink-0 rounded-full {dot_class}" }
                                "{status_str}{failed_code}"
                            }
                        }
                        // What the script DOES, from its own header. The file
                        // name says `flow_5_quarantine` and not what a pass
                        // means, and that sentence already exists in the script
                        // -- it was only ever shown in a sidebar tooltip.
                        if let Some(meta) = state.selected_meta.as_ref() {
                            div { class: "flex flex-wrap items-center gap-2 text-[11px] text-fg-faint",
                                span {
                                    class: "rounded border border-edge px-1.5 py-0.5 uppercase tracking-wider",
                                    "{meta.category}"
                                }
                                if !meta.summary.is_empty() {
                                    span { class: "min-w-0 flex-1 leading-relaxed", "{meta.summary}" }
                                }
                            }
                        }
                    }

                    // The controls band. Capped and scrollable so that on a short
                    // window it gives way to the logs instead of squeezing them out.
                    div { class: "flex max-h-[55%] min-h-0 shrink flex-col gap-3 overflow-y-auto pr-1",

                        // ONE ROW: environment on the left, run on the right.
                        // These were stacked in a column with a paragraph of
                        // guidance between them, so the primary action sat below
                        // the fold on a short window -- and the two halves of a
                        // single decision read as two unrelated sections.
                        div {
                            class: "flex shrink-0 flex-col gap-3 rounded-lg border border-edge \
                                    bg-surface p-3",
                            div { class: "flex flex-col gap-3 sm:flex-row sm:items-center",
                                div { class: "min-w-0 flex-1",
                                    EnvironmentSelector {
                                        selected: state.selected_env,
                                        on_select: move |e| props.on_env_select.call(e),
                                    }
                                }
                                div { class: "flex shrink-0 items-center gap-3",
                                    if no_env {
                                        span { class: "text-[11px] text-warn", "Select an environment" }
                                    }
                                    if let Some(other) = busy_with.as_ref() {
                                        span { class: "text-[11px] text-warn",
                                            "{other.replace(\"tools/\", \"\")} is still running" }
                                    }
                                    if is_running {
                                        button {
                                            class: "rounded-lg border border-danger bg-transparent px-3.5 \
                                                    py-1.5 font-mono text-xs text-danger \
                                                    hover:bg-danger-deep hover:text-white",
                                            onclick: move |_| props.on_stop.call(()),
                                            "Cancel"
                                        }
                                    } else {
                                        button {
                                            class: "rounded-lg border border-brand-deep bg-brand-deep px-3.5 \
                                                    py-1.5 font-mono text-xs text-white \
                                                    hover:border-brand hover:bg-brand \
                                                    disabled:cursor-default disabled:opacity-50",
                                            disabled: no_env || busy_with.is_some(),
                                            onclick: move |_| props.on_run.call(()),
                                            "{btn_text}"
                                        }
                                    }
                                }
                            }

                            // INSIDE the card, under the row it modifies. As a
                            // sibling it read as an unrelated section, and the
                            // flags belong to the run button beside them -- most
                            // of all `--apply`, whose consequence has to be
                            // visible in the same glance as the button that
                            // applies it.
                            ArgPicker {
                                args: state.selected_meta.as_ref().map(|m| m.args.clone()).unwrap_or_default(),
                                enabled: run.enabled_args.clone(),
                                disabled: is_running,
                                on_toggle: move |flag: String| props.on_toggle_arg.call(flag),
                            }
                        }

                        if has_started {
                            div {
                                class: "flex shrink-0 flex-wrap gap-6 rounded-lg border \
                                        border-edge-soft bg-elevated px-4 py-2.5",
                                div { class: "flex flex-col gap-0.5",
                                    span { class: "text-[10px] uppercase tracking-wider text-fg-faint",
                                        "Started" }
                                    span { class: "font-mono text-xs text-fg", "{started_str}" }
                                }
                                div { class: "flex flex-col gap-0.5",
                                    span { class: "text-[10px] uppercase tracking-wider text-fg-faint",
                                        "Duration" }
                                    span { class: "font-mono text-xs text-fg", "{duration_str}" }
                                }
                            }
                        }

                        // A script that declares `steps=none` gets NO stepper. It
                        // reaches no pipeline stage, and eleven grey nodes above
                        // its output said otherwise -- which is what
                        // reset_test_documents looked like before the declaration
                        // existed.
                        if has_started && !state.selected_meta.as_ref().is_some_and(|m| m.has_no_steps()) {
                            WorkflowStepper {
                                // Only the stages this script says it can reach.
                                // A stepper showing eleven stages for a flow that
                                // touches four spends most of its width on nodes
                                // that will never light, and the four that matter
                                // are indistinguishable from the seven that were
                                // never going to happen.
                                steps: state.selected_meta.as_ref().and_then(|m| m.steps().map(|s| s.to_vec())),
                                // Seconds in the CURRENT stage. The one thing
                                // on screen that separates waiting from stuck.
                                step_elapsed_s: run.step_started.map(|t| {
                                    Local::now().signed_duration_since(t).num_seconds().max(0) as u64
                                }),
                                active_step: run.active_step.clone(),
                                step_history: run.step_history.clone(),
                                is_running,
                                is_failed,
                                is_succeeded,
                            }
                        }

                        // The stepper shows WHERE a run got to, this shows WHAT
                        // it concluded. A suite's later flow can be mid-chain
                        // while three earlier verdicts already stand.
                        VerdictPanel {
                            verdicts: run.verdicts.clone(),
                            on_jump: move |label: String| props.on_jump_to_run.call(label),
                        }
                    }

                    // NOT a `details` element any more. `details` gives no way
                    // to say "fill the remaining height when open", so the pane
                    // had a fixed h-64/h-96 and was cropped by whatever the
                    // controls above happened to need. As a controlled block it
                    // can be `flex-1 min-h-0` when open and `shrink-0` when shut.
                    div {
                        class: if props.logs_open {
                            "flex min-h-[8rem] flex-1 flex-col overflow-hidden rounded-md \
                             border border-edge bg-surface"
                        } else {
                            "flex shrink-0 flex-col overflow-hidden rounded-md border \
                             border-edge bg-surface"
                        },
                        button {
                            r#type: "button",
                            class: "flex shrink-0 items-center gap-1.5 bg-elevated px-3 py-2 \
                                    text-[10px] uppercase tracking-wider text-fg-faint \
                                    hover:text-fg-soft",
                            onclick: move |_| props.on_toggle_logs.call(()),
                            i {
                                class: if props.logs_open { "ph ph-caret-down" } else { "ph ph-caret-right" },
                            }
                            span { "Technical Logs" }
                            span { class: "ml-2 opacity-60", "{run.logs.len()}" }
                        }
                        if props.logs_open {
                            div { class: "flex min-h-0 flex-1 flex-col p-3",
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
