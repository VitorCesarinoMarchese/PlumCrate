use super::{Catalog, Controls, EntryId};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::{self, OpenOptions},
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
struct Book {
    groups: BTreeMap<String, BTreeSet<String>>,
}

impl Book {
    fn load(path: &Path) -> Result<Self, String> {
        match fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|error| format!("Cannot read collections: {error}")),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(format!("Cannot read collections: {error}")),
        }
    }

    fn validate_name(&self, input: &str, previous: Option<&str>) -> Result<String, String> {
        let name = input.trim();
        if name.is_empty() || name.chars().count() > 80 || name.chars().any(char::is_control) {
            return Err("Enter a collection name between 1 and 80 characters.".into());
        }
        let normalized = name.to_lowercase();
        if self
            .groups
            .keys()
            .any(|key| Some(key.as_str()) != previous && key.to_lowercase() == normalized)
        {
            return Err("A collection with that name already exists.".into());
        }
        Ok(name.into())
    }

    fn create(&mut self, input: &str, members: BTreeSet<String>) -> Result<String, String> {
        let name = self.validate_name(input, None)?;
        self.groups.insert(name.clone(), members);
        Ok(name)
    }

    fn rename(&mut self, from: &str, input: &str) -> Result<String, String> {
        let name = self.validate_name(input, Some(from))?;
        let members = self
            .groups
            .remove(from)
            .ok_or("Collection no longer exists.")?;
        self.groups.insert(name.clone(), members);
        Ok(name)
    }

    fn members_mut(&mut self, name: &str) -> Result<&mut BTreeSet<String>, String> {
        self.groups
            .get_mut(name)
            .ok_or_else(|| "Collection no longer exists.".into())
    }
}

enum Edit {
    Create(BTreeSet<String>),
    Rename(String),
    Delete(String),
}

struct CollectionDialog {
    edit: Edit,
    name: String,
    error: Option<String>,
}

pub(super) struct Collections {
    path: PathBuf,
    book: Book,
    active: Option<String>,
    filter_dirty: bool,
    load_error: Option<String>,
    dialog: Option<CollectionDialog>,
}

impl Collections {
    pub(super) fn open(path: PathBuf) -> Self {
        let mut collections = Self {
            path,
            book: Book::default(),
            active: None,
            filter_dirty: false,
            load_error: None,
            dialog: None,
        };
        collections.reload();
        collections
    }

    fn reload(&mut self) {
        match Book::load(&self.path) {
            Ok(book) => {
                self.book = book;
                self.load_error = None;
                self.filter_dirty = true;
            }
            Err(error) => self.load_error = Some(error),
        }
    }

    fn update<T>(
        &mut self,
        edit: impl FnOnce(&mut Book) -> Result<T, String>,
    ) -> Result<T, String> {
        let parent = self.path.parent().expect("collections path has a parent");
        plumcrate_core::fs_scope::create_dir_all(parent).map_err(|error| error.message)?;
        let lock = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(false)
            .open(parent.join("collections.lock"))
            .map_err(|error| format!("Cannot open collections lock: {error}"))?;
        lock.try_lock()
            .map_err(|error| format!("Cannot lock collections: {error}"))?;
        let mut latest = Book::load(&self.path)?;
        if latest != self.book {
            self.book = latest;
            self.filter_dirty = true;
            return Err(
                "Collections changed in another window. Review the updated list and try again."
                    .into(),
            );
        }
        let output = edit(&mut latest)?;
        let bytes = serde_json::to_vec_pretty(&latest).map_err(|error| error.to_string())?;
        plumcrate_core::fs_scope::atomic_write(&self.path, &bytes)
            .map_err(|error| format!("Cannot save collections: {}", error.message))?;
        self.book = latest;
        self.filter_dirty = true;
        Ok(output)
    }

    pub(super) fn dialog_open(&self) -> bool {
        self.dialog.is_some()
    }

    fn apply_filter(&mut self, catalog: &mut Catalog) {
        if self.filter_dirty {
            let members = self
                .active
                .as_ref()
                .and_then(|name| self.book.groups.get(name));
            if members.is_none() {
                self.active = None;
            }
            catalog.set_collection_filter(members.cloned());
            self.filter_dirty = false;
        }
    }

