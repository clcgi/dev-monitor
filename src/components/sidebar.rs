use dioxus::prelude::*;
use std::fs;

#[derive(Props, Clone, PartialEq)]
pub struct SidebarProps {
    pub selected_script: Option<String>,
    pub on_select: EventHandler<String>,
}

#[component]
pub fn Sidebar(props: SidebarProps) -> Element {
    let mut scripts = use_signal(Vec::<String>::new);

    use_effect(move || {
        let workspace_root = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .canonicalize()
            .unwrap_or_default();
        
        let cdw_path = if workspace_root.ends_with("dev-monitor") {
            workspace_root.parent().unwrap().to_path_buf()
        } else {
            workspace_root
        };
        
        let tools_dir = cdw_path.join("CentralDocumentWarehouse").join("tools");
        
        let mut found = Vec::new();
        if let Ok(entries) = fs::read_dir(tools_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
                    if ext == "py" || ext == "sh" {
                        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                            // Exclude common utility scripts that shouldn't be executed directly if any,
                            // or just list them all as per requirements.
                            found.push(format!("tools/{}", name));
                        }
                    }
                }
            }
        }
        found.sort();
        scripts.set(found);
    });

    rsx! {
        // RESPONSIVE: the fixed 260px inline width is gone. Below `md` the list
        // collapses to an icon rail -- on a narrow window the sidebar previously
        // took a quarter of the screen and left the log viewer unusable.
        aside {
            class: "flex w-14 shrink-0 flex-col border-r border-edge bg-surface \
                    md:w-56 lg:w-64",

            div { class: "flex items-center justify-between px-3 pb-1 pt-3",
                div {
                    class: "hidden text-[10px] uppercase tracking-wider text-fg-faint md:block",
                    "DEV Scripts"
                }
                // The rail still needs a heading at narrow widths; an icon is the
                // only thing that fits.
                i { class: "ph ph-list text-fg-faint md:hidden" }
            }

            div { class: "min-h-0 flex-1 overflow-y-auto p-1.5",
                if scripts.read().is_empty() {
                    div {
                        class: "hidden px-3 py-5 text-[11px] leading-relaxed text-fg-faint md:block",
                        "No scripts found in tools/"
                    }
                } else {
                    for script in scripts.read().clone() {
                        div {
                            key: "{script}",
                            // border-l-2 on BOTH states, transparent when off, so
                            // selecting a row does not shift its contents by 2px.
                            class: if Some(&script) == props.selected_script.as_ref() {
                                "flex cursor-pointer items-center gap-2 rounded-md border-l-2 \
                                 border-brand bg-elevated px-2 py-2 justify-center md:justify-start"
                            } else {
                                "flex cursor-pointer items-center gap-2 rounded-md border-l-2 \
                                 border-transparent px-2 py-2 hover:bg-elevated justify-center \
                                 md:justify-start"
                            },
                            title: "{script}",
                            onclick: {
                                let s = script.clone();
                                move |_| props.on_select.call(s.clone())
                            },

                            span { class: "shrink-0 leading-none",
                                if script.ends_with(".py") { "🐍" } else { "🐚" }
                            }
                            span {
                                class: "hidden min-w-0 truncate text-xs text-fg-soft md:block",
                                "{script.replace(\"tools/\", \"\")}"
                            }
                        }
                    }
                }
            }
        }
    }
}
