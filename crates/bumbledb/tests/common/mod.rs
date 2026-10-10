use std::path::{Path, PathBuf};

use bumbledb::{Admission, Result, Violations, WorkContext, WriteOutcome};

#[allow(dead_code)]
pub fn work() -> WorkContext {
    WorkContext::new()
}

/// A write or an admission that may carry a rejection.
pub trait Rejectable {
    fn rejection(self) -> std::result::Result<Violations, String>;
}

impl<T: std::fmt::Debug> Rejectable for Result<WriteOutcome<T>> {
    fn rejection(self) -> std::result::Result<Violations, String> {
        match self {
            Ok(WriteOutcome::Rejected(violations)) => Ok(violations),
            other => Err(format!("{other:?}")),
        }
    }
}

impl<T: std::fmt::Debug> Rejectable for Result<Admission<T>> {
    fn rejection(self) -> std::result::Result<Violations, String> {
        match self {
            Ok(Admission::Rejected(violations)) => Ok(violations),
            other => Err(format!("{other:?}")),
        }
    }
}

#[allow(dead_code)]
#[track_caller]
pub fn expect_rejected(result: impl Rejectable) -> Violations {
    result
        .rejection()
        .unwrap_or_else(|other| panic!("expected a rejection, got {other}"))
}

#[allow(dead_code)]
#[track_caller]
pub fn expect_admitted<T: std::fmt::Debug>(result: Result<WriteOutcome<T>>) -> T {
    result.expect("write").unwrap().value
}

pub struct TempDir(PathBuf);

impl TempDir {
    pub fn new(tag: &str) -> Self {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let serial = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "bumbledb-it-{tag}-{}-{serial}.bdb",
            std::process::id()
        ));
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
