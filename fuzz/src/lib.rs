//! The shared fuzz harness (docs/architecture/60-validation.md § the
//! fuzzing charter): fuzzer bytes → [`Rng::from_bytes`] → generation in
//! `bumbledb-bench`'s `corpus_gen` → a scenario runner returning typed
//! verdicts. Each target in `fuzz_targets/` is one thin `fuzz_target!`
//! call into one runner here; the harness owns no logic worth fuzzing
//! (refusal: we do not fuzz the harness).
//!
//! Error matches in this crate are TOTAL — zero catch-all arms over
//! engine error enums, so a future variant addition is a compile error
//! here: the matcher is itself a census instrument.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use bumbledb::error::SchemaError;
use bumbledb::schema::SchemaDescriptor;
use bumbledb::schema::fingerprint::{self, SchemaFingerprint};
use bumbledb::{Db, Error};
use bumbledb_bench::corpus_gen::Rng;
use bumbledb_bench::corpus_gen::theorygen;

/// The theory target: fuzzer bytes → a structurally-free
/// [`SchemaDescriptor`] (the random-descriptor arm — deliberately-invalid
/// shapes alongside valid ones) → the engine's acceptance judgment, under
/// three oracles. Oracle 1 (no-panic totality) is the run itself: any
/// panic below — engine or harness assert — is a finding by definition.
pub fn theory(data: &[u8]) {
    let mut rng = Rng::from_bytes(data);
    let descriptor = theorygen::random_descriptor(&mut rng);

    // Oracle 3: judgment determinism — the same descriptor against two
    // fresh stores yields the identical verdict (rejections compared
    // payload-exact, acceptances by schema fingerprint).
    let store = StoreDir::new();
    let first = judge(&descriptor, store.path());
    let twin = StoreDir::new();
    let second = judge(&descriptor, twin.path());
    assert_eq!(first, second, "judgment determinism");

    if let Verdict::Accepted(created) = first {
        // Oracle 3, continued: an accepted schema re-opens cleanly on the
        // store it created, to the same sealed theory, and `verify_store`
        // passes on the empty store.
        let db = match Db::open(store.path(), descriptor) {
            Ok(db) => db,
            Err(err) => panic!("accepted schema failed to reopen: {err:?}"),
        };
        let report = match db.verify_store() {
            Ok(report) => report,
            Err(err) => panic!("verify_store errored on a fresh store: {err:?}"),
        };
        assert!(
            report.findings.is_empty(),
            "fresh store of an accepted schema has findings: {:?} (fingerprint {created:?})",
            report.findings
        );
    }
}

