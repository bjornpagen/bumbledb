//! [`Db::verify_store`]: the offline sweep of one coherent snapshot, with
//! every key derivation, fingerprint and law taken from the engine itself.

use crate::Db;
use crate::error::{Error, Result, Violations};
use crate::storage::store::VerifyCorruption;
use crate::storage::store::verify::VerifyFinding;
use crate::work::WorkContext;

/// The sweep's findings: physical corruption, and the statements the
/// complete judgment finds violated. Both empty is coherence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreReport {
    pub corruption: Box<[VerifyCorruption]>,
    pub violations: Option<Violations>,
}

impl StoreReport {
    #[must_use]
    pub fn is_coherent(&self) -> bool {
        self.corruption.is_empty() && self.violations.is_none()
    }
}

impl<S> Db<S> {
    /// Sweep one coherent snapshot: O(store), off any hot path.
    /// # Errors
    /// Storage failure or cancellation; never a shortened report.
    pub fn verify_store(&self, work: &WorkContext) -> Result<StoreReport> {
        let snapshot = self.store.snapshot(work).map_err(Error::from_store)?;
        let findings = crate::storage::store::verify::sweep(&snapshot, self.schema(), work)
            .map_err(Error::from_store)?;
        let mut corruption = Vec::new();
        let mut judged = Vec::new();
        for finding in findings {
            match finding {
                VerifyFinding::Corruption(found) => corruption.push(found),
                VerifyFinding::Judgment(violation) => judged.push(violation),
            }
        }
        let violations = if judged.is_empty() {
            None
        } else {
            Some(crate::api::db::violations_from_judged(
                self.schema(),
                judged.into_boxed_slice(),
                work,
            )?)
        };
        Ok(StoreReport {
            corruption: corruption.into_boxed_slice(),
            violations,
        })
    }
}

#[cfg(test)]
mod tests;
