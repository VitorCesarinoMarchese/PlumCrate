//! Real SIGKILL at transaction boundaries, followed by recovery in a new process.
use super::*;
use std::{
    collections::BTreeMap,
    os::unix::process::ExitStatusExt,
    process::Command,
    time::{Duration, Instant},
};

thread_local! {
    static PAUSE_AFTER: Cell<Option<usize>> = const { Cell::new(None) };
}

pub(super) fn pause_after_side_effect() {
    let pause = PAUSE_AFTER.with(|count| match count.get() {
        Some(1) => {
            count.set(None);
            true
        }
        Some(n) => {
            count.set(Some(n - 1));
            false
        }
        None => false,
    });
    if pause {
        let root = PathBuf::from(std::env::var_os("TS4_CRASH_FIXTURE").unwrap());
        fs::write(root.join("paused"), b"durable side effect reached").unwrap();
        unsafe {
            libc::raise(libc::SIGSTOP);
        }
    }
}

#[test]
#[ignore = "subprocess fixture for sigkill_recovers_import_migration_and_restore"]
fn fixture_child() {
    let root = PathBuf::from(std::env::var_os("TS4_CRASH_FIXTURE").unwrap());
    let managed = root.join("managed");
    let action = std::env::var("TS4_CRASH_ACTION").unwrap();
    if action == "recover" || action == "recover-conflict" {
        for _ in 0..2 {
            let issues = recover_pending(&managed).unwrap();
            assert_eq!(issues.is_empty(), action == "recover", "{issues:?}");
        }
        return;
    }
    PAUSE_AFTER.with(|count| {
        count.set(Some(
            std::env::var("TS4_CRASH_AFTER").unwrap().parse().unwrap(),
        ))
    });
    match action.as_str() {
        "import" => {
            crate::managed_storage::create_managed_mod(
                &managed,
                crate::managed_storage::ImportRequest {
                    name: "Example".into(),
                    slug: None,
                    source_dir: root.join("source"),
                },
            )
            .unwrap();
        }
        "browser" => {
            import_browser_fixture(&root).unwrap();
        }
        "migration" => {
            crate::external_migration::migrate_external_mod(
                &managed,
                &root.join("Mods"),
                "Example",
            )
            .unwrap();
        }
        "restore" | "managed-restore" => {
            let entry = if action == "managed-restore" {
                fs::read_to_string(root.join("restore-entry")).unwrap()
            } else {
                "legacy-123".into()
            };
            crate::lifecycle::restore_trashed_mod(
                &managed,
                &root.join("Mods"),
                &root.join("Trash/files"),
                &entry,
            )
            .unwrap();
        }
        _ => panic!("Unknown fixture operation"),
    }
    panic!("Operation finished without reaching the requested pause");
}

fn import_browser_fixture(
    root: &Path,
) -> Result<crate::managed_storage::ModMetadata, crate::error::ManagerError> {
    use crate::{
        browser_import::{self, DownloadWatch, ImportPermit},
        curseforge_client::{
            CurseForgeClient, CurseForgeRequest, CurseForgeTransport, SourceLookupError,
        },
    };
    use md5::Md5;
    use sha1::{Digest, Sha1};
    struct Provider(serde_json::Value);
    impl CurseForgeTransport for Provider {
        fn get(&self, request: &CurseForgeRequest) -> Result<(u16, String), SourceLookupError> {
            let body = if request.url.ends_with("/42") {
                serde_json::json!({"data":{"id":42,"gameId":78062,"name":"Browser fixture","links":{"websiteUrl":"https://www.curseforge.com/sims4/mods/fixture"}}})
            } else {
                serde_json::json!({"data":[self.0],"pagination":{"index":0,"pageSize":50,"resultCount":1,"totalCount":1}})
            };
            Ok((200, body.to_string()))
        }
    }
    let bytes = fs::read(root.join("source/a.package")).unwrap();
    let file = serde_json::json!({"id":100,"modId":42,"fileName":"a.package","fileDate":"2026-10-01T00:00:00Z","releaseType":1,"isAvailable":true,"fileStatus":4,"gameVersions":[],"fileLength":bytes.len(),"hashes":[{"algo":1,"value":format!("{:x}",Sha1::digest(&bytes))},{"algo":2,"value":format!("{:x}",Md5::digest(&bytes))}]});
    let client = CurseForgeClient::new(Some("fixture"), Provider(file)).unwrap();
    let project = browser_import::load_browser_project(&client, 42)?;
    let release = &project.releases[0];
    let permit = ImportPermit::default();
    let mut watch = DownloadWatch::new(&root.join("source"), release)?;
    assert!(watch.poll(&permit)?.is_none());
    let matched = watch.poll(&permit)?.unwrap();
    browser_import::import_browser_download(
        &root.join("managed"),
        &root.join("Mods"),
        &project,
        release,
        &matched,
        &permit,
    )
}

