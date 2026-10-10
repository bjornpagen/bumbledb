//! bumbledb: an embedded, typed, set-semantic relational database over
//! LMDB, executing conjunctive queries with Free Join.
//! Declare a schema with [`schema!`]: its `pub Name;` header creates a
//! type implementing [`Theory`], and relation declarations create typed
//! facts. Optional field wrappers keep application identities distinct:
//!
//! ```compile_fail
//! bumbledb::schema! {
//!     pub Ledger;
//!     relation Holder { id: uuid as HolderId }
//!     relation Account { id: uuid as AccountId }
//!     Holder(id) -> Holder;
//!     Account(id) -> Account;
//! }
//! let account = AccountId(bumbledb::Uuid::from_bytes([1; 16]));
//! let _holder: HolderId = account; // mismatched types: rustc refuses
//! ```
//! The schema typestate closes the cross-schema hole the same way: an
//! `Inventory` fact cannot be inserted into a `Ledger` database:
//!
//! ```compile_fail
//! bumbledb::schema! {
//!     pub Ledger;
//!     relation Holder { id: uuid as HolderId }
//!     Holder(id) -> Holder;
//! }
//! bumbledb::schema! {
//!     pub Inventory;
//!     relation Item { id: uuid as ItemId }
//!     Item(id) -> Item;
//! }
//! # let dir = std::env::temp_dir().join("cross-schema.bdb");
//! # let work = bumbledb::WorkContext::new();
//! # let _ = std::fs::remove_dir_all(&dir);
//! let db = bumbledb::Db::create(&dir, Ledger, work).unwrap().unwrap();
//! db.write(|tx| {
//!     let id = ItemId(bumbledb::Uuid::from_bytes([7; 16]));
//!     tx.insert([&Item { id }]) // schema-B fact, schema-A database: rustc refuses
//!         .map(|_| ())
//! })
//! .unwrap();
//! ```
#![deny(unreachable_pub)]

#[cfg(target_pointer_width = "32")]
compile_error!("bumbledb targets 64-bit platforms only");

#[cfg(test)]
extern crate self as bumbledb;

pub mod allen;
/// Counting allocator for allocation gates and benchmark diagnostics. Not
/// embedding API.
#[doc(hidden)]
pub mod alloc_counter;
pub(crate) mod api;
pub mod canonical;
pub mod changes;
pub(crate) mod digest;
pub(crate) mod encoding;
pub mod error;
pub(crate) mod exec;
pub(crate) mod image;
mod interval;
pub mod ir;
pub(crate) mod plan;
pub mod scalar;
pub mod schema;
pub(crate) mod storage;
mod verify_store;
pub mod work;

pub use allen::{AllenMask, Basic, classify};
/// The bridge's parse-once write representation for `WriteTx::insert_accepted`.
#[doc(hidden)]
pub use api::db::AcceptedCollection;
pub use api::db::host;
pub use api::db::{
    Committed, Db, Fact, InstanceBuilder, Key, MutationReport, OwnedInstance, OwnedRead, ReadFrame,
    RowReader, Witness, WriteOutcome, WriteTx,
};
pub use api::prepared::{
    Answer, AnswerValue, Answers, BindArgs, BindValue, CompleteResult, DeliveryTicket, ParamArg,
    PreparedQuery, ResultCursor, ResultIdentity, ResultPage, ResultRow,
};
pub use bumbledb_theory::interval::SegmentOp;
pub use bumbledb_theory::{F64, F64CastError, F64ParseError, Uuid};
pub use changes::{ChangeError, ChangeSet, ChangeSetBuilder};
pub use error::{
    Admission, Capacity, CorruptionError, Counter, Error, ErrorKind, Exceeded, HostKeyFault,
    IoFailure, LmdbFailure, Mismatch, OverflowKind, Result, Violation, Violations,
};
pub use exec::kernel::numeric::{F64Math, FloatCardinalityOverflow, NonDefaultFloatEnvironment};
pub use interval::{Discrete, Element, FloatMeasureError, Interval};
/// The grounding off switch, for dependent crates' differential tests.
#[cfg(any(test, feature = "testing"))]
pub use plan::ground::with_grounding_disabled;
pub use scalar::{NumericCast, Rounding, ScalarError, ScalarEvaluator, ScalarExpr};
pub use storage::GenerationId;
pub use storage::store::{CloseReport, Durability, Options};
pub use work::{WorkContext, WorkError};

/// Kernels at an explicit SIMD level and their scalar twins, for the bench
/// crate's micro report. Not embedding API.
#[doc(hidden)]
pub mod kernels {
    pub use crate::exec::kernel::bench::*;
    pub use crate::exec::kernel::reference;
}

pub use ir::{
    Atom, AtomSource, CmpOp, Comparison, ConditionTree, FindTerm, FoldOp, HeadOp, HeadTerm,
    Interior, InteriorId, MAX_CONDITION_DEPTH, MAX_RULES, NonEmpty, OrderCmp, ParamId,
    ProjectionRule, Query, Rec, RecRule, RecStep, Rule, Term, Value, VarId, WordCmp,
};

