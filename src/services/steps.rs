use serde::{Deserialize, Serialize};

/// A step's stable identity. Lowercase alphanumeric, assigned once.
pub type StepId = String;

/// One stage, as configured.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct StepDef {
    pub id: StepId,
    /// Shown under the node, and matched against markers like any alias.
    pub name: String,
    /// A Phosphor class without the `ph-` prefix, e.g. `database`.
    pub icon: String,
    /// Extra spellings that resolve to this step.
    #[serde(default)]
    pub aliases: Vec<String>,
    /// On the linear chain, or a branch off it. Quarantine and Rejected are
    /// branches: they are outcomes, not positions, and drawing them inline
    /// would put every run through them.
    #[serde(default = "on_chain_default")]
    pub on_chain: bool,
}

fn on_chain_default() -> bool {
    true
}

/// Fold a name to its comparable form: lowercase, alphanumeric only.
///
/// Makes spacing and capitalisation stop being part of the contract, so
/// `Container App Jobs`, `ContainerAppJobs` and `container-app-jobs` are one
/// name. Carried over unchanged from the enum-based resolver.
pub fn normalise(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// Every step, in the order the stepper draws them.
///
/// Order is the vector's order, not a field: drag-to-reorder then moves an
/// element rather than renumbering the rest, and two steps cannot claim the
/// same position.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct StepCatalog {
    pub steps: Vec<StepDef>,
}

impl Default for StepCatalog {
    fn default() -> Self {
        Self::defaults()
    }
}

impl StepCatalog {
    /// The built-in set: exactly the names, icons and order the app shipped
    /// with before any of this was configurable. An install with no config file
    /// must be indistinguishable from that, which is the acceptance test for
    /// the whole change.
    pub fn defaults() -> Self {
        let step = |id: &str, name: &str, icon: &str, on_chain: bool| StepDef {
            id: id.to_string(),
            name: name.to_string(),
            icon: icon.to_string(),
            aliases: Vec::new(),
            on_chain,
        };
        Self {
            steps: vec![
                step("neo", "NEO", "database", true),
                step("authentication", "Authentication", "lock-key", true),
                step("apim", "APIM", "cloud", true),
                step("landing", "Landing", "folder-simple", true),
                step("eventgrid", "Event Grid", "lightning", true),
                step("raw", "Raw", "file-code", true),
                step("servicebus", "Service Bus", "envelope-simple", true),
                step("containerappjobs", "Container App Jobs", "cpu", true),
                step("processing", "Processing", "gear", true),
                step("curated", "Curated", "medal", true),
                step("verification", "Verification", "check-circle", true),
                step("quarantine", "Quarantine", "warning-circle", false),
                step("rejected", "Rejected", "x-circle", false),
            ],
        }
    }

    /// Resolve a marker payload to a step id.
    ///
    /// Matches the id, the display name and every alias, all normalised. The
    /// name is included so a marker keeps resolving after a rename only if the
    /// user kept the old spelling as an alias -- and the id is included so the
    /// original marker text keeps working regardless, since ids are seeded from
    /// the original names.
    pub fn resolve(&self, raw: &str) -> Option<StepId> {
        let wanted = normalise(raw);
        self.steps
            .iter()
            .find(|s| {
                normalise(&s.id) == wanted
                    || normalise(&s.name) == wanted
                    || s.aliases.iter().any(|a| normalise(a) == wanted)
            })
            .map(|s| s.id.clone())
    }

    pub fn get(&self, id: &str) -> Option<&StepDef> {
        self.steps.iter().find(|s| s.id == id)
    }

    /// Display name, falling back to the id so a step deleted mid-run still
    /// renders as something rather than as an empty label.
    pub fn name_of(&self, id: &str) -> String {
        self.get(id).map(|s| s.name.clone()).unwrap_or_else(|| id.to_string())
    }

    /// Full Phosphor class. `question` for an unknown step: a missing icon
    /// should look like a question, not like a gap in the layout.
    pub fn icon_of(&self, id: &str) -> String {
        format!("ph-{}", self.get(id).map(|s| s.icon.as_str()).unwrap_or("question"))
    }

    /// The linear chain, in order.
    pub fn chain(&self) -> Vec<&StepDef> {
        self.steps.iter().filter(|s| s.on_chain).collect()
    }

    /// Position along the chain, or None for a branch.
    pub fn chain_index(&self, id: &str) -> Option<usize> {
        self.chain().iter().position(|s| s.id == id)
    }

    /// A fresh id for a user-added step, derived from its name and made unique.
    pub fn mint_id(&self, name: &str) -> StepId {
        let base = normalise(name);
        let base = if base.is_empty() { "step".to_string() } else { base };
        if self.get(&base).is_none() {
            return base;
        }
        (2..).map(|n| format!("{base}{n}")).find(|c| self.get(c).is_none()).unwrap()
    }

