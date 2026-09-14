use super::model::Probe;
use serde::de::DeserializeOwned;
use std::path::PathBuf;
use std::time::Duration;
use tokio::process::Command;

/// Log Analytics plus an archive download can take a while on a cold login.
const TIMEOUT: Duration = Duration::from_secs(150);

#[derive(Clone, Debug, PartialEq)]
pub enum Request {
    Overview,
    Trace(String),
    DeadLetters,
    Timing,
}

impl Request {
    fn args(&self) -> Vec<String> {
        match self {
            Request::Overview => vec!["overview".into()],
            Request::Trace(q) => vec!["trace".into(), q.clone()],
            Request::DeadLetters => vec!["deadletters".into()],
            Request::Timing => vec!["timing".into()],
        }
    }
}

/// The workspace holding both `dev-monitor` and `CentralDocumentWarehouse`.
pub fn workspace_root() -> PathBuf {
    let cwd = std::env::current_dir()
        .and_then(|p| p.canonicalize())
        .unwrap_or_else(|_| PathBuf::from("."));
    if cwd.ends_with("dev-monitor") {
        cwd.parent().map(PathBuf::from).unwrap_or(cwd)
    } else {
        cwd
    }
}

fn quote(arg: &str) -> String {
    format!("'{}'", arg.replace('\'', r"'\''"))
}

pub const PROBE_OVERRIDE: &str = "CDW_MONITOR_PROBE";

/// The Azure CLI profile for an environment.
pub fn azure_config_dir(env: &str, home: &std::path::Path) -> Option<PathBuf> {
    [env.to_uppercase(), env.to_lowercase()]
        .into_iter()
        .map(|name| home.join(".azure").join(format!("sbm-{name}")))
        .find(|dir| dir.is_dir())
}

pub fn command_line(env: &str, request: &Request) -> String {
    let probe = std::env::var_os(PROBE_OVERRIDE)
        .map(PathBuf::from)
        .unwrap_or_else(|| workspace_root().join("CentralDocumentWarehouse/tools/monitor_probe.py"));
    let args: String = request.args().iter().map(|a| format!(" {}", quote(a))).collect();
    let profile = dirs::home_dir()
        .and_then(|home| azure_config_dir(env, &home))
        .map(|dir| format!("export AZURE_CONFIG_DIR={}; ", quote(&dir.to_string_lossy())))
        .unwrap_or_default();
    // 00-variables prints progress on stdout; it must not reach the JSON reader.
    format!(
        "{profile}export CDW_ENV={}; source deploy/00-variables.sh >/dev/null 2>&1 && exec .venv/bin/python {}{}",
        quote(env),
        quote(&probe.to_string_lossy()),
        args
    )
}

pub async fn run<T: DeserializeOwned>(env: &str, request: Request) -> Probe<T> {
    let cdw = workspace_root().join("CentralDocumentWarehouse");
    if !cdw.join(".venv/bin/python").exists() {
        return Probe::Unavailable {
            message: format!(
                "No Python environment at {}/.venv. The probe reuses the platform's own \
                 Azure SDKs; create it with `make install` in CentralDocumentWarehouse.",
                cdw.display()
            ),
        };
    }
    let child = Command::new("bash")
        .arg("-c")
        .arg(command_line(env, &request))
        .current_dir(&cdw)
        .kill_on_drop(true)
        .output();
    let output = match tokio::time::timeout(TIMEOUT, child).await {
        Err(_) => {
            return Probe::Error {
                message: format!("The probe did not answer within {}s.", TIMEOUT.as_secs()),
            }
        }
        Ok(Err(e)) => return Probe::Error { message: format!("Could not start the probe: {e}") },
        Ok(Ok(out)) => out,
    };
    parse(&String::from_utf8_lossy(&output.stdout), &String::from_utf8_lossy(&output.stderr))
}

/// The LAST line is the document: SDK warnings may precede it on stdout.
pub fn parse<T: DeserializeOwned>(stdout: &str, stderr: &str) -> Probe<T> {
    let Some(line) = stdout.lines().rev().find(|l| l.trim_start().starts_with('{')) else {
        let tail: Vec<&str> = stderr.lines().rev().take(6).collect();
        let tail: Vec<&str> = tail.into_iter().rev().collect();
        return Probe::Error {
            message: if tail.is_empty() {
                "The probe exited without a result.".into()
            } else {
                format!("The probe exited without a result: {}", tail.join(" ⏎ "))
            },
        };
    };
    let parsed = serde_json::from_str::<serde_json::Value>(line).and_then(|mut value| {
        drop_nulls(&mut value);
        serde_json::from_value(value)
    });
    parsed.unwrap_or_else(|e| Probe::Error {
        message: format!("The probe answered in a shape this build does not read: {e}"),
    })
}

