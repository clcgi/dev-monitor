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
        div {
            class: "identity-picks",
            style: "margin-bottom: 15px;",
            
            for env in envs.into_iter() {
                button {
                    class: if Some(env) == props.selected {
                        "identity-pick identity-pick-on"
                    } else {
                        "identity-pick"
                    },
                    style: if Some(env) == props.selected {
                        "border-color: var(--green); color: var(--text1); background: var(--bg2);"
                    } else {
                        ""
                    },
                    onclick: move |_| props.on_select.call(env),
                    "{env}"
                }
            }
        }
    }
}
