//! Placeholder test-support crate retained for the Free Join paper alignment rebuild.
//!
//! The old fixtures targeted the purged v4 storage/query engine. PRD 19 will
//! rebuild this crate around formal Free Join, v5 storage, COLT, and exact
//! set-semantics differential tests.

#![allow(clippy::result_large_err)]

/// Marker proving the crate intentionally contains no old v4 fixtures.
pub const PURGED_FOR_REBUILD: bool = true;

/// Returns whether this crate has been purged for the rebuild.
pub fn purged_for_rebuild() -> bool {
    PURGED_FOR_REBUILD
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crate_is_purged_for_rebuild() {
        assert!(purged_for_rebuild());
    }
}
