use std::process::Stdio;
use tokio::process::{Child, Command};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tokio::io::{AsyncBufReadExt, BufReader};
use chrono::Local;
use crate::services::state::{LogMsg, StreamType};

pub struct ScriptRunner;

impl ScriptRunner {
    pub fn spawn(script_path: &str, env_str: &str) -> std::io::Result<(Child, UnboundedReceiver<LogMsg>)> {
        // Find the absolute path to the workspace root to ensure predictable execution
        let workspace_root = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .canonicalize()?;
        
        let cdw_path = if workspace_root.ends_with("dev-monitor") {
            workspace_root.parent().unwrap().to_path_buf()
        } else {
            workspace_root
        };

        let command_str = if script_path.ends_with(".py") {
            format!(".venv/bin/python {}", script_path)
        } else {
            format!("bash {}", script_path)
        };

        let bash_cmd = format!(
            "export CDW_ENV={}; source deploy/00-variables.sh && {}",
            env_str,
            command_str
        );

        let mut child = Command::new("bash")
            .arg("-c")
            .arg(&bash_cmd)
            .current_dir(cdw_path.join("CentralDocumentWarehouse"))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let (tx, rx) = unbounded_channel();

        if let Some(stdout) = child.stdout.take() {
            let tx = tx.clone();
            tokio::spawn(async move {
                let mut reader = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    let _ = tx.send(LogMsg {
                        timestamp: Local::now(),
                        stream: StreamType::Stdout,
                        content: line,
                    });
                }
            });
        }

        if let Some(stderr) = child.stderr.take() {
            let tx = tx.clone();
            tokio::spawn(async move {
                let mut reader = BufReader::new(stderr).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    let _ = tx.send(LogMsg {
                        timestamp: Local::now(),
                        stream: StreamType::Stderr,
                        content: line,
                    });
                }
            });
        }

        Ok((child, rx))
    }
}
