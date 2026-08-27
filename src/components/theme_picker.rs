//! Palette selector.
//!
//! WHY THIS EXISTS. The old stylesheet carried four fully-specified palettes since
//! the app was written -- `theme-electric-autumn`, `theme-warm-editorial`,
//! `theme-seasonless-blue`, `theme-digital-romance`, each with a light
//! counterpart. **Nothing ever selected one.** The only class the app set on
//! `<body>` was `light`, so 8 of the 9 palette blocks were unreachable and the
//! README's "change the active palette by modifying the `<body>` tag class"
//! meant editing the source and rebuilding.
//!
//! Written entirely in Tailwind utilities: no rule for it exists in the legacy
//! stylesheet, and none needs to.

use crate::THEMES;
use dioxus::prelude::*;

#[component]
pub fn ThemePicker() -> Element {
    // The same signal `App` writes to <body>, so choosing here re-themes every
    // utility on the page: the palette lives in CSS custom properties that each
    // Tailwind colour utility already reads.
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
                // `hidden sm:inline` on the label: at narrow widths the palette
                // name is the first thing worth dropping -- the swatch alone
                // still says which theme is active.
                class: "flex items-center gap-2 rounded-md border border-edge bg-elevated \
                        px-2.5 py-1.5 text-fg-soft transition-colors hover:bg-active \
                        hover:text-fg focus:outline-none",
                onclick: move |_| { let v = *open.read(); open.set(!v); },
                i { class: "ph ph-palette text-base" }
                span { class: "hidden text-xs sm:inline", "{label}" }
                i { class: "ph ph-caret-down text-xs opacity-60" }
            }
            if *open.read() {
                // A click-catcher behind the menu. Without it the only way to
                // dismiss is to pick something, which makes the menu feel stuck.
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
