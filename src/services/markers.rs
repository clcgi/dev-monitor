//! Parsing the `[CDW_...]` markers scripts print to stdout.
//!
//! WHY THIS IS A MODULE AND NOT THIRTEEN `contains` CALLS.
//!
//! The previous matcher compared against thirteen exact literals, five of which
//! no script author would ever type: `[CDW_STEP: NEO]`, `[CDW_STEP: APIM]`,
//! `[CDW_STEP: Event Grid]`, `[CDW_STEP: Service Bus]`,
//! `[CDW_STEP: Container App Jobs]`. Anyone writing `EventGrid` or `Apim` --
//! which is what the step names look like everywhere else -- produced a line
//! that matched nothing.
//!
//! **And the failure was silent.** The script printed, the stepper sat still,
//! and nothing anywhere reported that a marker had been seen and not
//! understood. That is the worst shape a contract between two programs can
//! have, so the matcher now NORMALISES instead: case is folded and every
//! non-alphanumeric character is dropped, which makes `NEO`, `Neo`,
//! `Event Grid`, `EventGrid` and `event_grid` all the same marker.
//!
//! Scripts stay dumb on purpose. A `print()` or an `echo` is the whole
//! integration; nothing here requires a library, a schema or a version.

use crate::services::state::WorkflowStep;

/// One understood marker.
#[derive(Clone, PartialEq, Debug)]
pub enum Marker {
    /// A step has STARTED.
    Step(WorkflowStep),
    /// A step has FINISHED. Distinct from `Step` because without it a slow step
    /// and a hung one are indistinguishable -- the monitor only ever learned
    /// that something began.
    StepDone(WorkflowStep),
    /// A new pass over the chain has begun; the stepper should start over.
    ///
    /// `simulate_upload.py` runs the whole chain twice to compare the two
    /// ingress routes. Without this the stepper walks forward to
    /// ContainerAppJobs and then appears to jump backwards to Apim, which reads
    /// as a fault rather than a second pass.
    Run(String),
    /// A run reached a VERDICT about itself.
    ///
    /// The exit code already carries pass/fail for a whole process, so this
    /// exists for what the exit code cannot express: `flow_all.py` runs six
    /// flows and the runner sees ONE code, leaving a single failure among them
    /// unattributable. The label names which flow the verdict belongs to.
    ///
    /// It also arrives WHEN THE VERDICT IS REACHED rather than when the process
    /// ends, so a suite that is half done shows three results instead of
    /// nothing.
    Result { ok: bool, label: String },
}

/// Fold a marker payload to its comparable form.
///
/// Lowercase and alphanumeric-only, so spacing and capitalisation stop being
/// part of the contract. `"Container App Jobs"`, `"ContainerAppJobs"` and
/// `"container-app-jobs"` are one name.
fn normalise(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// The payload of `[PREFIX: payload]`, if the line carries one.
fn payload<'a>(line: &'a str, prefix: &str) -> Option<&'a str> {
    let start = line.find(prefix)? + prefix.len();
    let rest = &line[start..];
    let end = rest.find(']')?;
    Some(rest[..end].trim())
}

impl WorkflowStep {
    /// Every step, in the order the pipeline visits them.
    ///
    /// The two exception zones are deliberately absent: they are branches off
    /// the chain, not positions along it.
    pub const LINEAR: [WorkflowStep; 11] = [
        WorkflowStep::Neo,
        WorkflowStep::Authentication,
        WorkflowStep::Apim,
        WorkflowStep::Landing,
        WorkflowStep::EventGrid,
        WorkflowStep::Raw,
        WorkflowStep::ServiceBus,
        WorkflowStep::ContainerAppJobs,
        WorkflowStep::Processing,
        WorkflowStep::Curated,
        WorkflowStep::Verification,
    ];

    /// Resolve a marker payload against the step names, ignoring case and
    /// spacing. Returns None for anything unrecognised -- see
    /// `parse_or_warn` for why that is reported rather than swallowed.
    pub fn from_marker(raw: &str) -> Option<Self> {
        let wanted = normalise(raw);
        let all = [
            WorkflowStep::Neo,
            WorkflowStep::Authentication,
            WorkflowStep::Apim,
            WorkflowStep::Landing,
            WorkflowStep::EventGrid,
            WorkflowStep::Raw,
            WorkflowStep::ServiceBus,
            WorkflowStep::ContainerAppJobs,
            WorkflowStep::Processing,
            WorkflowStep::Curated,
            WorkflowStep::Verification,
            WorkflowStep::Quarantine,
            WorkflowStep::Rejected,
        ];
        all.into_iter().find(|s| normalise(s.name()) == wanted)
    }

