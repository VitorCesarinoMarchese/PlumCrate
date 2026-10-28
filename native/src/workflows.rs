use crate::{
    catalog::EntryId,
    controls::Controls,
    management::{Action, Output},
    settings::Settings,
    worker,
};
use std::{io::Write, path::PathBuf};
struct Fixture {
    controls: Controls,
    managed: PathBuf,
    root: PathBuf,
    next: u64,
}
impl Fixture {
    fn run(&mut self, entry: Option<EntryId>, action: Action) -> Result<Output, String> {
        self.next += 1;
        self.controls
            .fixture_action(self.root.clone(), entry, action)
    }

    fn toggle(&mut self, id: &str, enabled: bool) -> Result<(), String> {
        let entry = Some(EntryId::Managed(id.into()));
        self.run(entry, Action::Toggle { enabled })?;
        Ok(())
    }
}
pub fn verify() -> Result<serde_json::Value, String> {
    let directory = tempfile::tempdir().map_err(|e| e.to_string())?;
    let base = directory.path();
    let root = base.join("game-a");
    let root_b = base.join("game-b");
    let source = base.join("source");
    for path in [root.join("Mods"), root_b.join("Mods"), source.clone()] {
        std::fs::create_dir_all(path).map_err(|e| e.to_string())?;
    }
    let bytes = b"fixture original bytes";
    std::fs::write(source.join("sample.package"), bytes).map_err(|e| e.to_string())?;
    let mut fixture = Fixture {
        controls: Controls::new(
            base.join("managed"),
            base.to_path_buf(),
            Settings::default(),
            || {},
        )
        .map_err(|e| e.to_string())?,
        managed: base.join("managed"),
        root: root.clone(),
        next: 1,
    };
    fixture.run(
        None,
        Action::ImportFolder {
            path: source.clone(),
            name: "Fixture folder".into(),
        },
    )?;
    let catalog = worker::scan(&root, &fixture.managed)?;
    let id = catalog
        .mods
        .iter()
        .find_map(|m| m.id.clone())
        .ok_or("Folder import missing")?;
    if !catalog.mods.iter().any(|entry| entry.id.as_deref() == Some(&id) && entry.enabled) {
        return Err("Folder import did not start enabled".into());
    }
    fixture.root = root_b.clone();
    fixture.toggle(&id, true)?;
    fixture.root = root.clone();
    fixture.toggle(&id, false)?;
    let enabled_b = worker::scan(&root_b, &fixture.managed)?
        .mods
        .into_iter()
        .any(|m| m.id.as_deref() == Some(&id) && m.enabled);
    if !enabled_b {
        return Err("Disabling first root changed second root".into());
    }
    fixture.root = root_b.clone();
    fixture.toggle(&id, false)?;
    fixture.root = root.clone();
    fixture.run(
        Some(EntryId::Managed(id.clone())),
        Action::Rename("Renamed fixture".into()),
    )?;
    fixture.run(
        Some(EntryId::Managed(id.clone())),
        Action::ManualSource("https://example.org/fixture".into()),
    )?;
    if worker::scan(&root, &fixture.managed)?
        .mods
        .iter()
        .find(|m| m.id.as_deref() == Some(&id))
        .and_then(|m| m.source_url.as_deref())
        != Some("https://example.org/fixture")
    {
        return Err("Manual source did not refresh".into());
    }
    fixture.run(Some(EntryId::Managed(id.clone())), Action::RemoveSource)?;
    match fixture.run(Some(EntryId::Managed(id.clone())), Action::Lookup(None)) {
        Err(error) if error.contains("SOURCE_MISSING_API_KEY") => {}
        _ => return Err("Missing API key must report SOURCE_MISSING_API_KEY".into()),
    }

    fixture.run(Some(EntryId::Managed(id.clone())), Action::Trash)?;
    let Output::Trash(entries) = fixture.run(None, Action::ListTrash)? else {
        return Err("Missing trash list".into());
    };
    let name = entries
        .first()
        .ok_or("Missing trashed fixture")?
        .name
        .clone();
    fixture.run(None, Action::Restore(name))?;
    let archive = base.join("fixture.zip");
    let mut zip = zip::ZipWriter::new(std::fs::File::create(&archive).map_err(|e| e.to_string())?);
    zip.start_file("zip.package", zip::write::SimpleFileOptions::default())
        .map_err(|e| e.to_string())?;
    zip.write_all(bytes).map_err(|e| e.to_string())?;
    zip.finish().map_err(|e| e.to_string())?;
    fixture.run(
        None,
        Action::ImportZip {
            path: archive,
            name: "Fixture ZIP".into(),
        },
    )?;
    if !worker::scan(&root, &fixture.managed)?.mods.iter().any(|entry| entry.files == ["zip.package"] && entry.enabled) {
        return Err("ZIP import did not start enabled".into());
    }
    std::fs::create_dir_all(root.join("Mods/External")).map_err(|e| e.to_string())?;
    std::fs::write(root.join("Mods/External/external.package"), bytes)
        .map_err(|e| e.to_string())?;
    let external = worker::scan(&root, &fixture.managed)?
        .mods
        .into_iter()
        .find(|m| m.id.is_none())
        .ok_or("Missing external fixture")?;
    fixture.run(
        Some(EntryId::External(external.key)),
        Action::Migrate {
            files: external.files,
        },
    )?;
    let catalog = worker::scan(&root, &fixture.managed)?;
    if catalog.mods.len() != 3 {
        return Err(format!(
            "Expected 3 refreshed mods, found {}",
            catalog.mods.len()
        ));
    }
    if std::fs::read(source.join("sample.package")).map_err(|e| e.to_string())? != bytes {
        return Err("Import altered source bytes".into());
    }
    for entry in &catalog.mods {
        let metadata = plumcrate_core::managed_storage::read_managed_mod(
            &fixture.managed,
            entry.id.as_deref().ok_or("Unmanaged result")?,
        )
        .map_err(|e| e.message)?;
        for file in &metadata.files {
            let path = fixture
                .managed
                .join("mods")
                .join(entry.id.as_ref().ok_or("Unmanaged result")?)
                .join("files")
                .join(file);
            if std::fs::read(path).map_err(|e| e.to_string())? != bytes {
                return Err("Managed content bytes changed".into());
            }
        }
    }
    Ok(
        serde_json::json!({"passed":true,"workflows":["folder import","ZIP import","direct two-instance enable and disable","external migration","rename","confirmed manual source attach and remove","trash list and restore","missing API key error","refreshed catalog","source and managed byte preservation"],"mods":catalog.mods.len(),"operations":fixture.next-1,"fixturePathsOnly":true}),
    )
}
