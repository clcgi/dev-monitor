use crate::services::marker_syntax::{MarkerKind, MarkerSyntax};
use crate::services::steps::{StepCatalog, StepId};

/// One understood marker.
#[derive(Clone, PartialEq, Debug)]
pub enum Marker {
    /// A step has STARTED.
    Step(StepId),
    /// A step has FINISHED. Distinct from `Step` because without it a slow step
    /// and a hung one are indistinguishable.
    StepDone(StepId),
    /// A new pass over the chain; the stepper starts over. simulate_upload.py
    /// runs the chain twice, and without this the stepper appears to jump
    /// backwards rather than restart.
    Run(String),
    /// A run's verdict about itself. Separate from the process exit code, which
    /// is one value for a suite of six flows.
    Result { ok: bool, label: String },
}

/// Parse one line of script output.
///
/// DONE IS TESTED BEFORE START. With the default tokens `CDW_STEP` is a prefix
/// of `CDW_STEP_DONE`, so the other order reads every completion as a start and
/// nothing ever finishes. That holds for any pair a user configures where one
/// token is a prefix of the other, which is why the order is fixed here rather
/// than left to the token list.
pub fn parse(line: &str, catalog: &StepCatalog, syntax: &MarkerSyntax) -> Option<Marker> {
    if let Some(raw) = syntax.payload(line, MarkerKind::StepFinished) {
        return catalog.resolve(raw).map(Marker::StepDone);
    }
    if let Some(raw) = syntax.payload(line, MarkerKind::StepStarted) {
        return catalog.resolve(raw).map(Marker::Step);
    }
    if let Some(raw) = syntax.payload(line, MarkerKind::Run) {
        return Some(Marker::Run(raw.to_string()));
    }
    if let Some(raw) = syntax.payload(line, MarkerKind::Result) {
        // Anything but a known verdict word is NOT a verdict. Guessing would
        // invent a result no script reported, and a guessed pass is the worse
        // of the two.
        let (verdict, label) = raw.split_once(char::is_whitespace).unwrap_or((raw, ""));
        let ok = match crate::services::steps::normalise(verdict).as_str() {
            "pass" | "passed" | "ok" | "success" => true,
            "fail" | "failed" | "error" => false,
            _ => return None,
        };
        return Some(Marker::Result { ok, label: label.trim().to_string() });
    }
    None
}

/// True when a line carries a verdict token but no readable verdict.
///
/// A STEP marker naming an unconfigured step is NOT reported: steps are
/// user-editable, so an unknown one means the user removed it.
pub fn is_unrecognised(line: &str, catalog: &StepCatalog, syntax: &MarkerSyntax) -> bool {
    syntax.payload(line, MarkerKind::Result).is_some() && parse(line, catalog, syntax).is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cat() -> StepCatalog {
        StepCatalog::defaults()
    }

    fn syn() -> MarkerSyntax {
        MarkerSyntax::default()
    }

    #[test]
    fn the_names_a_script_author_writes_are_accepted() {
        for (raw, id) in [
            ("Neo", "neo"),
            ("Apim", "apim"),
            ("EventGrid", "eventgrid"),
            ("ServiceBus", "servicebus"),
            ("ContainerAppJobs", "containerappjobs"),
        ] {
            assert_eq!(
                parse(&format!("[CDW_STEP: {raw}]"), &cat(), &syn()),
                Some(Marker::Step(id.to_string())),
                "{raw}"
            );
        }
    }

    #[test]
    fn the_spaced_names_the_ui_shows_are_also_accepted() {
        for (raw, id) in [
            ("NEO", "neo"),
            ("Event Grid", "eventgrid"),
            ("Service Bus", "servicebus"),
            ("Container App Jobs", "containerappjobs"),
        ] {
            assert_eq!(parse(&format!("[CDW_STEP: {raw}]"), &cat(), &syn()), Some(Marker::Step(id.into())));
        }
    }

    #[test]
    fn step_done_is_not_read_as_step_started() {
        // "[CDW_STEP:" is a prefix of "[CDW_STEP_DONE:". The wrong order makes
        // every completion register as a start and nothing ever finishes.
        assert_eq!(
            parse("[CDW_STEP_DONE: Raw]", &cat(), &syn()),
            Some(Marker::StepDone("raw".into()))
        );
    }

    #[test]
    fn a_marker_is_found_inside_an_ordinary_log_line() {
        assert_eq!(
            parse("  4. curated move [CDW_STEP: Curated] -- done", &cat(), &syn()),
            Some(Marker::Step("curated".into()))
        );
    }

    #[test]
    fn a_run_marker_carries_its_label() {
        assert_eq!(parse("[CDW_RUN: flow_1]", &cat(), &syn()), Some(Marker::Run("flow_1".into())));
    }

    #[test]
    fn a_verdict_carries_its_flow_name() {
        assert_eq!(
            parse("[CDW_RESULT: PASS flow_1_park]", &cat(), &syn()),
            Some(Marker::Result { ok: true, label: "flow_1_park".into() })
        );
        assert_eq!(
            parse("[CDW_RESULT: FAIL flow_3]", &cat(), &syn()),
            Some(Marker::Result { ok: false, label: "flow_3".into() })
        );
    }

    #[test]
    fn an_unknown_verdict_word_is_reported_rather_than_guessed() {
        assert_eq!(parse("[CDW_RESULT: MAYBE x]", &cat(), &syn()), None);
        assert!(is_unrecognised("[CDW_RESULT: MAYBE x]", &cat(), &syn()));
    }

    #[test]
    fn a_marker_for_a_removed_step_does_nothing_and_is_not_reported() {
        // Steps are user-editable, so an unknown step means the user removed
        // it -- not that a script has a typo. Decided 2026-09-02.
        let mut c = cat();
        c.remove("raw");
        assert_eq!(parse("[CDW_STEP: Raw]", &c, &syn()), None);
        assert!(!is_unrecognised("[CDW_STEP: Raw]", &c, &syn()));
    }

    #[test]
    fn a_user_added_alias_resolves() {
        let mut c = cat();
        c.steps[7].aliases.push("CAJ".into());
        assert_eq!(parse("[CDW_STEP: CAJ]", &c, &syn()), Some(Marker::Step("containerappjobs".into())));
    }

    #[test]
    fn an_unterminated_marker_is_not_a_marker() {
        assert_eq!(parse("[CDW_STEP: Raw", &cat(), &syn()), None);
    }
}