    /// Position along the linear chain, or None for an exception zone.
    pub fn linear_index(&self) -> Option<usize> {
        Self::LINEAR.iter().position(|s| s == self)
    }
}

/// Parse one line of script output.
///
/// `CDW_STEP_DONE` is tested BEFORE `CDW_STEP`, because the latter is a prefix
/// of the former and checking in the other order would read every completion as
/// a start.
pub fn parse(line: &str) -> Option<Marker> {
    if let Some(raw) = payload(line, "[CDW_STEP_DONE:") {
        return WorkflowStep::from_marker(raw).map(Marker::StepDone);
    }
    if let Some(raw) = payload(line, "[CDW_STEP:") {
        return WorkflowStep::from_marker(raw).map(Marker::Step);
    }
    if let Some(raw) = payload(line, "[CDW_RUN:") {
        return Some(Marker::Run(raw.to_string()));
    }
    if let Some(raw) = payload(line, "[CDW_RESULT:") {
        // `PASS <label>` / `FAIL <label>`. The verdict word is normalised like
        // every other payload, so `pass`, `PASS` and `Passed` are one value --
        // and ANYTHING ELSE IS NOT A VERDICT. Treating an unknown word as a
        // failure would invent a result no script reported; treating it as a
        // success would be worse. It falls through to `is_unrecognised`.
        let (verdict, label) = raw.split_once(char::is_whitespace).unwrap_or((raw, ""));
        let ok = match normalise(verdict).as_str() {
            "pass" | "passed" | "ok" | "success" => true,
            "fail" | "failed" | "error" => false,
            _ => return None,
        };
        return Some(Marker::Result { ok, label: label.trim().to_string() });
    }
    None
}

