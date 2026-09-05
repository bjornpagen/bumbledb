//! The successor identity table: every refusal kind and outcome tag the
//! history boundary carries, one array per enum, rendered by [`emit`].
//!
//! Refusal/outcome kinds are spelled by each enum's exhaustive match here —
//! the one speller — over a witness roster whose exhaustive `match` is the
//! compile lock: a new variant without a row refuses to compile, so a
//! unilateral kind cannot become a silent new wire string. This replaces the
//! deleted braid/vector conformance roster; there is no braid, sidecar,
//! manifest, lease-counter or split-outcome family here.

use std::fmt::Write as _;

use crate::history::admission::Refusal;
use crate::history::authority::AuthorityError;
use crate::history::command::FrameError;
use crate::writer::LogError;
use crate::writer::verbs::{ConditionalOutcome, PutOutcome};
use crate::writer::{ResolveOutcome, SubmitOutcome};

/// One row per variant, declaration order. The `match` is the compile lock;
/// the literal is the kind string.
macro_rules! kinds {
    ($fn_name:ident for $enum_ty:ty { $($pat:pat => $tag:literal),+ $(,)? }) => {
        fn $fn_name() -> Vec<&'static str> {
            let _lock = |value: &$enum_ty| -> &'static str {
                match value {
                    $($pat => $tag),+
                }
            };
            vec![$($tag),+]
        }
    };
}

kinds! { frame for FrameError {
    FrameError::LimitExceeded => "limitExceeded",
    FrameError::LengthOverflow => "lengthOverflow",
    FrameError::Allocation => "allocation",
    FrameError::Truncated { .. } => "truncated",
    FrameError::Family => "family",
    FrameError::Layout { .. } => "layout",
    FrameError::Kind { .. } => "kind",
    FrameError::Tag { .. } => "tag",
    FrameError::InvalidEpoch => "invalidEpoch",
    FrameError::StateIdentityMismatch => "stateIdentityMismatch",
    FrameError::EmptyChangeSummary => "emptyChangeSummary",
    FrameError::EmptyEvidence => "emptyEvidence",
    FrameError::InvalidTerminalStamp => "invalidTerminalStamp",
    FrameError::InvalidPreconditionEvidence => "invalidPreconditionEvidence",
    FrameError::InvalidPolicy => "invalidPolicy",
    FrameError::InvalidSequence => "invalidSequence",
    FrameError::InvalidCount => "invalidCount",
    FrameError::TrailingBytes { .. } => "trailingBytes",
} }

kinds! { admission_refusal for Refusal {
    Refusal::IdentityMismatch => "identityMismatch",
    Refusal::DatabaseDeleted => "databaseDeleted",
    Refusal::ReceiptExpiredUnknown => "receiptExpiredUnknown",
    Refusal::CommandIdentityConflict => "commandIdentityConflict",
    Refusal::CommandEpochClosed => "commandEpochClosed",
    Refusal::CommandEpochNotOpen { .. } => "commandEpochNotOpen",
    Refusal::DatabaseFrozen => "databaseFrozen",
    Refusal::StateIdentityMismatch => "stateIdentityMismatch",
    Refusal::InvalidRetainedReceipt => "invalidRetainedReceipt",
} }

kinds! { authority for AuthorityError {
    AuthorityError::Deleted => "deleted",
    AuthorityError::Frozen { .. } => "frozen",
    AuthorityError::NotFrozen => "notFrozen",
    AuthorityError::OperationMismatch { .. } => "operationMismatch",
    AuthorityError::ActivationEvidenceMismatch => "activationEvidenceMismatch",
    AuthorityError::InvalidGenesis => "invalidGenesis",
    AuthorityError::Exhausted(_) => "exhausted",
    AuthorityError::Policy(_) => "policy",
} }

