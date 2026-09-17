use std::path::Path;
use std::process::Stdio;
use tokio::process::{Child, Command};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver};
use tokio::io::{AsyncBufReadExt, BufReader};
use chrono::Local;
use crate::services::state::{LogMsg, StreamType};

pub struct ScriptRunner;

impl ScriptRunner {
    pub fn spawn(
        script_path: &str,
        env_str: &str,
        args: &[String],
    ) -> std::io::Result<(Child, UnboundedReceiver<LogMsg>)> {
        // Find the absolute path to the workspace root to ensure predictable execution
        let workspace_root = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("."))
            .canonicalize()?;
        
        let cdw_path = if workspace_root.ends_with("dev-monitor") {
            workspace_root.parent().unwrap().to_path_buf()
        } else {
            workspace_root
        };

        let mut child = spawn_in(
            &cdw_path.join("CentralDocumentWarehouse"),
            &command_line(script_path, env_str, args),
        )?;

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

/// The `bash -c` text that runs one script with the environment's profile.
fn command_line(script_path: &str, env_str: &str, args: &[String]) -> String {
    // Args are QUOTED individually.
    let quoted: String = args
        .iter()
        .map(|a| format!(" '{}'", a.replace('\'', r"'\''")))
        .collect();

    // `exec` so the script REPLACES the wrapping bash rather than running as
    // its child. Cancel kills the pid this runner returns; without `exec` that
    // was the wrapper alone, and the script ran on, re-parented to launchd.
    let command_str = if script_path.ends_with(".py") {
        // -u so the child does not block-buffer its stdout into the pipe.
        format!("exec .venv/bin/python -u {}{}", script_path, quoted)
    } else {
        format!("exec bash {}{}", script_path, quoted)
    };

    format!(
        "export CDW_ENV={}; source deploy/00-variables.sh && {}",
        env_str, command_str
    )
}

fn spawn_in(dir: &Path, command: &str) -> std::io::Result<Child> {
    Command::new("bash")
        .arg("-c")
        .arg(command)
        .current_dir(dir)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;
    use std::time::Duration;

    fn alive(pid: u32) -> bool {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// A CentralDocumentWarehouse stand-in whose "python" records its own pid
    /// and then sleeps -- long enough to still be there when Cancel arrives.
    fn fake_workspace(name: &str) -> std::path::PathBuf {
        // Per test: the two run in parallel threads of one process.
        let dir = std::env::temp_dir().join(format!("dev-monitor-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("deploy")).unwrap();
        std::fs::create_dir_all(dir.join(".venv/bin")).unwrap();
        std::fs::write(dir.join("deploy/00-variables.sh"), "").unwrap();
        let python = dir.join(".venv/bin/python");
        std::fs::write(&python, "#!/bin/bash\necho $$ > interpreter.pid\nexec sleep 30\n").unwrap();
        std::fs::set_permissions(&python, std::fs::Permissions::from_mode(0o755)).unwrap();
        dir
    }

    /// Cancel kills the process this runner hands back. When that was the
    /// `bash -c` wrapper, the interpreter survived it, was re-parented to
    /// launchd, and went on deleting DEV's records beside the next run
    /// (2026-09-17).
    async fn interpreter_pid(dir: &std::path::Path) -> u32 {
        let pid_file = dir.join("interpreter.pid");
        for _ in 0..100 {
            if let Ok(text) = std::fs::read_to_string(&pid_file) {
                if let Ok(pid) = text.trim().parse::<u32>() {
                    return pid;
                }
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        panic!("the fake interpreter never started");
    }

    /// Gone, or a zombie waiting for tokio's reaper -- either way no longer
    /// running. `kill -0` alone reports a zombie as alive.
    fn stopped(pid: u32) -> bool {
        let stat = std::process::Command::new("ps")
            .args(["-o", "stat=", "-p", &pid.to_string()])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default();
        !alive(pid) || stat.starts_with('Z')
    }

    fn finish(dir: &std::path::Path, pid: u32, survived: bool) {
        if survived {
            let _ = std::process::Command::new("kill").arg(pid.to_string()).status();
        }
        let _ = std::fs::remove_dir_all(dir);
    }

    #[tokio::test]
    async fn cancelling_the_child_stops_the_script_itself() {
        let dir = fake_workspace("cancel");
        let mut child = spawn_in(&dir, &command_line("tool.py", "dev", &[])).unwrap();
        let interpreter = interpreter_pid(&dir).await;

        child.kill().await.unwrap();
        let survived = !stopped(interpreter);
        finish(&dir, interpreter, survived);
        assert!(!survived, "the script outlived Cancel (pid {interpreter})");
    }

    /// A child the runner loses track of -- replaced, or dropped with its
    /// task -- must not keep running unseen.
    #[tokio::test]
    async fn dropping_the_child_stops_the_script() {
        let dir = fake_workspace("drop");
        let child = spawn_in(&dir, &command_line("tool.py", "dev", &[])).unwrap();
        let interpreter = interpreter_pid(&dir).await;

        drop(child);
        let mut survived = true;
        for _ in 0..40 {
            if stopped(interpreter) {
                survived = false;
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        finish(&dir, interpreter, survived);
        assert!(!survived, "the script outlived its dropped child (pid {interpreter})");
    }
}
