//! bumbledb: an embedded, typed, set-semantic relational database over
//! LMDB, executing conjunctive queries with Free Join.
//! The surface is plain data in, plain data out:
//! - Declare a schema with the [`schema!`] macro — its `pub Name;` header
//!   names a unit struct implementing [`Theory`], and the body expands
//!   (variable-width fields borrowed: `str` → `&str`, `bytes` → `&[u8]`).
//!   `write` + [`WriteTx::insert_dyn`]. After a failed apply, later
//! ```compile_fail
//! bumbledb::schema! {
//!     pub Ledger;
//!     relation Holder { id: u64 as HolderId, fresh }
//!     relation Account { id: u64 as AccountId, fresh }
//! }
//! let account = AccountId(1);
//! let _holder: HolderId = account; // mismatched types: rustc refuses
//! ```
//! The schema typestate closes the cross-schema hole the same way: an
//! `Inventory` fact into a `Ledger` database is a compile error, not a
//! ```compile_fail
//! bumbledb::schema! {
//!     pub Ledger;
//!     relation Holder { id: u64 as HolderId, fresh }
//! }
//! bumbledb::schema! {
//!     pub Inventory;
//!     relation Item { id: u64 as ItemId, fresh }
//! }
//! # let dir = std::env::temp_dir().join("bumbledb-doc-cross-schema");
//! # let _ = std::fs::remove_dir_all(&dir);
//! let db = bumbledb::Db::create(&dir, Ledger).unwrap().unwrap();
//! db.write(|tx| {
//!     let id = tx.reserve::<ItemId>(1)?.start().expect("count 1").unwrap();
//!     tx.insert([&Item { id }]) // schema-B fact, schema-A database: rustc refuses
//!         .map(|_| ())
//! })
//! .unwrap();
//! ```
//! change:
//! The workspace holds the three-command contract — green after every
//! ```text
//! cargo fmt --all --check
//! cargo clippy --workspace --all-targets -- -D warnings
//! cargo test --workspace
//! ```
#![feature(try_blocks)]
#![feature(portable_simd)]
#[cfg(target_pointer_width = "32")]
compile_error!("bumbledb targets 64-bit platforms only");

#[cfg(test)]
extern crate self as bumbledb;

pub mod allen;
/// Counting allocator for the zero-warm-allocation gate and the bench
/// harness. Not embedding API.
#[doc(hidden)]
pub mod alloc_counter;
pub(crate) mod api;
pub(crate) mod arena;
/// Content digest used by the bench corpus/stamp harness. Not embedding API.
#[doc(hidden)]
pub mod digest;
pub(crate) mod encoding;
pub mod error;
pub(crate) mod exec;
pub(crate) mod image;
mod interval;
pub mod ir;
/// Execution tracing used by the bench harness. Not embedding API.
#[doc(hidden)]
pub mod obs;
pub(crate) mod plan;
pub mod schema;
pub(crate) mod storage;
mod value;
mod verify_store;

pub use allen::{AllenMask, Basic, classify};
/// the bridge crates' parse-once write representation, consumed by the
/// doc-hidden `*_accepted` verbs. A transport form, not embedding API.
#[doc(hidden)]
pub use api::db::{AcceptedCollection, CollectionBuilder};
pub use api::db::{
    CodecRead, CodecWrite, Db, Fact, Fresh, FreshRange, FreshRangeIter, InstanceBuilder, Key,
    MutationReport, OwnedInstance, Probe, ReadInstance, Witness, WriteTx,
};
pub use api::prepared::{
    Answer, AnswerValue, Answers, BindArgs, BindValue, ParamArg, PreparedQuery,
};
pub use error::{
    Admission, Check, Committed, ConditionalWrite, Conflict, Direction, Error, ErrorFamily,
    Exceeded, IoFailure, LmdbFailure, Mismatch, OverflowKind, Result, Violation, Violations,
};
pub use interval::Interval;
/// The grounding's test-support off switch (`plan/ground.rs`): reachable only
/// under the `ground-off` feature, which the bench crate's dual-run
/// differential unit tests (as a dev-dependency) enable.
#[cfg(feature = "ground-off")]
pub use plan::ground::with_grounding_disabled;
/// The storage format version (`storage/env.rs`), public so
/// store-shaped derived identities (the bench corpus cache, stamps) can
/// key on it: a format bump must regenerate every store-derived
/// artifact, never reuse one.
pub use storage::env::FORMAT_VERSION as STORAGE_FORMAT_VERSION;
pub use storage::env::GenerationId;