    /// Move a step, for drag-to-reorder. Out-of-range indices are ignored
    /// rather than panicking: a drag that ends outside the list is a no-op, not
    /// a crash.
    pub fn reorder(&mut self, from: usize, to: usize) {
        if from >= self.steps.len() || to >= self.steps.len() || from == to {
            return;
        }
        let item = self.steps.remove(from);
        self.steps.insert(to, item);
    }

    pub fn remove(&mut self, id: &str) {
        self.steps.retain(|s| s.id != id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_what_the_app_shipped_with() {
        // THE ACCEPTANCE TEST for the whole change: an install with no config
        // must be indistinguishable from the hardcoded version.
        let c = StepCatalog::defaults();
        assert_eq!(c.steps.len(), 13);
        assert_eq!(c.chain().len(), 11, "Quarantine and Rejected are branches");
        let names: Vec<&str> = c.chain().iter().map(|s| s.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "NEO", "Authentication", "APIM", "Landing", "Event Grid", "Raw",
                "Service Bus", "Container App Jobs", "Processing", "Curated", "Verification"
            ]
        );
        assert_eq!(c.icon_of("neo"), "ph-database");
        assert_eq!(c.icon_of("rejected"), "ph-x-circle");
    }

    #[test]
    fn markers_resolve_the_way_they_did_before() {
        let c = StepCatalog::defaults();
        for raw in ["ContainerAppJobs", "Container App Jobs", "container-app-jobs", "CONTAINERAPPJOBS"] {
            assert_eq!(c.resolve(raw).as_deref(), Some("containerappjobs"), "{raw}");
        }
        assert_eq!(c.resolve("Event Grid").as_deref(), Some("eventgrid"));
        assert_eq!(c.resolve("Sausages"), None);
    }

    #[test]
    fn an_alias_resolves_to_its_step() {
        let mut c = StepCatalog::defaults();
        c.steps[7].aliases.push("CAJ".into());
        assert_eq!(c.resolve("CAJ").as_deref(), Some("containerappjobs"));
        assert_eq!(c.resolve("caj").as_deref(), Some("containerappjobs"));
    }

    #[test]
    fn renaming_a_step_keeps_its_marker_working() {
        // The whole reason identity is `id` and not `name`. A user renames the
        // label; every script still emits [CDW_STEP: Raw].
        let mut c = StepCatalog::defaults();
        c.steps[5].name = "Raw storage".into();
        assert_eq!(c.resolve("Raw").as_deref(), Some("raw"));
        assert_eq!(c.resolve("Raw storage").as_deref(), Some("raw"));
    }

    #[test]
    fn a_deleted_step_no_longer_resolves_and_does_not_panic() {
        // Everything is deletable, and a marker for a deleted step does
        // nothing -- decided 2026-09-02.
        let mut c = StepCatalog::defaults();
        c.remove("raw");
        assert_eq!(c.resolve("Raw"), None);
        assert_eq!(c.chain().len(), 10);
        assert_eq!(c.name_of("raw"), "raw", "falls back to the id rather than empty");
        assert_eq!(c.icon_of("raw"), "ph-question");
    }

    #[test]
    fn a_minted_id_never_collides() {
        let mut c = StepCatalog::defaults();
        assert_eq!(c.mint_id("Purge"), "purge");
        assert_eq!(c.mint_id("Raw"), "raw2", "the built-in raw already holds it");
        c.steps.push(StepDef {
            id: "raw2".into(), name: "Raw 2".into(), icon: "file".into(),
            aliases: vec![], on_chain: true,
        });
        assert_eq!(c.mint_id("Raw"), "raw3");
        assert_eq!(c.mint_id("!!!"), "step", "a name with no alphanumerics still yields one");
    }

    #[test]
    fn reordering_moves_one_step_and_leaves_the_rest_in_order() {
        let mut c = StepCatalog::defaults();
        c.reorder(0, 2);
        let ids: Vec<&str> = c.steps.iter().take(3).map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["authentication", "apim", "neo"]);
    }

    #[test]
    fn an_out_of_range_drag_changes_nothing() {
        // A drag ending outside the list is a no-op, not a panic.
        let mut c = StepCatalog::defaults();
        let before = c.clone();
        c.reorder(0, 99);
        c.reorder(99, 0);
        c.reorder(3, 3);
        assert_eq!(c, before);
    }

    #[test]
    fn chain_index_is_none_for_a_branch() {
        let c = StepCatalog::defaults();
        assert_eq!(c.chain_index("raw"), Some(5));
        assert_eq!(c.chain_index("quarantine"), None);
        assert_eq!(c.chain_index("rejected"), None);
    }
}
