//! The generated identity table: every refusal kind and outcome tag the
//! log boundary carries, one array per enum, rendered by [`emit`] into
//! `conformance/v3/identities.json` and its byte-identical twin
//! `ts/crate/log-identities.json` (the bridge's mint table). Refusal
//! rows are spelled by each enum's `identity()` — the one speller —
//! over a witness roster whose exhaustive match is the compile lock: a
//! core variant without a roster row refuses to compile. Outcome rows
//! (the arm tags the tagged hosts narrow) are spelled here under the
//! same lock; those sums carry no `identity()` because their Rust
//! consumers narrow the arms directly. The census regenerates the
//! emission (`cargo run -p bumbledb-log --bin identities`) and diffs it
//! against both checked-in files, so a unilateral kind is a red census,
//! never a silent new wire string.

use std::fmt::Write as _;

use bumbledb::Admission;
use bumbledb::schema::{FieldId, RelationDescriptor, RelationId, SchemaDescriptor, ValueType};

use crate::braids::{BraidId, braids};
use crate::codec::{DecodeError, EncodeError, ValueShape};
use crate::lease::LeaseRefusal;
use crate::manifest::{CheckpointError, ManifestError};
use crate::replica::{Refreshed, Waited};
use crate::sidecar::SidecarError;

/// One row per variant, core declaration order: the pattern is the
/// compile lock (exhaustive, no wildcard — a core variant refuses to
/// compile until its row lands), and the witness feeds the family's
/// `identity()`, the one speller of the kind string.
macro_rules! refusal_family {
    ($fn_name:ident for $enum_ty:ident { $($pat:pat => $witness:expr),+ $(,)? }) => {
        fn $fn_name() -> Vec<&'static str> {
            let witnesses = [$($witness),+];
            for witness in &witnesses {
                match witness {
                    $($pat => ()),+
                }
            }
            witnesses.iter().map($enum_ty::identity).collect()
        }
    };
}

/// One row per arm, core declaration order: the pattern is the same
/// exhaustive compile lock, and the row's literal is the arm tag's one
/// Rust speller — the string the tagged hosts narrow.
macro_rules! outcome_family {
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

/// A braid witness: relation 0 of a one-relation theory is its own
/// braid.
fn one_braid() -> BraidId {
    let descriptor = SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "sample".into(),
            fields: vec![],
            extension: None,
        }],
        statements: vec![],
    };
    braids(&descriptor)
        .parse(0)
        .expect("relation 0 is its own braid")
}

refusal_family! { batch_decode for DecodeError {
    DecodeError::Truncated { .. } => DecodeError::Truncated { offset: 0 },
    DecodeError::BadMagic { .. } => DecodeError::BadMagic { got: [0; 4] },
    DecodeError::Version { .. } => DecodeError::Version { got: 0 },
    DecodeError::Flags { .. } => DecodeError::Flags { got: 1 },
    DecodeError::FingerprintMismatch { .. } => DecodeError::FingerprintMismatch { got: [0; 32] },
    DecodeError::UnknownBraid { .. } => DecodeError::UnknownBraid { got: 1 },
    DecodeError::UnknownOpKind { .. } => DecodeError::UnknownOpKind { op: 0, got: 3 },
    DecodeError::UnknownRelation { .. } => DecodeError::UnknownRelation {
        op: 0,
        relation: RelationId(9),
    },
    DecodeError::ClosedRelation { .. } => DecodeError::ClosedRelation {
        op: 0,
        relation: RelationId(0),
    },
    DecodeError::OpRelationOutsideBraid { .. } => DecodeError::OpRelationOutsideBraid {
        op: 0,
        relation: RelationId(0),
        braid: one_braid(),
    },
    DecodeError::TagMismatch { .. } => DecodeError::TagMismatch {
        relation: RelationId(0),
        row: 0,
        field: 0,
        expected: ValueType::Bool,
        got: 9,
    },
    DecodeError::BoolByte { .. } => DecodeError::BoolByte {
        relation: RelationId(0),
        row: 0,
        field: 0,
        got: 2,
    },
    DecodeError::InvalidUtf8 { .. } => DecodeError::InvalidUtf8 {
        relation: RelationId(0),
        row: 0,
        field: 0,
    },
    DecodeError::EmptyInterval { .. } => DecodeError::EmptyInterval {
        relation: RelationId(0),
        row: 0,
        field: 0,
    },
    DecodeError::IntervalOverflow { .. } => DecodeError::IntervalOverflow {
        relation: RelationId(0),
        row: 0,
        field: 0,
    },
    DecodeError::TrailingBytes { .. } => DecodeError::TrailingBytes { at: 0 },
} }

