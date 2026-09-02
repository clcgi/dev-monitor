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
                    class: "absolute bottom-4 right-4 z-10 rounded-full bg-white/10 px-4 py-2                             text-xs text-white shadow-lg backdrop-blur-md                             hover:bg-white/20 transition-colors border border-white/10",
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
                class: "min-h-0 flex-1 overflow-y-auto bg-transparent px-2 py-1 font-mono text-[13px] leading-relaxed",
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
                    div { class: "p-2 text-white/40", "Waiting for output..." }
                } else {
                    for (idx, log) in props.logs.iter().enumerate() {
                        {
                            let tone = match log.stream {
                                StreamType::Stdout => "text-white/80",
                                StreamType::Stderr => "text-danger",
                                StreamType::System => "text-accent italic",
                            };
                            let time_str = log.timestamp.format("%H:%M:%S").to_string();
                            let mark = if *highlighted.read() == Some(idx) {
                                " -mx-2 rounded bg-accent/20 px-2 ring-1 ring-accent/50"
                            } else {
                                ""
                            };
                            rsx! {
                                div {
                                    id: "log-line-{idx}",
                                    class: "flex gap-4 break-all py-0.5 {tone}{mark}",
                                    span { class: "shrink-0 text-white/30", "{time_str} " }
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
