//! Dark/light toggle. TWO states on screen: light and dark.
//!
//! WHY THE SIGNAL IS STILL `Option<bool>` when the button offers two states.
//! The app follows the OS until somebody chooses, and polls it every two
//! seconds. `None` means "nobody has chosen yet", which lets that poll write;
//! `Some(_)` is a choice and the poll leaves it alone. Without that distinction
//! the poll would overwrite the user's pick within a tick -- the toggle
//! appearing to do nothing, intermittently, which is the worst way for a
//! control to fail.
//!
//! So the follow-the-OS state exists but is NOT OFFERED: the button shows the
//! mode currently in effect and switches to the other one. There is no third
//! click, and no label for a state a user never selected.

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
    // The mode in EFFECT, whether or not it was chosen. Before the first click
    // that is whatever the OS says, and the button must show it -- a control
    // labelled "Light" on a dark screen is worse than no control.
    let is_light = props.preference.unwrap_or(props.system_is_light);
    let (icon, label) = if is_light { ("ph-sun", "Light") } else { ("ph-moon", "Dark") };

    rsx! {
        button {
            r#type: "button",
            // The tooltip carries BOTH the current state and what a click does.
            // A three-state control whose next step is not visible is a control
            // people click twice and give up on.
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
