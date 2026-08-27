use std::process::Stdio;
use tokio::process::{Child, Command};

pub struct ScriptRunner;

impl ScriptRunner {
    pub fn spawn(script_path: &str, env_str: &str) -> std::io::Result<Child> {
        // Find the absolute path to the workspace root to ensure predictable execution
        let workspace_root = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .canonicalize()?;
        
        let cdw_path = if workspace_root.ends_with("dev-monitor") {
            workspace_root.parent().unwrap().to_path_buf()
        } else {
            workspace_root
        };

        // Determine how to run the script. It could be .py or .sh
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

        Command::new("bash")
            .arg("-c")
            .arg(&bash_cmd)
            .current_dir(cdw_path.join("CentralDocumentWarehouse"))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
    }
}
