use crate::services::scripts::ScriptArg;
use dioxus::prelude::*;

#[derive(Props, Clone, PartialEq)]
pub struct ArgPickerProps {
    pub args: Vec<ScriptArg>,
    pub enabled: Vec<String>,
    pub disabled: bool,
    pub on_toggle: EventHandler<String>,
}

#[component]
pub fn ArgPicker(props: ArgPickerProps) -> Element {
    // Hidden when empty: an "Options: none" row implies a script lost some.
    if props.args.is_empty() {
        return rsx! {};
    }

    rsx! {
        div { class: "flex flex-col gap-3",
            div { class: "text-caption-strong text-fg-faint", "Options" }
            div { class: "flex flex-wrap gap-2.5",
                for arg in props.args.iter() {
                    {
                        let on = props.enabled.contains(&arg.flag);
                        let flag = arg.flag.clone();
                        rsx! {
                            button {
                                key: "{arg.flag}",
                                r#type: "button",
                                disabled: props.disabled,
                                title: "{arg.help}",
                                class: if on {
                                    "flex items-center gap-2 rounded-full border border-accent                                      bg-accent px-4 py-2 text-button-utility text-white shadow-sm                                      disabled:opacity-50 transition-colors"
                                } else {
                                    "flex items-center gap-2 rounded-full border border-border-hard                                      bg-transparent px-4 py-2 text-button-utility text-fg-muted                                      hover:border-fg hover:text-fg disabled:opacity-50 transition-colors"
                                },
                                onclick: move |_| props.on_toggle.call(flag.clone()),
                                i {
                                    class: if on { "ph-fill ph-check-circle" } else { "ph ph-circle" },
                                }
                                span { class: "font-mono text-xs font-medium", "{arg.flag}" }
                            }
                        }
                    }
                }
            }
            for arg in props.args.iter().filter(|a| props.enabled.contains(&a.flag)) {
                if !arg.help.is_empty() {
                    div {
                        key: "{arg.flag}",
                        class: "text-caption text-fg-muted",
                        span { class: "font-mono font-medium text-fg", "{arg.flag}" }
                        " — {arg.help}"
                    }
                }
            }
        }
    }
}
