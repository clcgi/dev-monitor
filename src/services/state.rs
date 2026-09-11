use std::collections::{BTreeMap, HashMap};

use chrono::{DateTime, Local};

use crate::services::marker_syntax::MarkerSyntax;
use crate::services::steps::{StepCatalog, StepId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Environment {
    Sandbox,
    Dev,
    Stg,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Sandbox => "sandbox",
            Environment::Dev => "dev",
            Environment::Stg => "stg",
        }
    }
}

impl std::fmt::Display for Environment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self {
            Environment::Sandbox => "Sandbox",
            Environment::Dev => "Development",
            Environment::Stg => "Staging",
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum StreamType {
    Stdout,
    Stderr,
    System,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct LogMsg {
    pub timestamp: DateTime<Local>,
    pub stream: StreamType,
    pub content: String,
}

#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub enum ScriptStatus {
    #[default]
    Idle,
    Running,
    Succeeded,
    Failed(i32),
    Cancelled,
    AppError(String),
}

#[derive(Clone, PartialEq, Debug)]
pub struct HistoryEntry {
    pub script_name: String,
    pub environment: Environment,
    pub start_time: DateTime<Local>,
    pub end_time: DateTime<Local>,
    pub status: ScriptStatus,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Verdict {
    pub label: String,
    pub ok: bool,
}

/// Everything about ONE script: what it is configured to run with, and what.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct ScriptState {
    pub enabled_args: Vec<String>,
    /// flag -> value, for the flags a script declares with `CDW_CHOICE`.
    /// Separate from `enabled_args` because a toggle is its own presence and
    /// a choice is not: `--case` without a value is not a shorter command, it
    /// is a script that refuses to start.
    pub chosen: HashMap<String, String>,
    pub status: ScriptStatus,
    pub logs: Vec<LogMsg>,
    pub verdicts: Vec<Verdict>,
    pub active_step: Option<StepId>,
    pub step_history: Vec<StepId>,
    pub step_started: Option<DateTime<Local>>,
    pub traces: BTreeMap<String, Vec<LogMsg>>,
    pub in_trace: Option<String>,
    pub start_time: Option<DateTime<Local>>,
    pub end_time: Option<DateTime<Local>>,
}

impl ScriptState {
    pub fn has_started(&self) -> bool {
        self.start_time.is_some()
    }

    /// The whole command line: the toggles, then each choice as two argv
    /// entries.
    ///
    /// TWO ENTRIES, never one joined by `=`. The corpus case ids contain
    /// spaces and dots (`001.17033.000001-AA001-10 - A.pdf`); the runner
    /// quotes each argv entry separately, so `--case` and its value survive
    /// as they were chosen, while a joined string would have to be quoted as
    /// one and argparse would see a flag it does not know.
    ///
    /// A CHOICE WITH NO VALUE IS OMITTED rather than sent empty: an empty
    /// `--case ''` reaches the script as a case id that matches nothing, and
    /// its error would name the empty string rather than the real problem.
    pub fn command_args(&self) -> Vec<String> {
        let mut args = self.enabled_args.clone();
        let mut chosen: Vec<(&String, &String)> = self.chosen.iter().collect();
        // A HashMap iterates in an arbitrary order, and an argv that changes
        // between runs of the same configuration is impossible to compare in
        // a log.
        chosen.sort();
        for (flag, value) in chosen {
            if !value.is_empty() {
                args.push(flag.clone());
                args.push(value.clone());
            }
        }
        args
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct AppState {
    pub selected_env: Option<Environment>,
    pub selected_script: Option<String>,
    pub selected_meta: Option<crate::services::scripts::ScriptMeta>,
    pub scripts: HashMap<String, ScriptState>,
    pub running_script: Option<String>,
    pub catalog: StepCatalog,
    pub syntax: MarkerSyntax,
    pub history: Vec<HistoryEntry>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            selected_env: None,
            selected_script: None,
            selected_meta: None,
            scripts: HashMap::new(),
            running_script: None,
            catalog: crate::services::step_config::load(),
            syntax: crate::services::step_config::load_syntax(),
            history: Vec::new(),
        }
    }

