use dioxus::prelude::*;
use crate::services::steps::{StepCatalog, StepId};

#[derive(Props, Clone, PartialEq)]
pub struct WorkflowStepperProps {
    /// The stages to draw.
    pub steps: Option<Vec<StepId>>,
    /// Seconds spent in the CURRENT stage, when one is running.
    pub step_elapsed_s: Option<u64>,
    pub active_step: Option<StepId>,
    pub step_history: Vec<StepId>,
    pub is_running: bool,
    pub is_failed: bool,
    pub is_succeeded: bool,
    pub catalog: StepCatalog,
}

#[component]
pub fn WorkflowStepper(props: WorkflowStepperProps) -> Element {
    // In the CATALOG's order, not the order a script listed them: a declaration
    // written out of order would draw a chain that runs backwards.
    let chain: Vec<StepId> = props.catalog.chain().iter().map(|s| s.id.clone()).collect();
    let all_steps: Vec<StepId> = match &props.steps {
        Some(declared) => chain.iter().filter(|id| declared.contains(id)).cloned().collect(),
        None => chain.clone(),
    };
    // A declaration naming only branches -- or only steps since removed --
    // leaves this empty, and an empty track renders as a bare box.
    let all_steps = if all_steps.is_empty() { chain } else { all_steps };

    let mut current_idx = 0;
    if let Some(active) = &props.active_step {
        current_idx = all_steps.iter().position(|s| s == active).unwrap_or(0);
    } else if props.is_succeeded {
        current_idx = all_steps.len() - 1;
    } else if !props.is_running && !props.is_failed {
        current_idx = 0; // Idle
    }

    // Special handling if current step is a zone like Quarantine/Rejected
    // A branch rather than a position on the chain.
    let is_zone = props
        .active_step
        .as_ref()
        .is_some_and(|id| props.catalog.chain_index(id).is_none());

    // Status is now a VALUE, not a class name.
    #[derive(PartialEq, Clone, Copy)]
    enum NodeState { Completed, Active, Failed, Zone, Pending }

    let state_of = |step: &StepId, idx: usize| -> NodeState {
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


    let render_node = |step: &StepId, idx: usize, is_current: bool| -> Element {
        let state = state_of(step, idx);
        let name = props.catalog.name_of(step);

        // ring-4 in the app colour punches the connecting line out from behind.
        let circle = match state {
            NodeState::Completed => "bg-fg-muted border-fg-muted text-card animate-land",
            NodeState::Active if props.is_running =>
                "bg-accent border-accent text-card animate-breathe shadow-lg shadow-accent/30",
            NodeState::Active    => "bg-accent border-accent text-card animate-pop",
            NodeState::Failed    => "bg-danger border-danger text-card animate-pop",
            NodeState::Zone      => "bg-warn border-warn text-card animate-pop",
            NodeState::Pending   => "bg-transparent border-border-soft text-fg-muted",
        };
        // The NEXT stage telegraphs where the run is heading.
        let anticipate = if props.is_running && state == NodeState::Pending && idx == current_idx + 1 {
            " animate-ready"
        } else {
            ""
        };
        let label = match state {
            NodeState::Completed => "text-fg",
            NodeState::Active    => "text-accent font-semibold",
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
                            border-2 ring-4 ring-card transition-colors duration-300 {circle}",
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
                        i { class: "ph-fill {props.catalog.icon_of(step)} text-xl" }
                    } else {
                        i { class: "ph-fill {props.catalog.icon_of(step)} text-xl" }
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
                            class: "rounded-full bg-accent/15 px-1.5 py-px font-mono text-[9px] \
                                    tabular-nums text-accent",
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
                    border border-border-soft bg-card shadow-sm rounded-2xl px-4 py-6 sm:px-5 sm:py-8",
            div { class: "flex min-w-full w-max items-center justify-center px-4",
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
                            const TRACK: &str = "z-[1] -mx-3 min-w-8 flex-1";
                            let line = if in_flight {
                                // Thicker while travelling, so the band has room to read as a band.
                                format!(
                                    "{TRACK} relative h-1 rounded-full animate-flow \
                                     bg-[length:200%_100%] \
                                     bg-[linear-gradient(90deg,var(--color-border-soft)_0%,var(--color-accent)_50%,var(--color-border-soft)_100%)]"
                                )
                            } else if done {
                                format!("{TRACK} h-0.5 bg-fg-muted transition-all duration-500")
                            } else {
                                format!("{TRACK} h-0.5 bg-border-soft transition-all duration-500")
                            };
                            rsx! {
                                div { class: "{line}",
                                    // A moving object reads as motion where a moving fill reads as a stripe.
                                    if in_flight {
                                        span {
                                            class: "pointer-events-none absolute top-1/2 size-2 \
                                                    rounded-full bg-accent shadow-md shadow-accent/50 \
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