refusal_family! { batch_encode for EncodeError {
    EncodeError::FingerprintMismatch => EncodeError::FingerprintMismatch,
    EncodeError::UnknownBraid { .. } => EncodeError::UnknownBraid { braid: 1 },
    EncodeError::UnknownRelation { .. } => EncodeError::UnknownRelation {
        op: 0,
        relation: RelationId(9),
    },
    EncodeError::ClosedRelation { .. } => EncodeError::ClosedRelation {
        op: 0,
        relation: RelationId(0),
    },
    EncodeError::OpRelationOutsideBraid { .. } => EncodeError::OpRelationOutsideBraid {
        op: 0,
        relation: RelationId(0),
        braid: one_braid(),
    },
    EncodeError::Arity { .. } => EncodeError::Arity {
        op: 0,
        relation: RelationId(0),
        row: 0,
    },
    EncodeError::Value { .. } => EncodeError::Value {
        op: 0,
        relation: RelationId(0),
        row: 0,
        field: 0,
        cause: ValueShape::Kind {
            expected: ValueType::Bool,
        },
    },
    EncodeError::TooManyOps => EncodeError::TooManyOps,
    EncodeError::TooManyRows { .. } => EncodeError::TooManyRows { op: 0 },
} }

refusal_family! { manifest for ManifestError {
    ManifestError::Malformed { .. } => ManifestError::Malformed { at: 0 },
    ManifestError::Version { .. } => ManifestError::Version { got: 0 },
} }

refusal_family! { checkpoint for CheckpointError {
    CheckpointError::Malformed { .. } => CheckpointError::Malformed { at: 0 },
    CheckpointError::Version { .. } => CheckpointError::Version { got: 0 },
    CheckpointError::Overflow => CheckpointError::Overflow,
    CheckpointError::UnknownBraid { .. } => CheckpointError::UnknownBraid { got: 1 },
    CheckpointError::BraidSet => CheckpointError::BraidSet,
} }

refusal_family! { sidecar for SidecarError {
    SidecarError::Malformed { .. } => SidecarError::Malformed { at: 0 },
    SidecarError::Version { .. } => SidecarError::Version { got: 0 },
    SidecarError::UnknownBraid { .. } => SidecarError::UnknownBraid { got: 1 },
    SidecarError::Overflow => SidecarError::Overflow,
} }

refusal_family! { counter for LeaseRefusal {
    LeaseRefusal::Counter { .. } => LeaseRefusal::Counter {
        relation: RelationId(0),
        field: FieldId(0),
    },
    LeaseRefusal::Exhausted { .. } => LeaseRefusal::Exhausted {
        relation: RelationId(0),
        field: FieldId(0),
    },
    LeaseRefusal::OverWidth { .. } => LeaseRefusal::OverWidth { requested: 0 },
} }

outcome_family! { admission for Admission<()> {
    Admission::Accepted(()) => "accepted",
    Admission::Rejected(_) => "rejected",
} }

outcome_family! { waited for Waited {
    Waited::Reached(_) => "reached",
    Waited::Wedged { .. } => "wedged",
    Waited::Refused(_) => "refused",
} }

// The refresh outcome's vector arm narrows as `advanced` in the tagged
// hosts; the Rust arm name carries the payload noun.
outcome_family! { refresh_outcome for Refreshed {
    Refreshed::Vector(_) => "advanced",
    Refreshed::Refused(_) => "refused",
} }

/// The table's family rows, emission order: the refusal families, then
/// the outcome sums.
fn families() -> [(&'static str, Vec<&'static str>); 9] {
    [
        ("batchDecode", batch_decode()),
        ("batchEncode", batch_encode()),
        ("manifest", manifest()),
        ("checkpoint", checkpoint()),
        ("sidecar", sidecar()),
        ("counter", counter()),
        ("admission", admission()),
        ("waited", waited()),
        ("refreshOutcome", refresh_outcome()),
    ]
}

/// The comment row the emission carries: the file describes itself.
const COMMENT: &str = "The refusal-identity and outcome-tag table, generated from the \
bumbledb-log enums by `cargo run -p bumbledb-log --bin identities` \
(src/identities.rs): one array per boundary enum, refusal kinds spelled by each \
enum's `identity()`, outcome arms spelled as the tags consumers narrow. \
crates/bumbledb-log/conformance/v3/identities.json and ts/crate/log-identities.json \
are both this emission — the in-crate golden pins the conformance copy, the census \
regeneration lane diffs both — and the bridge marshal refuses to cross any kind \
outside its family row.";

/// Renders the table: fixed key order, one kind per line, a trailing
/// newline — byte-exact for the golden pins and the census diff.
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
    fn the_checked_in_table_is_the_emission() {
        // The ts/crate twin is held to the same bytes by the census
        // regeneration lane and parsed as JSON by the bridge's own
        // mint-table golden.
        assert_eq!(
            include_str!("../conformance/v3/identities.json"),
            super::emit(),
            "conformance/v3/identities.json drifted from the emitter — regenerate: \
             cargo run -p bumbledb-log --bin identities \
             > crates/bumbledb-log/conformance/v3/identities.json"
        );
    }
}
