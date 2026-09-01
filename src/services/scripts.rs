//! What the app knows about each script, and where it learns it.
//!
//! THE DECLARATION LIVES IN THE SCRIPT, not in a manifest here. A list in this
//! file would be a second copy of something the script already knows, and the
//! copy rots silently: a renamed flag keeps working from the terminal and
//! quietly stops working from the app, with the button still offering it.
//!
//! The format is a comment, so it is inert to both Python and bash and the
//! standalone `python tools/x.py` path is untouched -- the same constraint the
//! `[CDW_STEP: ...]` markers were written under:
//!
//!     # CDW_SCRIPT: category=Flows; steps=Neo,Authentication,Apim,Landing
//!     # CDW_ARG: --apply  Actually delete. Without it this is a dry run.
//!
//! EVERY FIELD IS OPTIONAL and a script with no header still runs. That is not
//! politeness: `tools/` holds fifteen scripts written before this existed, and
//! a reader that hid or broke them would be worse than one that shows them
//! plainly. Absent `steps` means "unknown", never "none" -- see `steps()`.

use std::fs;
use std::path::Path;

use crate::services::state::WorkflowStep;

/// A flag a script accepts, offered in the UI as a toggle.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ScriptArg {
    pub flag: String,
    pub help: String,
    /// Pre-selected when the script is chosen. For flags whose ABSENCE is the
    /// dangerous state -- there are none today, and the default is off because
    /// the reverse mistake (a destructive flag on by accident) is worse.
    pub default_on: bool,
}

/// What a script says about the pipeline stages it can reach.
///
/// THREE STATES, because "I touch nothing" and "I have not said" are different
/// claims and only one of them is safe to draw. `reset_test_documents.py`
/// reaches no stage at all and must show no stepper; a script that simply has
/// not been annotated has made no such claim, and rendering an empty track for
/// it would put words in its mouth.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum DeclaredSteps {
    /// No declaration. The caller shows the whole chain.
    Unknown,
    /// `steps=none` -- explicitly touches no pipeline stage. Show no stepper.
    None,
    /// The stages named, resolved. Never empty.
    Only(Vec<WorkflowStep>),
}

/// One script, as the app understands it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ScriptMeta {
    /// Repo-relative, e.g. `tools/flow_1_park.py`.
    pub path: String,
    /// Sidebar group. `Other` when the script does not say.
    pub category: String,
    /// Stages the script can reach.
    pub declared_steps: DeclaredSteps,
    pub args: Vec<ScriptArg>,
    /// A one-line summary, taken from the header when given.
    pub summary: String,
    /// Not offered for running. Libraries live in `tools/` beside the scripts
    /// and are indistinguishable by extension; launching one does nothing,
    /// which reads as a broken script rather than a category error.
    pub library: bool,
}

impl ScriptMeta {
    pub fn file_name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }

    /// The stages to draw, or `None` to draw the whole chain.
    pub fn steps(&self) -> Option<&[WorkflowStep]> {
        match &self.declared_steps {
            DeclaredSteps::Unknown => None,
            DeclaredSteps::None => Some(&[]),
            DeclaredSteps::Only(steps) => Some(steps),
        }
    }

    /// True when the script says it reaches no pipeline stage, so the stepper
    /// should not be drawn at all.
    pub fn has_no_steps(&self) -> bool {
        self.declared_steps == DeclaredSteps::None
    }

    /// `py` or `sh`. Used by the language filter, which is a second axis over
    /// the same list rather than a second grouping -- a script has one category
    /// and one language, and nesting them would make four of the groups empty
    /// most of the time.
    pub fn language(&self) -> &'static str {
        if self.path.ends_with(".py") { "py" } else { "sh" }
    }
}

/// Scripts a script author would never want offered as runnable.
///
/// Named here rather than in each file because these predate the header and
/// are not ours to churn. A `library=true` field in the header overrides.
const KNOWN_LIBRARIES: [&str; 1] = ["cdw_client.py"];