/// True when a line LOOKS like a marker but names nothing known.
///
/// Exists so an unrecognised step can be surfaced instead of ignored. A typo in
/// a script is otherwise indistinguishable from a step that never ran.
pub fn is_unrecognised(line: &str) -> bool {
    (line.contains("[CDW_STEP:")
        || line.contains("[CDW_STEP_DONE:")
        || line.contains("[CDW_RESULT:"))
        && parse(line).is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_names_the_brief_specifies_are_accepted() {
        // These are what a script author writes. Every one of them matched
        // NOTHING before this module existed.
        for (raw, expected) in [
            ("Neo", WorkflowStep::Neo),
            ("Apim", WorkflowStep::Apim),
            ("EventGrid", WorkflowStep::EventGrid),
            ("ServiceBus", WorkflowStep::ServiceBus),
            ("ContainerAppJobs", WorkflowStep::ContainerAppJobs),
        ] {
            assert_eq!(
                parse(&format!("[CDW_STEP: {raw}]")),
                Some(Marker::Step(expected)),
                "{raw} should resolve"
            );
        }
    }

    #[test]
    fn the_spaced_names_the_ui_uses_are_still_accepted() {
        // The old literals must keep working: scripts may already emit them.
        for (raw, expected) in [
            ("NEO", WorkflowStep::Neo),
            ("APIM", WorkflowStep::Apim),
            ("Event Grid", WorkflowStep::EventGrid),
            ("Service Bus", WorkflowStep::ServiceBus),
            ("Container App Jobs", WorkflowStep::ContainerAppJobs),
        ] {
            assert_eq!(parse(&format!("[CDW_STEP: {raw}]")), Some(Marker::Step(expected)));
        }
    }

    #[test]
    fn case_and_separators_are_not_part_of_the_contract() {
        for raw in ["container app jobs", "CONTAINERAPPJOBS", "container-app-jobs", "Container_App_Jobs"] {
            assert_eq!(
                parse(&format!("[CDW_STEP: {raw}]")),
                Some(Marker::Step(WorkflowStep::ContainerAppJobs)),
                "{raw}"
            );
        }
    }

    #[test]
    fn a_marker_is_found_inside_an_ordinary_log_line() {
        // Scripts print markers alongside their normal output; the line is not
        // reserved for the marker.
        assert_eq!(
            parse("  4. curated move [CDW_STEP: Curated] -- INVENTED naming"),
            Some(Marker::Step(WorkflowStep::Curated))
        );
    }

    #[test]
    fn step_done_is_not_read_as_step_started() {
        // "[CDW_STEP:" is a prefix of "[CDW_STEP_DONE:". Checked in the wrong
        // order, every completion registers as a start and nothing ever finishes.
        assert_eq!(
            parse("[CDW_STEP_DONE: Raw]"),
            Some(Marker::StepDone(WorkflowStep::Raw))
        );
    }

    #[test]
    fn a_run_marker_carries_its_label() {
        assert_eq!(
            parse("[CDW_RUN: uploadSmall]"),
            Some(Marker::Run("uploadSmall".to_string()))
        );
    }

    #[test]
    fn the_exception_zones_resolve_but_are_not_on_the_linear_chain() {
        for zone in [WorkflowStep::Quarantine, WorkflowStep::Rejected] {
            assert_eq!(parse(&format!("[CDW_STEP: {}]", zone.name())), Some(Marker::Step(zone.clone())));
            assert_eq!(zone.linear_index(), None, "{} is a branch, not a position", zone.name());
        }
    }

    #[test]
    fn the_linear_chain_is_ordered_and_complete() {
        assert_eq!(WorkflowStep::LINEAR[0], WorkflowStep::Neo);
        assert_eq!(WorkflowStep::LINEAR[10], WorkflowStep::Verification);
        assert_eq!(WorkflowStep::Apim.linear_index(), Some(2));
    }

    #[test]
    fn an_unknown_step_is_reported_rather_than_ignored() {
        // A typo in a script must not look like a step that never ran.
        assert!(parse("[CDW_STEP: Sausages]").is_none());
        assert!(is_unrecognised("[CDW_STEP: Sausages]"));
        assert!(!is_unrecognised("ordinary output"));
        assert!(!is_unrecognised("[CDW_STEP: Raw]"));
    }

    #[test]
    fn a_verdict_carries_its_flow_name() {
        // The label is the whole point: flow_all runs six flows through one
        // exit code, so a verdict without a name attributes nothing.
        assert_eq!(
            parse("[CDW_RESULT: PASS flow_1_park]"),
            Some(Marker::Result { ok: true, label: "flow_1_park".to_string() })
        );
        assert_eq!(
            parse("[CDW_RESULT: FAIL flow_3_extract]"),
            Some(Marker::Result { ok: false, label: "flow_3_extract".to_string() })
        );
    }

    #[test]
    fn the_verdict_word_is_case_insensitive_like_every_other_payload() {
        for raw in ["pass", "PASS", "Passed", "ok", "success"] {
            assert_eq!(
                parse(&format!("[CDW_RESULT: {raw} x]")),
                Some(Marker::Result { ok: true, label: "x".to_string() }),
                "{raw}"
            );
        }
        for raw in ["fail", "FAILED", "error"] {
            assert_eq!(
                parse(&format!("[CDW_RESULT: {raw} x]")),
                Some(Marker::Result { ok: false, label: "x".to_string() }),
                "{raw}"
            );
        }
    }

    #[test]
    fn an_unknown_verdict_word_is_reported_rather_than_guessed() {
        // Guessing either way invents a result no script reported, and "maybe"
        // silently recorded as a pass is the worse of the two.
        assert_eq!(parse("[CDW_RESULT: MAYBE flow_1]"), None);
        assert!(is_unrecognised("[CDW_RESULT: MAYBE flow_1]"));
    }

    #[test]
    fn a_verdict_with_no_label_still_parses() {
        // A script may report only its own outcome. Losing the verdict because
        // it carried no name would be worse than an unnamed one.
        assert_eq!(
            parse("[CDW_RESULT: PASS]"),
            Some(Marker::Result { ok: true, label: String::new() })
        );
    }

    #[test]
    fn an_unterminated_marker_is_not_a_marker() {
        assert_eq!(parse("[CDW_STEP: Raw"), None);
    }
}
