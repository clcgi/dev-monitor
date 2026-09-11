use std::collections::HashSet;

use dioxus::prelude::*;

use crate::services::scripts::{self, ScriptMeta};
use crate::services::marker_syntax::MarkerSyntax;
use crate::services::state::ScriptStatus;
use crate::services::steps::StepCatalog;

#[derive(Props, Clone, PartialEq)]
pub struct SidebarProps {
    pub selected_script: Option<String>,
    /// The script currently executing, if any.
    pub running_script: Option<String>,
    /// Each script's last outcome, keyed by path. Absent = never run.
    pub statuses: std::collections::HashMap<String, ScriptStatus>,
    pub catalog: StepCatalog,
    pub syntax: MarkerSyntax,
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
    let catalog = props.catalog.clone();
    let syntax = props.syntax.clone();
    let mut language = use_signal(|| Language::All);
    // COLLAPSED, not expanded, is the set that is tracked.
    let mut collapsed = use_signal(HashSet::<String>::new);

    let mut refreshed = use_signal(|| 0u32);
    use_effect(move || {
        let _ = refreshed.read();
        groups.set(scripts::discover(&tools_dir(), &catalog, &syntax));
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
                button {
                    r#type: "button",
                    title: "Rescan tools/ for new or edited scripts",
                    class: "rounded p-1 text-fg-faint transition-colors hover:text-fg",
                    onclick: move |_| { let n = *refreshed.read(); refreshed.set(n + 1); },
                    i { class: "ph ph-arrows-clockwise text-sm" }
                }
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
                                    {
                                    let running = props.running_script.as_ref() == Some(&meta.path);
                                    // The last outcome, in the colours a Postman
                                    // user expects. A script never run keeps the
                                    // neutral colour -- green would claim a pass
                                    // that never happened.
                                    let name_class = name_class(props.statuses.get(&meta.path), running);
                                    let element: Element = rsx! {
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

                                        if running {
                                            i { class: "ph ph-spinner-gap animate-spin shrink-0 text-accent" }
                                        } else {
                                            span { class: "shrink-0 leading-none opacity-80",
                                                if meta.path.ends_with(".py") { "🐍" } else { "🐚" }
                                            }
                                        }
                                        span { class: "{name_class}", "{meta.file_name()}" }
                                        if running {
                                            span {
                                                class: "ml-auto hidden size-2 shrink-0 animate-pulse rounded-full bg-accent md:block",
                                            }
                                        }
                                    }
                                    };
                                    element
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

/// The colour a script's name carries, from its last outcome.
///
/// Postman's convention: green passed, red failed, amber somewhere in between.
/// A script never run stays neutral -- green would claim a pass that never
/// happened, and that is the one wrong answer a user would act on.
fn name_class(status: Option<&ScriptStatus>, running: bool) -> &'static str {
    const BASE: &str = "hidden min-w-0 truncate text-body md:block";
    if running {
        return "hidden min-w-0 truncate text-body-strong text-accent md:block";
    }
    match status {
        Some(ScriptStatus::Succeeded) => "hidden min-w-0 truncate text-body text-success md:block",
        // The 300-499 band, as asked. Note exit codes are 0-255 on every
        // platform this runs on, so it fires only for a script that reports an
        // HTTP-shaped code deliberately -- see the test.
        Some(ScriptStatus::Failed(c)) if (300..=499).contains(c) => {
            "hidden min-w-0 truncate text-body text-warn md:block"
        }
        Some(ScriptStatus::Failed(_)) | Some(ScriptStatus::AppError(_)) => {
            "hidden min-w-0 truncate text-body text-danger md:block"
        }
        Some(ScriptStatus::Cancelled) => "hidden min-w-0 truncate text-body text-warn md:block",
        _ => BASE,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn colour(status: Option<ScriptStatus>) -> &'static str {
        let c = name_class(status.as_ref(), false);
        for (needle, name) in [
            ("text-success", "green"), ("text-danger", "red"),
            ("text-warn", "amber"), ("text-accent", "accent"),
        ] {
            if c.contains(needle) {
                return name;
            }
        }
        "neutral"
    }

    #[test]
    fn a_pass_is_green_and_a_failure_is_red() {
        assert_eq!(colour(Some(ScriptStatus::Succeeded)), "green");
        assert_eq!(colour(Some(ScriptStatus::Failed(1))), "red");
        assert_eq!(colour(Some(ScriptStatus::Failed(2))), "red");
        assert_eq!(colour(Some(ScriptStatus::AppError("spawn".into()))), "red");
    }

    #[test]
    fn the_300_to_499_band_is_amber() {
        for code in [300, 404, 422, 499] {
            assert_eq!(colour(Some(ScriptStatus::Failed(code))), "amber", "{code}");
        }
        assert_eq!(colour(Some(ScriptStatus::Failed(299))), "red");
        assert_eq!(colour(Some(ScriptStatus::Failed(500))), "red");
    }

    #[test]
    fn a_cancelled_run_is_amber_rather_than_red() {
        // Stopping a run is not a failure of the script, and colouring it red
        // would report a defect the user caused on purpose.
        assert_eq!(colour(Some(ScriptStatus::Cancelled)), "amber");
    }

    #[test]
    fn a_script_never_run_is_neutral() {
        // The one answer that must not be green.
        assert_eq!(colour(None), "neutral");
        assert_eq!(colour(Some(ScriptStatus::Idle)), "neutral");
    }

    #[test]
    fn a_running_script_overrides_its_last_outcome() {
        // What it is doing now beats what it did last time.
        assert!(name_class(Some(&ScriptStatus::Failed(1)), true).contains("text-accent"));
        assert!(name_class(None, true).contains("text-accent"));
    }
}
