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
    Quarantine,
    /// Copy one object out of a zone into a local directory, under `name` when it has one.
    Fetch { container: String, path: String, destination: String, name: String },
}

impl Request {
    fn args(&self) -> Vec<String> {
        match self {
            Request::Overview => vec!["overview".into()],
            Request::Trace(q) => vec!["trace".into(), q.clone()],
            Request::DeadLetters => vec!["deadletters".into()],
            Request::Timing => vec!["timing".into()],
            Request::Quarantine => vec!["quarantine".into()],
            Request::Fetch { container, path, destination, name } => {
                vec!["fetch".into(), container.clone(), path.clone(), destination.clone(), name.clone()]
            }
        }
    }
}

/// The workspace holding both `dev-monitor` and `CentralDocumentWarehouse`.
pub fn workspace_root() -> PathBuf {
    let cwd = std::env::current_dir()
        .and_then(|p| p.canonicalize())
        .map(without_verbatim_prefix)
        .unwrap_or_else(|_| PathBuf::from("."));
    if cwd.ends_with("dev-monitor") {
        cwd.parent().map(PathBuf::from).unwrap_or(cwd)
    } else {
        cwd
    }
}

/// Windows' canonicalize() returns `\\?\C:\...`, which bash and Python do not read as a path.
fn without_verbatim_prefix(path: PathBuf) -> PathBuf {
    match path.to_str().and_then(|p| p.strip_prefix(r"\\?\")) {
        Some(plain) => PathBuf::from(plain),
        None => path,
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

pub fn login_commands(env: &str) -> String {
    login_commands_for(env, dirs::home_dir().as_deref())
}

fn login_commands_for(env: &str, home: Option<&std::path::Path>) -> String {
    let mut lines = Vec::new();
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    if std::path::Path::new("/opt/homebrew/bin/az").is_file() {
        lines.push("export PATH='/opt/homebrew/bin':\"$PATH\"".to_string());
    }
    // A plain login is what the app reads first-class; the per-environment profile is
    // offered second, for whoever keeps one, and is never required.
    lines.push("az login".to_string());
    lines.push(format!("az account set --subscription <the {env} subscription>"));
    if let Some(dir) = home.and_then(|home| azure_config_dir(env, home)) {
        lines.push(format!("# or, to refresh your {env} profile instead:", env = env.to_uppercase()));
        lines.push(format!("AZURE_CONFIG_DIR={} az login", quote(&dir.to_string_lossy())));
    }
    lines.join("\n")
}

pub fn command_line(env: &str, request: &Request) -> String {
    command_line_for(env, request, dirs::home_dir().as_deref())
}

fn command_line_for(env: &str, request: &Request, home: Option<&std::path::Path>) -> String {
    // Relative: the probe runs from CentralDocumentWarehouse, so no absolute (or Windows-specific) path is needed.
    let probe = std::env::var_os(PROBE_OVERRIDE)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("tools/monitor_probe.py"));
    let args: String = request.args().iter().map(|a| format!(" {}", quote(a))).collect();
    // 00-variables prints progress on stdout, which must not reach the JSON reader — but its
    // stderr is kept: sourced, it exits the shell on a bad login or subscription, and silencing
    // that left "the probe exited without a result" as the only symptom.
    let attempt = format!(
        "source deploy/00-variables.sh >/dev/null && exec .venv/bin/python {}{}",
        quote(&probe.to_string_lossy()),
        args
    );
    let default_profile = format!("( {attempt} )");
    match home.and_then(|home| azure_config_dir(env, home)) {
        // The per-environment profile is a PREFERENCE, never a requirement: it is what
        // azsbm_login writes and this app cannot inherit, but a plain `az login` must keep
        // working. Each try is a subshell, because 00-variables is sourced and its exit
        // would otherwise end the whole attempt chain with it.
        Some(dir) => format!(
            "export CDW_ENV={}; ( export AZURE_CONFIG_DIR={}; {attempt} ) || {default_profile}",
            quote(env),
            quote(&dir.to_string_lossy()),
        ),
        None => format!("export CDW_ENV={}; {default_profile}", quote(env)),
    }
}

/// Which Azure CLI logins were tried, since every one of them failing reads as the
/// platform being broken rather than as a login to refresh.
fn profile_note(env: &str) -> String {
    let env_name = env.to_uppercase();
    match dirs::home_dir().and_then(|home| azure_config_dir(env, &home)) {
        Some(dir) => format!(
            " Tried the {} profile and then your default `az login`; neither could read {env_name}. \
             `az login` refreshes the default one.",
            dir.display()
        ),
        None => format!(" This used your default `az login`, whose active subscription must be {env_name}'s."),
    }
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
    let mut result = parse(&String::from_utf8_lossy(&output.stdout), &String::from_utf8_lossy(&output.stderr));
    // Nothing on stdout means the environment script exited before the probe ran, which is
    // almost always the profile it ran with rather than anything about the environment.
    if let Probe::Error { message } = &result {
        let expired = ["AADSTS", "az login", "refresh token"].iter().any(|needle| message.contains(needle));
        let note = format!("{message}{}", profile_note(env));
        result = if expired { Probe::Auth { message: note } } else { Probe::Error { message: note } };
    }
    let status = match &result {
        Probe::Ok { .. } => "ok",
        Probe::Auth { .. } => "auth",
        Probe::Unavailable { .. } => "unavailable",
        Probe::Error { .. } => "error",
    };
    // Diagnose the actual desktop subprocess without logging document data or credentials.
    eprintln!("Monitoring probe ({env}): {status}");
    result
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

/// Where a downloaded document is saved: the operator's own Downloads folder.
pub fn download_dir() -> Option<PathBuf> {
    dirs::download_dir().or_else(dirs::home_dir).filter(|d| d.is_dir())
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

    /// CDW_PROBE_SAMPLES=<dir with quarantine.json> cargo test -- --ignored quarantine
    #[test]
    #[ignore]
    fn a_recorded_quarantine_answer_parses_and_derives_its_rows() {
        use crate::monitoring::model::Quarantine;
        let dir = std::path::PathBuf::from(std::env::var("CDW_PROBE_SAMPLES").expect("CDW_PROBE_SAMPLES"));
        let recorded = std::fs::read_to_string(dir.join("quarantine.json")).expect("quarantine.json");
        let Probe::Ok { data } = parse::<Quarantine>(&recorded, "") else { panic!("not ok: {:?}", parse::<Quarantine>(&recorded, "")) };
        let (kpis, rows) = crate::monitoring::fleet_view::quarantine(&data, chrono::Utc::now());
        eprintln!("container {} · {} docs · {} objects · {} rows", data.container, data.docs.len(), data.blobs.len(), rows.len());
        for k in &kpis {
            eprintln!("  KPI {}: {} {}", k.label, k.value, k.unit);
        }
        for r in &rows {
            eprintln!("  {} | {} | moved {} | {} | {} | {}", r.key, r.reason, r.moved, r.held, r.size, r.address);
            eprintln!("     why: {}\n     get: {}", r.why, r.download);
        }
        assert!(rows.len() >= data.docs.len(), "every quarantined document has a row");
        assert!(rows.iter().all(|r| r.address.starts_with(&data.container)), "every row names where its bytes are");
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
    fn the_environment_script_keeps_its_stderr_so_a_failure_to_start_says_why() {
        let line = command_line("dev", &Request::Overview);
        assert!(line.contains("source deploy/00-variables.sh >/dev/null &&"), "{line}");
        assert!(!line.contains("2>&1"), "{line}");
    }

    #[test]
    fn a_plain_az_login_is_used_when_the_environment_has_no_profile_of_its_own() {
        let home = scratch_home("no-profile");
        let line = command_line_for("dev", &Request::Overview, Some(&home));
        assert!(!line.contains("AZURE_CONFIG_DIR"), "{line}");
        assert!(line.contains("export CDW_ENV='dev'; ( source deploy/00-variables.sh"), "{line}");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn an_environment_profile_is_preferred_but_a_plain_az_login_still_answers() {
        let home = scratch_home("prefer-then-fall-back");
        std::fs::create_dir_all(home.join(".azure/sbm-DEV")).unwrap();
        let line = command_line_for("dev", &Request::Overview, Some(&home));
        let profile = home.join(".azure/sbm-DEV").to_string_lossy().to_string();
        // The profile is tried first...
        assert!(line.contains(&format!("( export AZURE_CONFIG_DIR='{profile}'; source deploy")), "{line}");
        // ...and a stale one falls through to the default profile instead of failing the app.
        assert!(line.contains(") || ( source deploy/00-variables.sh"), "{line}");
        // Each try is its own subshell: a sourced exit must not take the fallback with it.
        assert_eq!(line.matches("source deploy/00-variables.sh").count(), 2, "{line}");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_failure_before_the_probe_ran_names_the_profile_it_used() {
        let note = profile_note("dev");
        assert!(note.contains("`az login`"), "{note}");
        assert!(note.contains("DEV"), "{note}");
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
    fn the_default_probe_is_run_by_its_path_relative_to_the_platform() {
        if std::env::var_os(PROBE_OVERRIDE).is_none() {
            assert!(command_line("dev", &Request::Overview).contains(" 'tools/monitor_probe.py' 'overview'"));
        }
    }

    #[test]
    fn the_login_help_offers_a_plain_login_first() {
        let home = scratch_home("login-help");
        std::fs::create_dir_all(home.join(".azure/sbm-DEV")).unwrap();
        let help = login_commands_for("dev", Some(&home));
        let plain = help.find("az login").expect("a plain login");
        assert!(plain < help.find("AZURE_CONFIG_DIR").expect("the profile alternative"), "{help}");
        let _ = std::fs::remove_dir_all(&home);
    }

    #[test]
    fn a_windows_verbatim_prefix_is_dropped() {
        assert_eq!(without_verbatim_prefix(PathBuf::from(r"\\?\C:\Work\CDW")), PathBuf::from(r"C:\Work\CDW"));
        assert_eq!(without_verbatim_prefix(PathBuf::from("/Users/x/CDW")), PathBuf::from("/Users/x/CDW"));
    }

    #[test]
    fn a_download_names_the_object_and_where_to_put_it() {
        let line = command_line("dev", &Request::Fetch {
            container: "quarantine".into(),
            path: "D1/v1/g1".into(),
            destination: "/Users/x/Downloads".into(),
            name: "inv mismatch.pdf".into(),
        });
        assert!(line.contains(" 'fetch' 'quarantine' 'D1/v1/g1' '/Users/x/Downloads' 'inv mismatch.pdf'"), "{line}");
    }

    #[test]
    fn query_is_quoted_so_it_cannot_break_out_of_the_shell() {
        let line = command_line("dev", &Request::Trace("a'; rm -rf /".into()));
        assert!(line.contains(r#" 'trace' 'a'\''; rm -rf /'"#), "{line}");
        // Quoted in every attempt, not only the first.
        assert_eq!(line.matches(r#"'a'\''; rm -rf /'"#).count(), line.matches("monitor_probe.py").count(), "{line}");
    }
}
