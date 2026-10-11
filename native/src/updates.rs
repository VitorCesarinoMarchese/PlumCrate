use crate::catalog::Catalog;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};
use ts4_mod_manager_core::mod_updates::{UpdateReport, UpdateStatus};

#[derive(Default)]
pub struct UpdateState {
    pub reports: HashMap<String, Result<UpdateReport, String>>,
    pub pending: bool,
    pub epoch: Arc<AtomicU64>,
    scope: Option<(Option<PathBuf>, u64, Option<String>)>,
}

impl UpdateState {
    pub fn sync(&mut self, catalog: &Catalog, saved_key: Option<&str>) {
        let scope = (
            catalog.root.clone(),
            catalog.generation(),
            saved_key.map(str::to_owned),
        );
        if self.scope.as_ref() != Some(&scope) {
            self.invalidate();
            self.scope = Some(scope);
        }
    }
    pub fn invalidate(&mut self) {
        self.epoch.fetch_add(1, Ordering::AcqRel);
        self.pending = false;
        self.reports.clear();
    }
    pub fn begin(&mut self) -> u64 {
        self.invalidate();
        self.pending = true;
        self.epoch.load(Ordering::Acquire)
    }
    pub fn accept(
        &mut self,
        token: u64,
        reports: Vec<(String, Result<UpdateReport, String>)>,
    ) -> bool {
        if !self.pending || self.epoch.load(Ordering::Acquire) != token {
            return false;
        }
        self.pending = false;
        self.reports = reports.into_iter().collect();
        true
    }
    pub fn available(&self, mod_id: &str) -> bool {
        matches!(self.reports.get(mod_id), Some(Ok(report)) if matches!(report.status, UpdateStatus::Available(_)))
    }
    pub fn token(&self) -> u64 {
        self.epoch.load(Ordering::Acquire)
    }
}
