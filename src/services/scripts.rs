use std::fs;
use std::path::Path;

use crate::services::marker_syntax::{MarkerKind, MarkerSyntax};
use crate::services::steps::{StepCatalog, StepId};

/// A flag a script accepts, offered in the UI as a toggle.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ScriptArg {
    pub flag: String,
    pub help: String,
    /// Pre-selected when the script is chosen.
    pub default_on: bool,
}

/// A flag a script accepts that takes a VALUE, offered in the UI as a list.
///
/// A toggle cannot express this. `dev_corpus_e2e.py` runs one document out of
/// a manifest of ten, and which one is not a yes/no: the app has to offer the
/// ten and send one back as `--case <value>`.
///
/// THE VALUES ARE NOT IN THE HEADER, deliberately. They live in the same file
/// the script itself reads, so adding a document to the corpus changes the
/// dropdown with no edit here and no edit there. A list transcribed into the
/// header would go stale silently -- the dropdown would offer a case the
/// script has never heard of, and the script's own error message would be the
/// first anyone knew.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ScriptChoice {
    pub flag: String,
    /// Repo-relative, as declared: `tools/fixtures/dev_test_manifest.csv`.
    pub source: String,
    /// The column read out of it.
    pub column: String,
    pub help: String,
    /// What the column holds. EMPTY IS A STATE, not a failure: the file may
    /// not have been fetched yet, and the picker says so rather than offering
    /// nothing and letting the script be launched without the flag.
    pub values: Vec<String>,
}

/// What a script says about the pipeline stages it can reach.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum DeclaredSteps {
    /// No declaration.
    Unknown,
    /// `steps=none` -- explicitly touches no pipeline stage.
    None,
    /// The stages named, resolved.
    Only(Vec<StepId>),
}

/// One script, as the app understands it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ScriptMeta {
    /// Repo-relative, e.g.
    pub path: String,
    /// Sidebar group.
    pub category: String,
    /// Stages the script can reach.
    pub declared_steps: DeclaredSteps,
    pub args: Vec<ScriptArg>,
    /// Flags that take a value, each with the list to choose from.
    pub choices: Vec<ScriptChoice>,
    /// A one-line summary, taken from the header when given.
    pub summary: String,
    /// Not offered for running.
    pub library: bool,
}

impl ScriptMeta {
    pub fn file_name(&self) -> &str {
        self.path.rsplit('/').next().unwrap_or(&self.path)
    }

    /// The stages to draw, or `None` to draw the whole chain.
    pub fn steps(&self) -> Option<&[StepId]> {
        match &self.declared_steps {
            DeclaredSteps::Unknown => None,
            DeclaredSteps::None => Some(&[]),
            DeclaredSteps::Only(steps) => Some(steps),
        }
    }

    /// True when the script says it reaches no pipeline stage, so the stepper.
    pub fn has_no_steps(&self) -> bool {
        self.declared_steps == DeclaredSteps::None
    }

    /// `py` or `sh`.
    pub fn language(&self) -> &'static str {
        if self.path.ends_with(".py") { "py" } else { "sh" }
    }
}

/// Scripts a script author would never want offered as runnable.
const KNOWN_LIBRARIES: [&str; 1] = ["cdw_client.py"];

/// How many lines from the top to scan for the header.
const HEADER_LINES: usize = 60;

fn parse_pairs(payload: &str) -> Vec<(String, String)> {
    payload
        .split(';')
        .filter_map(|part| part.split_once('='))
        .map(|(k, v)| (k.trim().to_lowercase(), v.trim().to_string()))
        .collect()
}

