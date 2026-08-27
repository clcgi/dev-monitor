use dioxus::prelude::*;
use crate::services::state::WorkflowStep;

#[derive(Props, Clone, PartialEq)]
pub struct WorkflowStepperProps {
    pub active_step: Option<WorkflowStep>,
    pub step_history: Vec<WorkflowStep>,
    pub is_running: bool,
    pub is_failed: bool,
    pub is_succeeded: bool,
}

#[component]
pub fn WorkflowStepper(props: WorkflowStepperProps) -> Element {
    // ONE definition of the chain, shared with the marker parser's
    // `complete_up_to`. Two copies would drift the moment a step is inserted,
    // and the drift is silent: the stepper would render one order while the
    // auto-complete walked another.
    let all_steps = WorkflowStep::LINEAR.to_vec();

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

    // Status is now a VALUE, not a class name. The old code returned strings
    // like "step-completed" that a descendant selector then turned into circle
    // and label colours; utilities have no descendant combinator, so the state
    // has to be readable where each element is written.
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

        // ring-4 in the app colour replaces the old `box-shadow: 0 0 0 4px`,
        // which existed to punch the connecting line out from behind the circle.
        let circle = match state {
            NodeState::Completed => "bg-fg-soft border-fg-soft text-app",
            NodeState::Active    => "bg-brand border-brand text-app",
            NodeState::Failed    => "bg-danger border-danger text-app",
            NodeState::Zone      => "bg-warn border-warn text-app",
            NodeState::Pending   => "bg-surface border-edge text-fg",
        };
        let label = match state {
            NodeState::Completed => "text-fg",
            NodeState::Active    => "text-brand font-semibold",
            NodeState::Failed    => "text-danger",
            NodeState::Zone      => "text-warn",
            NodeState::Pending   => "text-fg-faint",
        };
        // RESPONSIVE: nodes were a fixed 120px, so eleven of them needed 1320px
        // before the track scrolled. They now shrink with the window.
        let scale = if is_current { "scale-110" } else { "" };

        rsx! {
            div {
                class: "relative z-[2] flex w-16 shrink-0 flex-col items-center gap-2 \
                        transition-transform sm:w-20 lg:w-28 {scale}",
                div {
                    class: "flex size-10 items-center justify-center rounded-full border-2 \
                            ring-4 ring-app transition-colors {circle}",
                    if state == NodeState::Completed {
                        i { class: "ph-fill ph-check text-xl" }
                    } else if state == NodeState::Failed {
                        i { class: "ph-fill ph-x text-xl" }
                    } else if state == NodeState::Active {
                        i { class: "ph ph-spinner ph-spin text-xl" }
                    } else {
                        i { class: "ph-fill {get_step_icon(step)} text-xl" }
                    }
                }
                // The label is the first thing to go when space runs out; the
                // icon still identifies the stage.
                div { class: "hidden text-center text-[10px] leading-tight sm:block {label}",
                    "{name}"
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
                        // -mx-3 tucks the line under the circles so it starts and
                        // ends behind them rather than at their edges. z-[1] keeps
                        // it below the nodes.
                        div {
                            class: if idx + 1 <= current_idx
                                || props.step_history.contains(&all_steps[idx + 1])
                                || props.is_succeeded {
                                "z-[1] -mx-3 h-0.5 min-w-2 flex-1 bg-fg-soft transition-colors"
                            } else {
                                "z-[1] -mx-3 h-0.5 min-w-2 flex-1 bg-edge transition-colors"
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
