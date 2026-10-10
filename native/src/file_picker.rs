//! Desktop-owned file dialogs. Selection never mutates managed files.
#[cfg(not(test))]
use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
};

pub enum Kind {
    GameFolder,
    ImportFolder,
    ImportZip,
    SettingsImport,
    SettingsExport,
}

#[cfg(not(test))]
pub fn open(
    kind: Kind,
    initial: PathBuf,
    ctx: egui::Context,
) -> std::io::Result<Receiver<Option<PathBuf>>> {
    let (sender, receiver) = mpsc::channel();
    std::thread::Builder::new()
        .name("desktop-file-picker".into())
        .spawn(move || {
            let dialog = rfd::FileDialog::new().set_directory(initial);
            let path = match kind {
                Kind::GameFolder => dialog
                    .set_title("Choose The Sims 4 folder containing Mods")
                    .pick_folder(),
                Kind::ImportFolder => dialog
                    .set_title("Choose a mod folder to import")
                    .pick_folder(),
                Kind::ImportZip => dialog
                    .set_title("Choose a mod ZIP to import")
                    .add_filter("ZIP archives", &["zip"])
                    .pick_file(),
                Kind::SettingsImport => dialog
                    .set_title("Choose settings to import")
                    .add_filter("JSON settings", &["json"])
                    .pick_file(),
                Kind::SettingsExport => dialog
                    .set_title("Export settings to a new file")
                    .add_filter("JSON settings", &["json"])
                    .set_file_name("ts4-mod-manager-settings.json")
                    .save_file(),
            };
            let _ = sender.send(path);
            ctx.request_repaint();
        })?;
    Ok(receiver)
}
