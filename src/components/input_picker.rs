use crate::services::scripts::ScriptInput;
use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct InputPickerProps {
    pub inputs: Vec<ScriptInput>,
    /// flag -> the value currently set for it, shared with the dropdowns.
    pub chosen: std::collections::HashMap<String, String>,
    pub disabled: bool,
    /// (flag, value).
    pub on_set: EventHandler<(String, String)>,
}

/// Flags whose value is typed: a batch id, a date, a count.
///
/// These reach the command line through the same `chosen` map the dropdowns
/// write to, so an empty field is an omitted flag rather than `--batch-id ''`.
#[component]
pub fn InputPicker(props: InputPickerProps) -> Element {
    // Hidden when empty, like the other two pickers: an empty "Values" band on
    // a script that takes none implies it lost them.
    if props.inputs.is_empty() {
        return rsx! {};
    }

    rsx! {
        div { class: "flex flex-col gap-3",
            div { class: "text-caption-strong text-fg-faint", "Values" }
            div { class: "flex flex-col gap-2.5",
                for input in props.inputs.iter() {
                    {
                        let flag = input.flag.clone();
                        let value = props.chosen.get(&input.flag).cloned().unwrap_or_default();
                        let help = input.help.clone();
                        let suggestions = input.suggestions.clone();
                        // A datalist, not a select: the list is a shortcut, and a batch id
                        // or a GUID nobody listed still has to be typeable.
                        let list_id = format!("values-{}", input.flag.trim_start_matches('-'));
                        rsx! {
                            // Laid out like the dropdowns: the flag and the control on one
                            // row, the help under them. The field was full width with its
                            // help above it, which read as a paragraph with an input after.
                            div { key: "{input.flag}", class: "flex flex-col gap-1.5",
                                div { class: "flex items-center gap-2.5",
                                    span { class: "font-mono text-xs font-medium text-fg-muted", "{input.flag}" }
                                    input {
                                        class: "flex-1 rounded-lg border border-border-hard bg-card px-3 py-1.5
                                                text-button-utility text-fg outline-none
                                                hover:border-accent disabled:opacity-50",
                                        r#type: "text",
                                        list: "{list_id}",
                                        value: "{value}",
                                        placeholder: "blank — the flag is not sent",
                                        spellcheck: "false",
                                        disabled: props.disabled,
                                        oninput: move |e| props.on_set.call((flag.clone(), e.value())),
                                    }
                                }
                                if !suggestions.is_empty() {
                                    datalist { id: "{list_id}",
                                        for value in suggestions.iter() {
                                            option { key: "{value}", value: "{value}" }
                                        }
                                    }
                                }
                                if !help.is_empty() {
                                    div { class: "text-caption text-fg-muted", "{help}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
