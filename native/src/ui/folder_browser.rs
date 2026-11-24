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
    pub fn open(files: &[String], generation: u64) -> Self {
        let root = files
            .first()
            .and_then(|f| f.split_once('/'))
            .map(|(name, _)| format!("{name}/"))
            .filter(|prefix| files.iter().all(|f| f.starts_with(prefix)))
            .unwrap_or_default();
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
            let mut destination = self.breadcrumbs(ui);
            ui.separator();
            egui::ScrollArea::vertical()
                .id_salt(("folder-files", &self.current))
                .max_height(crate::popup::body_height(ctx))
                .show_rows(
                    ui,
                    28.0,
                    self.folders.len() + self.files.len(),
                    |ui, rows| {
                        for index in rows {
                            if let Some(folder) = self.folders.get(index) {
                                let button = egui::Button::image_and_text(
                                    Icon::Folder.image(ui.visuals().text_color(), 16.0),
                                    folder,
                                )
                                .truncate();
                                if ui
                                    .add_sized([ui.available_width(), 28.0], button)
                                    .on_hover_text(folder)
                                    .clicked()
                                {
                                    destination = Some(format!("{}{folder}/", self.current));
                                }
                            } else {
                                let name = &self.files[index - self.folders.len()];
                                ui.allocate_ui_with_layout(
                                    egui::vec2(ui.available_width(), 28.0),
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
        ui.horizontal_wrapped(|ui| {
            let mut path = self.root.clone();
            let root_label = if path.is_empty() {
                "Mods"
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
                if ui.link(folder).clicked() {
                    destination = Some(path.clone());
                }
            }
        });
        destination
    }
}
