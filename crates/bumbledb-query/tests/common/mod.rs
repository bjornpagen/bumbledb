//! Shared integration-test scaffolding.
use std::path::{Path, PathBuf};

pub struct TempDir(PathBuf);

impl TempDir {
    /// The path carries the process id: the test runner gives every test
    /// its own process, so two tests sharing a tag (each corpus builder
    /// replays the whole case table) can never share a store directory.
    pub fn new(tag: &str) -> Self {
        let path =
            std::env::temp_dir().join(format!("bumbledb-query-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        Self(path)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
