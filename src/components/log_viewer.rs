use dioxus::prelude::*;
use dioxus::document::eval;
use crate::services::state::{LogMsg, StreamType};

#[derive(Props, Clone, PartialEq)]
pub struct LogViewerProps {
    pub logs: Vec<LogMsg>,
    /// (nonce, line index) to scroll to.
    ///
    /// A SIGNAL, not a plain value, and that is load-bearing: `use_effect`
    /// re-runs when a signal it READS changes, and a copied prop is not a
    /// signal. Passed by value it would have been read once at mount and the
    /// jump would never fire again.
    ///
    /// The nonce makes a second jump to the SAME line distinct, so clicking one
    /// verdict twice scrolls twice.
    pub jump: Signal<Option<(u64, usize)>>,
}

#[component]
pub fn LogViewer(props: LogViewerProps) -> Element {
    let mut auto_scroll = use_signal(|| true);
    
    // FOLLOW THE TAIL. `use_reactive` is what makes this fire on each new line:
    // an effect re-runs when a SIGNAL it reads changes, and `logs_len` is a
    // plain prop, not a signal. Written without it -- as this was -- the effect
    // ran once at mount and then only when `auto_scroll` itself changed, so the
    // pane silently stopped following output and had to be scrolled by hand.
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
        // AUTO-SCROLL OFF FIRST. The effect above follows the tail on every new
        // line, so during a running suite it would drag the view straight back
        // to the bottom and the jump would look like it did nothing.
        auto_scroll.set(false);
        highlighted.set(Some(index));
        let _ = eval(&format!(
            "let el = document.getElementById('log-line-{index}'); \
             if (el) {{ el.scrollIntoView({{block: 'start'}}); }}"
        ));
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
                    for (idx, log) in props.logs.iter().enumerate() {
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
                            // The marker that says where a jump landed. It stays
                            // until the next jump: a highlight that faded on a
                            // timer would be gone by the time someone finished
                            // reading the line they asked for.
                            let mark = if *highlighted.read() == Some(idx) {
                                " -mx-1 rounded bg-brand/15 px-1 ring-1 ring-brand/40"
                            } else {
                                ""
                            };
                            rsx! {
                                div {
                                    // Addressable, so a jump has something to
                                    // scroll to. Index-based because the log is
                                    // append-only within a run: a line's position
                                    // never changes once written.
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
