use std::path::{Path, PathBuf};
use std::time::Duration;

use bumbledb::{Admission, Committed, ExecutionPolicy, Result, Violations, WorkContext};

#[allow(dead_code)]
pub fn work() -> WorkContext {
    ExecutionPolicy {
        input_bytes: 1 << 30,
        working_bytes: 1 << 30,
        scratch_bytes: 1 << 30,
        result_bytes: 1 << 30,
        rows: 1 << 30,
        work_units: 1 << 30,
        timeout: Duration::from_secs(3600),
    }
    .start()
    .expect("work")
}

#[allow(dead_code)]
#[track_caller]
pub fn expect_rejected<T: std::fmt::Debug>(result: Result<Admission<T>>) -> Violations {
    match result {
        Ok(Admission::Rejected(violations)) => violations,
        Ok(Admission::Accepted(_)) => panic!("expected admission rejection, the write admitted"),
        Err(error) => panic!("expected admission rejection, the engine said {error:?}"),
    }
}

#[allow(dead_code)]
#[track_caller]
pub fn expect_admitted<T: std::fmt::Debug>(result: Result<Admission<Committed<T>>>) -> T {
    result.expect("write").unwrap().value
}

pub struct TempDir(PathBuf);

impl TempDir {
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