#[test]
fn browser_import_failures_undo_publication_links_and_ownership_together() {
    for step in 1..=3 {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::create_dir_all(root.join("source")).unwrap();
        fs::create_dir_all(root.join("Mods")).unwrap();
        fs::write(root.join("source/a.package"), b"original browser download").unwrap();
        fs::write(root.join("Mods/unmanaged.package"), b"unmanaged original").unwrap();
        FAILURE_AFTER.with(|count| count.set(Some(step)));
        assert!(import_browser_fixture(root).is_err());
        assert_eq!(
            fs::read(root.join("source/a.package")).unwrap(),
            b"original browser download"
        );
        assert_eq!(
            fs::read(root.join("Mods/unmanaged.package")).unwrap(),
            b"unmanaged original"
        );
        assert!(!fs_scope::exists(&root.join("Mods/a.package")));
        let mods = root.join("managed/mods");
        assert!(fs::read_dir(mods)
            .map(|entries| entries.count() == 0)
            .unwrap_or(true));
        assert!(recover_pending(&root.join("managed")).unwrap().is_empty());
    }
}

fn command(root: &Path, action: &str) -> Command {
    let mut command = Command::new(std::env::current_exe().unwrap());
    command
        .args([
            "--exact",
            "operation::crash_tests::fixture_child",
            "--ignored",
            "--nocapture",
        ])
        .env("TS4_CRASH_FIXTURE", root)
        .env("TS4_CRASH_ACTION", action);
    command
}