/// Query-image text token (process-scoped, never persisted) — public only
/// as the payload of [`error::CorruptionError::DanglingInternId`].
pub use crate::encoding::InternId;
pub use bumbledb_theory::schema::FixedIntervalElement;
pub use error::{
    AtomIndex, CitedFact, DynIdError, FactShapeError, FindIndex, RowIndex, RuleIndex, SchemaError,
    StatementErrorKind, ValidationError,
};
pub use schema::fingerprint::SchemaFingerprint;
pub use schema::{
    CompileError, FieldId, Manifest, RelationId, RenderedFact, RenderedViolation, Schema,
    SchemaDescriptor, SchemaSpec, SchemaSpecError, StatementId, StatementKind, Theory,
    render_rejection,
};

/// The declarative schema surface: `schema! { pub Name; ... }` declares a
/// theory type implementing [`Theory`] and one typed fact struct per
/// relation. Names resolve to ids at expansion. The theory's name comes
/// first:
/// ```compile_fail
/// bumbledb::schema! {
///     relation Holder { id: u64 as HolderId }
/// }
/// ```
/// A field is `name: type` or `name: type as NewType`; modifiers do not exist
/// (``schema!: unknown field modifier `autoincrement` — a field is `name: type` or `name: type as NewType` ``):
/// ```compile_fail
/// bumbledb::schema! {
///     pub Ledger;
///     relation Holder { id: u64 as HolderId, autoincrement }
/// }
/// ```
/// An FD's right side is its own relation (`R(X) -> R`):
/// ```compile_fail
/// bumbledb::schema! {
///     pub Ledger;
///     relation Holder { id: u64 as HolderId }
///     relation Account { id: u64 as AccountId, holder: u64 as HolderId }
///     Account(holder) -> Holder;
/// }
/// ```
/// An FD takes no selection:
/// ```compile_fail
/// bumbledb::schema! {
///     pub Ledger;
///     closed relation Kind as KindId = { Checking, Savings };
///     relation Account {
///         id: u64 as AccountId,
///         kind: u64 as KindId,
///     }
///     Account(kind) <= Kind(id);
///     Account(id | kind == Savings) -> Account;
/// }
/// ```
/// An unknown field is refused with the relation and field named
/// (``schema!: relation `Holder` has no field `nope` ``):
/// ```compile_fail
/// bumbledb::schema! {
///     pub Ledger;
///     relation Holder { id: u64 as HolderId }
///     Holder(nope) -> Holder;
/// }
/// ```
/// Variable-width binary does not exist (``schema!: unknown type `bytes` — write `bytes<N>` ``):
/// ```compile_fail
/// bumbledb::schema! {
///     pub Ledger;
///     relation Blob { id: u64 as BlobId, payload: bytes }
/// }
/// ```
pub use bumbledb_macros::{params, query, schema};

#[cfg(doctest)]
#[doc = include_str!("../../../README.md")]
pub struct ReadmeDoctests;

#[cfg(doctest)]
#[doc = include_str!("../../../docs/cookbook.md")]
pub struct CookbookDoctests;

/// `schema!` expansion plumbing. Not API: no stability promises, nothing
/// here is part of the documented surface — the macro is the only caller.
#[doc(hidden)]
pub mod __private {
    pub use crate::api::db::plumbing::{fixed_interval_i64, fixed_interval_u64};
}

#[cfg(test)]
pub(crate) mod testutil {

    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use crate::WriteOutcome;
    use crate::error::{Result, Violations};

    #[track_caller]
    pub(crate) fn expect_rejected<T: std::fmt::Debug>(
        result: Result<WriteOutcome<T>>,
    ) -> Violations {
        match result {
            Ok(WriteOutcome::Rejected(violations)) => violations,
            Ok(other) => panic!("expected a rejection, the write said {other:?}"),
            Err(error) => panic!("expected a rejection, the engine said {error:?}"),
        }
    }

    pub(crate) struct TempDir {
        root: PathBuf,
        path: PathBuf,
    }

    impl TempDir {
        pub(crate) fn new(tag: &str) -> Self {
            static NEXT: AtomicU64 = AtomicU64::new(0);
            // Tests may share both a tag and a process. Exclusively claim
            // the parent; the store path itself must not exist at create.
            loop {
                let id = NEXT.fetch_add(1, Ordering::Relaxed);
                let root = std::env::temp_dir()
                    .join(format!("bumbledb-test-{tag}-{}-{id}", std::process::id()));
                match std::fs::create_dir(&root) {
                    Ok(()) => {
                        return Self {
                            path: root.join("store.bdb"),
                            root,
                        };
                    }
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
                    Err(error) => panic!("create test directory: {error}"),
                }
            }
        }

        pub(crate) fn path(&self) -> &Path {
            &self.path
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }
}
