use std::path::PathBuf;

use crate::services::marker_syntax::MarkerSyntax;
use crate::services::steps::StepCatalog;

const FILE: &str = "steps.json";
const SYNTAX_FILE: &str = "markers.json";
const APP_DIR: &str = "dev-monitor";

/// `~/Library/Application Support/dev-monitor/steps.json`, or the platform
/// equivalent. `None` when the OS reports no config directory at all, in which
/// case the app runs on defaults and simply cannot persist.
pub fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join(APP_DIR).join(FILE))
}

/// The saved catalog, or the defaults.
pub fn load() -> StepCatalog {
    let Some(path) = config_path() else {
        return StepCatalog::defaults();
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return StepCatalog::defaults();
    };
    match serde_json::from_str::<StepCatalog>(&text) {
        Ok(catalog) if !catalog.steps.is_empty() => catalog,
        // An EMPTY list is treated as a broken file rather than an honest
        // choice: it renders no stepper at all, which is indistinguishable
        // from the app being broken.
        Ok(_) => {
            eprintln!("steps.json defines no steps -- using defaults");
            StepCatalog::defaults()
        }
        Err(e) => {
            eprintln!("steps.json could not be read ({e}) -- using defaults, file left alone");
            StepCatalog::defaults()
        }
    }
}

pub fn save(catalog: &StepCatalog) -> Result<(), String> {
    let path = config_path().ok_or("no config directory on this system")?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    }
    let json = serde_json::to_string_pretty(catalog).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("cannot write {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::steps::StepDef;

    #[test]
    fn a_catalog_survives_a_round_trip() {
        let mut original = StepCatalog::defaults();
        original.steps[0].aliases.push("SRC".into());
        original.steps.push(StepDef {
            id: "purge".into(), name: "Purge".into(), icon: "trash".into(),
            aliases: vec!["Cleanup".into()], on_chain: false,
        });
        let json = serde_json::to_string(&original).unwrap();
        assert_eq!(serde_json::from_str::<StepCatalog>(&json).unwrap(), original);
    }

    #[test]
    fn a_step_written_without_the_optional_fields_still_loads() {
        // Hand-written JSON is not supported, but it should not be rejected
        // over a field the app would have defaulted anyway.
        let json = r#"{"steps":[{"id":"raw","name":"Raw","icon":"file-code"}]}"#;
        let c: StepCatalog = serde_json::from_str(json).unwrap();
        assert_eq!(c.steps[0].aliases, Vec::<String>::new());
        assert!(c.steps[0].on_chain, "on_chain defaults to true");
    }

    #[test]
    fn nonsense_is_a_parse_error_rather_than_an_empty_catalog() {
        assert!(serde_json::from_str::<StepCatalog>("not json").is_err());
        assert!(serde_json::from_str::<StepCatalog>(r#"{"steps":[{"id":"x"}]}"#).is_err());
    }
}

/// Where the marker tokens live.
fn syntax_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join(APP_DIR).join(SYNTAX_FILE))
}

/// The saved marker syntax, or the defaults.
///
/// Kept in its own file rather than nested in steps.json: the two are edited on
/// different tabs and a parse failure in one should not lose the other.
pub fn load_syntax() -> MarkerSyntax {
    let Some(path) = syntax_path() else {
        return MarkerSyntax::default();
    };
    let Ok(text) = std::fs::read_to_string(&path) else {
        return MarkerSyntax::default();
    };
    serde_json::from_str(&text).unwrap_or_else(|e| {
        eprintln!("markers.json could not be read ({e}) -- using defaults, file left alone");
        MarkerSyntax::default()
    })
}

pub fn save_syntax(syntax: &MarkerSyntax) -> Result<(), String> {
    let path = syntax_path().ok_or("no config directory on this system")?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    }
    let json = serde_json::to_string_pretty(syntax).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| format!("cannot write {}: {e}", path.display()))
}
