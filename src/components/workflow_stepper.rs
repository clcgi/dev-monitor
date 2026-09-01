use dioxus::prelude::*;
use crate::services::state::WorkflowStep;

#[derive(Props, Clone, PartialEq)]
pub struct WorkflowStepperProps {
    /// The stages to draw. `None` when the script declares nothing, and the
    /// whole chain is drawn -- an empty stepper would assert the script
    /// touches no stage, which it never claimed.
    pub steps: Option<Vec<WorkflowStep>>,
    /// Seconds spent in the CURRENT stage, when one is running.
    ///
    /// THE ONLY THING THAT SEPARATES WAITING FROM STUCK. A spinner looks
    /// identical at 5 seconds and at 500, and the waits here are genuinely
    /// long -- a KEDA cold start can be ten minutes of total silence.
    pub step_elapsed_s: Option<u64>,
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
    // Declared stages, in the PIPELINE's order rather than the order they were
    // listed in. A script that named them out of order would otherwise draw a
    // chain that runs backwards.
    let all_steps: Vec<WorkflowStep> = match &props.steps {
        Some(declared) => WorkflowStep::LINEAR
            .iter()
            .filter(|s| declared.contains(s))
            .cloned()
            .collect(),
        None => WorkflowStep::LINEAR.to_vec(),
    };
    // A declaration listing only exception zones (flow_6 is close) would leave
    // this empty, and an empty track renders as a bare box.
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
        //
        // MOTION IS PART OF THE STATE, not an ornament laid over it. `animate-pop`
        // fires when a node first becomes Completed or Active, so the eye is
        // taken to the stage that just changed; `animate-halo` runs only while a
        // stage is ACTIVE, which is the one thing a static stepper cannot say --
        // a coloured ring alone reads the same whether work is in progress or
        // stopped there.
        // `land` for Completed and `pop` for the rest is deliberate: a browser
        // replays an animation only when the animation-NAME changes, so sharing
        // one keyframe across both would make the Active -> Completed
        // transition -- the one most worth seeing -- silently not animate.
        //
        // `animate-breathe` runs only while a stage is genuinely ACTIVE. It is
        // the difference between "this node is where we got to" and "this node
        // is working right now", which no static styling can express.
        let circle = match state {
            NodeState::Completed => "bg-fg-soft border-fg-soft text-app animate-land",
            NodeState::Active if props.is_running =>
                "bg-brand border-brand text-app animate-breathe shadow-lg shadow-brand/30",
            NodeState::Active    => "bg-brand border-brand text-app animate-pop",
            NodeState::Failed    => "bg-danger border-danger text-app animate-pop",
            NodeState::Zone      => "bg-warn border-warn text-app animate-pop",
            NodeState::Pending   => "bg-surface border-edge text-fg",
        };
        // The NEXT stage telegraphs where the run is heading. Half the amplitude
        // of the active node's breathing and no colour shift, so it never
        // competes for attention -- it only stops the row ahead reading as dead.
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
        // RESPONSIVE: nodes were a fixed 120px, so eleven of them needed 1320px
        // before the track scrolled. They now shrink with the window.
        let scale = if is_current { "scale-110" } else { "" };

        rsx! {
            div {
                class: "relative z-[2] flex w-16 shrink-0 flex-col items-center gap-2 \
                        transition-transform duration-300 sm:w-20 lg:w-28 {scale}{anticipate}",
                div {
                    class: "relative flex size-10 items-center justify-center rounded-full \
                            border-2 ring-4 ring-app transition-colors duration-300 {circle}",
                    // The halo is its own element rather than a shadow on the
                    // node, so the expanding ring is not clipped by the node's
                    // own `ring-4` and does not repaint the icon underneath.
                    // `pointer-events-none` because it overlaps its neighbours
                    // at full expansion.
                    if state == NodeState::Active && props.is_running {
                        // TWO rings, the second offset by half the cycle. One
                        // ring pulses; two read as something radiating
                        // continuously, and the gap between cycles -- which is
                        // where a single ring looks stalled -- is filled.
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
                        // Reached but not running: the run finished, was
                        // cancelled, or is between processes. A spinner here
                        // would claim work that is not happening.
                        i { class: "ph-fill {get_step_icon(step)} text-xl" }
                    } else {
                        i { class: "ph-fill {get_step_icon(step)} text-xl" }
                    }
                }
                // The label is the first thing to go when space runs out; the
                // icon still identifies the stage.
                div { class: "hidden text-center text-[10px] leading-tight sm:block {label}",
                    "{name}"
                }
                // HOW LONG THIS STAGE HAS BEEN RUNNING. Shown only on the active
                // node, and only while running, because it is a live reading and
                // a frozen one on a finished node would be read as a duration.
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
                            // THE SEGMENT BEING TRAVELLED. Exactly one connector
                            // is in flight at a time -- the one leaving the
                            // active node -- and it carries a band moving in the
                            // direction of travel. That is what turns the
                            // stepper from a record of where a run got to into a
                            // picture of it moving.
                            let in_flight = props.is_running && !done && idx == current_idx;
                            // Computed here rather than as an `if/else if` chain
                            // inside the attribute: rsx can infer a two-branch
                            // `if/else` on an attribute value but not a three-way
                            // one, and the error it gives ("type annotations
                            // needed") points at the whole block rather than at
                            // the chain.
                            //
                            // -mx-3 tucks the line under the circles so it starts
                            // and ends behind them rather than at their edges.
                            // z-[1] keeps it below the nodes.
                            const TRACK: &str = "z-[1] -mx-3 min-w-2 flex-1";
                            let line = if in_flight {
                                // Thicker while travelling, so the band has room
                                // to read as a band. A 2px stripe with a gradient
                                // on it is indistinguishable from a static line
                                // at arm's length.
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
                                    // A DISCRETE OBJECT CROSSING THE GAP. The
                                    // gradient says "this segment is live"; the
                                    // dot says "and it is moving, that way".
                                    // Motion of a thing beats motion of a fill
                                    // for reading direction at a glance.
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


/// `45s`, `2m 05s`. Monospace and zero-padded so the reading does not jitter
/// horizontally as it counts -- a number that shifts every second is harder to
/// ignore than one that does not, and this sits under a node the eye returns to.
fn format_elapsed(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else {
        format!("{}m {:02}s", secs / 60, secs % 60)
    }
}
