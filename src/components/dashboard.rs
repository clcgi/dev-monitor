use dioxus::prelude::*;
use crate::services::state::{Environment, ScriptStatus};
use crate::components::environment_selector::EnvironmentSelector;
use crate::components::log_viewer::LogViewer;

#[derive(Props, Clone, PartialEq)]
pub struct DashboardProps {
    pub selected_script: Option<String>,
    pub selected_env: Option<Environment>,
    pub status: ScriptStatus,
    pub logs: Vec<String>,
    pub on_env_select: EventHandler<Environment>,
    pub on_run: EventHandler<()>,
    pub on_stop: EventHandler<()>,
}

#[component]
pub fn Dashboard(props: DashboardProps) -> Element {
    let script_name = props
        .selected_script
        .as_deref()
        .map(|s| s.replace("tools/", ""))
        .unwrap_or_else(|| "No script selected".to_string());

    let is_running = matches!(props.status, ScriptStatus::Running);
    let can_run = props.selected_script.is_some() && props.selected_env.is_some() && !is_running;

    let status_str = match props.status {
        ScriptStatus::Idle => "Idle",
        ScriptStatus::Running => "Running",
        ScriptStatus::Succeeded => "Succeeded",
        ScriptStatus::Failed(c) => return rsx! { span { class: "status-failed", "Failed ({c})" } },
    };

    let status_class = match props.status {
        ScriptStatus::Idle => "status-pending",
        ScriptStatus::Running => "status-running",
        ScriptStatus::Succeeded => "status-done",
        ScriptStatus::Failed(_) => "status-failed",
    };

    let dot_class = match props.status {
        ScriptStatus::Idle => "dot-pending",
        ScriptStatus::Running => "dot-running",
        ScriptStatus::Succeeded => "dot-done",
        ScriptStatus::Failed(_) => "dot-failed",
    };

    rsx! {
        div {
            class: "detail-col",
            style: "display: flex; flex-direction: column; height: 100%;",
            
            if props.selected_script.is_none() {
                div {
                    class: "placeholder",
                    div { class: "placeholder-title", "Select a script" }
                    div { class: "placeholder-sub", "Choose a script from the sidebar to monitor or run." }
                }
            } else {
                div {
                    class: "detail",
                    style: "display: flex; flex-direction: column; height: 100%; gap: 16px;",
                    
                    div {
                        class: "detail-head",
                        div { class: "detail-title", "{script_name}" }
                        
                        div { class: "detail-status {status_class}",
                            span { class: "dot {dot_class}", style: "margin-right: 6px;" }
                            "{status_str}"
                        }
                    }
                    
                    div { class: "field-note", "Select the target environment before executing." }
                    
                    EnvironmentSelector {
                        selected: props.selected_env,
                        on_select: move |e| props.on_env_select.call(e),
                    }
                    
                    div {
                        class: "contract",
                        if is_running {
                            button {
                                class: "btn btn-danger",
                                onclick: move |_| props.on_stop.call(()),
                                "Stop Script"
                            }
                        } else {
                            button {
                                class: "btn btn-primary",
                                disabled: !can_run,
                                onclick: move |_| props.on_run.call(()),
                                "Run Script"
                            }
                        }
                        
                        if props.selected_env.is_none() {
                            span { class: "field-note", style: "color: var(--orange); align-self: center;", "Please select an environment" }
                        }
                    }
                    
                    div { class: "log-head", "Execution Output" }
                    LogViewer { logs: props.logs.clone() }
                }
            }
        }
    }
}