pub use ir::{
    AggOp, Atom, AtomSource, CmpOp, Comparison, ConditionTree, FindTerm, FoldOp, HeadOp, HeadTerm,
    Interior, InteriorId, MAX_CONDITION_DEPTH, MAX_RULES, NonEmpty, OrderCmp, ParamId,
    ProjectionRule, Query, Rec, RecRule, RecStep, Rule, Term, Value, VarId, WordCmp,
};

pub use crate::encoding::InternId;
pub use error::{
    AtomIndex, CitedFact, DynIdError, FactShapeError, FindIndex, RowIndex, RuleIndex, SchemaError,
    StatementErrorKind, ValidationError,
};
pub use schema::fingerprint::SchemaFingerprint;
pub use schema::{
    FieldId, FreshField, Manifest, RelationId, RenderedFact, RenderedViolation, Schema,
    SchemaDescriptor, SchemaSpec, SchemaSpecError, StatementId, StatementKind, Theory,
    render_rejection,
};
/// Offline store sweeper used by the bench harness and engine tests.
/// Not embedding API.
#[doc(hidden)]
pub use verify_store::{StoreFinding, StoreReport, StoreVerdict};

/// The declarative schema surface. (The macro and the `schema`
/// module share a name across disjoint namespaces — deliberate:
/// `bumbledb::schema! {}` declares, `bumbledb::schema::…` are the
/// descriptor types.)
/// The grammar is parse-shape only and names resolve to ids at expansion;
/// semantics beyond names flow through schema validation (typed
/// [`error::SchemaError`] from [`Db::create`] / [`Db::open`]). The
/// invocation's first item is the header `pub Name;` — the unit struct
/// ```compile_fail
/// bumbledb::schema! {
///     relation Holder { id: u64 as HolderId, fresh }
/// }
/// ```
/// statement:
/// Field-level constraint words do not exist — everything relational is a
/// ```compile_fail
/// bumbledb::schema! {
///     pub Ledger;
///     relation Holder { id: u64 as HolderId, fresh, unique }
/// }
/// ```
/// An unknown modifier — the only modifier is `fresh`, and the dead SQL
/// (``schema!: unknown field modifier `autoincrement` (the only modifier is `fresh`)``):
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
///     relation Holder { id: u64 as HolderId, fresh }
///     relation Account { id: u64 as AccountId, fresh, holder: u64 as HolderId }
///     Account(holder) -> Holder;
/// }
/// ```
/// An FD takes no selection (the descriptor cannot represent one):
/// ```compile_fail
/// bumbledb::schema! {
///     pub Ledger;
///     closed relation Kind as KindId = { Checking, Savings };
///     relation Account {
///         id: u64 as AccountId, fresh,
///         kind: u64 as KindId,
///     }
///     Account(kind) <= Kind(id);
///     Account(id | kind == Savings) -> Account;
/// }
/// ```
/// declaration-order ids, so the error names the relation and field
/// (``schema!: relation `Holder` has no field `nope` ``):
/// ```compile_fail
/// bumbledb::schema! {
///     pub Ledger;
///     relation Holder { id: u64 as HolderId, fresh }
///     Holder(nope) -> Holder;
/// }
/// ```
/// (``schema!: unknown type `bytes` — write `bytes<N>` ``); variable-width
/// binary does not exist:
/// ```compile_fail
/// bumbledb::schema! {
///     pub Ledger;
///     relation Blob { id: u64 as BlobId, fresh, payload: bytes }
/// }
/// ```
pub use bumbledb_macros::schema;

/// `schema!` expansion plumbing. Not API: no stability promises, nothing
/// here is part of the documented surface — the macro is the only caller.
#[doc(hidden)]
pub mod __private {
    pub use crate::api::db::plumbing::{encode_fact_for, fixed_interval_i64, fixed_interval_u64};
    pub use crate::encoding::{ValueRef, append_field};
}

#[cfg(test)]
pub(crate) mod testutil {

    use std::path::{Path, PathBuf};

    use crate::error::{Admission, Result, Violations};

    #[track_caller]
    pub fn expect_rejected<T>(result: Result<Admission<T>>) -> Violations {
        match result {
            Ok(Admission::Rejected(violations)) => violations,
            Ok(Admission::Accepted(_)) => {
                panic!("expected admission rejection, the write admitted")
            }
            Err(error) => panic!("expected admission rejection, the engine said {error:?}"),
        }
    }

    pub struct TempDir(PathBuf);

    impl TempDir {
        pub fn new(tag: &str) -> Self {
            let path =
                std::env::temp_dir().join(format!("bumbledb-test-{tag}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&path);
            Self(path)
        }

        pub fn path(&self) -> &Path {
            &self.0
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}
