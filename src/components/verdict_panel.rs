//! Per-flow verdicts. The status pill is one exit code; a suite has six.

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
                // The tally is stated rather than left to be counted: "5 of 6" is the number.
                span {
                    class: if passed == total { "text-brand" } else { "text-danger" },
                    "{passed} of {total} passed"
                }
            }
            ul { class: "divide-y divide-edge",
                for verdict in verdicts.iter() {
                    // A BUTTON, not a list item with a click handler: this is the fastest route.
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
                        // The label, not the verdict word, is what identifies the row -- the icon.
                        span {
                            class: if verdict.ok { "text-fg-soft" } else { "text-danger" },
                            // An unlabelled verdict still shows; losing it would be worse than no name.
                            if verdict.label.is_empty() { "(unnamed run)" } else { "{verdict.label}" }
                        }
                        // The affordance.
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
