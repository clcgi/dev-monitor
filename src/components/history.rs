use dioxus::prelude::*;
use crate::services::state::{HistoryEntry, ScriptStatus};
use chrono::Duration;

#[derive(Props, Clone, PartialEq)]
pub struct HistoryPanelProps {
    pub history: Vec<HistoryEntry>,
}

#[component]
pub fn HistoryPanel(props: HistoryPanelProps) -> Element {
    rsx! {
        // max-h is responsive: a short window gave the log viewer almost nothing
        // once history filled its fixed 240px.
        div { class: "flex max-h-48 flex-col rounded-lg border border-edge bg-surface lg:max-h-60",
            div {
                class: "border-b border-edge-soft px-4 py-3 text-xs font-semibold uppercase \
                        tracking-wider text-fg-soft",
                "Execution History"
            }
            if props.history.is_empty() {
                div { class: "px-4 py-6 text-center text-[11px] text-fg-faint", "No executions yet." }
            } else {
                div { class: "flex flex-col overflow-y-auto",
                    for entry in props.history.iter().rev() {
                        {
                            let script_name = entry.script_name.replace("tools/", "");
                            let env_name = entry.environment.as_str();
                            let time_str = entry.start_time.format("%H:%M:%S").to_string();
                            let dur = format_duration(entry.end_time.signed_duration_since(entry.start_time));
                            rsx! {
                                div {
                                    class: "flex flex-col gap-1.5 border-b border-edge-soft \
                                            px-4 py-2.5 last:border-b-0",
                                    div { class: "flex items-center justify-between gap-2",
                                        // Long tool names truncate rather than
                                        // pushing the environment tag out of view.
                                        span { class: "truncate text-xs font-semibold text-fg",
                                            "{script_name}" }
                                        span {
                                            class: "shrink-0 rounded bg-elevated px-1.5 py-0.5 \
                                                    text-[10px] uppercase tracking-wider text-fg-soft",
                                            "{env_name}"
                                        }
                                    }
                                    div { class: "flex items-center gap-3 text-[11px] text-fg-faint",
                                        span { "{time_str}" }
                                        span { "{dur}" }
                                        {render_status(&entry.status)}
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn format_duration(d: Duration) -> String {
    let secs = d.num_seconds();
    if secs < 60 {
        format!("{}s", secs)
    } else {
        format!("{}m {}s", secs / 60, secs % 60)
    }
}

fn render_status(status: &ScriptStatus) -> Element {
    // `ml-auto` pins the dot to the trailing edge, which is what `.history-status`
    // did. NOTE: `dot-warning` was referenced here and DEFINED NOWHERE, before
    // this change -- Cancelled rendered an invisible dot. It is now `bg-warn`.
    const DOT: &str = "ml-auto size-2 shrink-0 rounded-full";
    match status {
        ScriptStatus::Succeeded => rsx! { span { class: "{DOT} bg-brand", title: "Succeeded" } },
        ScriptStatus::Failed(_) => rsx! { span { class: "{DOT} bg-danger", title: "Failed" } },
        ScriptStatus::Cancelled => rsx! { span { class: "{DOT} bg-warn", title: "Cancelled" } },
        ScriptStatus::AppError(_) => rsx! { span { class: "{DOT} bg-danger", title: "Error" } },
        _ => rsx! { span { class: "{DOT} bg-fg-faint" } }
    }
}
