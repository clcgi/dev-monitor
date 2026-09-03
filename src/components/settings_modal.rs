use dioxus::prelude::*;

use crate::components::icon_picker::IconPicker;
use crate::services::marker_syntax::{MarkerDef, MarkerKind, MarkerSyntax};
use crate::services::steps::{StepCatalog, StepDef};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Markers,
    Steps,
}

#[derive(Props, Clone, PartialEq)]
pub struct SettingsModalProps {
    pub catalog: StepCatalog,
    pub syntax: MarkerSyntax,
    pub on_save: EventHandler<(StepCatalog, MarkerSyntax)>,
    pub on_close: EventHandler<()>,
}

#[component]
pub fn SettingsModal(props: SettingsModalProps) -> Element {
    let mut draft = use_signal(|| props.catalog.clone());
    let mut syn = use_signal(|| props.syntax.clone());
    let mut tab = use_signal(|| Tab::Markers);
    // Index being dragged. Reordering is a move, so the source has to survive
    // until a drop tells us the destination.
    let mut dragging = use_signal(|| Option::<usize>::None);

    let current_tab = *tab.read();

    rsx! {
        // A scrim: the app behind stays visible but recedes. Black rather than a
        // theme token, so it dims identically in both themes -- an app-coloured
        // overlay lightens the page in light mode instead of dimming it.
        div { class: "fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4",
            div {
                class: "flex max-h-[85vh] w-full max-w-2xl flex-col overflow-hidden rounded-lg \
                        border border-border-soft bg-card shadow-2xl",

                div { class: "flex shrink-0 items-center gap-2 border-b border-border-soft px-4 py-3",
                    i { class: "ph ph-gear text-fg-muted" }
                    span { class: "flex-1 text-xs font-semibold uppercase tracking-wider text-fg-muted",
                        "Settings" }
                    button {
                        r#type: "button",
                        class: "rounded p-1 text-fg-faint hover:bg-app hover:text-fg",
                        onclick: move |_| props.on_close.call(()),
                        i { class: "ph ph-x" }
                    }
                }

                div { class: "flex shrink-0 gap-1 border-b border-border-soft px-3 pt-2",
                    for (value, label) in [(Tab::Markers, "Markers"), (Tab::Steps, "Steps & icons")] {
                        button {
                            key: "{label}",
                            r#type: "button",
                            class: if current_tab == value {
                                "border-b-2 border-accent px-3 py-2 text-xs text-accent"
                            } else {
                                "border-b-2 border-transparent px-3 py-2 text-xs text-fg-faint hover:text-fg-muted"
                            },
                            onclick: move |_| tab.set(value),
                            "{label}"
                        }
                    }
                }

                div { class: "min-h-0 flex-1 overflow-y-auto p-3",
                    if current_tab == Tab::Steps {
                        div { class: "flex flex-col gap-2",
                            for (i, step) in draft.read().steps.iter().enumerate() {
                                {
                                    let id = step.id.clone();
                                    rsx! {
                                        div {
                                            key: "{id}",
                                            class: "flex flex-col gap-2 rounded-lg border border-border-soft bg-app p-2.5",
                                            draggable: true,
                                            ondragstart: move |_| dragging.set(Some(i)),
                                            ondragover: move |e| e.prevent_default(),
                                            ondrop: move |_| {
                                                if let Some(from) = *dragging.read() {
                                                    draft.write().reorder(from, i);
                                                }
                                                dragging.set(None);
                                            },
                                            div { class: "flex items-center gap-2",
                                                i { class: "ph ph-dots-six-vertical cursor-grab text-fg-faint" }
                                                input {
                                                    r#type: "text",
                                                    class: "min-w-0 flex-1 rounded border border-border-soft bg-app px-2 py-1 text-xs text-fg",
                                                    value: "{step.name}",
                                                    oninput: move |e| draft.write().steps[i].name = e.value(),
                                                }
                                                // The id is shown and NOT editable: it is what
                                                // markers and script declarations resolve to, and
                                                // changing it would orphan both.
                                                span { class: "shrink-0 font-mono text-[10px] text-fg-faint", "{id}" }
                                                label { class: "flex shrink-0 items-center gap-1 text-[10px] text-fg-faint",
                                                    input {
                                                        r#type: "checkbox",
                                                        checked: step.on_chain,
                                                        onchange: move |e| draft.write().steps[i].on_chain = e.checked(),
                                                    }
                                                    "on chain"
                                                }
                                                button {
                                                    r#type: "button",
                                                    title: "Delete this step",
                                                    class: "shrink-0 rounded p-1 text-fg-faint hover:bg-danger hover:text-white",
                                                    onclick: {
                                                        let id = id.clone();
                                                        move |_| { draft.write().remove(&id); }
                                                    },
                                                    i { class: "ph ph-trash text-xs" }
                                                }
                                            }
                                            IconPicker {
                                                value: step.icon.clone(),
                                                on_change: move |v| draft.write().steps[i].icon = v,
                                            }
                                        }
                                    }
                                }
                            }
                            button {
                                r#type: "button",
                                class: "flex items-center justify-center gap-1.5 rounded-lg border \
                                        border-dashed border-border-soft px-3 py-2 text-xs text-fg-faint \
                                        hover:border-accent hover:text-accent",
                                onclick: move |_| {
                                    let mut d = draft.write();
                                    let id = d.mint_id("New step");
                                    d.steps.push(StepDef {
                                        id, name: "New step".into(), icon: "circle".into(),
                                        aliases: vec![], on_chain: true,
                                    });
                                },
                                i { class: "ph ph-plus" }
                                "Add a step"
                            }
                        }
                    } else {
                        div { class: "flex flex-col gap-2",
                            div { class: "px-1 text-[11px] leading-relaxed text-fg-faint",
                                "Every marker the app looks for. Delete any of them, or add your \
                                 own. The token is written without brackets or the colon: "
                                span { class: "font-mono", "CDW_STEP" }
                                " matches "
                                span { class: "font-mono", "[CDW_STEP: Raw]" }
                                "."
                            }
                            for (i, marker) in syn.read().markers.iter().enumerate() {
                                div {
                                    key: "{i}",
                                    class: "flex flex-col gap-1.5 rounded-lg border border-border-soft bg-app p-2.5",
                                    div { class: "flex items-center gap-2",
                                        input {
                                            r#type: "text",
                                            placeholder: "TOKEN",
                                            class: "w-44 shrink-0 rounded border border-border-soft bg-card px-2 py-1 font-mono text-xs text-fg",
                                            value: "{marker.token}",
                                            // Cleaned on every keystroke, not on
                                            // commit: someone pasting `[CDW_STEP:`
                                            // should see it become a token rather
                                            // than save one that cannot match.
                                            oninput: move |e| {
                                                syn.write().markers[i].token = MarkerSyntax::clean_token(&e.value());
                                            },
                                        }
                                        select {
                                            class: "min-w-0 flex-1 rounded border border-border-soft bg-card px-2 py-1 text-xs text-fg",
                                            onchange: move |e| {
                                                if let Some(k) = MarkerKind::ALL.iter()
                                                    .find(|k| k.label() == e.value()) {
                                                    syn.write().markers[i].kind = *k;
                                                }
                                            },
                                            for kind in MarkerKind::ALL {
                                                option {
                                                    key: "{kind.label()}",
                                                    value: "{kind.label()}",
                                                    selected: kind == marker.kind,
                                                    "{kind.label()}"
                                                }
                                            }
                                        }
                                        button {
                                            r#type: "button",
                                            title: "Delete this marker",
                                            class: "shrink-0 rounded p-1 text-fg-faint hover:bg-danger hover:text-white",
                                            onclick: move |_| { syn.write().markers.remove(i); },
                                            i { class: "ph ph-trash text-xs" }
                                        }
                                    }
                                    div { class: "font-mono text-[10px] text-fg-faint",
                                        {marker.kind.example().replace("TOKEN", if marker.token.is_empty() { "?" } else { &marker.token })}
                                    }
                                    if marker.token.is_empty() {
                                        span { class: "text-[10px] text-warn", "No token -- this row matches nothing." }
                                    }
                                }
                            }
                            button {
                                r#type: "button",
                                class: "flex items-center justify-center gap-1.5 rounded-lg border \
                                        border-dashed border-border-soft px-3 py-2 text-xs text-fg-faint \
                                        hover:border-accent hover:text-accent",
                                onclick: move |_| {
                                    syn.write().markers.push(MarkerDef {
                                        token: String::new(),
                                        kind: MarkerKind::StepStarted,
                                    });
                                },
                                i { class: "ph ph-plus" }
                                "Add a marker"
                            }
                            if syn.read().markers.is_empty() {
                                div { class: "rounded-lg border border-dashed border-warn p-3 text-[11px] text-warn",
                                    "No markers. Script output will not move the stepper at all."
                                }
                            }
                        }
                    }
                }

                div { class: "flex shrink-0 items-center gap-2 border-t border-border-soft px-4 py-3",
                    // The only way back from a deletion, so it is on both tabs.
                    button {
                        r#type: "button",
                        class: "rounded-lg border border-border-soft px-3 py-1.5 text-xs text-fg-muted hover:bg-app",
                        onclick: move |_| { draft.set(StepCatalog::defaults()); syn.set(MarkerSyntax::default()); },
                        "Restore defaults"
                    }
                    div { class: "flex-1" }
                    button {
                        r#type: "button",
                        class: "rounded-lg border border-border-soft px-3 py-1.5 text-xs text-fg-muted hover:bg-app",
                        onclick: move |_| props.on_close.call(()),
                        "Cancel"
                    }
                    button {
                        r#type: "button",
                        class: "rounded-lg border border-accent bg-accent px-3 py-1.5 text-xs \
                                text-white hover:border-accent hover:bg-accent",
                        onclick: move |_| props.on_save.call((draft.read().clone(), syn.read().clone())),
                        "Save"
                    }
                }
            }
        }
    }
}
