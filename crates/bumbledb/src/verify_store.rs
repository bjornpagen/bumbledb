//! [`Db::verify_store`] — the offline sweeper over the successor store:
//! one coherent owned snapshot, one pass per physical namespace, then the
//! complete production judgment re-run globally. Every key derivation,
//! fingerprint and semantic law is imported from the engine's own modules
//! ([`crate::storage::store`] and `schema::judge`) — the sweeper's
//! knowledge is the engine's knowledge, never a second implementation.
//!
//! The dictionary namespaces of the deleted transitional format (`_dict`
//! coherence, dangling-intern statistics, fresh-sequence ratchets) are gone
//! with their mechanisms: live tuple text is owned inline by canonical rows
//! (ENG-006) and no fresh issuance authority exists.

use crate::Db;
use crate::error::Result;

pub use crate::storage::store::verify::VerifyFinding;

/// One observed desync — re-exported sweep finding. Structural facts are
/// [`crate::storage::store::verify::VerifyCorruption`]; semantic facts are
/// the judge's own `JudgedViolation`s.
pub type StoreFinding = VerifyFinding;

/// The sweep's verdict: coherence, or every observed desync as a typed
/// finding. Empty findings are unrepresentable on the desynced arm.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StoreVerdict {
    Coherent,
    Desynced { findings: Box<[StoreFinding]> },
}

/// The sweep report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreReport {
    pub verdict: StoreVerdict,
}

impl StoreReport {
    fn from_sweep(findings: Vec<StoreFinding>) -> Self {
        let verdict = if findings.is_empty() {
            StoreVerdict::Coherent
        } else {
            StoreVerdict::Desynced {
                findings: findings.into_boxed_slice(),
            }
        };
        Self { verdict }
    }

    #[must_use]
    pub fn findings(&self) -> &[StoreFinding] {
        match &self.verdict {
            StoreVerdict::Coherent => &[],
            StoreVerdict::Desynced { findings } => findings,
        }
    }
}

impl<S> Db<S> {
    /// Read-only, one coherent snapshot, O(store) — harness tier, off any
    /// hot path.
    /// # Errors
    /// Storage failure or exhausted work; never a shortened report.
    #[doc(hidden)]
    pub fn verify_store(&self) -> Result<StoreReport> {
        let work = crate::start_operation(crate::ExecutionPolicy {
            input_bytes: 1 << 30,
            working_bytes: 1 << 30,
            scratch_bytes: 1 << 30,
            result_bytes: 1 << 30,
            rows: 1 << 30,
            work_units: 1 << 30,
            timeout: std::time::Duration::from_secs(3600),
        })?;
        let snapshot = self
            .integration_store()
            .snapshot(&work)
            .map_err(crate::error::Error::from_store)?;
        let findings = crate::storage::store::verify::sweep(&snapshot, self.schema(), &work)
            .map_err(crate::error::Error::from_store)?;
        Ok(StoreReport::from_sweep(findings))
    }
}

#[cfg(test)]
mod tests;
