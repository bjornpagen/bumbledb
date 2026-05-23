//! Internal LMDB storage boundary for Bumbledb.
//!
//! This crate intentionally keeps all LMDB details behind opaque environment and
//! transaction types. Higher layers should not depend on raw LMDB handles.

#![allow(clippy::result_large_err)]

pub mod allocation;
pub mod benchmark;
mod environment;
mod error;
#[cfg(feature = "test-failpoints")]
pub mod failpoints;
#[cfg(not(feature = "test-failpoints"))]
mod failpoints;
mod free_join;
mod planner_stats;
mod query;
mod query_image;
mod sorted_trie;
mod storage;
mod storage_schema;

pub(crate) use environment::RawDatabase;
pub use environment::{
    Environment, IndexDiagnostics, ReadTxn, RelationDiagnostics, StorageDiagnostics, WriteTxn,
};
pub use error::*;
pub(crate) use free_join::{
    AccessId, AtomId, FreeJoinPlan, NodeId, OutputPlan, PlanNode, ProjectPlan, VarId,
};
pub use planner_stats::PlannerStatsCacheDiagnostics;
pub use query::{
    AllocationPhaseStats, InputBindings, PlanCounters, QueryAllocationStats, QueryOutput,
    QueryPlan, QueryResultSet, QueryTimings, ResultColumn, ResultFact,
};
pub use query_image::QueryImageCacheDiagnostics;
pub(crate) use query_image::{
    EncodedRef, FieldId, QueryImage, QueryImageCache, RelationId, RelationImage,
};
pub(crate) use sorted_trie::{EncodedOwned, LinearIter, TrieIter};
pub use storage::{DeleteOutcome, Fact, InsertOutcome, Value};
pub use storage_schema::{BulkLoadReport, StorageSchema};

/// Current on-disk storage format version.
pub const STORAGE_FORMAT_VERSION: u32 = 4;
