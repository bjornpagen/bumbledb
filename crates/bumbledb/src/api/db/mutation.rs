//! Collection-valued mutation reports. The fresh-id range machinery is
//! deleted with the fresh mechanism itself (E-NO-RESERVE): identities are
//! application-owned values, never engine-minted counters.

/// Facts consumed vs facts that changed the in-memory final-state view at
/// call time. The length-1 report is `{ submitted: 1, changed: 0|1 }`.
/// `changed` counts recorded and cancelled net dispositions; already
/// matching state does not increment; `changed <= submitted` is the
/// invariant. The engine constructs reports; hosts only read them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MutationReport {
    submitted: u64,
    changed: u64,
}

impl MutationReport {
    pub const EMPTY: Self = Self {
        submitted: 0,
        changed: 0,
    };

    pub(super) const fn from_counts(submitted: u64, changed: u64) -> Self {
        debug_assert!(changed <= submitted);
        Self { submitted, changed }
    }

    #[must_use]
    pub const fn submitted(self) -> u64 {
        self.submitted
    }

    #[must_use]
    pub const fn changed(self) -> u64 {
        self.changed
    }
}

impl Default for MutationReport {
    fn default() -> Self {
        Self::EMPTY
    }
}
