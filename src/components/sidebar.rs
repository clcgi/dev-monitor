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
        // RESPONSIVE: the fixed 260px inline width is gone.
        aside {
            class: "flex w-14 shrink-0 flex-col border-r border-edge bg-surface \
                    md:w-56 lg:w-64",

            div { class: "flex items-center justify-between px-3 pb-1 pt-3",
                div {
                    class: "hidden text-[10px] uppercase tracking-wider text-fg-faint md:block",
                    "DEV Scripts"
                }
                i { class: "ph ph-list text-fg-faint md:hidden" }
            }

            // The language filter.
            div { class: "flex gap-1 px-2 pb-2 pt-1",
                for (value, label, icon) in [
                    (Language::All, "All", "ph-list-dashes"),
                    (Language::Python, "Python", "ph-file-py"),
                    (Language::Shell, "Shell", "ph-terminal-window"),
                ] {
                    button {
                        key: "{label}",
                        r#type: "button",
                        title: "{label}",
                        class: if lang == value {
                            "flex flex-1 items-center justify-center gap-1 rounded-md border \
                             border-brand bg-brand/15 px-1.5 py-1 text-[10px] text-brand"
                        } else {
                            "flex flex-1 items-center justify-center gap-1 rounded-md border \
                             border-transparent px-1.5 py-1 text-[10px] text-fg-faint \
                             hover:bg-elevated hover:text-fg-soft"
                        },
                        onclick: move |_| language.set(value),
                        i { class: "ph {icon}" }
                        span { class: "hidden md:inline", "{label}" }
                    }
                }
            }

            div { class: "min-h-0 flex-1 overflow-y-auto p-1.5",
                if visible.is_empty() {
                    div {
                        class: "hidden px-3 py-5 text-[11px] leading-relaxed text-fg-faint md:block",
                        // The two empties are different and say so: nothing found at all is a setup.
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
                        div { key: "{category}", class: "mb-2",
                            // The group heading.
                            div {
                                class: "hidden w-full cursor-pointer items-center gap-1.5 rounded \
                                        px-2 pb-1 pt-2 text-[10px] uppercase tracking-wider \
                                        text-fg-faint hover:text-fg-soft md:flex",
                                onclick: {
                                    let key = key.clone();
                                    move |_| {
                                        let mut set = collapsed.write();
                                        if !set.remove(&key) { set.insert(key.clone()); }
                                    }
                                },
                                // The caret points at what a click does: right when closed, down when open.
                                i {
                                    class: if is_collapsed { "ph ph-caret-right" } else { "ph ph-caret-down" },
                                }
                                i { class: "ph {category_icon(&category)}" }
                                span { "{category}" }
                                // The count stays visible when collapsed -- it is the only thing left saying.
                                span { class: "ml-auto opacity-60", "{list.len()}" }
                            }
                            // At rail width the heading is a divider and an icon, and it collapses too.
                            div {
                                class: "mx-2 mb-1 mt-2 flex cursor-pointer justify-center \
                                        border-t border-edge pt-2 text-fg-faint md:hidden",
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
                                    // border-l-2 on BOTH states, transparent when off, so selecting a row does.
                                    class: if Some(&meta.path) == props.selected_script.as_ref() {
                                        "flex cursor-pointer items-center gap-2 rounded-md border-l-2 \
                                         border-brand bg-elevated px-2 py-2 justify-center md:justify-start"
                                    } else {
                                        "flex cursor-pointer items-center gap-2 rounded-md border-l-2 \
                                         border-transparent px-2 py-2 hover:bg-elevated justify-center \
                                         md:justify-start"
                                    },
                                    // The summary in the tooltip: the row is too narrow for it and the file name.
                                    title: if meta.summary.is_empty() {
                                        "{meta.path}"
                                    } else {
                                        "{meta.path} -- {meta.summary}"
                                    },
                                    onclick: {
                                        let m = meta.clone();
                                        move |_| props.on_select.call(m.clone())
                                    },

                                    // The language glyph gives way to a spinner while this script runs.
                                    if props.running_script.as_ref() == Some(&meta.path) {
                                        i { class: "ph ph-spinner ph-spin shrink-0 text-brand" }
                                    } else {
                                        span { class: "shrink-0 leading-none",
                                            if meta.path.ends_with(".py") { "🐍" } else { "🐚" }
                                        }
                                    }
                                    span {
                                        class: if props.running_script.as_ref() == Some(&meta.path) {
                                            "hidden min-w-0 truncate text-xs text-brand md:block"
                                        } else {
                                            "hidden min-w-0 truncate text-xs text-fg-soft md:block"
                                        },
                                        "{meta.file_name()}"
                                    }
                                    // A live dot at the trailing edge, visible even past the label.
                                    if props.running_script.as_ref() == Some(&meta.path) {
                                        span {
                                            class: "ml-auto hidden size-1.5 shrink-0 animate-pulse \
                                                    rounded-full bg-brand md:block",
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