/// Read one script's declaration.
pub fn parse_meta(
    path: &Path,
    repo_relative: &str,
    // The repository root a `CDW_CHOICE` source is resolved against. Passed
    // rather than derived from `path`: under test the tools directory is a
    // temporary one that is not called `tools`, so stripping the relative
    // suffix would find the wrong root exactly where it is least noticed.
    repo_root: &Path,
    catalog: &StepCatalog,
    syntax: &MarkerSyntax,
) -> ScriptMeta {
    let mut meta = ScriptMeta {
        path: repo_relative.to_string(),
        category: "Other".to_string(),
        declared_steps: DeclaredSteps::Unknown,
        args: Vec::new(),
        choices: Vec::new(),
        summary: String::new(),
        library: KNOWN_LIBRARIES.contains(&repo_relative.rsplit('/').next().unwrap_or("")),
    };

    let Ok(text) = fs::read_to_string(path) else {
        return meta;
    };

    for line in text.lines().take(HEADER_LINES) {
        if let Some(payload) = syntax.payload(line, MarkerKind::ScriptHeader) {
            for (key, value) in parse_pairs(payload) {
                match key.as_str() {
                    "category" => meta.category = value,
                    "summary" => meta.summary = value,
                    "library" => meta.library = value.eq_ignore_ascii_case("true"),
                    "steps" => {
                        meta.declared_steps = if value.eq_ignore_ascii_case("none") {
                            DeclaredSteps::None
                        } else {
                            let resolved: Vec<StepId> = value
                                .split(',')
                                .filter_map(|s| catalog.resolve(s.trim()))
                                .collect();
                            // A declaration whose every name was a typo is not a claim to touch nothing.
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
        } else if let Some(payload) = syntax.payload(line, MarkerKind::ArgHeader) {
            // `--flag  help text`: first token is the flag, the rest is prose.
            let payload = payload.trim();
            let (flag, help) = payload.split_once(char::is_whitespace).unwrap_or((payload, ""));
            if flag.starts_with('-') {
                meta.args.push(ScriptArg {
                    flag: flag.to_string(),
                    help: help.trim().to_string(),
                    default_on: false,
                });
            }
        } else if let Some(payload) = syntax.payload(line, MarkerKind::ChoiceHeader) {
            if let Some(choice) = parse_choice(payload, repo_root) {
                meta.choices.push(choice);
            }
        }
    }

    meta
}

/// One `CDW_CHOICE` line: `--case @tools/fixtures/m.csv:source_filename  help`,
/// or an inline list, `--indicator RO|RW  help`.
///
/// Returns None rather than an empty choice when the source is absent. A
/// choice with nowhere to read from would render as a dropdown that can never
/// be filled, and the script would then be launched with no value for a flag
/// it requires.
fn parse_choice(payload: &str, repo_root: &Path) -> Option<ScriptChoice> {
    let payload = payload.trim();
    let (flag, rest) = payload.split_once(char::is_whitespace)?;
    if !flag.starts_with('-') {
        return None;
    }
    let rest = rest.trim();
    // INLINE VALUES, for a flag whose vocabulary is the script's own rather
    // than a column of a file: `--indicator RO|RW` names the two copies a DMS
    // document is delivered in. Read as a file spec it parses to nothing, the
    // choice is dropped, and the run silently sends the script's default copy.
    if !rest.starts_with('@') {
        let (list, help) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));
        let values: Vec<String> = list
            .split('|')
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .collect();
        if values.len() < 2 {
            return None;
        }
        return Some(ScriptChoice {
            flag: flag.to_string(),
            source: String::new(),
            column: String::new(),
            help: help.trim().to_string(),
            values,
        });
    }
    let spec = rest.strip_prefix('@')?;
    // rsplit, not split: a Windows-shaped path would contain a drive colon,
    // and the COLUMN is always the last segment.
    let (source, tail) = spec.rsplit_once(':')?;
    let (column, help) = tail.split_once(char::is_whitespace).unwrap_or((tail, ""));
    if source.is_empty() || column.is_empty() {
        return None;
    }

    Some(ScriptChoice {
        flag: flag.to_string(),
        source: source.to_string(),
        column: column.to_string(),
        help: help.trim().to_string(),
        values: read_column(&repo_root.join(source), column),
    })
}

/// One column of a delimited text file, in file order.
///
/// THE DELIMITER IS SNIFFED FROM THE HEADER, and only from the header: the
/// corpus manifest is semicolon-delimited and holds values containing commas
/// and spaces (`001.17033.000001-AA001-10 - A.pdf`), so sniffing per line
/// would split a value in half and offer a case the script cannot match.
///
/// Quoted fields are NOT supported. The one file this reads has none, and a
/// half-written quote parser that looks like it works is worse than an
/// explicit limit.
fn read_column(path: &Path, column: &str) -> Vec<String> {
    // A missing file is the ordinary state before anything has been fetched,
    // so it reads as "no values", not as an error.
    let Ok(text) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut lines = text.lines();
    // Excel writes a BOM. Left on, it becomes part of the first column's name
    // and that column can never be found.
    let Some(header) = lines.next().map(|l| l.trim_start_matches('\u{feff}')) else {
        return Vec::new();
    };
    let delimiter = if header.contains(';') { ';' } else { ',' };
    let Some(index) = header.split(delimiter).position(|h| h.trim() == column) else {
        return Vec::new();
    };

    // DEDUPED, IN FILE ORDER: the corpus manifest carries a DMS document twice,
    // once per copy, naming the same file. Listed twice the dropdown shows one
    // document as two identical entries and neither says which copy it sends;
    // the script runs both copies for one name, or takes --indicator to narrow.
    let mut seen = std::collections::HashSet::new();
    lines
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| line.split(delimiter).nth(index))
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .filter(|value| seen.insert(value.clone()))
        .collect()
}

pub fn discover(
    tools_dir: &Path,
    catalog: &StepCatalog,
    syntax: &MarkerSyntax,
) -> Vec<(String, Vec<ScriptMeta>)> {
    // A choice's source is declared repo-relative (`tools/fixtures/...`), the
    // same way a script's own path is, so it is resolved against the parent of
    // the tools directory.
    let repo_root = tools_dir.parent().unwrap_or(tools_dir).to_path_buf();
    let mut found: Vec<ScriptMeta> = Vec::new();
    collect(tools_dir, tools_dir, &repo_root, catalog, syntax, &mut found);

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

fn collect(
    root: &Path,
    dir: &Path,
    repo_root: &Path,
    catalog: &StepCatalog,
    syntax: &MarkerSyntax,
    found: &mut Vec<ScriptMeta>,
) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");

        if path.is_dir() {
            if name.starts_with('.') || name.starts_with("__") {
                continue;
            }
            collect(root, &path, repo_root, catalog, syntax, found);
            continue;
        }

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "py" && ext != "sh" {
            continue;
        }
        let Ok(relative) = path.strip_prefix(root) else { continue };
        let Some(relative) = relative.to_str() else { continue };
        let meta = parse_meta(&path, &format!("tools/{relative}"), repo_root, catalog, syntax);
        if !meta.library {
            found.push(meta);
        }
    }
}

