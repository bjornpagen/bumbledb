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
pub(crate) mod encoding;
pub mod error;
pub(crate) mod exec;
pub(crate) mod image;
pub mod ir;
pub(crate) mod plan;
pub mod schema;
pub(crate) mod storage;

pub use api::db::{BulkLoadError, Db, Fact, Serial, Snapshot, WriteTx};
pub use api::prepared::{PreparedQuery, ResultBuffer, ResultValue, Row};
pub use error::{Error, Result};
// The IR vocabulary a host needs to build a `Query`, and the id types that
// appear in `Db`'s own signatures — importable from the root, no
// module-path scavenger hunt.
pub use ir::{AggOp, Atom, CmpOp, Comparison, FindTerm, ParamId, Query, Term, Value, VarId};
pub use schema::{FieldId, RelationId, Schema};

/// The declarative schema surface (docs/architecture/60-api.md). (The macro and the `schema`
/// module share a name across disjoint namespaces — deliberate:
/// `bumbledb::schema! {}` declares, `bumbledb::schema::…` are the
/// descriptor types.)
pub use bumbledb_macros::schema;

/// `schema!` expansion plumbing. Not API: no stability promises, nothing
/// here is part of the documented surface — the macro is the only caller.
#[doc(hidden)]
pub mod __private {
    pub use crate::api::db::plumbing::{
        decode, encode_read_fact, encode_write_fact, intern_bytes_read, intern_bytes_write,
        intern_str_read, intern_str_write, resolve_bytes, resolve_string,
    };
    pub use crate::encoding::ValueRef;
    pub use crate::schema::runtime::{build_schema, FieldDecl, FieldTy, RelationDecl};
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
