use std::collections::HashSet;

use dioxus::prelude::*;

use crate::services::scripts::{self, ScriptMeta};

#[derive(Props, Clone, PartialEq)]
pub struct SidebarProps {
    pub selected_script: Option<String>,
    /// The script currently executing, if any.
    pub running_script: Option<String>,
    pub on_select: EventHandler<ScriptMeta>,
}

/// Where `tools/` is, from wherever the app was launched.
fn tools_dir() -> std::path::PathBuf {
    let cwd = std::env::current_dir()
        .unwrap_or_else(|_| std::path::PathBuf::from("."))
        .canonicalize()
        .unwrap_or_default();
    let root = if cwd.ends_with("dev-monitor") {
        cwd.parent().map(|p| p.to_path_buf()).unwrap_or(cwd)
    } else {
        cwd
    };
    root.join("CentralDocumentWarehouse").join("tools")
}

/// An icon per category, so the narrow rail still distinguishes the groups.
fn category_icon(category: &str) -> &'static str {
    match category {
        "Flows" => "ph-flow-arrow",
        "Verification" => "ph-magnifying-glass",
        "Simulation" => "ph-flask",
        "Maintenance" => "ph-wrench",
        _ => "ph-file-code",
    }
}

/// The language filter's three positions.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Language {
    All,
    Python,
    Shell,
}

impl Language {
    fn matches(&self, meta: &ScriptMeta) -> bool {
        match self {
            Language::All => true,
            Language::Python => meta.language() == "py",
            Language::Shell => meta.language() == "sh",
        }
    }
}

#[component]
pub fn Sidebar(props: SidebarProps) -> Element {
    let mut groups = use_signal(Vec::<(String, Vec<ScriptMeta>)>::new);
    let mut language = use_signal(|| Language::All);
    // COLLAPSED, not expanded, is the set that is tracked.
    let mut collapsed = use_signal(HashSet::<String>::new);

    use_effect(move || {
        groups.set(scripts::discover(&tools_dir()));
    });

    let lang = *language.read();
    // Filtered before the markup: `return` inside an rsx loop is ambiguous.
    let visible: Vec<(String, Vec<ScriptMeta>)> = groups
        .read()
        .iter()
        .filter_map(|(category, list)| {
            let kept: Vec<ScriptMeta> =
                list.iter().filter(|m| lang.matches(m)).cloned().collect();
            // A group with nothing left is dropped entirely.
            (!kept.is_empty()).then(|| (category.clone(), kept))
        })
        .collect();

    rsx! {
        // Sidebar is bg-sidebar (Tile-2 in dark, Canvas in light). Edge-to-edge border is softer.
        aside {
            class: "flex w-14 shrink-0 flex-col border-r border-border-soft bg-sidebar                     md:w-56 lg:w-64",

            div { class: "flex items-center justify-between px-4 pb-2 pt-4",
                div {
                    class: "hidden text-caption-strong text-fg-muted md:block",
                    "Scripts"
                }
                i { class: "ph ph-list text-fg-faint md:hidden" }
            }

            // Language filter styled as Apple configurator chips
            div { class: "flex gap-2 px-3 pb-3 pt-2",
                for (value, label, icon) in [
                    (Language::All, "All", "ph-list-dashes"),
                    (Language::Python, "Py", "ph-file-py"),
                    (Language::Shell, "Sh", "ph-terminal-window"),
                ] {
                    button {
                        key: "{label}",
                        r#type: "button",
                        title: "{label}",
                        class: if lang == value {
                            "flex flex-1 items-center justify-center gap-1.5 rounded-full border                              border-accent bg-accent text-button-utility text-white transition-all scale-100"
                        } else {
                            "flex flex-1 items-center justify-center gap-1.5 rounded-full border                              border-border-soft bg-transparent text-button-utility text-fg-muted                              hover:bg-black/5 dark:hover:bg-white/5 transition-colors scale-100 active:scale-95"
                        },
                        onclick: move |_| language.set(value),
                        i { class: "ph {icon}" }
                        span { class: "hidden md:inline", "{label}" }
                    }
                }
            }

            div { class: "min-h-0 flex-1 overflow-y-auto px-2 py-2",
                if visible.is_empty() {
                    div {
                        class: "hidden px-4 py-5 text-body text-fg-faint md:block",
                        if groups.read().is_empty() {
                            "No scripts found in tools/"
                        } else {
                            "No scripts match this language filter."
                        }
                    }
                } else {
                    for (category, list) in visible.clone() {
                        {
                        let is_collapsed = collapsed.read().contains(&category);
                        let key = category.clone();
                        rsx! {
                        div { key: "{category}", class: "mb-4",
                            div {
                                class: "hidden w-full cursor-pointer items-center gap-2 rounded-md                                         px-2 pb-1.5 pt-2 text-caption-strong text-fg-muted                                         hover:text-fg hover:bg-hover md:flex transition-colors",
                                onclick: {
                                    let key = key.clone();
                                    move |_| {
                                        let mut set = collapsed.write();
                                        if !set.remove(&key) { set.insert(key.clone()); }
                                    }
                                },
                                i {
                                    class: if is_collapsed { "ph ph-caret-right" } else { "ph ph-caret-down" },
                                }
                                i { class: "ph {category_icon(&category)}" }
                                span { "{category}" }
                                span { class: "ml-auto text-fg-faint text-xs", "{list.len()}" }
                            }
                            div {
                                class: "mx-2 mb-2 mt-3 flex cursor-pointer justify-center                                         border-t border-border-soft pt-2 text-fg-muted md:hidden",
                                onclick: {
                                    let key = key.clone();
                                    move |_| {
                                        let mut set = collapsed.write();
                                        if !set.remove(&key) { set.insert(key.clone()); }
                                    }
                                },
                                i { class: "ph {category_icon(&category)}", title: "{category}" }
                            }

                            if !is_collapsed {
                                for meta in list {
                                    div {
                                        key: "{meta.path}",
                                        class: if Some(&meta.path) == props.selected_script.as_ref() {
                                            "group flex cursor-pointer items-center gap-2 rounded-lg                                              bg-accent/10 px-3 py-1.5 justify-center md:justify-start                                              mb-0.5 text-accent"
                                        } else {
                                            "group flex cursor-pointer items-center gap-2 rounded-lg                                              bg-transparent px-3 py-1.5 hover:bg-black/5 dark:hover:bg-white/5                                              justify-center md:justify-start mb-0.5 text-fg"
                                        },
                                        title: if meta.summary.is_empty() {
                                            "{meta.path}"
                                        } else {
                                            "{meta.path} -- {meta.summary}"
                                        },
                                        onclick: {
                                            let m = meta.clone();
                                            move |_| props.on_select.call(m.clone())
                                        },

                                        if props.running_script.as_ref() == Some(&meta.path) {
                                            i { class: "ph ph-spinner ph-spin shrink-0 text-accent" }
                                        } else {
                                            span { class: "shrink-0 leading-none opacity-80",
                                                if meta.path.ends_with(".py") { "🐍" } else { "🐚" }
                                            }
                                        }
                                        span {
                                            class: if props.running_script.as_ref() == Some(&meta.path) {
                                                "hidden min-w-0 truncate text-body-strong md:block"
                                            } else {
                                                "hidden min-w-0 truncate text-body md:block"
                                            },
                                            "{meta.file_name()}"
                                        }
                                        if props.running_script.as_ref() == Some(&meta.path) {
                                            span {
                                                class: "ml-auto hidden size-2 shrink-0 animate-pulse                                                         rounded-full bg-accent md:block",
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        }
                        }
                    }
                }
            }
        }
    }
}
