use std::path::{Path, PathBuf};
use std::sync::{Arc, Condvar, Mutex, mpsc};
use std::time::Instant;
use plumcrate_core::{mod_scan::ScannedMod, path_detection::GameInstance};

#[derive(Debug)]
pub enum Request {
    Detect {
        generation: u64,
        home: PathBuf,
    },
    Scan {
        generation: u64,
        root: PathBuf,
        managed_root: PathBuf,
    },
}

pub enum Completion {
    Detected {
        generation: u64,
        instances: Vec<GameInstance>,
    },
    Scanned {
        generation: u64,
        result: Result<ScanOutput, String>,
    },
}

pub struct ScanOutput {
    pub instance: GameInstance,
    pub mods: Vec<ScannedMod>,
    pub elapsed_ms: f64,
}

#[derive(Default)]
struct Mailbox {
    pending: Option<Request>,
    closed: bool,
}
impl Mailbox {
    fn replace(&mut self, request: Request) {
        self.pending = Some(request);
    }
    fn take(&mut self) -> Option<Request> {
        self.pending.take()
    }
}

pub fn scan(root: &Path, managed_root: &Path) -> Result<ScanOutput, String> {
    let start = Instant::now();
    let instance = plumcrate_core::path_detection::validate_custom_instance(root)
        .map_err(|error| error.message)?;
    let mods_dir = instance.path.join("Mods");
    std::fs::read_dir(&mods_dir).map_err(|error| format!("Cannot read Mods folder: {error}"))?;
    let mods = plumcrate_core::mod_scan::scan_mods(&mods_dir, managed_root).map_err(|e| e.message)?;
    Ok(ScanOutput {
        instance,
        mods,
        elapsed_ms: start.elapsed().as_secs_f64() * 1000.0,
    })
}

pub struct Jobs {
    mailbox: Arc<(Mutex<Mailbox>, Condvar)>,
    results: mpsc::Receiver<Completion>,
}

impl Jobs {
    pub fn start(wake: impl Fn() + Send + 'static) -> std::io::Result<Self> {
        let mailbox = Arc::new((Mutex::new(Mailbox::default()), Condvar::new()));
        let inbox = Arc::clone(&mailbox);
        let (sender, results) = mpsc::channel();
        std::thread::Builder::new()
            .name("catalog-reader".into())
            .spawn(move || {
                loop {
                    let request = {
                        let (lock, ready) = &*inbox;
                        let mut slot = lock.lock().expect("catalog mailbox");
                        while slot.pending.is_none() && !slot.closed {
                            slot = ready.wait(slot).expect("catalog mailbox");
                        }
                        if slot.closed {
                            break;
                        }
                        slot.take().expect("pending request")
                    };
                    let completion = match request {
                        Request::Detect { generation, home } => Completion::Detected {
                            generation,
                            instances: plumcrate_core::path_detection::detect_game_instances(
                                &home,
                            ),
                        },
                        Request::Scan {
                            generation,
                            root,
                            managed_root,
                        } => Completion::Scanned {
                            generation,
                            result: scan(&root, &managed_root),
                        },
                    };
                    if sender.send(completion).is_err() {
                        break;
                    }
                    wake();
                }
            })?;
        Ok(Self { mailbox, results })
    }

    pub fn submit(&self, request: Request) {
        self.mailbox
            .0
            .lock()
            .expect("catalog mailbox")
            .replace(request);
        self.mailbox.1.notify_one();
    }

    pub fn drain(&self) -> impl Iterator<Item = Completion> + '_ {
        self.results.try_iter()
    }
}

impl Drop for Jobs {
    fn drop(&mut self) {
        let mut mailbox = self.mailbox.0.lock().expect("catalog mailbox");
        mailbox.closed = true;
        mailbox.pending = None;
        self.mailbox.1.notify_one();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_only_latest_pending_whole_request() {
        let mut mailbox = Mailbox::default();
        for generation in [1, 2, 3] {
            mailbox.replace(Request::Scan {
                generation,
                root: format!("root-{generation}").into(),
                managed_root: format!("managed-{generation}").into(),
            });
        }
        let Some(Request::Scan {
            generation,
            root,
            managed_root,
        }) = mailbox.take()
        else {
            panic!("request");
        };
        assert_eq!(generation, 3);
        assert_eq!(root, PathBuf::from("root-3"));
        assert_eq!(managed_root, PathBuf::from("managed-3"));
        assert!(mailbox.take().is_none());
    }

    #[test]
    fn scans_real_files_without_creating_managed_storage() {
        let fixture = tempfile::tempdir().expect("fixture");
        let root = fixture.path().join("The Sims 4");
        let managed = fixture.path().join("absent-managed");
        std::fs::create_dir_all(root.join("Mods/Café")).expect("dirs");
        let file = root.join("Mods/Café/script.ts4script");
        std::fs::write(&file, b"unchanged").expect("file");
        let output = scan(&root, &managed).expect("scan");
        assert_eq!(output.mods.len(), 1);
        assert_eq!(output.mods[0].name, "Café");
        assert_eq!(std::fs::read(file).expect("read"), b"unchanged");
        assert!(!managed.exists());
        assert!(scan(&fixture.path().join("missing"), &managed).is_err());
    }
}
