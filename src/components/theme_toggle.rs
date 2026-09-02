//! Dark/light toggle.

use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct ThemeToggleProps {
    /// None follows the OS; Some(true) is light, Some(false) is dark.
    pub preference: Option<bool>,
    /// What the OS currently reports, used only to draw the system state.
    pub system_is_light: bool,
    pub on_change: EventHandler<Option<bool>>,
}

#[component]
pub fn ThemeToggle(props: ThemeToggleProps) -> Element {
    // The mode in EFFECT, whether or not it was chosen.
    let is_light = props.preference.unwrap_or(props.system_is_light);
    let (icon, label) = if is_light { ("ph-sun", "Light") } else { ("ph-moon", "Dark") };

    rsx! {
        button {
            r#type: "button",
            // The tooltip carries BOTH the current state and what a click does.
            title: if is_light { "Light — click for dark" } else { "Dark — click for light" },
            class: "flex items-center gap-2 rounded-md border border-edge bg-elevated px-2.5 \
                    py-1.5 text-fg-soft transition-colors hover:bg-active hover:text-fg \
                    focus:outline-none",
            onclick: move |_| props.on_change.call(Some(!is_light)),
            i { class: "ph {icon} text-base" }
            span { class: "hidden text-xs sm:inline", "{label}" }
        }
    }
}
