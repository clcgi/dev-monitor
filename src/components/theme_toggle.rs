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
    let icon = if is_light { "ph-sun" } else { "ph-moon" };

    rsx! {
        button {
            r#type: "button",
            // The tooltip carries BOTH the current state and what a click does.
            title: if is_light { "Light — click for dark" } else { "Dark — click for light" },
            // The nav-link idiom its neighbours use: the bar is dark in both
            // themes, so its controls are white-on-dark rather than a themed pill.
            class: "text-nav-link text-white/80 hover:text-white transition-colors \
                    flex items-center gap-1",
            onclick: move |_| props.on_change.call(Some(!is_light)),
            i { class: "ph {icon} text-sm" }
        }
    }
}