/// The engine's judgment of one descriptor, as a comparable value.
#[derive(Debug, PartialEq)]
enum Verdict {
    /// Accepted: the sealed schema's fingerprint.
    Accepted(SchemaFingerprint),
    /// Rejected: the variant name (the total-match token) plus the full
    /// typed payload.
    Rejected(&'static str, SchemaError),
}

/// One acceptance pass through the REAL public API: `Db::create` on a
/// fresh directory. An accepted descriptor must also validate standalone
/// (the fingerprint's source); disagreement between the two entry points
/// is a finding.
fn judge(descriptor: &SchemaDescriptor, dir: &Path) -> Verdict {
    match Db::create(dir, descriptor.clone()) {
        Ok(db) => {
            drop(db);
            let schema = match descriptor.clone().validate() {
                Ok(schema) => schema,
                Err(err) => panic!("Db::create accepted what validate rejects: {err:?}"),
            };
            Verdict::Accepted(fingerprint::fingerprint(&schema))
        }
        Err(err) => {
            let (token, rejection) = schema_rejection(err);
            Verdict::Rejected(token, rejection)
        }
    }
}

/// Oracle 2, the boundary: schema acceptance rejects through
/// `Error::Schema` and nothing else. Every other variant is named — never
/// a catch-all — and is a finding on this path.
fn schema_rejection(err: Error) -> (&'static str, SchemaError) {
    match err {
        Error::Schema(rejection) => (schema_variant(&rejection), rejection),
        other @ (Error::FormatMismatch { .. }
        | Error::SchemaMismatch { .. }
        | Error::AlreadyInitialized
        | Error::EnvironmentLocked
        | Error::Io(_)
        | Error::Lmdb(_)
        | Error::ReadersFull { .. }
        | Error::Validation(_)
        | Error::FactShape(_)
        | Error::FunctionalityViolation { .. }
        | Error::ContainmentViolation { .. }
        | Error::FreshExhausted { .. }
        | Error::ClosedRelationWrite { .. }
        | Error::GenerationMoved { .. }
        | Error::CommitSync { .. }
        | Error::BulkLoad { .. }
        | Error::ForeignPreparedQuery
        | Error::ForeignSnapshot
        | Error::ParamCountMismatch { .. }
        | Error::ParamTypeMismatch { .. }
        | Error::ParamSetExpected { .. }
        | Error::ParamScalarExpected { .. }
        | Error::ParamElementTypeMismatch { .. }
        | Error::PointParamAtCeiling { .. }
        | Error::AllenMaskParamExpected { .. }
        | Error::EmptyAllenMaskParam { .. }
        | Error::FullAllenMaskParam { .. }
        | Error::MeasureOfRay { .. }
        | Error::Overflow(_)
        | Error::ResultBytesOverflow
        | Error::Corruption(_)) => {
            panic!("non-schema error from schema acceptance: {other:?}")
        }
    }
}

/// Oracle 2, the census: every rejection is a NAMED `SchemaError` variant.
/// Total match, zero catch-alls — a new variant is a compile error here.
fn schema_variant(rejection: &SchemaError) -> &'static str {
    match rejection {
        SchemaError::DuplicateRelationName { .. } => "DuplicateRelationName",
        SchemaError::DuplicateFieldName { .. } => "DuplicateFieldName",
        SchemaError::FreshOnNonU64 { .. } => "FreshOnNonU64",
        SchemaError::FixedBytesWidthOutOfRange { .. } => "FixedBytesWidthOutOfRange",
        SchemaError::EmptyExtension { .. } => "EmptyExtension",
        SchemaError::ExtensionTooManyRows { .. } => "ExtensionTooManyRows",
        SchemaError::DuplicateExtensionHandle { .. } => "DuplicateExtensionHandle",
        SchemaError::ExtensionArityMismatch { .. } => "ExtensionArityMismatch",
        SchemaError::ExtensionValueTypeMismatch { .. } => "ExtensionValueTypeMismatch",
        SchemaError::ExtensionIntervalEmpty { .. } => "ExtensionIntervalEmpty",
        SchemaError::ExtensionIntervalRay { .. } => "ExtensionIntervalRay",
        SchemaError::StrOnClosedRelation { .. } => "StrOnClosedRelation",
        SchemaError::FreshOnClosedRelation { .. } => "FreshOnClosedRelation",
        SchemaError::StatementUnknownRelation { .. } => "StatementUnknownRelation",
        SchemaError::StatementUnknownField { .. } => "StatementUnknownField",
        SchemaError::EmptyProjection { .. } => "EmptyProjection",
        SchemaError::DuplicateProjectionField { .. } => "DuplicateProjectionField",
        SchemaError::DuplicateSelectionField { .. } => "DuplicateSelectionField",
        SchemaError::FunctionalityMultipleIntervals { .. } => "FunctionalityMultipleIntervals",
        SchemaError::FunctionalityIntervalNotLast { .. } => "FunctionalityIntervalNotLast",
        SchemaError::DuplicateFunctionality { .. } => "DuplicateFunctionality",
        SchemaError::GuardKeyTooWide { .. } => "GuardKeyTooWide",
        SchemaError::ContainmentArityMismatch { .. } => "ContainmentArityMismatch",
        SchemaError::ContainmentTypeMismatch { .. } => "ContainmentTypeMismatch",
        SchemaError::SelectedFieldProjected { .. } => "SelectedFieldProjected",
        SchemaError::SelectionLiteralTypeMismatch { .. } => "SelectionLiteralTypeMismatch",
        SchemaError::SelectionLiteralNotUtf8 { .. } => "SelectionLiteralNotUtf8",
        SchemaError::SelectionIntervalEmpty { .. } => "SelectionIntervalEmpty",
        SchemaError::NoMatchingTargetKey { .. } => "NoMatchingTargetKey",
        SchemaError::NoPointwiseTargetKey { .. } => "NoPointwiseTargetKey",
        SchemaError::ClosedContainmentInterval { .. } => "ClosedContainmentInterval",
        SchemaError::ClosedStatementRefuted { .. } => "ClosedStatementRefuted",
        SchemaError::DuplicateStatement { .. } => "DuplicateStatement",
    }
}

/// A per-iteration LMDB store directory under the system temp root:
/// created fresh, removed on drop — Tiny-scale stores keep this cheap.
struct StoreDir(PathBuf);

static STORE_SEQ: AtomicU64 = AtomicU64::new(0);

impl StoreDir {
    fn new() -> Self {
        let seq = STORE_SEQ.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!("bumbledb-fuzz-{}-{seq}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        std::fs::create_dir_all(&path).expect("create fuzz store dir");
        Self(path)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for StoreDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