kinds! { log_error for LogError {
    LogError::Identity => "identity",
    LogError::CommandIdentityConflict => "commandIdentityConflict",
    LogError::DatabaseDeleted => "databaseDeleted",
    LogError::DatabaseFrozen => "databaseFrozen",
    LogError::CommandEpochClosed => "commandEpochClosed",
    LogError::ReceiptExpiredUnknown => "receiptExpiredUnknown",
    LogError::NotInitialized => "notInitialized",
    LogError::Corruption => "corruption",
    LogError::Work(_) => "work",
    LogError::Core(_) => "core",
    LogError::Storage(_) => "storage",
    LogError::HostSeal(_) => "hostSeal",
    LogError::Misuse => "misuse",
    LogError::IncompleteRejectionEvidence => "incompleteRejectionEvidence",
    LogError::Backend => "backend",
    LogError::MaintenanceRequired { .. } => "maintenanceRequired",
    LogError::MaterializationStale => "materializationStale",
} }

kinds! { submit_outcome for SubmitOutcome {
    SubmitOutcome::Decided { .. } => "decided",
    SubmitOutcome::NotSubmitted { .. } => "notSubmitted",
    SubmitOutcome::OutcomeUnknown { .. } => "outcomeUnknown",
} }

kinds! { resolve_outcome for ResolveOutcome {
    ResolveOutcome::Found(_) => "found",
    ResolveOutcome::NotRecordedAt { .. } => "notRecordedAt",
    ResolveOutcome::CommandEpochClosed => "commandEpochClosed",
    ResolveOutcome::ReceiptExpiredUnknown => "receiptExpiredUnknown",
} }

kinds! { conditional_outcome for ConditionalOutcome {
    ConditionalOutcome::Published { .. } => "published",
    ConditionalOutcome::PreconditionFailed => "preconditionFailed",
    ConditionalOutcome::Indeterminate => "indeterminate",
} }

kinds! { put_outcome for PutOutcome {
    PutOutcome::Stored => "stored",
    PutOutcome::Indeterminate => "indeterminate",
} }

fn families() -> [(&'static str, Vec<&'static str>); 8] {
    [
        ("frame", frame()),
        ("admissionRefusal", admission_refusal()),
        ("authority", authority()),
        ("logError", log_error()),
        ("submitOutcome", submit_outcome()),
        ("resolveOutcome", resolve_outcome()),
        ("conditionalOutcome", conditional_outcome()),
        ("putOutcome", put_outcome()),
    ]
}

const COMMENT: &str = "The successor history refusal-identity and outcome-tag table, generated \
from the bumbledb-log enums by `cargo run -p bumbledb-log --bin identities` (src/identities.rs): \
one array per boundary enum. There is no braid, sidecar, manifest, lease-counter or split-outcome \
family; the deleted braided roster is gone.";

/// Renders the table: fixed key order, one kind per line, trailing newline.
///
/// # Panics
#[must_use]
pub fn emit() -> String {
    let families = families();
    let mut out = String::from("{\n");
    writeln!(out, "  \"comment\": \"{COMMENT}\",").expect("String accepts fmt");
    let last_family = families.len() - 1;
    for (family_index, (family, kinds)) in families.iter().enumerate() {
        writeln!(out, "  \"{family}\": [").expect("String accepts fmt");
        let last_kind = kinds.len() - 1;
        for (kind_index, kind) in kinds.iter().enumerate() {
            let comma = if kind_index == last_kind { "" } else { "," };
            writeln!(out, "    \"{kind}\"{comma}").expect("String accepts fmt");
        }
        let comma = if family_index == last_family { "" } else { "," };
        writeln!(out, "  ]{comma}").expect("String accepts fmt");
    }
    out.push_str("}\n");
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn family_rows_are_distinct() {
        for (family, kinds) in super::families() {
            let mut seen = std::collections::BTreeSet::new();
            for kind in &kinds {
                assert!(seen.insert(*kind), "{family}: duplicate kind {kind}");
            }
        }
    }

    #[test]
    fn emission_is_stable_and_nonempty() {
        let emitted = super::emit();
        assert!(emitted.starts_with("{\n"));
        assert!(emitted.trim_end().ends_with('}'));
        assert_eq!(emitted, super::emit());
    }
}
