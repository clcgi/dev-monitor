use crate::services::scripts::ScriptChoice;
use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct ChoicePickerProps {
    pub choices: Vec<ScriptChoice>,
    /// flag -> the value currently selected for it.
    pub chosen: std::collections::HashMap<String, String>,
    pub disabled: bool,
    /// (flag, value).
    pub on_choose: EventHandler<(String, String)>,
}

#[component]
pub fn ChoicePicker(props: ChoicePickerProps) -> Element {
    // Hidden when empty, for the same reason the toggles are: a "Case: none"
    // row on the nine scripts that have no choice implies they lost one.
    if props.choices.is_empty() {
        return rsx! {};
    }

    rsx! {
        div { class: "flex flex-col gap-3",
            div { class: "text-caption-strong text-fg-faint", "Test case" }
            for choice in props.choices.iter() {
                {
                    let flag = choice.flag.clone();
                    let selected = props.chosen.get(&choice.flag).cloned().unwrap_or_default();
                    let empty = choice.values.is_empty();
                    rsx! {
                        div { key: "{choice.flag}", class: "flex flex-col gap-1.5",
                            div { class: "flex items-center gap-2.5",
                                span { class: "font-mono text-xs font-medium text-fg-muted", "{choice.flag}" }
                                select {
                                    // Disabled while running for the same reason the toggles are:
                                    // the value is read when the process is spawned, so changing
                                    // it mid-run would silently describe the wrong document.
                                    disabled: props.disabled || empty,
                                    class: "flex-1 rounded-full border border-border-hard bg-transparent px-4 py-2
                                            text-button-utility text-fg disabled:opacity-50
                                            hover:border-fg transition-colors",
                                    value: "{selected}",
                                    onchange: move |e| props.on_choose.call((flag.clone(), e.value())),
                                    for value in choice.values.iter() {
                                        option { key: "{value}", value: "{value}", "{value}" }
                                    }
                                }
                            }
                            if empty {
                                // NAMED, not a blank dropdown. The file is the one the script
                                // itself reads and refreshes on every run, so an empty list means
                                // it has never been fetched -- which an operator can act on.
                                div { class: "text-caption text-warn",
                                    "no values in "
                                    span { class: "font-mono", "{choice.source}" }
                                    " — it has not been fetched yet; any run of the script fetches it"
                                }
                            } else if !choice.help.is_empty() {
                                div { class: "text-caption text-fg-muted", "{choice.help}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