fn drop_nulls(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            map.retain(|_, v| !v.is_null());
            map.values_mut().for_each(drop_nulls);
        }
        serde_json::Value::Array(items) => items.iter_mut().for_each(drop_nulls),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitoring::model::Overview;

    #[test]
    fn reads_the_last_json_line_past_sdk_noise() {
        let out = "warning: something\n{\"status\":\"ok\",\"data\":{\"catalogRows\":3}}\n";
        match parse::<Overview>(out, "") {
            Probe::Ok { data } => assert_eq!(data.catalog_rows, 3),
            other => panic!("expected ok, got {other:?}"),
        }
    }

    #[test]
    fn explicit_nulls_from_cosmos_read_as_absent_fields() {
        // One line, as the probe prints it.
        let out = r#"{"status":"ok","data":{"catalogRows":1,"checkedAt":null,"parked":[{"documentId":"D","businessKey":null,"fileName":null,"pendingKey":null,"attributes":{"reference.x":null}}]}}"#;
        match parse::<Overview>(out, "") {
            Probe::Ok { data } => {
                assert_eq!(data.parked[0].document_id, "D");
                assert_eq!(data.parked[0].display_key(), "D");
                assert_eq!(data.parked[0].pending_key, None);
            }
            other => panic!("expected ok, got {other:?}"),
        }
    }

    #[test]
    #[ignore]
    fn recorded_probe_answers_parse() {
        use crate::monitoring::model::{DeadLetters, Timing, Trace};
        let dir = std::path::PathBuf::from(std::env::var("CDW_PROBE_SAMPLES").expect("CDW_PROBE_SAMPLES"));
        let read = |name: &str| std::fs::read_to_string(dir.join(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
        fn ok<T: std::fmt::Debug>(name: &str, probe: Probe<T>) -> T {
            match probe {
                Probe::Ok { data } => data,
                other => panic!("{name}: {:?}", other),
            }
        }
        let overview = ok("overview", parse::<Overview>(&read("overview.json"), ""));
        let trace = ok("trace", parse::<Trace>(&read("trace.json"), ""));
        let dead = ok("deadletters", parse::<DeadLetters>(&read("deadletters.json"), ""));
        let timing = ok("timing", parse::<Timing>(&read("timing.json"), ""));
        let now = chrono::Utc::now();
        assert!(crate::monitoring::trace_view::build(&trace, now).is_some(), "the recorded trace has a root");
        let _ = crate::monitoring::fleet_view::queue(&overview, now);
        let _ = crate::monitoring::fleet_view::stuck(&overview, now);
        let _ = crate::monitoring::fleet_view::references(&overview, now);
        let _ = crate::monitoring::fleet_view::dead_letters(&dead, now);
        let _ = crate::monitoring::fleet_view::timing(&timing, now);
        let _ = crate::monitoring::fleet_view::home(&overview, now);
        eprintln!(
            "parsed: {} parked, {} extracting, {} dead letters, {} timing docs",
            overview.parked.len(), overview.stuck.len(), dead.messages.len(), timing.docs.len()
        );
    }

    #[test]
    fn an_auth_answer_stays_an_auth_state() {
        let out = r#"{"status":"auth","message":"az login"}"#;
        assert_eq!(
            parse::<Overview>(out, ""),
            Probe::Auth { message: "az login".into() }
        );
    }

    #[test]
    fn no_json_surfaces_stderr_rather_than_nothing() {
        match parse::<Overview>("", "Traceback\nImportError: azure") {
            Probe::Error { message } => assert!(message.contains("ImportError")),
            other => panic!("expected error, got {other:?}"),
        }
    }

    fn scratch_home(name: &str) -> PathBuf {
        let home = std::env::temp_dir().join(format!("dev-monitor-probe-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&home);
        std::fs::create_dir_all(home.join(".azure")).unwrap();
        home
    }

    #[test]
    fn the_environment_s_own_login_is_used() {
        let home = scratch_home("found");
        std::fs::create_dir_all(home.join(".azure/sbm-DEV")).unwrap();
        let dir = azure_config_dir("dev", &home).expect("sbm-DEV is used");
        assert!(dir.ends_with("sbm-DEV") || dir.ends_with("sbm-dev"), "{dir:?}");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn an_environment_without_its_own_login_changes_nothing() {
        let home = scratch_home("absent");
        std::fs::create_dir_all(home.join(".azure/sbm-DEV")).unwrap();
        assert_eq!(azure_config_dir("stg", &home), None);
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn query_is_quoted_so_it_cannot_break_out_of_the_shell() {
        let line = command_line("dev", &Request::Trace("a'; rm -rf /".into()));
        assert!(line.ends_with(r#" 'trace' 'a'\''; rm -rf /'"#), "{line}");
    }
}
