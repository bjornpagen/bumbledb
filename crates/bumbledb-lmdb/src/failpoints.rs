//! Test-only storage failpoints.

#[cfg(feature = "test-failpoints")]
use std::sync::{Mutex, OnceLock};

use crate::Result;
#[cfg(feature = "test-failpoints")]
use crate::TestError;

/// Named test failpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Failpoint {
    /// Before writing a dictionary entry.
    BeforeDictionaryPut,
    /// After writing dictionary forward/reverse entries.
    AfterDictionaryPut,
    /// After writing a current index entry.
    AfterCurrentIndexPut,
    /// After updating stats metadata.
    AfterStatsUpdate,
    /// After appending a history record.
    AfterHistoryAppend,
    /// Immediately before LMDB commit.
    BeforeCommit,
}

impl Failpoint {
    /// Stable failpoint name.
    #[cfg(feature = "test-failpoints")]
    pub fn name(self) -> &'static str {
        match self {
            Failpoint::BeforeDictionaryPut => "before_dictionary_put",
            Failpoint::AfterDictionaryPut => "after_dictionary_put",
            Failpoint::AfterCurrentIndexPut => "after_current_index_put",
            Failpoint::AfterStatsUpdate => "after_stats_update",
            Failpoint::AfterHistoryAppend => "after_history_append",
            Failpoint::BeforeCommit => "before_commit",
        }
    }
}

#[cfg(feature = "test-failpoints")]
static ACTIVE: OnceLock<Mutex<Option<Failpoint>>> = OnceLock::new();

/// Sets the active failpoint.
#[cfg(feature = "test-failpoints")]
pub fn set(failpoint: Failpoint) {
    *lock_active() = Some(failpoint);
}

/// Clears all failpoints.
#[cfg(feature = "test-failpoints")]
pub fn clear() {
    *lock_active() = None;
}

pub(crate) fn check(failpoint: Failpoint) -> Result<()> {
    #[cfg(feature = "test-failpoints")]
    {
        if *lock_active() == Some(failpoint) {
            Err(TestError::InjectedFailpoint {
                name: failpoint.name(),
            }
            .into())
        } else {
            Ok(())
        }
    }
    #[cfg(not(feature = "test-failpoints"))]
    {
        let _ = failpoint;
        Ok(())
    }
}

#[cfg(feature = "test-failpoints")]
fn active() -> &'static Mutex<Option<Failpoint>> {
    ACTIVE.get_or_init(|| Mutex::new(None))
}

#[cfg(feature = "test-failpoints")]
fn lock_active() -> std::sync::MutexGuard<'static, Option<Failpoint>> {
    active()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}
