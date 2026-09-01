//! The verdicts a run reported about ITSELF.
//!
//! WHY THIS IS NOT THE STATUS PILL. The pill up top is derived from the process
//! exit code, and one process carries exactly one. `flow_all.py` runs six flows
//! through that single code: when it goes red, the code says the suite failed
//! and nothing says WHICH flow did. These do.
//!
//! They also arrive as each verdict is reached rather than when the process
//! ends, so a suite three flows in shows three results instead of nothing.
//!
//! EACH ROW JUMPS TO ITS OUTPUT. `flow_all` produces one log of six runs, and
//! finding the red one by scrolling is the difference between the panel being
//! useful and being decoration. The target is the `[CDW_RUN: <label>]` line the
//! flow prints at its own start, so the jump lands on the beginning of that
//! run rather than on its verdict at the end.
//!
//! HIDDEN WHEN EMPTY, deliberately. Most scripts report no verdict at all, and
//! an empty panel reading "no results" would suggest a run had failed to
//! produce something it was never going to produce.

use crate::services::state::Verdict;
use dioxus::prelude::*;

#[component]
pub fn VerdictPanel(verdicts: Vec<Verdict>, on_jump: EventHandler<String>) -> Element {
    if verdicts.is_empty() {
        return rsx! {};
    }

    let passed = verdicts.iter().filter(|v| v.ok).count();
    let total = verdicts.len();

    rsx! {
        div { class: "overflow-hidden rounded-md border border-edge bg-surface",
            div {
                class: "flex items-center justify-between bg-elevated px-3 py-2 text-[10px] \
                        uppercase tracking-wider text-fg-faint",
                span { "Results" }
                // The tally is stated rather than left to be counted: "5 of 6"
                // is the number someone reads first, and a panel that made them
                // count red dots would bury it.
                span {
                    class: if passed == total { "text-brand" } else { "text-danger" },
                    "{passed} of {total} passed"
                }
            }
            ul { class: "divide-y divide-edge",
                for verdict in verdicts.iter() {
                    // A BUTTON, not a list item with a click handler: this is the
                    // fastest route to the part of a 2000-line suite log that
                    // explains a red row, and it has to look pressable and be
                    // reachable from the keyboard.
                    li { key: "{verdict.label}",
                    button {
                        r#type: "button",
                        class: "flex w-full items-center gap-2 px-3 py-2 text-left text-xs \
                                hover:bg-active",
                        title: if verdict.label.is_empty() {
                            "No run label to jump to"
                        } else {
                            "Jump to this flow's output"
                        },
                        disabled: verdict.label.is_empty(),
                        onclick: {
                            let label = verdict.label.clone();
                            move |_| on_jump.call(label.clone())
                        },
                        i {
                            class: if verdict.ok {
                                "ph ph-check-circle text-brand"
                            } else {
                                "ph ph-x-circle text-danger"
                            },
                        }
                        // The label, not the verdict word, is what identifies the
                        // row -- the icon already carries pass or fail, and
                        // repeating it in text would crowd out the name.
                        span {
                            class: if verdict.ok { "text-fg-soft" } else { "text-danger" },
                            // An unlabelled verdict still shows: losing it because
                            // the script did not name itself would be worse than
                            // an anonymous row.
                            if verdict.label.is_empty() { "(unnamed run)" } else { "{verdict.label}" }
                        }
                        // The affordance. Without it the rows read as a static
                        // summary and nobody discovers they are clickable.
                        if !verdict.label.is_empty() {
                            i { class: "ml-auto ph ph-arrow-line-down text-fg-faint opacity-60" }
                        }
                    }
                    }
                }
            }
        }
    }
}
