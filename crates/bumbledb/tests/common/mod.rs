//! Shared integration-test scaffolding: the self-cleaning temp directory
//! — the integration twin of the lib's `testutil::TempDir` (integration
//! tests link bumbledb as an external crate, so the `pub(crate)` helper
//! is out of reach). No external dev-dependency — deps stay exactly
//! heed + blake3.

use std::path::{Path, PathBuf};

use bumbledb::{Admission, Committed, Result, Violations};

/// The theory-rejection payload of an admitted-or-rejected write.
/// Panics on infrastructure error or unexpected acceptance.
#[allow(dead_code)] // each integration binary includes this module; not every binary rejects
#[track_caller]
pub fn expect_rejected<T: std::fmt::Debug>(result: Result<Admission<T>>) -> Violations {
    match result {
        Ok(Admission::Rejected(violations)) => violations,
        Ok(Admission::Accepted(_)) => panic!("expected admission rejection, the write admitted"),
        Err(error) => panic!("expected admission rejection, the engine said {error:?}"),
    }
}

/// The callback value of an admitted write. Panics on rejection or
/// infrastructure error.
#[allow(dead_code)] // each integration binary includes this module; not every binary admits
#[track_caller]
pub fn expect_admitted<T: std::fmt::Debug>(result: Result<Admission<Committed<T>>>) -> T {
    result.expect("write").unwrap().value
}

pub struct TempDir(PathBuf);

impl TempDir {
    /// Creates (or wipes and recreates) a per-test directory. `tag` must
    /// be distinct per test function — across every integration binary,
    /// since cargo runs them in parallel — so tests never collide.
    /// Creates (or wipes) a per-test path. The directory is not
    /// created: `Db::create` refuses an existing destination,
    /// including an empty directory.
    pub fn new(tag: &str) -> Self {
        let path = std::env::temp_dir().join(format!("bumbledb-it-{tag}"));
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
