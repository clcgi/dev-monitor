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
                    // The old inline style referenced `--green`, `--text1` and
                    // `--bg2` -- three variables that exist nowhere in the
                    // stylesheet, so the SELECTED state was styled exactly like
                    // the unselected one. Environment choice is a required step
                    // here and there was no visual confirmation of it.
                    class: if Some(env) == props.selected {
                        "rounded-full border border-brand bg-elevated px-3 py-1 font-mono \
                         text-[11px] text-fg"
                    } else {
                        "rounded-full border border-edge-soft bg-elevated px-3 py-1 font-mono \
                         text-[11px] text-fg-soft hover:border-brand hover:text-fg"
                    },
                    onclick: move |_| props.on_select.call(env),
                    "{env}"
                }
            }
        }
    }
}