/// How many lines from the top to scan for the header.
///
/// Bounded so a large script is not read in full on every refresh, and so a
/// `CDW_SCRIPT:` string appearing in a docstring far below cannot be picked up
/// as a declaration.
const HEADER_LINES: usize = 60;

fn parse_pairs(payload: &str) -> Vec<(String, String)> {
    payload
        .split(';')
        .filter_map(|part| part.split_once('='))
        .map(|(k, v)| (k.trim().to_lowercase(), v.trim().to_string()))
        .collect()
}

/// Read one script's declaration. Never fails: an unreadable or undeclared
/// script yields defaults.
pub fn parse_meta(path: &Path, repo_relative: &str) -> ScriptMeta {
    let mut meta = ScriptMeta {
        path: repo_relative.to_string(),
        category: "Other".to_string(),
        declared_steps: DeclaredSteps::Unknown,
        args: Vec::new(),
        summary: String::new(),
        library: KNOWN_LIBRARIES.contains(&repo_relative.rsplit('/').next().unwrap_or("")),
    };

    let Ok(text) = fs::read_to_string(path) else {
        return meta;
    };

    for line in text.lines().take(HEADER_LINES) {
        if let Some(payload) = line.split_once("CDW_SCRIPT:").map(|(_, p)| p) {
            for (key, value) in parse_pairs(payload) {
                match key.as_str() {
                    "category" => meta.category = value,
                    "summary" => meta.summary = value,
                    "library" => meta.library = value.eq_ignore_ascii_case("true"),
                    "steps" => {
                        meta.declared_steps = if value.eq_ignore_ascii_case("none") {
                            DeclaredSteps::None
                        } else {
                            let resolved: Vec<WorkflowStep> = value
                                .split(',')
                                .filter_map(|s| WorkflowStep::from_marker(s.trim()))
                                .collect();
                            // A declaration whose every name was a typo is not
                            // a claim to touch nothing -- it is a broken
                            // declaration, and treating it as `none` would hide
                            // the stepper and look deliberate.
                            if resolved.is_empty() {
                                DeclaredSteps::Unknown
                            } else {
                                DeclaredSteps::Only(resolved)
                            }
                        };
                    }
                    _ => {}
                }
            }
        } else if let Some(payload) = line.split_once("CDW_ARG:").map(|(_, p)| p) {
            // `--flag  help text`: the flag is the first token, everything
            // after it is prose. Split on whitespace rather than on a
            // separator character so help text can contain anything.
            let payload = payload.trim();
            let (flag, help) = payload.split_once(char::is_whitespace).unwrap_or((payload, ""));
            if flag.starts_with('-') {
                meta.args.push(ScriptArg {
                    flag: flag.to_string(),
                    help: help.trim().to_string(),
                    default_on: false,
                });
            }
        }
    }

    meta
}

/// Every runnable script under `tools/`, grouped for the sidebar.
///
/// Returns `(category, scripts)` sorted by category then by file name, so the
/// order does not depend on the filesystem's.
pub fn discover(tools_dir: &Path) -> Vec<(String, Vec<ScriptMeta>)> {
    let mut found: Vec<ScriptMeta> = Vec::new();

    if let Ok(entries) = fs::read_dir(tools_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
            if ext != "py" && ext != "sh" {
                continue;
            }
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            let meta = parse_meta(&path, &format!("tools/{name}"));
            if !meta.library {
                found.push(meta);
            }
        }
    }

    found.sort_by(|a, b| {
        category_rank(&a.category)
            .cmp(&category_rank(&b.category))
            .then_with(|| a.category.cmp(&b.category))
            .then_with(|| a.path.cmp(&b.path))
    });

    let mut grouped: Vec<(String, Vec<ScriptMeta>)> = Vec::new();
    for meta in found {
        match grouped.last_mut() {
            Some((category, list)) if *category == meta.category => list.push(meta),
            _ => grouped.push((meta.category.clone(), vec![meta])),
        }
    }
    grouped
}