struct Child(std::process::Child);
impl Drop for Child {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn snapshot(path: &Path) -> BTreeMap<PathBuf, Vec<u8>> {
    fn visit(root: &Path, path: &Path, entries: &mut BTreeMap<PathBuf, Vec<u8>>) {
        let meta = fs::symlink_metadata(path).unwrap();
        if meta.is_dir() {
            for entry in fs::read_dir(path).unwrap() {
                visit(root, &entry.unwrap().path(), entries);
            }
        } else {
            let bytes = if meta.file_type().is_symlink() {
                use std::os::unix::ffi::OsStrExt;
                let mut bytes = b"symlink:".to_vec();
                bytes.extend(fs::read_link(path).unwrap().as_os_str().as_bytes());
                bytes
            } else {
                fs::read(path).unwrap()
            };
            entries.insert(path.strip_prefix(root).unwrap().into(), bytes);
        }
    }
    let mut entries = BTreeMap::new();
    visit(path, path, &mut entries);
    entries
}

#[test]
fn sigkill_recovers_import_migration_and_restore() {
    for (action, step, replacement) in [
        ("import", 1, false),
        ("browser", 1, false),
        ("browser", 2, false),
        ("browser", 3, false),
        ("migration", 1, false),
        ("migration", 2, false),
        ("migration", 3, false),
        ("migration", 4, false),
        ("migration", 5, false),
        ("restore", 1, false),
        ("restore", 2, false),
        ("restore", 3, false),
        ("managed-restore", 1, false),
        ("managed-restore", 2, false),
        ("migration", 2, true),
    ] {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        for dir in ["source", "Mods", "Trash/files/legacy-123", "Trash/info"] {
            fs::create_dir_all(root.join(dir)).unwrap();
        }
        fs::write(root.join("source/a.package"), b"import original").unwrap();
        fs::write(root.join("outside.package"), b"outside original").unwrap();
        std::os::unix::fs::symlink(
            root.join("outside.package"),
            root.join("Mods/Example_main.package"),
        )
        .unwrap();
        fs::write(root.join("Mods/unmanaged.package"), b"unmanaged original").unwrap();
        fs::write(
            root.join("Trash/files/legacy-123/legacy.package"),
            b"restore original",
        )
        .unwrap();
        fs::write(
            root.join("Trash/info/legacy-123.trashinfo"),
            b"[Trash Info]\nPath=/The Sims 4/Mods/legacy\n",
        )
        .unwrap();
        if action == "managed-restore" {
            let metadata = crate::managed_storage::create_managed_mod(
                &root.join("managed"),
                crate::managed_storage::ImportRequest {
                    name: "Stored".into(),
                    slug: None,
                    source_dir: root.join("source"),
                },
            )
            .unwrap();
            let trashed = crate::lifecycle::uninstall_managed_mod(
                &root.join("managed"),
                &root.join("Mods"),
                &root.join("Trash/files"),
                &metadata.mod_id,
            )
            .unwrap();
            fs::write(
                root.join("restore-entry"),
                Path::new(&trashed.trashed_path)
                    .file_name()
                    .unwrap()
                    .as_encoded_bytes(),
            )
            .unwrap();
        }
        let mut before = ["source", "Mods", "Trash"].map(|dir| snapshot(&root.join(dir)));
        let mut child = Child(
            command(root, action)
                .env("TS4_CRASH_AFTER", step.to_string())
                .spawn()
                .unwrap(),
        );
        let deadline = Instant::now() + Duration::from_secs(10);
        while !root.join("paused").exists() {
            assert!(
                child.0.try_wait().unwrap().is_none(),
                "{action} step {step}: child exited before pause"
            );
            assert!(
                Instant::now() < deadline,
                "{action} step {step}: no pause reached"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        let journals = fs::read_dir(root.join("managed/operations"))
            .unwrap()
            .count();
        assert_eq!(
            journals, 1,
            "{action} step {step}: pending journal required"
        );
        child.0.kill().unwrap();
        assert_eq!(child.0.wait().unwrap().signal(), Some(libc::SIGKILL));
        if replacement {
            fs::write(root.join("Mods/Example_main.package"), b"new user bytes").unwrap();
            before[1].insert(
                PathBuf::from("Example_main.package"),
                b"new user bytes".to_vec(),
            );
        }
        let recovery_action = if replacement {
            "recover-conflict"
        } else {
            "recover"
        };
        let mut recovery = Child(command(root, recovery_action).spawn().unwrap());
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            if let Some(status) = recovery.0.try_wait().unwrap() {
                assert!(status.success(), "{action} step {step}: recovery failed");
                break;
            }
            assert!(
                Instant::now() < deadline,
                "Recovery failed to acquire the dead writer's lock"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        let after = ["source", "Mods", "Trash"].map(|dir| snapshot(&root.join(dir)));
        assert_eq!(
            before, after,
            "{action} step {step}: original bytes and symlinks must survive"
        );
        assert_eq!(
            fs::read(root.join("outside.package")).unwrap(),
            b"outside original"
        );
        assert_eq!(
            fs::read_dir(root.join("managed/operations"))
                .unwrap()
                .filter(|entry| entry
                    .as_ref()
                    .unwrap()
                    .path()
                    .extension()
                    .is_some_and(|ext| ext == "json"))
                .count(),
            usize::from(replacement)
        );
        if action == "migration" && step >= 4 {
            let copies = fs::read_dir(root.join("managed/operations"))
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .filter(|path| {
                    path.file_name()
                        .unwrap()
                        .to_string_lossy()
                        .starts_with(".ts4-record-recovery-")
                })
                .collect::<Vec<_>>();
            assert_eq!(
                copies.len(),
                step - 3,
                "New bookkeeping must remain recoverable outside the rolled-back bundle"
            );
            for path in copies {
                let record: serde_json::Value =
                    serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
                assert!(record["modId"].is_string());
            }
        }
        if replacement {
            let backup = fs::read_dir(root)
                .unwrap()
                .map(|entry| entry.unwrap().path())
                .find(|path| {
                    path.file_name()
                        .unwrap()
                        .to_string_lossy()
                        .starts_with(".ts4-originals-")
                })
                .unwrap();
            assert_eq!(
                fs::read_link(backup.join("Example_main.package")).unwrap(),
                root.join("outside.package")
            );
        }
        let mods = root.join("managed/mods");
        assert!(
            !mods.exists() || fs::read_dir(mods).unwrap().next().is_none(),
            "No interrupted bundle may remain published"
        );
        println!("PASS SIGKILL {action} checkpoint {step}, replacement={replacement}: fresh-process recovery, original bytes, links, trash index, and writer lock");
    }
}
