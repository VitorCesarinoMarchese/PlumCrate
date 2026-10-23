use std::{collections::BTreeMap, path::PathBuf};
use ts4_mod_manager_core::mod_scan::ScannedMod;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum EntryId {
    Managed(String),
    External(String),
}

impl EntryId {
    pub fn of(entry: &ScannedMod) -> Self {
        match &entry.id {
            Some(id) => Self::Managed(id.clone()),
            None => Self::External(entry.key.clone()),
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModFilter {
    #[default]
    All,
    Installed,
    Stored,
}

impl ModFilter {
    pub fn label(self) -> &'static str {
        match self {
            Self::All => "All mods",
            Self::Installed => "Installed",
            Self::Stored => "Stored",
        }
    }
    pub fn matches(self, entry: &ScannedMod) -> bool {
        match self {
            Self::All => true,
            Self::Installed => entry.enabled,
            Self::Stored => !entry.enabled,
        }
    }
}

#[derive(Default)]
pub struct Catalog {
    pub mods: Vec<ScannedMod>,
    pub visible: Vec<usize>,
    pub selected: Option<EntryId>,
    pub loading: bool,
    pub error: Option<String>,
    pub root: Option<PathBuf>,
    pub query: String,
    pub installation_filter: ModFilter,
    generation: u64,
    search_keys: Vec<String>,
    curseforge_projects: BTreeMap<u64, EntryId>,
}

impl Catalog {
    pub fn generation(&self) -> u64 {
        self.generation
    }

    pub fn begin_scan(&mut self, root: PathBuf) -> u64 {
        self.generation += 1;
        if self.root.as_ref() != Some(&root) {
            self.mods.clear();
            self.curseforge_projects.clear();
            self.search_keys.clear();
            self.visible.clear();
            self.selected = None;
        }
        self.root = Some(root);
        self.loading = true;
        self.error = None;
        self.generation
    }

    pub fn accept_scan(
        &mut self,
        generation: u64,
        result: Result<Vec<ScannedMod>, String>,
    ) -> bool {
        if generation != self.generation {
            return false;
        }
        self.loading = false;
        match result {
            Ok(mut mods) => {
                mods.sort_by_cached_key(|entry| entry.name.to_lowercase());
                self.search_keys = mods
                    .iter()
                    .map(|entry| {
                        std::iter::once(entry.name.as_str())
                            .chain(entry.files.iter().map(String::as_str))
                            .collect::<Vec<_>>()
                            .join("\n")
                            .to_lowercase()
                    })
                    .collect();
                self.mods = mods;
                self.curseforge_projects.clear();
                for entry in &self.mods {
                    let Some(source) = entry.source_attachment.as_ref().filter(|source| {
                        source.provider_id == "curseforge"
                            && source.attached_by == "user"
                            && entry.source_url.as_deref() == Some(source.source_url.as_str())
                    }) else {
                        continue;
                    };
                    if let Some(project_id) = source.project_id.filter(|id| *id > 0) {
                        self.curseforge_projects
                            .entry(project_id)
                            .or_insert_with(|| EntryId::of(entry));
                    }
                }
                if self.selected_mod().is_none() {
                    self.selected = None;
                }
                self.error = None;
                self.filter();
            }
            Err(error) => self.error = Some(error),
        }
        true
    }

    pub fn set_enabled(&mut self, mod_id: &str, enabled: bool) {
        if let Some(entry) = self.mods.iter_mut().find(|entry| entry.id.as_deref() == Some(mod_id)) {
            entry.enabled = enabled;
            self.filter();
        }
    }

    pub fn search(&mut self, query: String) {
        if self.query != query {
            self.query = query;
            self.filter();
        }
    }

    pub fn set_filter(&mut self, filter: ModFilter) {
        if self.installation_filter != filter {
            self.installation_filter = filter;
            self.filter();
        }
    }

    fn filter(&mut self) {
        let query = self.query.trim().to_lowercase();
        self.visible = self
            .search_keys
            .iter()
            .enumerate()
            .filter_map(|(index, text)| {
                (text.contains(&query) && self.installation_filter.matches(&self.mods[index]))
                    .then_some(index)
            })
            .collect();
    }

    pub fn select(&mut self, index: usize) {
        self.selected = self.mods.get(index).map(EntryId::of);
    }

    pub fn selected_mod(&self) -> Option<&ScannedMod> {
        let selected = self.selected.as_ref()?;
        self.mods
            .iter()
            .find(|entry| EntryId::of(entry) == *selected)
    }

    pub fn curseforge_entry(&self, project_id: u64) -> Option<&EntryId> {
        self.curseforge_projects.get(&project_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ts4_mod_manager_core::mod_scan::ModSource;

    fn entry(name: &str, id: Option<&str>) -> ScannedMod {
        serde_json::from_value(serde_json::json!({
            "key": name, "id": id, "name": name, "files": [format!("{name}/script.ts4script")],
            "mod_files": [format!("{name}/script.ts4script")], "preview": null,
            "source": if id.is_some() { "managed" } else { "external" },
            "group_path": [name], "enabled": true
        }))
        .expect("fixture")
    }

    #[test]
    fn enabled_patch_preserves_catalog_state_and_reapplies_the_active_filter() {
        let mut catalog = Catalog::default();
        let generation = catalog.begin_scan("game".into());
        catalog.accept_scan(generation, Ok(vec![entry("First", Some("one")), entry("Second", Some("two"))]));
        catalog.select(1);
        catalog.search("script".into());
        catalog.set_filter(ModFilter::Installed);
        let untouched = catalog.mods[1].clone();
        let selected = catalog.selected.clone();
        catalog.set_enabled("one", false);
        assert_eq!(catalog.visible, vec![1]);
        assert_eq!(catalog.mods[1], untouched);
        assert_eq!(catalog.selected, selected);
        assert_eq!(catalog.query, "script");
        assert_eq!(catalog.generation(), generation);
        assert!(!catalog.loading);
        catalog.set_filter(ModFilter::Stored);
        assert_eq!(catalog.visible, vec![0]);
        catalog.set_enabled("one", true);
        assert!(catalog.visible.is_empty());
    }

    #[test]
    fn late_scan_and_error_cannot_replace_current_instance_or_loading_state() {
        let mut catalog = Catalog::default();
        let a = catalog.begin_scan("A".into());
        let b = catalog.begin_scan("B".into());
        assert!(!catalog.accept_scan(a, Err("old failure".into())));
        assert!(catalog.loading);
        assert!(catalog.error.is_none());
        assert!(catalog.accept_scan(b, Ok(vec![entry("B", None)])));
        assert!(!catalog.accept_scan(a, Ok(vec![entry("A", None)])));
        assert_eq!(catalog.mods[0].name, "B");
        assert!(!catalog.loading);
    }

    #[test]
    fn repeated_scan_of_same_instance_uses_new_generation_and_preserves_selection() {
        let mut catalog = Catalog::default();
        let a = catalog.begin_scan("A".into());
        catalog.accept_scan(a, Ok(vec![entry("First", Some("id"))]));
        catalog.select(0);
        let b = catalog.begin_scan("A".into());
        assert_ne!(a, b);
        assert!(!catalog.accept_scan(a, Ok(vec![])));
        catalog.accept_scan(b, Ok(vec![entry("Renamed", Some("id"))]));
        assert_eq!(catalog.selected_mod().expect("selection").name, "Renamed");
        catalog.begin_scan("B".into());
        assert!(catalog.mods.is_empty());
        assert!(catalog.selected.is_none());
    }

    #[test]
    fn search_matches_unicode_and_filenames_without_changing_selection() {
        let mut catalog = Catalog::default();
        let generation = catalog.begin_scan("A".into());
        catalog.accept_scan(
            generation,
            Ok(vec![entry("Café", None), entry("Other", None)]),
        );
        catalog.select(0);
        catalog.search("CAFÉ".into());
        assert_eq!(catalog.visible, vec![0]);
        catalog.search("ts4script".into());
        assert_eq!(catalog.visible.len(), 2);
        catalog.search("missing".into());
        assert!(catalog.visible.is_empty());
        assert_eq!(catalog.selected_mod().expect("selection").name, "Café");
    }

    #[test]
    fn managed_and_external_ids_with_same_text_stay_distinct() {
        let mut catalog = Catalog::default();
        let generation = catalog.begin_scan("A".into());
        catalog.accept_scan(
            generation,
            Ok(vec![entry("same", Some("same")), entry("same", None)]),
        );
        catalog.select(1);
        assert_eq!(catalog.selected, Some(EntryId::External("same".into())));
        assert_eq!(
            catalog.selected_mod().expect("selection").source,
            ModSource::External
        );
    }

    #[test]
    fn installation_filter_intersects_search_and_preserves_selection() {
        let mut catalog = Catalog::default();
        let generation = catalog.begin_scan("A".into());
        let installed = entry("Café outfit", Some("a"));
        let mut stored = entry("Café furniture", Some("b"));
        stored.enabled = false;
        catalog.accept_scan(generation, Ok(vec![installed, stored]));
        catalog.select(0);
        let selected = catalog.selected.clone();
        catalog.set_filter(ModFilter::Stored);
        catalog.search("CAFÉ".into());
        assert_eq!(catalog.visible.len(), 1);
        assert!(!catalog.mods[catalog.visible[0]].enabled);
        catalog.set_filter(ModFilter::Installed);
        assert_eq!(catalog.visible.len(), 1);
        assert!(catalog.mods[catalog.visible[0]].enabled);
        assert_eq!(catalog.selected, selected);
        catalog.search("furniture".into());
        assert!(catalog.visible.is_empty());
        catalog.set_filter(ModFilter::All);
        assert_eq!(catalog.visible.len(), 1);
    }
}
