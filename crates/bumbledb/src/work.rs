//! Cooperative cancellation for one operation.
//!
//! Clones share the cancellation flag. Allocation uses ordinary Rust owners;
//! this context imposes no byte, row, work, or elapsed-time allowance.
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

mod clock;
pub(crate) use clock::AdmissionStamp;

/// An operation stopped by its caller, or a capacity that cannot be allocated.
/// Fallible growth does not promise recovery from process-wide memory exhaustion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkError {
    Cancelled,
    Allocation,
}

impl std::fmt::Display for WorkError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => f.write_str("operation cancelled"),
            Self::Allocation => f.write_str("allocation capacity unavailable"),
        }
    }
}

impl std::error::Error for WorkError {}

/// Sendable cancellation context shared by queue admission and execution.
/// Cancellation requests cooperative stopping, not rollback of a published
/// durable decision. Cleanup does not depend on the cancelled context.
#[derive(Debug, Clone, Default)]
pub struct WorkContext(Arc<AtomicBool>);

impl WorkContext {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    /// Observe a caller's cancellation request at a cooperative stopping point.
    /// # Errors
    /// Returns [`WorkError::Cancelled`] after cancellation.
    pub fn checkpoint(&self) -> Result<(), WorkError> {
        if self.0.load(Ordering::Acquire) {
            Err(WorkError::Cancelled)
        } else {
            Ok(())
        }
    }
}

pub mod cache;

pub use crate::exec::scratch::{
    ScratchAppend, ScratchClaimKey, ScratchExactKey, ScratchLookup, ScratchMapId, ScratchProbe,
    ScratchRelation, ScratchVisit, ScratchVisitor, ScratchWideClaimKey, ScratchWordKey,
    ScratchWriteBatch,
};
pub use cache::{
    GenerationHandle, GenerationProtocol, GenerationState, ResolverView, WeakGenerationHandle,
};

#[cfg(test)]
mod tests;