    /// The selected script's state, or a default view of one never run.
    pub fn current(&self) -> ScriptState {
        self.selected_script
            .as_ref()
            .and_then(|path| self.scripts.get(path))
            .cloned()
            .unwrap_or_default()
    }

    /// Mutable access to one script's state, creating it on first touch.
    pub fn entry(&mut self, path: &str) -> &mut ScriptState {
        self.scripts.entry(path.to_string()).or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn log(content: &str) -> LogMsg {
        LogMsg {
            timestamp: Local::now(),
            stream: StreamType::Stdout,
            content: content.to_string(),
        }
    }

    #[test]
    fn two_scripts_keep_their_own_logs() {
        let mut state = AppState::new();
        state.entry("tools/flow_1_park.py").logs.push(log("park output"));
        state.entry("tools/flow_2_promote.py").logs.push(log("promote output"));

        assert_eq!(state.scripts["tools/flow_1_park.py"].logs.len(), 1);
        assert_eq!(
            state.scripts["tools/flow_1_park.py"].logs[0].content,
            "park output"
        );
        assert_eq!(
            state.scripts["tools/flow_2_promote.py"].logs[0].content,
            "promote output"
        );
    }

    #[test]
    fn selecting_another_script_does_not_disturb_a_running_one() {
        let mut state = AppState::new();
        state.running_script = Some("tools/flow_3_extract.py".into());
        let running = state.entry("tools/flow_3_extract.py");
        running.status = ScriptStatus::Running;
        running.logs.push(log("still going"));
        running.step_history.push("raw".to_string());

        state.selected_script = Some("tools/verify_ingestion.py".into());

        let live = &state.scripts["tools/flow_3_extract.py"];
        assert_eq!(live.status, ScriptStatus::Running);
        assert_eq!(live.logs.len(), 1);
        assert_eq!(live.step_history, vec!["raw".to_string()]);
        assert_eq!(state.running_script.as_deref(), Some("tools/flow_3_extract.py"));
    }

    #[test]
    fn coming_back_to_a_script_finds_its_run_where_it_was_left() {
        let mut state = AppState::new();
        let e = state.entry("tools/flow_3_extract.py");
        e.logs.push(log("line one"));
        e.verdicts.push(Verdict { label: "flow_3_extract".into(), ok: true });
        e.active_step = Some("servicebus".to_string());

        state.selected_script = Some("tools/verify_ingestion.py".into());
        assert!(state.current().logs.is_empty(), "a script never run shows nothing");

        state.selected_script = Some("tools/flow_3_extract.py".into());
        let back = state.current();
        assert_eq!(back.logs.len(), 1);
        assert_eq!(back.verdicts.len(), 1);
        assert_eq!(back.active_step, Some("servicebus".to_string()));
    }

    #[test]
    fn a_script_never_run_reads_as_idle_rather_than_missing() {
        let mut state = AppState::new();
        state.entry("tools/flow_1_park.py").status = ScriptStatus::Failed(2);
        state.selected_script = Some("tools/flow_9_new.py".into());

        let current = state.current();
        assert_eq!(current.status, ScriptStatus::Idle);
        assert!(!current.has_started());
    }

    #[test]
    fn enabled_flags_are_remembered_per_script() {
        // `--apply` chosen for the reset tool must not follow the user to another.
        let mut state = AppState::new();
        state.entry("tools/reset_test_documents.py").enabled_args = vec!["--apply".into()];
        state.entry("tools/verify_ingestion.py").enabled_args = vec!["--json".into()];

        state.selected_script = Some("tools/reset_test_documents.py".into());
        assert_eq!(state.current().enabled_args, vec!["--apply".to_string()]);
        state.selected_script = Some("tools/verify_ingestion.py".into());
        assert_eq!(state.current().enabled_args, vec!["--json".to_string()]);
    }

    #[test]
    fn a_choice_reaches_the_command_line_as_two_argv_entries() {
        // The corpus case ids contain spaces. Joined with `=` they would have
        // to be quoted as one string, and argparse would see an unknown flag.
        let mut state = AppState::new();
        let e = state.entry("tools/flows/dev_corpus_e2e.py");
        e.enabled_args = vec!["--refresh".into()];
        e.chosen.insert("--case".into(), "001.17033.000001-AA001-10 - A.pdf".into());
        state.selected_script = Some("tools/flows/dev_corpus_e2e.py".into());

        assert_eq!(
            state.current().command_args(),
            vec![
                "--refresh".to_string(),
                "--case".to_string(),
                "001.17033.000001-AA001-10 - A.pdf".to_string(),
            ]
        );
    }

    #[test]
    fn an_unchosen_value_is_left_off_rather_than_sent_empty() {
        // `--case ''` reaches the script as a case id matching nothing, and
        // the error would name the empty string rather than the real problem.
        let mut state = AppState::new();
        state.entry("x.py").chosen.insert("--case".into(), String::new());
        state.selected_script = Some("x.py".into());
        assert!(state.current().command_args().is_empty());
    }

    #[test]
    fn a_chosen_case_is_remembered_per_script_like_the_toggles() {
        let mut state = AppState::new();
        state.entry("a.py").chosen.insert("--case".into(), "one".into());
        state.entry("b.py").chosen.insert("--case".into(), "two".into());
        state.selected_script = Some("a.py".into());
        assert_eq!(state.current().chosen["--case"], "one");
        state.selected_script = Some("b.py".into());
        assert_eq!(state.current().chosen["--case"], "two");
    }

    #[test]
    fn a_trace_block_files_its_lines_under_its_component() {
        let mut state = AppState::new();
        let e = state.entry("tools/flow_7.py");
        e.in_trace = Some("HttpUploadSmall".into());
        e.traces.entry("HttpUploadSmall".into()).or_default().push(log("boom"));
        e.in_trace = None;

        let run = &state.scripts["tools/flow_7.py"];
        assert_eq!(run.traces["HttpUploadSmall"].len(), 1);
        assert!(run.in_trace.is_none());
    }

    #[test]
    fn traces_belong_to_their_script_like_everything_else() {
        let mut state = AppState::new();
        state.entry("a.py").traces.entry("Fn".into()).or_default().push(log("a"));
        state.entry("b.py").traces.entry("Fn".into()).or_default().push(log("b"));
        assert_eq!(state.scripts["a.py"].traces["Fn"][0].content, "a");
        assert_eq!(state.scripts["b.py"].traces["Fn"][0].content, "b");
    }

    #[test]
    fn a_script_with_no_traces_reads_as_empty_rather_than_missing() {
        let state = AppState::new();
        assert!(state.current().traces.is_empty());
        assert!(state.current().in_trace.is_none());
    }

    #[test]
    fn components_keep_a_stable_order() {
        let mut state = AppState::new();
        let e = state.entry("x.py");
        for name in ["Zeta", "Alpha", "Middle"] {
            e.traces.entry(name.into()).or_default().push(log("x"));
        }
        let order: Vec<&str> = state.scripts["x.py"].traces.keys().map(|s| s.as_str()).collect();
        assert_eq!(order, ["Alpha", "Middle", "Zeta"]);
    }

    #[test]
    fn history_spans_every_script() {
        let mut state = AppState::new();
        for name in ["tools/a.py", "tools/b.py"] {
            state.history.push(HistoryEntry {
                script_name: name.into(),
                environment: Environment::Dev,
                start_time: Local::now(),
                end_time: Local::now(),
                status: ScriptStatus::Succeeded,
            });
        }
        assert_eq!(state.history.len(), 2);
    }
}
