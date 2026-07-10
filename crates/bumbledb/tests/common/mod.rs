//! Shared integration-test scaffolding: the self-cleaning temp directory
//! — the integration twin of the lib's `testutil::TempDir` (integration
//! tests link bumbledb as an external crate, so the `pub(crate)` helper
//! is out of reach). No external dev-dependency — deps stay exactly
//! heed + blake3.

use std::path::{Path, PathBuf};

pub struct TempDir(PathBuf);

impl TempDir {
    /// Creates (or wipes and recreates) a per-test directory. `tag` must
    /// be distinct per test function — across every integration binary,
    /// since cargo runs them in parallel — so tests never collide.
    pub fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!("bumbledb-it-{tag}"));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create test dir");
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
