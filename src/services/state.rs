use std::collections::HashMap;

use chrono::{DateTime, Local};
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
pub enum WorkflowStep {
    Neo,
    Authentication,
    Apim,
    Landing,
    EventGrid,
    Raw,
    ServiceBus,
    ContainerAppJobs,
    Processing,
    Curated,
    Verification,
    Quarantine,
    Rejected,
}

impl WorkflowStep {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Neo => "NEO",
            Self::Authentication => "Authentication",
            Self::Apim => "APIM",
            Self::Landing => "Landing",
            Self::EventGrid => "Event Grid",
            Self::Raw => "Raw",
            Self::ServiceBus => "Service Bus",
            Self::ContainerAppJobs => "Container App Jobs",
            Self::Processing => "Processing",
            Self::Curated => "Curated",
            Self::Verification => "Verification",
            Self::Quarantine => "Quarantine",
            Self::Rejected => "Rejected",
        }
    }
}

/// One verdict a run reported about itself, from a `[CDW_RESULT: ...]` marker.
///
/// SEPARATE FROM `ScriptStatus`, which is derived from the process exit code.
/// The two answer different questions and can legitimately disagree: a suite
/// script exits non-zero because one of six flows failed, and the five that
/// passed are still results worth showing. The exit code is the runner's view;
/// these are the script's own.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Verdict {
    /// The flow that reported it. May be empty: a script that reports only its
    /// own outcome is still reporting something.
    pub label: String,
    pub ok: bool,
}

/// Everything about ONE script: what it is configured to run with, and what the
/// last run of it did.
///
/// WHY THIS IS PER SCRIPT AND NOT ONE SET OF FIELDS ON `AppState`. It used to be
/// global, and selecting another script wiped it -- so clicking away from a
/// running flow and back lost its logs, its stepper and its verdicts. Worse and
/// less obvious: the process kept running and kept appending, so its output
/// landed in whatever script was selected by then. Two scripts' runs ended up
/// interleaved in one log with nothing saying so.
///
/// Keyed by the script's path, so a run and the view of it cannot drift apart.
#[derive(Clone, PartialEq, Debug, Default)]
pub struct ScriptState {
    /// Flags switched on for the next run. Per script for the same reason as
    /// the rest: `--apply` chosen for the reset tool must not follow the user
    /// to another script, and must still be there when they come back.
    pub enabled_args: Vec<String>,
    pub status: ScriptStatus,
    pub logs: Vec<LogMsg>,
    pub verdicts: Vec<Verdict>,
    pub active_step: Option<WorkflowStep>,
    pub step_history: Vec<WorkflowStep>,
    pub step_started: Option<DateTime<Local>>,
    pub start_time: Option<DateTime<Local>>,
    pub end_time: Option<DateTime<Local>>,
}

impl ScriptState {
    /// True once a run has begun, so the view can show a stepper and timings.
    pub fn has_started(&self) -> bool {
        self.start_time.is_some()
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct AppState {
    pub selected_env: Option<Environment>,
    pub selected_script: Option<String>,
    pub selected_meta: Option<crate::services::scripts::ScriptMeta>,
    /// One entry per script the user has touched, keyed by path.
    pub scripts: HashMap<String, ScriptState>,
    /// The script currently executing, if any.
    ///
    /// Held here rather than derived from the statuses so the sidebar and the
    /// process loop cannot disagree, and so "something is running" survives the
    /// user selecting a different script.
    pub running_script: Option<String>,
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
            history: Vec::new(),
        }
    }

    /// The selected script's state, or a default view of one never run.
    ///
    /// Returns an owned value rather than a reference so a caller can hold it
    /// across a render without keeping the signal borrowed.
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
        // THE BUG THIS EXISTS FOR. Run state was global, so a running script
        // kept appending after the user selected another one -- and its output
        // landed in that other script's view, with nothing saying so.
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
        // Selecting is a change of VIEW. It used to clear the logs, the stepper
        // and the verdicts of a run that was still in progress.
        let mut state = AppState::new();
        state.running_script = Some("tools/flow_3_extract.py".into());
        let running = state.entry("tools/flow_3_extract.py");
        running.status = ScriptStatus::Running;
        running.logs.push(log("still going"));
        running.step_history.push(WorkflowStep::Raw);

        state.selected_script = Some("tools/verify_ingestion.py".into());

        let live = &state.scripts["tools/flow_3_extract.py"];
        assert_eq!(live.status, ScriptStatus::Running);
        assert_eq!(live.logs.len(), 1);
        assert_eq!(live.step_history, vec![WorkflowStep::Raw]);
        assert_eq!(state.running_script.as_deref(), Some("tools/flow_3_extract.py"));
    }

    #[test]
    fn coming_back_to_a_script_finds_its_run_where_it_was_left() {
        let mut state = AppState::new();
        let e = state.entry("tools/flow_3_extract.py");
        e.logs.push(log("line one"));
        e.verdicts.push(Verdict { label: "flow_3_extract".into(), ok: true });
        e.active_step = Some(WorkflowStep::ServiceBus);

        state.selected_script = Some("tools/verify_ingestion.py".into());
        assert!(state.current().logs.is_empty(), "a script never run shows nothing");

        state.selected_script = Some("tools/flow_3_extract.py".into());
        let back = state.current();
        assert_eq!(back.logs.len(), 1);
        assert_eq!(back.verdicts.len(), 1);
        assert_eq!(back.active_step, Some(WorkflowStep::ServiceBus));
    }

    #[test]
    fn a_script_never_run_reads_as_idle_rather_than_missing() {
        // The view must render for a script with no history at all, and it must
        // not inherit the previous selection's status.
        let mut state = AppState::new();
        state.entry("tools/flow_1_park.py").status = ScriptStatus::Failed(2);
        state.selected_script = Some("tools/flow_9_new.py".into());

        let current = state.current();
        assert_eq!(current.status, ScriptStatus::Idle);
        assert!(!current.has_started());
    }

    #[test]
    fn enabled_flags_are_remembered_per_script() {
        // `--apply` chosen for the reset tool must not follow the user to
        // another script, and must still be there when they come back.
        let mut state = AppState::new();
        state.entry("tools/reset_test_documents.py").enabled_args = vec!["--apply".into()];
        state.entry("tools/verify_ingestion.py").enabled_args = vec!["--json".into()];

        state.selected_script = Some("tools/reset_test_documents.py".into());
        assert_eq!(state.current().enabled_args, vec!["--apply".to_string()]);
        state.selected_script = Some("tools/verify_ingestion.py".into());
        assert_eq!(state.current().enabled_args, vec!["--json".to_string()]);
    }

    #[test]
    fn history_spans_every_script() {
        // History is the one thing that IS global: it is a record across runs,
        // which is why it moved out of the per-run view.
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
