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
        div { class: "overflow-hidden rounded-2xl border border-border-soft bg-card shadow-sm mt-2",
            div {
                class: "flex items-center justify-between border-b border-border-soft bg-black/5 dark:bg-white/5 px-4 py-3 text-caption-strong text-fg-muted",
                span { "Results" }
                span {
                    class: if passed == total { "text-accent" } else { "text-danger" },
                    "{passed} of {total} passed"
                }
            }
            ul { class: "divide-y divide-border-soft",
                for verdict in verdicts.iter() {
                    li { key: "{verdict.label}",
                    button {
                        r#type: "button",
                        class: "flex w-full items-center gap-3 px-4 py-3 text-left text-body hover:bg-black/5 dark:hover:bg-white/5 transition-colors",
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
                                "ph-fill ph-check-circle text-accent text-lg"
                            } else {
                                "ph-fill ph-x-circle text-danger text-lg"
                            },
                        }
                        span {
                            class: if verdict.ok { "text-fg" } else { "text-danger font-medium" },
                            if verdict.label.is_empty() { "(unnamed run)" } else { "{verdict.label}" }
                        }
                        if !verdict.label.is_empty() {
                            i { class: "ml-auto ph ph-caret-right text-fg-faint" }
                        }
                    }
                    }
                }
            }
        }
    }
}
