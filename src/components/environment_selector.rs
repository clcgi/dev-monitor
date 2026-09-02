use dioxus::prelude::*;
use crate::services::state::Environment;

#[derive(Props, Clone, PartialEq)]
pub struct EnvSelectorProps {
    pub selected: Option<Environment>,
    pub on_select: EventHandler<Environment>,
}

#[component]
pub fn EnvironmentSelector(props: EnvSelectorProps) -> Element {
    let envs = [Environment::Sandbox, Environment::Dev, Environment::Stg];

    rsx! {
        div { class: "mb-4 flex flex-wrap gap-1.5",
            for env in envs.into_iter() {
                button {
                    class: if Some(env) == props.selected {
                        "flex items-center gap-2 rounded-full border border-accent bg-accent px-4 py-2 text-button-utility text-white shadow-sm transition-colors"
                    } else {
                        "flex items-center gap-2 rounded-full border border-border-hard bg-transparent px-4 py-2 text-button-utility text-fg-muted hover:border-fg hover:text-fg transition-colors"
                    },
                    onclick: move |_| props.on_select.call(env),
                    "{env}"
                }
            }
        }
    }
}
