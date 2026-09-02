use dioxus::prelude::*;
use crate::services::state::{HistoryEntry, ScriptStatus};
use chrono::Duration;

#[derive(Props, Clone, PartialEq)]
pub struct HistoryPanelProps {
    pub history: Vec<HistoryEntry>,
    pub open: bool,
    pub on_close: EventHandler<()>,
}

/// Execution history, as a slide-over panel.
#[component]
pub fn HistoryPanel(props: HistoryPanelProps) -> Element {
    if !props.open {
        return rsx! {};
    }

    rsx! {
        // The scrim.
        div {
            class: "fixed inset-0 z-40 bg-black/40 backdrop-blur-sm",
            onclick: move |_| props.on_close.call(()),
        }
        aside {
            class: "fixed inset-y-0 right-0 z-50 flex w-full max-w-sm flex-col border-l \
                    border-border-soft bg-card shadow-2xl",
            div {
                class: "flex shrink-0 items-center gap-2 border-b border-border-soft px-4 py-3 \
                        text-xs font-semibold uppercase tracking-wider text-fg-muted",
                i { class: "ph ph-clock-counter-clockwise" }
                span { class: "flex-1", "Execution History" }
                span { class: "font-normal normal-case tracking-normal text-fg-faint",
                    "{props.history.len()}" }
                button {
                    r#type: "button",
                    class: "rounded p-1 text-fg-faint hover:bg-black/5 dark:hover:bg-white/5 hover:text-fg",
                    onclick: move |_| props.on_close.call(()),
                    i { class: "ph ph-x" }
                }
            }
            if props.history.is_empty() {
                div { class: "px-4 py-6 text-center text-[11px] text-fg-faint", "No executions yet." }
            } else {
                div { class: "flex min-h-0 flex-1 flex-col overflow-y-auto",
                    for entry in props.history.iter().rev() {
                        {
                            let script_name = entry.script_name.replace("tools/", "");
                            let env_name = entry.environment.as_str();
                            let time_str = entry.start_time.format("%H:%M:%S").to_string();
                            let dur = format_duration(entry.end_time.signed_duration_since(entry.start_time));
                            rsx! {
                                div {
                                    class: "flex flex-col gap-1.5 border-b border-border-soft \
                                            px-4 py-2.5 last:border-b-0",
                                    div { class: "flex items-center justify-between gap-2",
                                        // Long names truncate rather than push the environment tag out of view.
                                        span { class: "truncate text-xs font-semibold text-fg",
                                            "{script_name}" }
                                        span {
                                            class: "shrink-0 rounded bg-black/5 dark:bg-white/5 px-1.5 py-0.5 \
                                                    text-[10px] uppercase tracking-wider text-fg-muted",
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
    const DOT: &str = "ml-auto size-2 shrink-0 rounded-full";
    match status {
        ScriptStatus::Succeeded => rsx! { span { class: "{DOT} bg-accent", title: "Succeeded" } },
        ScriptStatus::Failed(_) => rsx! { span { class: "{DOT} bg-danger", title: "Failed" } },
        ScriptStatus::Cancelled => rsx! { span { class: "{DOT} bg-warn", title: "Cancelled" } },
        ScriptStatus::AppError(_) => rsx! { span { class: "{DOT} bg-danger", title: "Error" } },
        _ => rsx! { span { class: "{DOT} bg-fg-faint" } }
    }
}
