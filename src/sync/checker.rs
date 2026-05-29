use crate::index;
use anyhow::Result;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

/// Checks index freshness by comparing file mtimes with the index.
pub struct Checker {
    path: String,
    fix: bool,
}

#[derive(Debug)]
pub struct CheckReport {
    pub fresh: bool,
    pub total_files: usize,
    pub stale_files: Vec<StaleFile>,
    pub fixed: bool,
}

#[derive(Debug)]
pub struct StaleFile {
    pub path: String,
    /// Seconds since last index
    pub age_seconds: u64,
}

impl Checker {
    pub fn new(path: &str) -> Self {
        Self { path: path.to_string(), fix: false }
    }

    pub fn fix(mut self, v: bool) -> Self {
        self.fix = v;
        self
    }

    /// Run the freshness check (and optionally re-index stale files).
    pub fn run(&self) -> Result<CheckReport> {
        let root = Path::new(&self.path);
        let index_path = root.join(".codesnap").join("index.bin");

        if !index_path.exists() {
            return Ok(CheckReport {
                fresh: false, total_files: 0, stale_files: Vec::new(), fixed: false,
            });
        }

        let _data = fs::read(&index_path)?;
        let idx = index::open(&self.path)?;

        let mut stale_files = Vec::new();
        let mut indexed_paths = HashSet::new();

        // Check each indexed file against disk mtime
        for (file_path, indexed_mtime) in &idx.file_mtimes {
            indexed_paths.insert(file_path.clone());
            let full_path = root.join(file_path);
            if let Ok(metadata) = fs::metadata(&full_path) {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(duration) = modified.duration_since(UNIX_EPOCH) {
                        let disk_mtime = duration.as_secs();
                        if disk_mtime > *indexed_mtime {
                            let age = disk_mtime - *indexed_mtime;
                            stale_files.push(StaleFile { path: file_path.clone(), age_seconds: age });
                        }
                    }
                }
            }
        }

        let fresh = stale_files.is_empty();
        let total_files = idx.file_mtimes.len();
        let mut fixed = false;

        if !fresh && self.fix {
            // Re-index stale files by rebuilding the index
            index::Builder::new(&self.path).force(true).quiet(true).build()?;
            fixed = true;
        }

        Ok(CheckReport { fresh, total_files, stale_files, fixed })
    }
}
