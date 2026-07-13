//! bumbledb: an embedded, typed, set-semantic relational database over
//! LMDB, executing conjunctive queries with Free Join.
//!
//! The surface is plain data in, plain data out (`docs/architecture/`, the
//! normative design):
//!
//! - Declare a schema with the [`schema!`] macro — its `pub Name;` header
//!   names a unit struct implementing [`Theory`], and the body expands
//!   to host newtypes and one typed [`Fact`] struct per relation
//!   (variable-width fields borrowed: `str` → `&str`, `bytes` → `&[u8]`).
//!   The macro is sugar; [`schema::SchemaDescriptor`] is the contract.
//! - Open a handle with [`Db::create`] / [`Db::open`] — `Db::create(path,
//!   Ledger)` — and share it across threads (`Send + Sync`; the engine
//!   owns zero threads). `Db<S>` carries the schema as typestate: a
//!   schema-A fact cannot reach a schema-B database (see below).
//! - Write through [`Db::write`]: the transaction is an in-memory delta —
//!   set arithmetic, statements judged at commit against the final
//!   state, an abort never touched disk. `delete(old); insert(new)` in
//!   either order is the blessed mutation idiom.
//! - Query through [`Db::prepare`] ([`ir::Query`] is the IR) and execute
//!   inside [`Db::read`] snapshots into a reusable [`ResultBuffer`] —
//!   results are sets; the host sorts.
//! - Migrate by ETL: [`Snapshot::scan`] exports, [`Db::bulk_load`] imports
//!   (schema change = a new database, never in place).
//!
//! Newtypes are the nominal safety layer — mixing two of them is a host
//! compile error:
//!
//! ```compile_fail
//! bumbledb::schema! {
//!     pub Ledger;
//!     relation Holder { id: u64 as HolderId, fresh }
//!     relation Account { id: u64 as AccountId, fresh }
//! }
//! let account = AccountId(1);
//! let _holder: HolderId = account; // mismatched types: rustc refuses
//! ```
//!
//! The schema typestate closes the cross-schema hole the same way: an
//! `Inventory` fact into a `Ledger` database is a compile error, not a
//! runtime surprise —
//!
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
//! # std::fs::create_dir_all(&dir).unwrap();
//! let db = bumbledb::Db::create(&dir, Ledger).unwrap();
//! db.write(|tx| {
//!     let id = tx.alloc::<ItemId>()?;
//!     tx.insert(&Item { id }) // schema-B fact, schema-A database: rustc refuses
//!         .map(|_| ())
//! })
//! .unwrap();
//! ```
//!
//! The workspace holds the three-command contract — green after every
//! change:
//!
//! ```text
//! cargo fmt --all --check
//! cargo clippy --workspace --all-targets -- -D warnings
//! cargo test --workspace
//! ```

// Nightly dividend (docs/prd-crucible/02-nightly-dividend.md): `try`
// blocks replace the immediately-invoked-closure error idiom — the
// block states "this region fails as a unit" without a fake function
// call.
#![feature(try_blocks)]
// Nightly dividend (docs/prd-crucible/03-portable-simd.md): the
// predicate-scan, dense-fold, and index-gather kernels are `std::simd`
// bodies on every target — measured at or above the retired hand-NEON
// twins, deleting the intrinsic dual and most of the kernel layer's
// `unsafe`, and Miri-interpretable for the UB lane (PRD 15). The Allen
// configuration kernel alone stays intrinsic per that PRD's measured
// verdict matrix.
#![feature(portable_simd)]

// 64-bit only (docs/architecture/00-product.md): `usize` is 8 bytes everywhere
// and no design decision accommodates narrower platforms. Building for a
// 32-bit target (e.g. `--target i686-unknown-linux-gnu`) fails with this
// explicit error instead of miscompiling pointer-width assumptions.
#[cfg(target_pointer_width = "32")]
compile_error!("bumbledb targets 64-bit platforms only (docs/architecture/00-product.md)");

pub mod allen;
#[cfg(feature = "alloc-counter")]
pub mod alloc_counter;
pub(crate) mod api;
pub(crate) mod arena;
pub mod digest;
pub(crate) mod encoding;
pub mod error;
pub(crate) mod exec;
pub(crate) mod image;
mod interval;
pub mod ir;
pub mod obs;
pub(crate) mod plan;
pub mod schema;
pub(crate) mod storage;
mod value;
mod verify_store;

