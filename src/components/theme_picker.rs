use crate::THEMES;
use dioxus::prelude::*;

#[component]
pub fn ThemePicker() -> Element {
    // The same signal `App` writes to <body>, so choosing here re-themes every.
    let mut theme = use_context::<Signal<String>>();
    let mut open = use_signal(|| false);
    let current = theme.read().clone();
    let label = THEMES
        .iter()
        .find(|(class, _)| *class == current)
        .map(|(_, name)| *name)
        .unwrap_or("Theme");

    rsx! {
        div { class: "relative",
            button {
                // The label drops first when space runs out; the swatch still identifies it.
                class: "flex items-center gap-2 rounded-md border border-edge bg-elevated \
                        px-2.5 py-1.5 text-fg-soft transition-colors hover:bg-active \
                        hover:text-fg focus:outline-none",
                onclick: move |_| { let v = *open.read(); open.set(!v); },
                i { class: "ph ph-palette text-base" }
                span { class: "hidden text-xs sm:inline", "{label}" }
                i { class: "ph ph-caret-down text-xs opacity-60" }
            }
            if *open.read() {
                // A click-catcher behind the menu.
                div {
                    class: "fixed inset-0 z-40",
                    onclick: move |_| open.set(false),
                }
                div {
                    class: "absolute right-0 z-50 mt-1 w-48 overflow-hidden rounded-md border \
                            border-edge bg-elevated shadow-lg",
                    for (class_name, name) in THEMES.iter() {
                        button {
                            key: "{class_name}",
                            class: if current == *class_name {
                                "flex w-full items-center gap-2 px-3 py-2 text-left text-xs \
                                 bg-active text-fg"
                            } else {
                                "flex w-full items-center gap-2 px-3 py-2 text-left text-xs \
                                 text-fg-soft hover:bg-active hover:text-fg"
                            },
                            onclick: move |_| {
                                theme.set(class_name.to_string());
                                open.set(false);
                            },
                            i {
                                class: if current == *class_name {
                                    "ph ph-check text-brand"
                                } else {
                                    "ph ph-circle opacity-30"
                                },
                            }
                            "{name}"
                        }
                    }
                }
            }
        }
    }
}