/// Category order in the sidebar.
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
    fn a_script_in_a_subdirectory_is_found_and_keeps_its_folder_in_the_path() {
        let dir = tempdir("nested");
        fs::create_dir_all(dir.join("flows")).unwrap();
        write(&dir.join("flows"), "cdr_flat_promotes.py", "# CDW_SCRIPT: category=Flows\n");

        let groups = discover(&dir, &StepCatalog::defaults(), &MarkerSyntax::default());

        let flows = &groups.iter().find(|(c, _)| c == "Flows").expect("no Flows group").1;
        assert_eq!(flows.len(), 1);
        // The folder has to survive into the path: ScriptRunner hands this
        // string to bash from the repository root.
        assert_eq!(flows[0].path, "tools/flows/cdr_flat_promotes.py");
    }

    #[test]
    fn a_library_in_a_subdirectory_stays_out_of_the_sidebar() {
        let dir = tempdir("nested-library");
        fs::create_dir_all(dir.join("cdw_workflows")).unwrap();
        write(&dir.join("cdw_workflows"), "flow.py", "# CDW_SCRIPT: library=true\n");
        write(&dir, "real.py", "# CDW_SCRIPT: category=Flows\n");

        let groups = discover(&dir, &StepCatalog::defaults(), &MarkerSyntax::default());

        let names: Vec<&str> = groups.iter().flat_map(|(_, v)| v.iter().map(|m| m.file_name())).collect();
        assert_eq!(names, vec!["real.py"]);
    }

    #[test]
    fn generated_directories_are_not_walked() {
        let dir = tempdir("nested-pycache");
        fs::create_dir_all(dir.join("__pycache__")).unwrap();
        fs::create_dir_all(dir.join(".hidden")).unwrap();
        write(&dir.join("__pycache__"), "flow.py", "# CDW_SCRIPT: category=Flows\n");
        write(&dir.join(".hidden"), "sneaky.py", "# CDW_SCRIPT: category=Flows\n");

        let groups = discover(&dir, &StepCatalog::defaults(), &MarkerSyntax::default());

        assert!(groups.is_empty(), "a generated or hidden directory was walked");
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
        let meta = parse_meta(&path, "tools/x.py", &dir, &StepCatalog::defaults(), &MarkerSyntax::default());
        assert_eq!(meta.category, "Flows");
        assert_eq!(meta.summary, "does a thing");
        assert_eq!(
            meta.declared_steps,
            DeclaredSteps::Only(vec!["neo".into(), "landing".into()])
        );
        assert_eq!(meta.args.len(), 1);
        assert_eq!(meta.args[0].flag, "--apply");
        assert_eq!(meta.args[0].help, "Actually delete.");
    }

    #[test]
    fn an_undeclared_script_still_appears_and_claims_nothing() {
        // fifteen scripts predate this header.
        let dir = tempdir("undeclared");
        let path = write(&dir, "old.sh", "#!/usr/bin/env bash\necho hello\n");
        let meta = parse_meta(&path, "tools/old.sh", &dir, &StepCatalog::defaults(), &MarkerSyntax::default());
        assert_eq!(meta.category, "Other");
        assert!(meta.args.is_empty());
        assert_eq!(meta.steps(), None, "unknown must not be reported as none");
    }

    #[test]
    fn an_unknown_step_name_is_dropped_not_guessed() {
        let dir = tempdir("badstep");
        let path = write(&dir, "x.py", "# CDW_SCRIPT: steps=Neo,Sausages,Landing\n");
        let meta = parse_meta(&path, "tools/x.py", &dir, &StepCatalog::defaults(), &MarkerSyntax::default());
        assert_eq!(
            meta.declared_steps,
            DeclaredSteps::Only(vec!["neo".into(), "landing".into()])
        );
    }

    #[test]
    fn step_names_are_matched_the_way_markers_are() {
        // One matcher for both, so a name that works in a marker works here.
        let dir = tempdir("stepnames");
        let path = write(&dir, "x.py", "# CDW_SCRIPT: steps=event grid,CONTAINERAPPJOBS\n");
        let meta = parse_meta(&path, "tools/x.py", &dir, &StepCatalog::defaults(), &MarkerSyntax::default());
        assert_eq!(
            meta.declared_steps,
            DeclaredSteps::Only(vec!["eventgrid".into(), "containerappjobs".into()])
        );
    }

    #[test]
    fn a_declaration_below_the_header_window_is_ignored() {
        // Otherwise a docstring quoting the format -- which this project's.
        let dir = tempdir("deep");
        let body = format!("{}# CDW_SCRIPT: category=Flows\n", "x\n".repeat(HEADER_LINES + 5));
        let path = write(&dir, "x.py", &body);
        assert_eq!(parse_meta(&path, "tools/x.py", &dir, &StepCatalog::defaults(), &MarkerSyntax::default()).category, "Other");
    }

    #[test]
    fn an_arg_without_a_leading_dash_is_not_an_arg() {
        // A positional would be appended as a bare word and change the target.
        let dir = tempdir("badarg");
        let path = write(&dir, "x.py", "# CDW_ARG: apply  no dash\n# CDW_ARG: --ok  fine\n");
        let meta = parse_meta(&path, "tools/x.py", &dir, &StepCatalog::defaults(), &MarkerSyntax::default());
        assert_eq!(meta.args.len(), 1);
        assert_eq!(meta.args[0].flag, "--ok");
    }

    #[test]
    fn steps_none_is_a_claim_to_touch_nothing_and_hides_the_stepper() {
        // The reason this state exists: reset_test_documents.py reaches no pipeline.
        let dir = tempdir("stepsnone");
        let path = write(&dir, "x.py", "# CDW_SCRIPT: steps=none\n");
        let meta = parse_meta(&path, "tools/x.py", &dir, &StepCatalog::defaults(), &MarkerSyntax::default());
        assert_eq!(meta.declared_steps, DeclaredSteps::None);
        assert!(meta.has_no_steps());
        assert_eq!(meta.steps(), Some(&[][..]));
    }

    #[test]
    fn an_undeclared_script_is_not_treated_as_touching_nothing() {
        // Unknown and none must not collapse: one draws the whole chain, the other.
        let dir = tempdir("unknownsteps");
        let path = write(&dir, "x.py", "# CDW_SCRIPT: category=Flows\n");
        let meta = parse_meta(&path, "tools/x.py", &dir, &StepCatalog::defaults(), &MarkerSyntax::default());
        assert_eq!(meta.declared_steps, DeclaredSteps::Unknown);
        assert!(!meta.has_no_steps());
        assert_eq!(meta.steps(), None);
    }

    #[test]
    fn a_declaration_of_only_typos_is_broken_not_empty() {
        // Falling back to `none` would hide the stepper and look deliberate.
        let dir = tempdir("alltypos");
        let path = write(&dir, "x.py", "# CDW_SCRIPT: steps=Sausages,Custard\n");
        assert_eq!(parse_meta(&path, "tools/x.py", &dir, &StepCatalog::defaults(), &MarkerSyntax::default()).declared_steps, DeclaredSteps::Unknown);
    }

    #[test]
    fn the_language_is_read_from_the_extension() {
        let dir = tempdir("lang");
        let py = write(&dir, "a.py", "#\n");
        let sh = write(&dir, "b.sh", "#\n");
        assert_eq!(parse_meta(&py, "tools/a.py", &dir, &StepCatalog::defaults(), &MarkerSyntax::default()).language(), "py");
        assert_eq!(parse_meta(&sh, "tools/b.sh", &dir, &StepCatalog::defaults(), &MarkerSyntax::default()).language(), "sh");
    }

    #[test]
    fn libraries_are_not_offered_as_runnable() {
        let dir = tempdir("lib");
        write(&dir, "cdw_client.py", "# a library\n");
        write(&dir, "flow_x.py", "# CDW_SCRIPT: category=Flows\n");
        let names: Vec<String> = discover(&dir, &StepCatalog::defaults(), &MarkerSyntax::default())
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
        assert!(parse_meta(&path, "tools/helper.py", &dir, &StepCatalog::defaults(), &MarkerSyntax::default()).library);
    }

    #[test]
    fn groups_come_back_in_reading_order_not_alphabetical() {
        let dir = tempdir("order");
        write(&dir, "a.py", "# CDW_SCRIPT: category=Maintenance\n");
        write(&dir, "b.py", "# CDW_SCRIPT: category=Flows\n");
        write(&dir, "c.py", "# no declaration\n");
        write(&dir, "d.py", "# CDW_SCRIPT: category=Verification\n");
        let categories: Vec<String> = discover(&dir, &StepCatalog::defaults(), &MarkerSyntax::default()).into_iter().map(|(c, _)| c).collect();
        assert_eq!(categories, vec!["Flows", "Verification", "Maintenance", "Other"]);
    }

    // ------------------------------------------------------------- choices

    /// A script declaring one choice, and the CSV it reads its values from.
    fn with_manifest(tag: &str, csv: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let root = tempdir(tag);
        let tools = root.join("tools");
        fs::create_dir_all(tools.join("fixtures")).unwrap();
        write(&tools.join("fixtures"), "m.csv", csv);
        let path = write(
            &tools,
            "corpus.py",
            "# CDW_SCRIPT: category=Flows\n\
             # CDW_CHOICE: --case @tools/fixtures/m.csv:source_filename  Which document to send\n\
             # CDW_ARG: --all  Every row\n",
        );
        (root, path)
    }

    #[test]
    fn a_choice_header_yields_its_flag_column_and_help() {
        let (root, path) = with_manifest("choice", "source_filename;ext\na.pdf;PDF\n");
        let meta = parse_meta(&path, "tools/corpus.py", &root, &StepCatalog::defaults(), &MarkerSyntax::default());
        assert_eq!(meta.choices.len(), 1);
        assert_eq!(meta.choices[0].flag, "--case");
        assert_eq!(meta.choices[0].column, "source_filename");
        assert_eq!(meta.choices[0].help, "Which document to send");
    }

    #[test]
    fn the_values_come_from_the_named_column_of_the_named_file() {
        let (root, path) = with_manifest("choice-values", "source_filename;ext\na.pdf;PDF\nb.ZIP;ZIP\n");
        let meta = parse_meta(&path, "tools/corpus.py", &root, &StepCatalog::defaults(), &MarkerSyntax::default());
        assert_eq!(meta.choices[0].values, vec!["a.pdf", "b.ZIP"]);
    }

    #[test]
    fn a_byte_order_mark_does_not_hide_the_first_column() {
        // Excel writes one; without stripping it the first column is named
        // "\u{feff}source_filename" and the dropdown comes back empty.
        let (root, path) = with_manifest("choice-bom", "\u{feff}source_filename;ext\na.pdf;PDF\n");
        let meta = parse_meta(&path, "tools/corpus.py", &root, &StepCatalog::defaults(), &MarkerSyntax::default());
        assert_eq!(meta.choices[0].values, vec!["a.pdf"]);
    }

    #[test]
    fn a_comma_delimited_file_is_read_too() {
        let (root, path) = with_manifest("choice-comma", "source_filename,ext\na.pdf,PDF\n");
        let meta = parse_meta(&path, "tools/corpus.py", &root, &StepCatalog::defaults(), &MarkerSyntax::default());
        assert_eq!(meta.choices[0].values, vec!["a.pdf"]);
    }

    #[test]
    fn a_value_containing_the_other_delimiter_survives() {
        // "001.17033.000001-AA001-10 - A.pdf" has no semicolon but the corpus
        // is full of dots and spaces; sniffing the wrong delimiter would split
        // a name in half and offer a case that cannot be selected.
        let (root, path) = with_manifest("choice-sniff", "source_filename;ext\n001.1-AA, B.pdf;PDF\n");
        let meta = parse_meta(&path, "tools/corpus.py", &root, &StepCatalog::defaults(), &MarkerSyntax::default());
        assert_eq!(meta.choices[0].values, vec!["001.1-AA, B.pdf"]);
    }

    #[test]
    fn a_missing_file_leaves_the_choice_present_with_no_values() {
        // The picker then says so. Dropping the choice entirely would let the
        // script be launched with no `--case` at all, which it refuses.
        let root = tempdir("choice-missing");
        let tools = root.join("tools");
        fs::create_dir_all(&tools).unwrap();
        let path = write(
            &tools,
            "corpus.py",
            "# CDW_SCRIPT: category=Flows\n\
             # CDW_CHOICE: --case @tools/fixtures/absent.csv:source_filename  Which document\n",
        );
        let meta = parse_meta(&path, "tools/corpus.py", &root, &StepCatalog::defaults(), &MarkerSyntax::default());
        assert_eq!(meta.choices.len(), 1);
        assert!(meta.choices[0].values.is_empty());
    }

    #[test]
    fn a_column_that_is_not_in_the_file_yields_no_values_rather_than_the_first_column() {
        let (root, path) = with_manifest("choice-column", "other;ext\na.pdf;PDF\n");
        let meta = parse_meta(&path, "tools/corpus.py", &root, &StepCatalog::defaults(), &MarkerSyntax::default());
        assert!(meta.choices[0].values.is_empty());
    }

    #[test]
    fn a_choice_and_a_toggle_coexist() {
        let (root, path) = with_manifest("choice-and-arg", "source_filename;ext\na.pdf;PDF\n");
        let meta = parse_meta(&path, "tools/corpus.py", &root, &StepCatalog::defaults(), &MarkerSyntax::default());
        assert_eq!(meta.args.len(), 1);
        assert_eq!(meta.args[0].flag, "--all");
        assert_eq!(meta.choices.len(), 1);
    }

    #[test]
    fn a_document_delivered_as_two_copies_is_offered_once() {
        // The manifest carries a DMS document twice, RO and RW, naming the same
        // file. Listed twice the dropdown shows one document as two identical
        // entries, and neither says which copy it would send.
        let (root, path) = with_manifest(
            "choice-copies",
            "source_filename;file_type_indicator\nHEER.ZIP;RO\nHEER.ZIP;RW\nb.pdf;RO\n",
        );
        let meta = parse_meta(&path, "tools/corpus.py", &root, &StepCatalog::defaults(), &MarkerSyntax::default());
        assert_eq!(meta.choices[0].values, vec!["HEER.ZIP", "b.pdf"]);
    }

    #[test]
    fn a_choice_can_list_its_values_inline() {
        // `--indicator RO|RW` has no file to read: the two copies are the
        // script's own vocabulary. Ignored, the flag never reaches the picker
        // and the run sends whichever copy the script defaults to.
        let root = tempdir("choice-inline");
        let tools = root.join("tools");
        fs::create_dir_all(&tools).unwrap();
        let path = write(
            &tools,
            "corpus.py",
            "# CDW_SCRIPT: category=Flows\n\
             # CDW_CHOICE: --indicator RO|RW  Which copy to send\n",
        );
        let meta = parse_meta(&path, "tools/corpus.py", &root, &StepCatalog::defaults(), &MarkerSyntax::default());
        assert_eq!(meta.choices.len(), 1);
        assert_eq!(meta.choices[0].flag, "--indicator");
        assert_eq!(meta.choices[0].values, vec!["RO", "RW"]);
        assert_eq!(meta.choices[0].help, "Which copy to send");
    }

    #[test]
    fn a_choice_with_no_source_is_ignored_rather_than_offered_empty() {
        let root = tempdir("choice-nosource");
        let tools = root.join("tools");
        fs::create_dir_all(&tools).unwrap();
        let path = write(&tools, "x.py", "# CDW_CHOICE: --case  no source given\n");
        let meta = parse_meta(&path, "tools/x.py", &root, &StepCatalog::defaults(), &MarkerSyntax::default());
        assert!(meta.choices.is_empty());
    }

    #[test]
    fn scripts_within_a_group_are_ordered_by_name_not_by_the_filesystem() {
        let dir = tempdir("within");
        for name in ["flow_3.py", "flow_1.py", "flow_2.py"] {
            write(&dir, name, "# CDW_SCRIPT: category=Flows\n");
        }
        let (_, list) = discover(&dir, &StepCatalog::defaults(), &MarkerSyntax::default()).into_iter().next().unwrap();
        let names: Vec<&str> = list.iter().map(|m| m.file_name()).collect();
        assert_eq!(names, vec!["flow_1.py", "flow_2.py", "flow_3.py"]);
    }
}