pub use allen::{AllenMask, Basic, classify};
pub use api::db::{BulkLoadError, Db, Fact, Fresh, FreshKeyed, Snapshot, WriteTx};
pub use api::prepared::{
    BindValue, OccurrenceDrift, ParamArg, PreparedQuery, ResultBuffer, ResultValue, Row, Staleness,
};
pub use api::stats::{
    CoverStats, DeadRule, DisjointRules, EliminatedOccurrence, ExecutionStats, FoldedOccurrence,
    GuardStats, NodeStats, PinnedRows, RuleStats,
};
pub use error::{Direction, Error, OverflowKind, Result, Violation, Violations};
pub use interval::Interval;
/// The statically-empty fold's off switch (`ir/normalize/fold.rs`):
/// reachable only under the `fold-off` fuzz-oracle feature. History,
/// recorded honestly: deleted as dead configuration 2026-07-12 (nothing
/// in-workspace consumed it), revived 2026-07-13 with a named consumer —
/// the detached fuzz crate's `rewrites` dual-pipeline differential
/// (docs/prd-crucible/13-fuzz-query-rewrites.md), which an external
/// crate can only reach through a feature, never through `cfg(test)`.
#[cfg(feature = "fold-off")]
pub use ir::normalize::with_fold_disabled;
/// The chase's test-support off switch (`plan/chase.rs`): reachable only
/// under the `chase-off` feature, which the bench crate's dual-run
/// differential unit tests (as a dev-dependency) and the fuzz crate's
/// `rewrites` dual-pipeline differential enable.
#[cfg(feature = "chase-off")]
pub use plan::chase::with_chase_disabled;
/// The crashpoint table (`storage/commit.rs`): the commit pipeline's
/// named phase boundaries with their expected recovery sides, reachable
/// only under the `crashpoint` fuzz-oracle feature. The detached fuzz
/// crate's `crash` target (docs/prd-crucible/14-fuzz-crash.md) consumes
/// the table as its single authority — the harness draws points from it
/// and judges recovery by its sides, so the engine's claimed atomicity
/// structure and the adversary's expectations are one value.
#[cfg(feature = "crashpoint")]
pub use storage::commit::{CRASHPOINTS, CrashpointSide};
/// The storage format version (`storage/env.rs`), public so
/// store-shaped derived identities (the bench corpus cache, stamps) can
/// key on it: a format bump must regenerate every store-derived
/// artifact, never reuse one.
pub use storage::env::FORMAT_VERSION as STORAGE_FORMAT_VERSION;
// The IR vocabulary a host needs to build a `Query`, and the id types that
// appear in `Db`'s own signatures — importable from the root, no
// module-path scavenger hunt.
pub use ir::{
    AggOp, Atom, CmpOp, Comparison, ConditionTree, FindTerm, HeadOp, HeadTerm, MAX_CONDITION_DEPTH,
    MAX_RULES, MaskTerm, ParamId, Query, Rule, Term, Value, VarId,
};
pub use schema::{FieldId, FreshField, RelationId, Schema, StatementId, Theory};
pub use verify_store::{StoreFinding, StoreReport};

/// The declarative schema surface (docs/architecture/70-api.md). (The macro and the `schema`
/// module share a name across disjoint namespaces — deliberate:
/// `bumbledb::schema! {}` declares, `bumbledb::schema::…` are the
/// descriptor types.)
///
/// The grammar is parse-shape only and names resolve to ids at expansion;
/// semantics beyond names flow through schema validation (typed
/// [`error::SchemaError`] from [`Db::create`] / [`Db::open`]). The
/// invocation's first item is the header `pub Name;` — the unit struct
/// that names the schema ([`Theory`]) and disambiguates multiple
/// schemas in one module. Six shapes the macro itself refuses:
///
/// A missing header:
///
/// ```compile_fail
/// bumbledb::schema! {
///     relation Holder { id: u64 as HolderId, fresh }
/// }
/// ```
///
/// Field-level constraint words do not exist — everything relational is a
/// statement:
///
/// ```compile_fail
/// bumbledb::schema! {
///     pub Ledger;
///     relation Holder { id: u64 as HolderId, fresh, unique }
/// }
/// ```
///
/// An unknown modifier — the only modifier is `fresh`, and the dead SQL
/// generation word takes this same path
/// (``schema!: unknown field modifier `autoincrement` (the only modifier is `fresh`)``):
///
/// ```compile_fail
/// bumbledb::schema! {
///     pub Ledger;
///     relation Holder { id: u64 as HolderId, autoincrement }
/// }
/// ```
///
/// An FD's right side is its own relation (`R(X) -> R`):
///
/// ```compile_fail
/// bumbledb::schema! {
///     pub Ledger;
///     relation Holder { id: u64 as HolderId, fresh }
///     relation Account { id: u64 as AccountId, fresh, holder: u64 as HolderId }
///     Account(holder) -> Holder;
/// }
/// ```
///
/// An FD takes no selection (the descriptor cannot represent one):
///
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
///
/// An unknown field name in a statement — expansion resolves names to
/// declaration-order ids, so the error names the relation and field
/// (``schema!: relation `Holder` has no field `nope` ``):
///
/// ```compile_fail
/// bumbledb::schema! {
///     pub Ledger;
///     relation Holder { id: u64 as HolderId, fresh }
///     Holder(nope) -> Holder;
/// }
/// ```
///
/// Bare `bytes` is not a type — the width is the type
/// (``schema!: unknown type `bytes` — write `bytes<N>` ``); variable-width
/// binary does not exist (`docs/architecture/10-data-model.md`):
///
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
    pub use crate::api::db::plumbing::{
        decode, decode_write, encode_read_fact, encode_write_fact, intern_str_delete,
        intern_str_read, intern_str_write, resolve_string, resolve_string_write,
    };
    pub use crate::encoding::ValueRef;
}

#[cfg(test)]
pub(crate) mod testutil {
    //! Shared test scaffolding: a self-cleaning temp directory (no external
    //! dev-dependency — deps stay exactly heed + blake3).

    use std::path::{Path, PathBuf};

    pub struct TempDir(PathBuf);

    impl TempDir {
        /// Creates (or wipes and recreates) a per-test directory. `tag` must
        /// be distinct per test function so parallel tests never collide.
        pub fn new(tag: &str) -> Self {
            let path = std::env::temp_dir().join(format!("bumbledb-test-{tag}"));
            let _ = std::fs::remove_dir_all(&path);
            std::fs::create_dir_all(&path).expect("create test dir");
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
