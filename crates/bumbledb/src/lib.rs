//! bumbledb: an embedded, typed, set-semantic relational database over
//! LMDB, executing conjunctive queries with Free Join.
//!
//! The surface is plain data in, plain data out (`docs/architecture/`, the
//! normative design):
//!
//! - Declare a schema with the [`schema!`] macro — it expands to a
//!   `schema()` constructor, host newtypes, and one typed [`Fact`] struct
//!   per relation. The macro is sugar; [`schema::SchemaDescriptor`] is the
//!   contract.
//! - Open a handle with [`Db::create`] / [`Db::open`] and share it across
//!   threads (`Send + Sync`; the engine owns zero threads).
//! - Write through [`Db::write`]: the transaction is an in-memory delta —
//!   set arithmetic, constraints checked at commit against the final
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
//!     relation Holder { id: u64 as HolderId, serial }
//!     relation Account { id: u64 as AccountId, serial }
//! }
//! let account = AccountId(1);
//! let _holder: HolderId = account; // mismatched types: rustc refuses
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

// 64-bit only (docs/architecture/00-product.md): `usize` is 8 bytes everywhere
// and no design decision accommodates narrower platforms. Building for a
// 32-bit target (e.g. `--target i686-unknown-linux-gnu`) fails with this
// explicit error instead of miscompiling pointer-width assumptions.
#[cfg(target_pointer_width = "32")]
compile_error!("bumbledb targets 64-bit platforms only (docs/architecture/00-product.md)");

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

pub use api::db::{BulkLoadError, Db, Fact, Serial, SerialKeyed, Snapshot, WriteTx};
pub use api::prepared::{ParamArg, PreparedQuery, ResultBuffer, ResultValue, Row};
pub use api::stats::{CoverStats, ExecutionStats, GuardStats, NodeStats};
pub use error::{Direction, Error, Result};
pub use interval::Interval;
// The IR vocabulary a host needs to build a `Query`, and the id types that
// appear in `Db`'s own signatures — importable from the root, no
// module-path scavenger hunt.
pub use ir::{AggOp, Atom, CmpOp, Comparison, FindTerm, ParamId, Query, Term, Value, VarId};
pub use schema::{FieldId, RelationId, Schema, StatementId};

/// The declarative schema surface (docs/architecture/70-api.md). (The macro and the `schema`
/// module share a name across disjoint namespaces — deliberate:
/// `bumbledb::schema! {}` declares, `bumbledb::schema::…` are the
/// descriptor types.)
///
/// The grammar is parse-shape only; semantics flow through schema
/// validation. Three shapes the grammar itself refuses:
///
/// Field-level constraint words do not exist — everything relational is a
/// statement:
///
/// ```compile_fail
/// bumbledb::schema! {
///     relation Holder { id: u64 as HolderId, serial, unique }
/// }
/// ```
///
/// An FD's right side is its own relation (`R(X) -> R`):
///
/// ```compile_fail
/// bumbledb::schema! {
///     relation Holder { id: u64 as HolderId, serial }
///     relation Account { id: u64 as AccountId, serial, holder: u64 as HolderId }
///     Account(holder) -> Holder;
/// }
/// ```
///
/// An FD takes no selection (the descriptor cannot represent one):
///
/// ```compile_fail
/// bumbledb::schema! {
///     relation Account {
///         id: u64 as AccountId, serial,
///         kind: enum Kind { Checking, Savings },
///     }
///     Account(id | kind == Savings) -> Account;
/// }
/// ```
pub use bumbledb_macros::schema;

/// `schema!` expansion plumbing. Not API: no stability promises, nothing
/// here is part of the documented surface — the macro is the only caller.
#[doc(hidden)]
pub mod __private {
    pub use crate::api::db::plumbing::{
        decode, decode_write, encode_read_fact, encode_write_fact, intern_bytes_delete,
        intern_bytes_read, intern_bytes_write, intern_str_delete, intern_str_read,
        intern_str_write, resolve_bytes, resolve_bytes_write, resolve_string, resolve_string_write,
    };
    pub use crate::encoding::ValueRef;
    pub use crate::schema::runtime::{
        build_schema, FieldDecl, FieldTy, LiteralDecl, RelationDecl, SideDecl, StatementDecl,
    };
    pub use crate::schema::IntervalElement;
}

#[cfg(test)]
pub(crate) mod testutil {
    //! Shared test scaffolding: a self-cleaning temp directory (no external
    //! dev-dependency — deps stay exactly heed + blake3).

    use std::path::{Path, PathBuf};

    pub struct TempDir(PathBuf);

    impl TempDir {
        /// Creates (or wipes and recreates) a per-test directory. `tag` must
        /// be unique per test function so parallel tests never collide.
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
