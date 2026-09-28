use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct ThemeToggleProps {
    /// None follows the OS; Some(true) is light, Some(false) is dark.
    pub preference: Option<bool>,
    /// What the OS currently reports, used only when no explicit choice is stored.
    pub system_is_light: bool,
    pub on_change: EventHandler<Option<bool>>,
}

#[component]
pub fn ThemeToggle(props: ThemeToggleProps) -> Element {
    let is_light = props.preference.unwrap_or(props.system_is_light);
    let icon = if is_light { "ph-sun" } else { "ph-moon" };

    rsx! {
        button {
            r#type: "button",
            title: if is_light { "Light — click for dark" } else { "Dark — click for light" },
            class: "text-nav-link text-fg-muted hover:text-fg transition-colors flex items-center gap-1",
            onclick: move |_| props.on_change.call(Some(!is_light)),
            i { class: "ph {icon} text-sm" }
        }
    }
}
