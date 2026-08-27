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
        div {
            class: "sidebar",
            style: "width: 260px;",
            
            div {
                class: "sidebar-head",
                div { class: "sidebar-title", "DEV Scripts" }
            }
            
            div {
                class: "sidebar-list",
                
                if scripts.read().is_empty() {
                    div { class: "sidebar-empty", "No scripts found in tools/" }
                } else {
                    for script in scripts.read().clone() {
                        div {
                            class: if Some(&script) == props.selected_script.as_ref() {
                                "sidebar-row sidebar-row-on"
                            } else {
                                "sidebar-row"
                            },
                            onclick: {
                                let s = script.clone();
                                move |_| props.on_select.call(s.clone())
                            },
                            
                            div { class: "forge-wrap",
                                span {
                                    class: "forge-icon forge-plain",
                                    if script.ends_with(".py") {
                                        "🐍"
                                    } else {
                                        "🐚"
                                    }
                                }
                            }
                            div {
                                class: "sidebar-main",
                                div { class: "sidebar-label-row",
                                    span { class: "sidebar-label", "{script.replace(\"tools/\", \"\")}" }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
