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
        div {
            div {
                class: "flex items-center justify-between px-5 pt-3 pb-2",
                span { class: "text-caption-strong text-fg-faint uppercase tracking-wider", "Results" }
                span {
                    class: if passed == total {
                        "text-caption-strong text-accent tabular-nums"
                    } else {
                        "text-caption-strong text-danger tabular-nums"
                    },
                    "{passed} of {total} passed"
                }
            }
            ul { class: "divide-y divide-border-soft border-t border-border-soft",
                for verdict in verdicts.iter() {
                    li { key: "{verdict.label}",
                    button {
                        r#type: "button",
                        class: "flex w-full items-center gap-3 px-5 py-2.5 text-left text-body hover:bg-black/5 dark:hover:bg-white/5 transition-colors",
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
                                "ph-fill ph-check-circle text-accent"
                            } else {
                                "ph-fill ph-x-circle text-danger"
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
