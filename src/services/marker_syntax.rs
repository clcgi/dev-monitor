//! The markers the app looks for in script output.
//!
//! A FLAT, EDITABLE LIST. Any marker can be deleted and new ones added; the six
//! defaults are only what a fresh install starts with, not a privileged set.
//!
//! What a marker cannot be is arbitrary: the app has a fixed set of things it
//! knows how to do with one, so a marker is a TOKEN plus the KIND of behaviour
//! it triggers. A token with no kind would parse and then do nothing.

use serde::{Deserialize, Serialize};

/// What the app does when it sees a marker.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum MarkerKind {
    /// Lights a stage: `[TOKEN: Raw]`.
    StepStarted,
    /// Marks it complete. Separate from the start, because without it a slow
    /// stage and a hung one look the same.
    StepFinished,
    /// A new pass; the stepper restarts rather than appearing to jump back.
    Run,
    /// A verdict: `[TOKEN: PASS flow_1]`.
    Result,
    /// A header line in a script: `# TOKEN: category=...`.
    ScriptHeader,
    /// A header line declaring one flag: `# TOKEN: --apply  help`.
    ArgHeader,
}

impl MarkerKind {
    pub const ALL: [MarkerKind; 6] = [
        MarkerKind::StepStarted,
        MarkerKind::StepFinished,
        MarkerKind::Run,
        MarkerKind::Result,
        MarkerKind::ScriptHeader,
        MarkerKind::ArgHeader,
    ];

    pub fn label(&self) -> &'static str {
        match self {
            Self::StepStarted => "Step started",
            Self::StepFinished => "Step finished",
            Self::Run => "Run",
            Self::Result => "Result",
            Self::ScriptHeader => "Script header",
            Self::ArgHeader => "Argument header",
        }
    }

    pub fn example(&self) -> &'static str {
        match self {
            Self::StepStarted | Self::StepFinished => "[TOKEN: Raw]",
            Self::Run => "[TOKEN: flow_1]",
            Self::Result => "[TOKEN: PASS flow_1]",
            Self::ScriptHeader => "# TOKEN: category=Flows; steps=...",
            Self::ArgHeader => "# TOKEN: --apply  help text",
        }
    }

    /// Headers run to the end of the line; the rest are bracketed.
    pub fn is_header(&self) -> bool {
        matches!(self, Self::ScriptHeader | Self::ArgHeader)
    }
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct MarkerDef {
    /// Without brackets or the colon -- `CDW_STEP`, not `[CDW_STEP:`.
    pub token: String,
    pub kind: MarkerKind,
}

#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct MarkerSyntax {
    pub markers: Vec<MarkerDef>,
}

impl Default for MarkerSyntax {
    fn default() -> Self {
        let m = |token: &str, kind: MarkerKind| MarkerDef { token: token.to_string(), kind };
        Self {
            markers: vec![
                m("CDW_STEP", MarkerKind::StepStarted),
                m("CDW_STEP_DONE", MarkerKind::StepFinished),
                m("CDW_RUN", MarkerKind::Run),
                m("CDW_RESULT", MarkerKind::Result),
                m("CDW_SCRIPT", MarkerKind::ScriptHeader),
                m("CDW_ARG", MarkerKind::ArgHeader),
            ],
        }
    }
}

impl MarkerSyntax {
    /// The payload of the first marker of `kind` that `line` carries.
    pub fn payload<'a>(&self, line: &'a str, kind: MarkerKind) -> Option<&'a str> {
        for def in self.markers.iter().filter(|d| d.kind == kind) {
            if def.token.is_empty() {
                continue;
            }
            if kind.is_header() {
                if let Some((_, rest)) = line.split_once(&format!("{}:", def.token)) {
                    return Some(rest);
                }
            } else {
                let open = format!("[{}:", def.token);
                if let Some(start) = line.find(&open) {
                    let rest = &line[start + open.len()..];
                    if let Some(end) = rest.find(']') {
                        return Some(rest[..end].trim());
                    }
                }
            }
        }
        None
    }

    /// Strip what a user is likely to paste. Someone copying a marker out of a
    /// log types `[CDW_STEP:`, and a token stored that way can never match.
    pub fn clean_token(raw: &str) -> String {
        raw.trim()
            .trim_start_matches('[')
            .trim_end_matches(']')
            .trim_end_matches(':')
            .trim()
            .to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_the_markers_the_app_shipped_with() {
        let s = MarkerSyntax::default();
        assert_eq!(s.markers.len(), 6);
        let tokens: Vec<&str> = s.markers.iter().map(|m| m.token.as_str()).collect();
        assert_eq!(
            tokens,
            ["CDW_STEP", "CDW_STEP_DONE", "CDW_RUN", "CDW_RESULT", "CDW_SCRIPT", "CDW_ARG"]
        );
    }

    #[test]
    fn a_payload_is_found_inside_an_ordinary_log_line() {
        let s = MarkerSyntax::default();
        assert_eq!(s.payload("  4. done [CDW_STEP: Raw] ok", MarkerKind::StepStarted), Some("Raw"));
    }

    #[test]
    fn a_marker_can_be_removed() {
        // The point of the whole tab: any marker, including a default.
        let mut s = MarkerSyntax::default();
        s.markers.retain(|m| m.token != "CDW_STEP");
        assert_eq!(s.payload("[CDW_STEP: Raw]", MarkerKind::StepStarted), None);
        assert_eq!(s.markers.len(), 5);
    }

    #[test]
    fn a_marker_can_be_added() {
        let mut s = MarkerSyntax::default();
        s.markers.push(MarkerDef { token: "STAGE".into(), kind: MarkerKind::StepStarted });
        assert_eq!(s.payload("[STAGE: Raw]", MarkerKind::StepStarted), Some("Raw"));
        // And the original still works: adding must not displace.
        assert_eq!(s.payload("[CDW_STEP: Raw]", MarkerKind::StepStarted), Some("Raw"));
    }

    #[test]
    fn every_default_marker_can_be_deleted_leaving_nothing() {
        let mut s = MarkerSyntax::default();
        s.markers.clear();
        for kind in MarkerKind::ALL {
            assert_eq!(s.payload("[CDW_STEP: Raw]", kind), None);
        }
    }

    #[test]
    fn a_header_runs_to_the_end_of_the_line() {
        let s = MarkerSyntax::default();
        assert_eq!(
            s.payload("# CDW_SCRIPT: category=Flows", MarkerKind::ScriptHeader),
            Some(" category=Flows")
        );
    }

    #[test]
    fn an_unterminated_bracket_marker_is_not_a_marker() {
        let s = MarkerSyntax::default();
        assert_eq!(s.payload("[CDW_STEP: Raw", MarkerKind::StepStarted), None);
    }

    #[test]
    fn an_empty_token_matches_nothing_rather_than_everything() {
        // A half-typed row must not turn every line into a marker.
        let s = MarkerSyntax { markers: vec![MarkerDef { token: String::new(), kind: MarkerKind::StepStarted }] };
        assert_eq!(s.payload("[CDW_STEP: Raw]", MarkerKind::StepStarted), None);
        assert_eq!(s.payload("anything at all", MarkerKind::StepStarted), None);
    }

    #[test]
    fn a_pasted_marker_is_cleaned_to_its_token() {
        for raw in ["[CDW_STEP:", "CDW_STEP:", " [CDW_STEP: ", "CDW_STEP"] {
            assert_eq!(MarkerSyntax::clean_token(raw), "CDW_STEP", "{raw}");
        }
    }
}
