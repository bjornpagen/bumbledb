//! Fresh cancellation contexts and stores for independent benchmark operations.

use std::path::Path;

use bumbledb::schema::Theory;
use bumbledb::{Admission, Db, WorkContext, WriteOutcome};

/// Each operation starts uncancelled, independent of earlier operations.
#[must_use]
pub fn bench_work() -> WorkContext {
    WorkContext::new()
}

/// A fresh store holding the empty state of `schema`.
/// # Errors
pub fn create_db<S: Theory>(path: &Path, schema: S) -> Result<Db<S>, String> {
    match Db::create(path, schema, bench_work()) {
        Err(error) => Err(format!("create {}: {error:?}", path.display())),
        Ok(Admission::Accepted(db)) => Ok(db),
        Ok(Admission::Rejected(violations)) => Err(format!(
            "create {}: empty state rejected: {violations}",
            path.display()
        )),
    }
}

/// The value a write committed, or why it did not commit.
/// # Errors
pub fn committed<R>(what: &str, outcome: bumbledb::Result<WriteOutcome<R>>) -> Result<R, String> {
    match outcome {
        Ok(WriteOutcome::Committed(committed)) => Ok(committed.value),
        Ok(WriteOutcome::Rejected(violations)) => Err(format!("{what} rejected: {violations:?}")),
        Ok(WriteOutcome::Moved { witnessed, current }) => Err(format!(
            "{what}: the store moved from {witnessed:?} to {current:?}"
        )),
        Err(error) => Err(format!("{what}: {error:?}")),
    }
}

/// # Errors
pub fn open_db<S: Theory>(path: &Path, schema: S) -> Result<Db<S>, String> {
    Db::open(path, schema, bench_work())
        .map_err(|error| format!("open {}: {error:?}", path.display()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn cancelled_operation_does_not_poison_the_next_operation() {
        let work = super::bench_work();
        work.checkpoint().unwrap();
        work.cancel();
        assert_eq!(work.checkpoint(), Err(bumbledb::WorkError::Cancelled));
        super::bench_work().checkpoint().unwrap();
    }
}