    fn edit(&mut self, edit: Edit, name: String) {
        self.dialog = Some(CollectionDialog {
            edit,
            name,
            error: None,
        });
    }
}

impl Controls {
    pub(crate) fn clear_collection_filter(&mut self, catalog: &mut Catalog) {
        self.collections.active = None;
        self.collections.filter_dirty = true;
        self.collections.apply_filter(catalog);
    }

    pub(crate) fn collection_toolbar(&mut self, ui: &mut egui::Ui, catalog: &mut Catalog) {
        self.collections.apply_filter(catalog);
        if let Some(error) = &self.collections.load_error {
            ui.label(error);
            if ui.button("Reload collections").clicked() {
                self.collections.reload();
            }
            return;
        }
        ui.horizontal_wrapped(|ui| {
            let previous = self.collections.active.clone();
            egui::ComboBox::from_id_salt("library-collections")
                .selected_text(previous.as_deref().unwrap_or("Collections"))
                .width(150.0)
                .wrap_mode(egui::TextWrapMode::Truncate)
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.collections.active, None, "Entire library");
                    for name in self.collections.book.groups.keys() {
                        ui.selectable_value(
                            &mut self.collections.active,
                            Some(name.clone()),
                            name.as_str(),
                        );
                    }
                });
            if self.collections.active != previous {
                self.collections.filter_dirty = true;
                self.collections.apply_filter(catalog);
            }
            if ui.button("New collection").clicked() {
                self.collections
                    .edit(Edit::Create(BTreeSet::new()), String::new());
            }
            if let Some(name) = self.collections.active.clone() {
                let mut toggle = None;
                ui.menu_button("Manage collection", |ui| {
                    if ui.button("Rename…").clicked() {
                        self.collections
                            .edit(Edit::Rename(name.clone()), name.clone());
                        ui.close();
                    }
                    let ready = !catalog.loading && catalog.root.is_some() && !self.busy();
                    if let Some(ids) = self.collections.book.groups.get(&name) {
                        for (enabled, label) in
                            [(true, "Enable collection"), (false, "Disable collection")]
                        {
                            let actionable = catalog.mods.iter().any(|entry| {
                                crate::catalog::needs_toggle(entry, enabled)
                                    && entry.id.as_ref().is_some_and(|id| ids.contains(id))
                            });
                            if ui
                                .add_enabled(ready && actionable, egui::Button::new(label))
                                .clicked()
                            {
                                let mod_ids = catalog
                                    .mods
                                    .iter()
                                    .filter(|entry| crate::catalog::needs_toggle(entry, enabled))
                                    .filter_map(|entry| {
                                        entry.id.as_ref().filter(|id| ids.contains(*id)).cloned()
                                    })
                                    .collect();
                                toggle = Some((enabled, mod_ids));
                                ui.close();
                                break;
                            }
                        }
                    }
                    if ui.button("Delete collection…").clicked() {
                        self.collections.edit(Edit::Delete(name.clone()), name);
                        ui.close();
                    }
                });
                if let Some((enabled, mod_ids)) = toggle {
                    self.submit_toggle_batch(catalog, enabled, mod_ids);
                }
            }
        });
    }

    pub(crate) fn collection_selection(&mut self, ui: &mut egui::Ui, catalog: &mut Catalog) {
        if self.collections.load_error.is_some() || catalog.marked.is_empty() {
            return;
        }
        let has_managed = catalog
            .marked
            .iter()
            .any(|id| matches!(id, EntryId::Managed(_)));
        let members = || {
            catalog
                .marked
                .iter()
                .filter_map(|id| match id {
                    EntryId::Managed(id) => Some(id.clone()),
                    EntryId::External(_) => None,
                })
                .collect::<BTreeSet<_>>()
        };
        let mut add_to = None;
        ui.add_enabled_ui(has_managed, |ui| {
            ui.menu_button("Add to collection", |ui| {
                if ui.button("New collection…").clicked() {
                    self.collections
                        .edit(Edit::Create(members()), String::new());
                    ui.close();
                }
                egui::ScrollArea::vertical()
                    .max_height(240.0)
                    .show(ui, |ui| {
                        for name in self.collections.book.groups.keys() {
                            if ui.button(name.as_str()).clicked() {
                                add_to = Some(name.clone());
                                ui.close();
                            }
                        }
                    });
            });
        });
        if !has_managed {
            ui.label("Move external mods into managed storage to add them to collections.");
        }
        let result = if let Some(name) = add_to {
            Some(self.collections.update(|book| {
                book.members_mut(&name)?.extend(members());
                Ok(format!("Added to {name}"))
            }))
        } else if let Some(name) = self.collections.active.clone()
            && ui.button("Remove from collection").clicked()
        {
            let removed = members();
            Some(self.collections.update(|book| {
                book.members_mut(&name)?.retain(|id| !removed.contains(id));
                Ok(format!("Removed from {name}"))
            }))
        } else {
            None
        };
        match result {
            Some(Ok(notice)) => {
                self.notice = Some(notice);
                self.collections.apply_filter(catalog);
            }
            Some(Err(error)) => self.error = Some(error),
            None => {}
        }
    }

    pub(super) fn show_collection_dialog(&mut self, ctx: &egui::Context) -> bool {
        let Some(mut dialog) = self.collections.dialog.take() else {
            return false;
        };
        let (title, approve) = match &dialog.edit {
            Edit::Create(_) => ("New collection", "Create"),
            Edit::Rename(_) => ("Rename collection", "Rename"),
            Edit::Delete(_) => ("Delete collection", "Delete collection"),
        };
        let popup = crate::popup::show(ctx, "edit-library-collection", title, |ui| {
            if let Edit::Delete(name) = &dialog.edit {
                ui.label(format!(
                    "Delete “{name}”? Its mods will stay in your Library."
                ));
            } else {
                crate::popup::text_field(ui, "Collection name", &mut dialog.name);
                if let Edit::Create(members) = &dialog.edit
                    && !members.is_empty()
                {
                    ui.label(format!("{} selected managed mods", members.len()));
                }
            }
            if let Some(error) = &dialog.error {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
            let (approve, cancel) = crate::popup::actions(ui, approve, true);
            (approve.clicked(), cancel.clicked())
        });
        let (approved, cancelled) = popup.inner;
        if approved {
            let result = self.collections.update(|book| match &dialog.edit {
                Edit::Create(ids) => book.create(&dialog.name, ids.clone()).map(Some),
                Edit::Rename(from) => book.rename(from, &dialog.name).map(Some),
                Edit::Delete(name) => book
                    .groups
                    .remove(name)
                    .map(|_| None)
                    .ok_or_else(|| "Collection no longer exists.".into()),
            });
            match result {
                Ok(active) => {
                    self.collections.active = active;
                    self.collections.filter_dirty = true;
                    ctx.request_repaint();
                    return true;
                }
                Err(error) => dialog.error = Some(error),
            }
        }
        if !cancelled && !popup.should_close() {
            self.collections.dialog = Some(dialog);
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collections_persist_membership_and_reject_stale_writes() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("collections.json");
        let mut first = Collections::open(path.clone());
        let mut second = Collections::open(path.clone());
        let ids = BTreeSet::from(["one".to_owned(), "two".to_owned()]);
        first
            .update(|book| book.create("  Favorites ", ids.clone()))
            .unwrap();
        assert!(
            second
                .update(|book| book.create("Stale", BTreeSet::new()))
                .is_err()
        );
        second
            .update(|book| book.rename("Favorites", "Weekend"))
            .unwrap();
        assert_eq!(Collections::open(path).book.groups["Weekend"], ids);
    }

    #[test]
    fn names_are_unique_and_membership_is_a_set() {
        let mut book = Book::default();
        book.create("Favorites", BTreeSet::from(["one".into()]))
            .unwrap();
        assert!(book.create("favorites", BTreeSet::new()).is_err());
        assert!(book.create("   ", BTreeSet::new()).is_err());
        book.members_mut("Favorites")
            .unwrap()
            .extend(["one".to_owned(), "two".to_owned()]);
        book.members_mut("Favorites").unwrap().remove("one");
        assert_eq!(book.groups["Favorites"], BTreeSet::from(["two".into()]));
    }
}
