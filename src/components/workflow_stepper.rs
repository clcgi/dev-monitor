use dioxus::prelude::*;
use crate::services::state::WorkflowStep;

#[derive(Props, Clone, PartialEq)]
pub struct WorkflowStepperProps {
    /// The stages to draw.
    pub steps: Option<Vec<WorkflowStep>>,
    /// Seconds spent in the CURRENT stage, when one is running.
    pub step_elapsed_s: Option<u64>,
    pub active_step: Option<WorkflowStep>,
    pub step_history: Vec<WorkflowStep>,
    pub is_running: bool,
    pub is_failed: bool,
    pub is_succeeded: bool,
}

#[component]
pub fn WorkflowStepper(props: WorkflowStepperProps) -> Element {
    // One definition of the chain, shared with the marker parser.
    let all_steps: Vec<WorkflowStep> = match &props.steps {
        Some(declared) => WorkflowStep::LINEAR
            .iter()
            .filter(|s| declared.contains(s))
            .cloned()
            .collect(),
        None => WorkflowStep::LINEAR.to_vec(),
    };
    // A declaration of only exception zones would leave this empty.
    let all_steps = if all_steps.is_empty() {
        WorkflowStep::LINEAR.to_vec()
    } else {
        all_steps
    };

    let mut current_idx = 0;
    if let Some(active) = &props.active_step {
        current_idx = all_steps.iter().position(|s| s == active).unwrap_or(0);
    } else if props.is_succeeded {
        current_idx = all_steps.len() - 1;
    } else if !props.is_running && !props.is_failed {
        current_idx = 0; // Idle
    }

    // Special handling if current step is a zone like Quarantine/Rejected
    let is_zone = matches!(
        props.active_step,
        Some(WorkflowStep::Quarantine) | Some(WorkflowStep::Rejected)
    );

    // Status is now a VALUE, not a class name.
    #[derive(PartialEq, Clone, Copy)]
    enum NodeState { Completed, Active, Failed, Zone, Pending }

    let state_of = |step: &WorkflowStep, idx: usize| -> NodeState {
        if let Some(active) = &props.active_step {
            if step == active {
                if props.is_failed { return NodeState::Failed; }
                if props.is_succeeded { return NodeState::Completed; }
                if is_zone { return NodeState::Zone; }
                return NodeState::Active;
            }
        }
        if props.step_history.contains(step) || idx < current_idx || props.is_succeeded {
            return NodeState::Completed;
        }
        NodeState::Pending
    };

    let get_step_icon = |step: &WorkflowStep| -> &'static str {
        match step {
            WorkflowStep::Neo => "ph-database",
            WorkflowStep::Authentication => "ph-lock-key",
            WorkflowStep::Apim => "ph-cloud",
            WorkflowStep::Landing => "ph-folder-simple",
            WorkflowStep::EventGrid => "ph-lightning",
            WorkflowStep::Raw => "ph-file-code",
            WorkflowStep::ServiceBus => "ph-envelope-simple",
            WorkflowStep::ContainerAppJobs => "ph-cpu",
            WorkflowStep::Processing => "ph-gear",
            WorkflowStep::Curated => "ph-medal",
            WorkflowStep::Verification => "ph-check-circle",
            WorkflowStep::Quarantine => "ph-warning-circle",
            WorkflowStep::Rejected => "ph-x-circle",
        }
    };

    let render_node = |step: &WorkflowStep, idx: usize, is_current: bool| -> Element {
        let state = state_of(step, idx);
        let name = step.name();

        // ring-4 in the app colour punches the connecting line out from behind.
        let circle = match state {
            NodeState::Completed => "bg-fg-soft border-fg-soft text-app animate-land",
            NodeState::Active if props.is_running =>
                "bg-brand border-brand text-app animate-breathe shadow-lg shadow-brand/30",
            NodeState::Active    => "bg-brand border-brand text-app animate-pop",
            NodeState::Failed    => "bg-danger border-danger text-app animate-pop",
            NodeState::Zone      => "bg-warn border-warn text-app animate-pop",
            NodeState::Pending   => "bg-surface border-edge text-fg",
        };
        // The NEXT stage telegraphs where the run is heading.
        let anticipate = if props.is_running && state == NodeState::Pending && idx == current_idx + 1 {
            " animate-ready"
        } else {
            ""
        };
        let label = match state {
            NodeState::Completed => "text-fg",
            NodeState::Active    => "text-brand font-semibold",
            NodeState::Failed    => "text-danger",
            NodeState::Zone      => "text-warn",
            NodeState::Pending   => "text-fg-faint",
        };
        // RESPONSIVE: nodes were a fixed 120px, so eleven of them needed 1320px.
        let scale = if is_current { "scale-110" } else { "" };

        rsx! {
            div {
                class: "relative z-[2] flex w-16 shrink-0 flex-col items-center gap-2 \
                        transition-transform duration-300 sm:w-20 lg:w-28 {scale}{anticipate}",
                div {
                    class: "relative flex size-10 items-center justify-center rounded-full \
                            border-2 ring-4 ring-app transition-colors duration-300 {circle}",
                    // Its own element, so the ring is not clipped by the node's own ring-4.
                    if state == NodeState::Active && props.is_running {
                        // TWO rings, the second offset by half the cycle.
                        span {
                            class: "pointer-events-none absolute inset-0 rounded-full animate-halo",
                        }
                        span {
                            class: "pointer-events-none absolute inset-0 rounded-full animate-halo-late",
                        }
                    }
                    if state == NodeState::Completed {
                        i { class: "ph-fill ph-check text-xl" }
                    } else if state == NodeState::Failed {
                        i { class: "ph-fill ph-x text-xl" }
                    } else if state == NodeState::Active && props.is_running {
                        i { class: "ph ph-spinner ph-spin text-xl" }
                    } else if state == NodeState::Active {
                        // Reached but not running: the run finished, was cancelled, or is between.
                        i { class: "ph-fill {get_step_icon(step)} text-xl" }
                    } else {
                        i { class: "ph-fill {get_step_icon(step)} text-xl" }
                    }
                }
                // The label is the first thing to go when space runs out; the icon still.
                div { class: "hidden text-center text-[10px] leading-tight sm:block {label}",
                    "{name}"
                }
                // Seconds in this stage: the only thing separating waiting from stuck.
                if is_current && props.is_running {
                    if let Some(secs) = props.step_elapsed_s {
                        span {
                            class: "rounded-full bg-brand/15 px-1.5 py-px font-mono text-[9px] \
                                    tabular-nums text-brand",
                            {format_elapsed(secs)}
                        }
                    }
                }
            }
        }
    };

    rsx! {
        div {
            class: "relative mb-5 flex items-center justify-center overflow-x-auto rounded-lg \
                    border border-edge bg-surface px-4 py-6 sm:px-5 sm:py-8",
            div { class: "flex w-full max-w-3xl items-center justify-center",
                for (idx, step) in all_steps.iter().enumerate() {
                    {render_node(step, idx, if is_zone { false } else { idx == current_idx })}

                    if idx < all_steps.len() - 1 {
                        {
                            let done = idx + 1 <= current_idx
                                || props.step_history.contains(&all_steps[idx + 1])
                                || props.is_succeeded;
                            // THE SEGMENT BEING TRAVELLED. Exactly one connector is in flight at a time.
                            let in_flight = props.is_running && !done && idx == current_idx;
                            // Computed here rather than as an `if/else if` chain inside the attribute.
                            const TRACK: &str = "z-[1] -mx-3 min-w-2 flex-1";
                            let line = if in_flight {
                                // Thicker while travelling, so the band has room to read as a band.
                                format!(
                                    "{TRACK} relative h-1 rounded-full animate-flow \
                                     bg-[length:200%_100%] \
                                     bg-[linear-gradient(90deg,var(--color-edge)_0%,var(--color-brand)_50%,var(--color-edge)_100%)]"
                                )
                            } else if done {
                                format!("{TRACK} h-0.5 bg-fg-soft transition-all duration-500")
                            } else {
                                format!("{TRACK} h-0.5 bg-edge transition-all duration-500")
                            };
                            rsx! {
                                div { class: "{line}",
                                    // A moving object reads as motion where a moving fill reads as a stripe.
                                    if in_flight {
                                        span {
                                            class: "pointer-events-none absolute top-1/2 size-2 \
                                                    rounded-full bg-brand shadow-md shadow-brand/50 \
                                                    animate-travel",
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if is_zone {
                    div { class: "z-[1] -mx-3 h-0.5 min-w-2 flex-1 bg-warn" }
                    {render_node(props.active_step.as_ref().unwrap(), all_steps.len(), true)}
                }
            }
        }
    }
}

/// `45s`, `2m 05s`.
fn format_elapsed(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else {
        format!("{}m {:02}s", secs / 60, secs % 60)
    }
}
