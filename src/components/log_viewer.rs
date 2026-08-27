use dioxus::prelude::*;
use dioxus::document::eval;
use crate::services::state::{LogMsg, StreamType};

#[derive(Props, Clone, PartialEq)]
pub struct LogViewerProps {
    pub logs: Vec<LogMsg>,
}

#[component]
pub fn LogViewer(props: LogViewerProps) -> Element {
    let mut auto_scroll = use_signal(|| true);
    
    let logs_len = props.logs.len();
    use_effect(move || {
        let _ = logs_len;
        if *auto_scroll.read() {
            let _ = eval(
                "let el = document.getElementById('log-viewer-scroll'); if (el) { el.scrollTop = el.scrollHeight; }"
            );
        }
    });

    rsx! {
        // The inline `style:` attributes are gone with the classes: they were
        // doing layout the stylesheet could not express, and utilities can.
        div { class: "relative flex min-h-0 flex-1 flex-col",

            if !*auto_scroll.read() && logs_len > 0 {
                button {
                    class: "absolute bottom-4 right-4 z-10 rounded-lg border border-edge \
                            bg-elevated px-3 py-1.5 text-xs text-fg-soft shadow-lg \
                            hover:border-brand hover:bg-active hover:text-fg",
                    onclick: move |_| {
                        auto_scroll.set(true);
                        let _ = eval(
                            "let el = document.getElementById('log-viewer-scroll'); if (el) { el.scrollTop = el.scrollHeight; }"
                        );
                    },
                    "↓ Resume Auto-scroll"
                }
            }

            div {
                id: "log-viewer-scroll",
                // font-mono and the tight leading are what made this read as a
                // terminal; `break-all` keeps a long single-token line (a path,
                // a base64 blob) from widening the pane instead of wrapping.
                class: "min-h-0 flex-1 overflow-y-auto rounded-lg border border-edge \
                        bg-app p-3 font-mono text-[11px] leading-relaxed",
                onscroll: move |_evt| {
                    let mut eval_obj = eval(
                        "let el = document.getElementById('log-viewer-scroll'); if (el) { let isBottom = Math.abs(el.scrollHeight - el.scrollTop - el.clientHeight) < 10; if (isBottom) { dioxus.send('bottom'); } else { dioxus.send('scrolled'); } }"
                    );
                    spawn(async move {
                        if let Ok(msg) = eval_obj.recv::<String>().await {
                            if msg == "scrolled" && *auto_scroll.read() {
                                auto_scroll.set(false);
                            } else if msg == "bottom" && !*auto_scroll.read() {
                                auto_scroll.set(true);
                            }
                        }
                    });
                },

                if props.logs.is_empty() {
                    div { class: "p-2 text-fg-faint", "Waiting for output..." }
                } else {
                    for log in props.logs.iter() {
                        {
                            // Only the COLOUR varies by stream; the row layout is
                            // shared, so it stays on the element rather than being
                            // repeated three times.
                            let tone = match log.stream {
                                StreamType::Stdout => "text-fg-soft",
                                StreamType::Stderr => "text-danger",
                                StreamType::System => "text-info italic",
                            };
                            let time_str = log.timestamp.format("%H:%M:%S").to_string();
                            rsx! {
                                div {
                                    class: "flex gap-3 break-all py-0.5 {tone}",
                                    span { class: "shrink-0 text-fg-faint", "{time_str} " }
                                    span { class: "flex-1 whitespace-pre-wrap", "{log.content}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
