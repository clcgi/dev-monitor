//! The flags a script accepts, offered as toggles before it runs.

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
        div { class: "flex flex-col gap-2",
            div { class: "text-[10px] uppercase tracking-wider text-fg-faint", "Options" }
            div { class: "flex flex-wrap gap-2",
                for arg in props.args.iter() {
                    {
                        let on = props.enabled.contains(&arg.flag);
                        let flag = arg.flag.clone();
                        rsx! {
                            button {
                                key: "{arg.flag}",
                                r#type: "button",
                                disabled: props.disabled,
                                // The help text is the tooltip.
                                title: "{arg.help}",
                                class: if on {
                                    "flex items-center gap-1.5 rounded-md border border-brand \
                                     bg-brand/15 px-2.5 py-1.5 font-mono text-[11px] text-brand \
                                     disabled:opacity-40"
                                } else {
                                    "flex items-center gap-1.5 rounded-md border border-edge \
                                     bg-elevated px-2.5 py-1.5 font-mono text-[11px] text-fg-soft \
                                     hover:bg-active hover:text-fg disabled:opacity-40"
                                },
                                onclick: move |_| props.on_toggle.call(flag.clone()),
                                i {
                                    class: if on { "ph-fill ph-check-square" } else { "ph ph-square" },
                                }
                                "{arg.flag}"
                            }
                        }
                    }
                }
            }
            // The help of every enabled flag, spelled out.
            for arg in props.args.iter().filter(|a| props.enabled.contains(&a.flag)) {
                if !arg.help.is_empty() {
                    div {
                        key: "{arg.flag}",
                        class: "text-[11px] leading-relaxed text-fg-faint",
                        span { class: "font-mono text-fg-soft", "{arg.flag}" }
                        " — {arg.help}"
                    }
                }
            }
        }
    }
}
