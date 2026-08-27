use dioxus::prelude::*;
use tokio::io::AsyncBufReadExt;
use futures_util::StreamExt;
use crate::components::sidebar::Sidebar;
use crate::components::dashboard::Dashboard;
use crate::services::state::{AppState, Environment, ScriptStatus};
use crate::services::process::ScriptRunner;

#[derive(Props, Clone, PartialEq)]
pub struct MainWindowProps {
    pub show_auth_reminder: Signal<bool>,
}

enum ProcessCommand {
    Run { script: String, env: Environment },
    Kill, // To abort the running process
}

#[component]
pub fn MainWindow(mut props: MainWindowProps) -> Element {
    let mut state = use_signal(AppState::new);
    let mut logs = use_signal(Vec::<String>::new);
    
    // Coroutine to handle script execution
    let process_coroutine = use_coroutine(move |mut rx: UnboundedReceiver<ProcessCommand>| async move {
        let mut child_proc: Option<tokio::process::Child> = None;
        
        loop {
            tokio::select! {
                cmd = rx.next() => {
                    match cmd {
                        Some(ProcessCommand::Run { script, env }) => {
                            state.write().script_status = ScriptStatus::Running;
                            logs.write().clear();
                            logs.write().push(format!("$ bash -c \"export CDW_ENV={}; source deploy/00-variables.sh && ... {}\"", env.as_str(), script));
                            
                            match ScriptRunner::spawn(&script, env.as_str()) {
                                Ok(mut child) => {
                                    let stdout = child.stdout.take();
                                    let stderr = child.stderr.take();
                                    
                                    // We spawn detached tasks to read stdout/stderr so we can keep listening to rx for Kill command
                                    let (tx, mut rx_logs) = tokio::sync::mpsc::unbounded_channel();
                                    
                                    if let Some(stdout) = stdout {
                                        let tx = tx.clone();
                                        tokio::spawn(async move {
                                            let mut reader = tokio::io::BufReader::new(stdout).lines();
                                            while let Ok(Some(line)) = reader.next_line().await {
                                                let _ = tx.send(line);
                                            }
                                        });
                                    }
                                    if let Some(stderr) = stderr {
                                        let tx = tx.clone();
                                        tokio::spawn(async move {
                                            let mut reader = tokio::io::BufReader::new(stderr).lines();
                                            while let Ok(Some(line)) = reader.next_line().await {
                                                let _ = tx.send(line);
                                            }
                                        });
                                    }
                                    
                                    child_proc = Some(child);
                                    
                                    // Note: We can't easily wait for the process here while also streaming logs 
                                    // AND checking for the Kill command inside a single select without a slightly more complex state machine.
                                    // For simplicity in Dioxus coroutine, we'll poll the child status and the logs.
                                    
                                    loop {
                                        tokio::select! {
                                            Some(line) = rx_logs.recv() => {
                                                logs.write().push(line);
                                            }
                                            kill = rx.next() => {
                                                if let Some(ProcessCommand::Kill) = kill {
                                                    if let Some(mut p) = child_proc.take() {
                                                        let _ = p.kill().await;
                                                        logs.write().push("Process killed by user.".to_string());
                                                        state.write().script_status = ScriptStatus::Failed(-1);
                                                    }
                                                    break;
                                                }
                                            }
                                            status = async {
                                                if let Some(p) = &mut child_proc {
                                                    p.wait().await
                                                } else {
                                                    std::future::pending::<std::io::Result<std::process::ExitStatus>>().await
                                                }
                                            } => {
                                                if let Ok(exit_status) = status {
                                                    if exit_status.success() {
                                                        state.write().script_status = ScriptStatus::Succeeded;
                                                    } else {
                                                        state.write().script_status = ScriptStatus::Failed(exit_status.code().unwrap_or(-1));
                                                    }
                                                }
                                                child_proc = None;
                                                break;
                                            }
                                        }
                                    }
                                }
                                Err(e) => {
                                    logs.write().push(format!("Failed to spawn process: {}", e));
                                    state.write().script_status = ScriptStatus::Failed(1);
                                }
                            }
                        },
                        Some(ProcessCommand::Kill) => {
                            if let Some(mut p) = child_proc.take() {
                                let _ = p.kill().await;
                            }
                            state.write().script_status = ScriptStatus::Failed(-1);
                        },
                        None => break,
                    }
                }
            }
        }
    });

    rsx! {
        div { class: "screen",
            
            // Topbar
            div { class: "topbar",
                div { class: "topbar-brand", "DEV Monitor" }
                div { class: "topbar-title",
                    span { class: "topbar-repo", "CentralDocumentWarehouse" }
                    span { class: "topbar-path", "Monitoring DEV ingestion pipeline" }
                }
            }
            
            // Body
            div { class: "body",
                Sidebar {
                    selected_script: state.read().selected_script.clone(),
                    on_select: move |script| {
                        let mut s = state.write();
                        s.selected_script = Some(script);
                        s.script_status = ScriptStatus::Idle;
                        logs.write().clear();
                    },
                }
                
                Dashboard {
                    selected_script: state.read().selected_script.clone(),
                    selected_env: state.read().selected_env,
                    status: state.read().script_status.clone(),
                    logs: logs.read().clone(),
                    on_env_select: move |env| {
                        state.write().selected_env = Some(env);
                    },
                    on_run: move |_| {
                        let s = state.read();
                        if let (Some(script), Some(env)) = (&s.selected_script, s.selected_env) {
                            process_coroutine.send(ProcessCommand::Run { 
                                script: script.clone(), 
                                env 
                            });
                        }
                    },
                    on_stop: move |_| {
                        process_coroutine.send(ProcessCommand::Kill);
                    },
                }
            }
            
            // Auth reminder modal overlay
            if *props.show_auth_reminder.read() {
                div { class: "modal-backdrop",
                    div { class: "modal",
                        div { class: "modal-head",
                            span { "Azure Authentication Required" }
                            button { class: "modal-close", onclick: move |_| props.show_auth_reminder.set(false), "×" }
                        }
                        div { class: "modal-body",
                            div { class: "welcome-empty",
                                p { "You must be authenticated with Azure to interact with the environment." }
                                p { style: "margin-top: 10px;", "Please ensure you have run:" }
                                p { class: "remedy-out", style: "margin-top: 10px;", "az login" }
                                button {
                                    class: "btn btn-primary",
                                    style: "margin-top: 15px; width: 100%;",
                                    onclick: move |_| props.show_auth_reminder.set(false),
                                    "I have authenticated"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
