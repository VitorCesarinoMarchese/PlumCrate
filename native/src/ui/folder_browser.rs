use super::Icon;
use egui::{Context, Ui};
use std::collections::BTreeSet;

pub(super) struct FolderBrowser {
    pub generation: u64,
    // A shared top-level directory is the collection's natural starting point.
    root: String,
    current: String,
    folders: Vec<String>,
    files: Vec<String>,
}

impl FolderBrowser {
    fn root_for(files: &[String]) -> String {
        files
            .first()
            .and_then(|f| f.split_once('/'))
            .map(|(name, _)| format!("{name}/"))
            .filter(|prefix| files.iter().all(|f| f.starts_with(prefix)))
            .unwrap_or_default()
    }

    pub fn open(files: &[String], generation: u64) -> Self {
        let root = Self::root_for(files);
        let mut browser = Self {
            generation,
            current: root.clone(),
            root,
            folders: Vec::new(),
            files: Vec::new(),
        };
        browser.read_folder(files);
        browser
    }

    pub fn refresh(&mut self, files: &[String], generation: u64) {
        self.generation = generation;
        self.root = Self::root_for(files);
        if !self.current.starts_with(&self.root) {
            self.current.clone_from(&self.root);
        }
        while self.current.len() > self.root.len()
            && !files.iter().any(|file| file.starts_with(&self.current))
        {
            self.current.pop();
            let parent_len = self.current.rfind('/').map_or(0, |index| index + 1);
            self.current.truncate(parent_len);
        }
        self.read_folder(files);
    }

    fn read_folder(&mut self, paths: &[String]) {
        let mut folders = BTreeSet::new();
        self.files.clear();
        for child in paths
            .iter()
            .filter_map(|path| path.strip_prefix(&self.current))
        {
            if let Some((folder, _)) = child.split_once('/') {
                folders.insert(folder);
            } else {
                self.files.push(child.to_owned());
            }
        }
        self.folders = folders.into_iter().map(str::to_owned).collect();
        self.files.sort_unstable();
    }

    pub fn show(&mut self, ctx: &Context, title: &str, files: &[String]) -> bool {
        let modal = crate::popup::show_width(ctx, "collection-files", title, 600.0, |ui| {
            let mut destination = if self.current != self.root {
                egui::ScrollArea::horizontal()
                    .id_salt("folder-breadcrumbs")
                    .stick_to_right(true)
                    .show(ui, |ui| self.breadcrumbs(ui))
                    .inner
            } else {
                let root = self.root.trim_end_matches('/');
                let label = if root.is_empty() { "Files" } else { root };
                if label != title {
                    ui.label(label);
                }
                None
            };
            ui.separator();
            let row_height = ui.spacing().interact_size.y;
            egui::ScrollArea::vertical()
                .auto_shrink([false, true])
                .id_salt(("folder-files", &self.current))
                .max_height(crate::popup::body_height(ctx))
                .show_rows(
                    ui,
                    row_height,
                    self.folders.len() + self.files.len(),
                    |ui, rows| {
                        for index in rows {
                            if let Some(folder) = self.folders.get(index) {
                                let button = egui::Button::new((
                                    Icon::Folder.image(ui.visuals().hyperlink_color, 16.0),
                                    folder.as_str(),
                                ))
                                .right_text("›")
                                .frame(false)
                                .gap(ui.spacing().item_spacing.x)
                                .truncate();
                                if ui
                                    .add_sized([ui.available_width(), row_height], button)
                                    .on_hover_text(folder)
                                    .clicked()
                                {
                                    destination = Some(format!("{}{folder}/", self.current));
                                }
                            } else {
                                let name = &self.files[index - self.folders.len()];
                                ui.allocate_ui_with_layout(
                                    egui::vec2(ui.available_width(), row_height),
                                    egui::Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        ui.add(
                                            Icon::File.image(ui.visuals().weak_text_color(), 16.0),
                                        );
                                        ui.add(egui::Label::new(name).truncate().selectable(true))
                                            .on_hover_text(name);
                                    },
                                );
                            }
                        }
                    },
                );
            if let Some(path) = destination {
                self.current = path;
                self.read_folder(files);
            }
            crate::popup::close(ui).clicked()
        });
        modal.inner || modal.should_close()
    }

    fn breadcrumbs(&self, ui: &mut Ui) -> Option<String> {
        let mut destination = None;
        ui.horizontal(|ui| {
            let mut path = self.root.clone();
            let root_label = if path.is_empty() {
                "Files"
            } else {
                path.trim_end_matches('/')
            };
            if ui.link(root_label).clicked() {
                destination = Some(path.clone());
            }
            for folder in self.current[self.root.len()..].split_terminator('/') {
                path.push_str(folder);
                path.push('/');
                ui.label("/");
                if path == self.current {
                    ui.label(folder);
                } else if ui.link(folder).clicked() {
                    destination = Some(path.clone());
                }
            }
        });
        destination
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_keeps_the_nearest_surviving_folder() {
        let paths = |names: &[&str]| names.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        let mut browser = FolderBrowser::open(&paths(&["Clothes/Tops/old.package"]), 0);
        browser.current = "Clothes/Tops/".into();
        for (generation, (file, expected)) in [
            ("Clothes/Tops/new.package", "Clothes/Tops/"),
            ("Clothes/Shoes/boots.package", "Clothes/"),
            ("Hair/short.package", "Hair/"),
        ]
        .into_iter()
        .enumerate()
        {
            browser.refresh(&paths(&[file]), generation as u64 + 1);
            assert_eq!(browser.current, expected);
        }
    }
}
