//! Test-only storage failpoints.

use std::sync::{Mutex, OnceLock};

use crate::{Error, Result};

/// Named test failpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failpoint {
    /// Before writing a dictionary entry.
    BeforeDictionaryPut,
    /// After writing dictionary forward/reverse entries.
    AfterDictionaryPut,
    /// After writing a current row record.
    AfterCurrentRowPut,
    /// After writing a current index entry.
    AfterCurrentIndexPut,
    /// After writing a unique guard.
    AfterUniqueGuardPut,
    /// After updating stats metadata.
    AfterStatsUpdate,
    /// After appending a history record.
    AfterHistoryAppend,
    /// Immediately before LMDB commit.
    BeforeCommit,
}

impl Failpoint {
    /// Stable failpoint name.
    pub fn name(self) -> &'static str {
        match self {
            Failpoint::BeforeDictionaryPut => "before_dictionary_put",
            Failpoint::AfterDictionaryPut => "after_dictionary_put",
            Failpoint::AfterCurrentRowPut => "after_current_row_put",
            Failpoint::AfterCurrentIndexPut => "after_current_index_put",
            Failpoint::AfterUniqueGuardPut => "after_unique_guard_put",
            Failpoint::AfterStatsUpdate => "after_stats_update",
            Failpoint::AfterHistoryAppend => "after_history_append",
            Failpoint::BeforeCommit => "before_commit",
        }
    }
}

static ACTIVE: OnceLock<Mutex<Option<Failpoint>>> = OnceLock::new();

/// Sets the active failpoint.
#[allow(dead_code)]
pub fn set(failpoint: Failpoint) {
    *active().lock().unwrap() = Some(failpoint);
}

/// Clears all failpoints.
#[allow(dead_code)]
pub fn clear() {
    *active().lock().unwrap() = None;
}

pub(crate) fn check(failpoint: Failpoint) -> Result<()> {
    if *active().lock().unwrap() == Some(failpoint) {
        Err(Error::InjectedFailpoint {
            name: failpoint.name(),
        })
    } else {
        Ok(())
    }
}

fn active() -> &'static Mutex<Option<Failpoint>> {
    ACTIVE.get_or_init(|| Mutex::new(None))
}