/// Category order in the sidebar. Deliberately not alphabetical: the groups a
/// user reaches for most sit at the top, and `Other` -- the undeclared
/// scripts -- sits last rather than wherever its initial letter falls.
fn category_rank(category: &str) -> u8 {
    match category {
        "Flows" => 0,
        "Verification" => 1,
        "Simulation" => 2,
        "Maintenance" => 3,
        _ => 9,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write(dir: &Path, name: &str, body: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(body.as_bytes()).unwrap();
        path
    }

    fn tempdir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("cdw-scripts-test-{tag}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_declared_script_yields_its_category_steps_and_args() {
        let dir = tempdir("declared");
        let path = write(
            &dir,
            "x.py",
            "#!/usr/bin/env python3\n\
             # CDW_SCRIPT: category=Flows; steps=Neo,Landing; summary=does a thing\n\
             # CDW_ARG: --apply  Actually delete.\n\
             print('hi')\n",
        );
        let meta = parse_meta(&path, "tools/x.py");
        assert_eq!(meta.category, "Flows");
        assert_eq!(meta.summary, "does a thing");
        assert_eq!(
            meta.declared_steps,
            DeclaredSteps::Only(vec![WorkflowStep::Neo, WorkflowStep::Landing])
        );
        assert_eq!(meta.args.len(), 1);
        assert_eq!(meta.args[0].flag, "--apply");
        assert_eq!(meta.args[0].help, "Actually delete.");
    }

    #[test]
    fn an_undeclared_script_still_appears_and_claims_nothing() {
        // fifteen scripts predate this header. Hiding them, or rendering an
        // empty stepper that asserts they touch no stage, would both be worse
        // than saying nothing about them.
        let dir = tempdir("undeclared");
        let path = write(&dir, "old.sh", "#!/usr/bin/env bash\necho hello\n");
        let meta = parse_meta(&path, "tools/old.sh");
        assert_eq!(meta.category, "Other");
        assert!(meta.args.is_empty());
        assert_eq!(meta.steps(), None, "unknown must not be reported as none");
    }

    #[test]
    fn an_unknown_step_name_is_dropped_not_guessed() {
        let dir = tempdir("badstep");
        let path = write(&dir, "x.py", "# CDW_SCRIPT: steps=Neo,Sausages,Landing\n");
        let meta = parse_meta(&path, "tools/x.py");
        assert_eq!(
            meta.declared_steps,
            DeclaredSteps::Only(vec![WorkflowStep::Neo, WorkflowStep::Landing])
        );
    }

    #[test]
    fn step_names_are_matched_the_way_markers_are() {
        // One matcher for both, so a name that works in a marker works here.
        let dir = tempdir("stepnames");
        let path = write(&dir, "x.py", "# CDW_SCRIPT: steps=event grid,CONTAINERAPPJOBS\n");
        let meta = parse_meta(&path, "tools/x.py");
        assert_eq!(
            meta.declared_steps,
            DeclaredSteps::Only(vec![WorkflowStep::EventGrid, WorkflowStep::ContainerAppJobs])
        );
    }

    #[test]
    fn a_declaration_below_the_header_window_is_ignored() {
        // Otherwise a docstring quoting the format -- which this project's
        // documentation does -- would be read as a declaration.
        let dir = tempdir("deep");
        let body = format!("{}# CDW_SCRIPT: category=Flows\n", "x\n".repeat(HEADER_LINES + 5));
        let path = write(&dir, "x.py", &body);
        assert_eq!(parse_meta(&path, "tools/x.py").category, "Other");
    }

    #[test]
    fn an_arg_without_a_leading_dash_is_not_an_arg() {
        // A positional would be appended to the command line as a bare word and
        // silently change what the script operates on.
        let dir = tempdir("badarg");
        let path = write(&dir, "x.py", "# CDW_ARG: apply  no dash\n# CDW_ARG: --ok  fine\n");
        let meta = parse_meta(&path, "tools/x.py");
        assert_eq!(meta.args.len(), 1);
        assert_eq!(meta.args[0].flag, "--ok");
    }

    #[test]
    fn steps_none_is_a_claim_to_touch_nothing_and_hides_the_stepper() {
        // The reason this state exists: reset_test_documents.py reaches no
        // pipeline stage, and it was rendering all eleven nodes.
        let dir = tempdir("stepsnone");
        let path = write(&dir, "x.py", "# CDW_SCRIPT: steps=none\n");
        let meta = parse_meta(&path, "tools/x.py");
        assert_eq!(meta.declared_steps, DeclaredSteps::None);
        assert!(meta.has_no_steps());
        assert_eq!(meta.steps(), Some(&[][..]));
    }

    #[test]
    fn an_undeclared_script_is_not_treated_as_touching_nothing() {
        // Unknown and none must not collapse: one draws the whole chain, the
        // other draws no chain, and guessing either way misreports the script.
        let dir = tempdir("unknownsteps");
        let path = write(&dir, "x.py", "# CDW_SCRIPT: category=Flows\n");
        let meta = parse_meta(&path, "tools/x.py");
        assert_eq!(meta.declared_steps, DeclaredSteps::Unknown);
        assert!(!meta.has_no_steps());
        assert_eq!(meta.steps(), None);
    }

    #[test]
    fn a_declaration_of_only_typos_is_broken_not_empty() {
        // Falling back to `none` would hide the stepper and look deliberate.
        let dir = tempdir("alltypos");
        let path = write(&dir, "x.py", "# CDW_SCRIPT: steps=Sausages,Custard\n");
        assert_eq!(parse_meta(&path, "tools/x.py").declared_steps, DeclaredSteps::Unknown);
    }

    #[test]
    fn the_language_is_read_from_the_extension() {
        let dir = tempdir("lang");
        let py = write(&dir, "a.py", "#\n");
        let sh = write(&dir, "b.sh", "#\n");
        assert_eq!(parse_meta(&py, "tools/a.py").language(), "py");
        assert_eq!(parse_meta(&sh, "tools/b.sh").language(), "sh");
    }

    #[test]
    fn libraries_are_not_offered_as_runnable() {
        let dir = tempdir("lib");
        write(&dir, "cdw_client.py", "# a library\n");
        write(&dir, "flow_x.py", "# CDW_SCRIPT: category=Flows\n");
        let names: Vec<String> = discover(&dir)
            .into_iter()
            .flat_map(|(_, list)| list)
            .map(|m| m.path)
            .collect();
        assert_eq!(names, vec!["tools/flow_x.py"]);
    }

    #[test]
    fn a_script_may_declare_itself_a_library() {
        let dir = tempdir("selflib");
        let path = write(&dir, "helper.py", "# CDW_SCRIPT: library=true\n");
        assert!(parse_meta(&path, "tools/helper.py").library);
    }

    #[test]
    fn groups_come_back_in_reading_order_not_alphabetical() {
        let dir = tempdir("order");
        write(&dir, "a.py", "# CDW_SCRIPT: category=Maintenance\n");
        write(&dir, "b.py", "# CDW_SCRIPT: category=Flows\n");
        write(&dir, "c.py", "# no declaration\n");
        write(&dir, "d.py", "# CDW_SCRIPT: category=Verification\n");
        let categories: Vec<String> = discover(&dir).into_iter().map(|(c, _)| c).collect();
        assert_eq!(categories, vec!["Flows", "Verification", "Maintenance", "Other"]);
    }

    #[test]
    fn scripts_within_a_group_are_ordered_by_name_not_by_the_filesystem() {
        let dir = tempdir("within");
        for name in ["flow_3.py", "flow_1.py", "flow_2.py"] {
            write(&dir, name, "# CDW_SCRIPT: category=Flows\n");
        }
        let (_, list) = discover(&dir).into_iter().next().unwrap();
        let names: Vec<&str> = list.iter().map(|m| m.file_name()).collect();
        assert_eq!(names, vec!["flow_1.py", "flow_2.py", "flow_3.py"]);
    }
}
