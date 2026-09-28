use dioxus::prelude::*;
use crate::services::state::Environment;

#[derive(Props, Clone, PartialEq)]
pub struct EnvSelectorProps {
    pub selected: Option<Environment>,
    pub on_select: EventHandler<Environment>,
}

#[component]
pub fn EnvironmentSelector(props: EnvSelectorProps) -> Element {
    let envs = [Environment::Dev, Environment::Stg];

    rsx! {
        div { class: "inline-flex gap-[3px] rounded-lg bg-track p-[3px]",
            for env in envs.into_iter() {
                button {
                    class: if Some(env) == props.selected {
                        "rounded-md bg-card px-4 py-1.5 text-button-utility font-semibold text-fg shadow-sm"
                    } else {
                        "rounded-md bg-transparent px-4 py-1.5 text-button-utility text-fg-muted hover:text-fg"
                    },
                    onclick: move |_| props.on_select.call(env),
                    "{env}"
                }
            }
        }
    }
}
