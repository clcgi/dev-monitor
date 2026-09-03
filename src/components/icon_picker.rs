use dioxus::prelude::*;

/// Icon names, generated from assets/phosphor.css by tools/vendor-phosphor.sh.
const ICON_NAMES: &str = include_str!("../../assets/phosphor-icons.txt");

/// How many matches to render. 1,530 nodes would make every keystroke rebuild
/// the whole grid; nobody scrolls past the first few dozen anyway.
const MAX_RESULTS: usize = 60;

pub fn is_known_icon(name: &str) -> bool {
    ICON_NAMES.lines().any(|l| l == name)
}

#[derive(Props, Clone, PartialEq)]
pub struct IconPickerProps {
    pub value: String,
    pub on_change: EventHandler<String>,
}

#[component]
pub fn IconPicker(props: IconPickerProps) -> Element {
    let mut query = use_signal(String::new);
    let mut open = use_signal(|| false);

    let current = props.value.clone();
    let known = is_known_icon(&current);
    let q = query.read().to_lowercase();
    let matches: Vec<&str> = ICON_NAMES
        .lines()
        .filter(|n| q.is_empty() || n.contains(&q))
        .take(MAX_RESULTS)
        .collect();

    rsx! {
        div { class: "flex flex-col gap-1.5",
            div { class: "flex items-center gap-2",
                // A live preview beside the field: the fastest way to see that a
                // typed name resolves is to look at it.
                span {
                    class: if known {
                        "flex size-8 shrink-0 items-center justify-center rounded border border-border-soft bg-app"
                    } else {
                        "flex size-8 shrink-0 items-center justify-center rounded border border-danger bg-app"
                    },
                    i { class: "ph-fill ph-{current} text-lg" }
                }
                input {
                    r#type: "text",
                    class: "min-w-0 flex-1 rounded border border-border-soft bg-app px-2 py-1 font-mono text-xs text-fg",
                    value: "{current}",
                    oninput: move |e| props.on_change.call(e.value()),
                }
                button {
                    r#type: "button",
                    class: "shrink-0 rounded border border-border-soft bg-app px-2 py-1 text-xs text-fg-muted hover:bg-app",
                    onclick: move |_| { let v = *open.read(); open.set(!v); },
                    "Browse"
                }
            }
            if !known && !current.is_empty() {
                span { class: "text-[10px] text-danger",
                    "No icon named ph-{current} -- it will render blank." }
            }
            if *open.read() {
                div { class: "flex flex-col gap-1.5 rounded border border-border-soft bg-app p-2",
                    input {
                        r#type: "text",
                        class: "rounded border border-border-soft bg-card px-2 py-1 text-xs text-fg",
                        placeholder: "Search {ICON_NAMES.lines().count()} icons",
                        value: "{query}",
                        oninput: move |e| query.set(e.value()),
                    }
                    div { class: "grid max-h-48 grid-cols-8 gap-1 overflow-y-auto",
                        for name in matches.iter() {
                            button {
                                key: "{name}",
                                r#type: "button",
                                title: "{name}",
                                class: if *name == current {
                                    "flex aspect-square items-center justify-center rounded border border-accent bg-accent/15 text-accent"
                                } else {
                                    "flex aspect-square items-center justify-center rounded border border-transparent text-fg-muted hover:bg-app"
                                },
                                onclick: {
                                    let n = name.to_string();
                                    move |_| { props.on_change.call(n.clone()); open.set(false); }
                                },
                                i { class: "ph-fill ph-{name} text-base" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_default_step_icon_is_a_real_icon() {
        // The defaults must not ship a blank square. This would have caught a
        // typo in StepCatalog::defaults before it reached the stepper.
        for step in crate::services::steps::StepCatalog::defaults().steps {
            assert!(is_known_icon(&step.icon), "{} -> ph-{}", step.name, step.icon);
        }
    }

    #[test]
    fn the_fallback_icon_exists() {
        // icon_of() returns ph-question for an unknown step.
        assert!(is_known_icon("question"));
    }

    #[test]
    fn a_made_up_name_is_rejected() {
        assert!(!is_known_icon("definitely-not-an-icon"));
        assert!(!is_known_icon(""));
    }
}
