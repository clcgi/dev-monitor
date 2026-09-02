use dioxus::prelude::*;
use dioxus::document::eval;
use crate::services::state::{LogMsg, StreamType};

#[derive(Props, Clone, PartialEq)]
pub struct LogViewerProps {
    pub logs: Vec<LogMsg>,
    /// (nonce, line index) to scroll to. A Signal, so the effect has a dependency.
    pub jump: Signal<Option<(u64, usize)>>,
}

#[component]
pub fn LogViewer(props: LogViewerProps) -> Element {
    let mut auto_scroll = use_signal(|| true);
    
    // FOLLOW THE TAIL. `use_reactive` is what makes this fire on each new line.
    let logs_len = props.logs.len();
    use_effect(use_reactive((&logs_len,), move |(_len,)| {
        if *auto_scroll.read() {
            let _ = eval(
                "let el = document.getElementById('log-viewer-scroll'); if (el) { el.scrollTop = el.scrollHeight; }"
            );
        }
    }));

    // Jumping to a line.
    let jump = props.jump;
    let mut highlighted = use_signal(|| Option::<usize>::None);
    use_effect(move || {
        let Some((_nonce, index)) = *jump.read() else { return };
        // AUTO-SCROLL OFF FIRST. The effect above follows the tail on every new line.
        auto_scroll.set(false);
        highlighted.set(Some(index));
        let _ = eval(&format!(
            "let el = document.getElementById('log-line-{index}'); \
             if (el) {{ el.scrollIntoView({{block: 'start'}}); }}"
        ));
    });

    rsx! {
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
                // font-mono and the tight leading are what made this read as a terminal.
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
                    for (idx, log) in props.logs.iter().enumerate() {
                        {
                            // Only the colour varies by stream; the row layout is shared.
                            let tone = match log.stream {
                                StreamType::Stdout => "text-fg-soft",
                                StreamType::Stderr => "text-danger",
                                StreamType::System => "text-info italic",
                            };
                            let time_str = log.timestamp.format("%H:%M:%S").to_string();
                            // The marker that says where a jump landed.
                            let mark = if *highlighted.read() == Some(idx) {
                                " -mx-1 rounded bg-brand/15 px-1 ring-1 ring-brand/40"
                            } else {
                                ""
                            };
                            rsx! {
                                div {
                                    id: "log-line-{idx}",
                                    class: "flex gap-3 break-all py-0.5 {tone}{mark}",
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
