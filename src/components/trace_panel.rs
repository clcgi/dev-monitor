use dioxus::prelude::*;
use std::collections::BTreeMap;

use crate::services::state::{LogMsg, StreamType};

#[derive(Props, Clone, PartialEq)]
pub struct TracePanelProps {
    pub traces: BTreeMap<String, Vec<LogMsg>>,
    /// The component whose block is open right now, if any.
    pub in_trace: Option<String>,
    pub is_running: bool,
}

fn has_error(lines: &[LogMsg]) -> bool {
    lines.iter().any(|l| {
        l.stream == StreamType::Stderr
            || l.content.contains("[ERROR]")
            || l.content.contains("[CRITICAL]")
    })
}

#[component]
pub fn TracePanel(props: TracePanelProps) -> Element {
    let mut selected = use_signal(|| Option::<String>::None);

    if props.traces.is_empty() {
        return rsx! {
            div { class: "flex flex-1 flex-col items-center justify-center gap-2 p-8 text-center",
                i { class: "ph ph-broadcast text-lg text-fg-faint" }
                div { class: "text-caption text-fg-faint",
                    if props.is_running {
                        "Waiting for the platform. Traces run about half a minute behind the pipeline."
                    } else {
                        "No platform traces for this run."
                    }
                }
            }
        };
    }

    let names: Vec<String> = props.traces.keys().cloned().collect();
    let current = selected
        .read()
        .clone()
        .filter(|c| props.traces.contains_key(c))
        .unwrap_or_else(|| names[0].clone());
    let lines = props.traces.get(&current).cloned().unwrap_or_default();

    rsx! {
        div { class: "flex min-h-0 flex-1",
            div { class: "flex w-44 shrink-0 flex-col overflow-y-auto border-r border-border-soft",
                for name in names.iter() {
                    {
                        let n = name.clone();
                        let count = props.traces.get(name).map(|l| l.len()).unwrap_or(0);
                        let failed = props.traces.get(name).is_some_and(|l| has_error(l));
                        let live = props.in_trace.as_ref() == Some(name);
                        let dot = if failed {
                            "size-1.5 shrink-0 rounded-full bg-danger"
                        } else if live {
                            "size-1.5 shrink-0 animate-pulse rounded-full bg-accent"
                        } else {
                            "size-1.5 shrink-0 rounded-full bg-fg-faint"
                        };
                        let element: Element = rsx! {
                            button {
                                key: "{name}",
                                r#type: "button",
                                class: if *name == current {
                                    "flex items-center gap-2 border-l-[3px] border-accent bg-selected px-3 py-2 text-left text-caption text-fg"
                                } else {
                                    "flex items-center gap-2 border-l-[3px] border-transparent px-3 py-2 text-left text-caption text-fg-muted hover:bg-hover hover:text-fg"
                                },
                                onclick: move |_| selected.set(Some(n.clone())),
                                span { class: "{dot}" }
                                span { class: "min-w-0 flex-1 truncate font-mono", "{name}" }
                                span { class: "shrink-0 text-fg-faint", "{count}" }
                            }
                        };
                        element
                    }
                }
            }

            div { class: "min-h-0 flex-1 overflow-y-auto px-3 py-1 text-log",
                for (i, line) in lines.iter().enumerate() {
                    {
                        let clock = line.timestamp.format("%H:%M:%S").to_string();
                        let text = line.content.trim().to_string();
                        let failed = text.contains("[ERROR]") || text.contains("[CRITICAL]");
                        let element: Element = rsx! {
                            div {
                                key: "{i}",
                                class: if failed {
                                    "flex gap-3 break-all py-0.5 text-danger"
                                } else {
                                    "flex gap-3 break-all py-0.5 text-fg"
                                },
                                span { class: "shrink-0 text-fg-faint", "{clock}" }
                                span { class: "flex-1 whitespace-pre-wrap", "{text}" }
                            }
                        };
                        element
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Local;

    fn msg(content: &str, stream: StreamType) -> LogMsg {
        LogMsg { timestamp: Local::now(), stream, content: content.to_string() }
    }

    #[test]
    fn a_component_that_reported_an_error_is_flagged() {
        assert!(has_error(&[msg("   [ERROR] TypeError: load_settings()", StreamType::Stdout)]));
        assert!(has_error(&[msg("[CRITICAL] host is down", StreamType::Stdout)]));
        assert!(has_error(&[msg("anything", StreamType::Stderr)]));
    }

    #[test]
    fn an_ordinary_line_is_not_an_error() {
        assert!(!has_error(&[msg("   14:22:31  promoting document", StreamType::Stdout)]));
        assert!(!has_error(&[]));
    }

    #[test]
    fn the_word_error_inside_a_message_does_not_flag_the_component() {
        assert!(!has_error(&[msg("   14:22:31  0 errors found", StreamType::Stdout)]));
    }
}
