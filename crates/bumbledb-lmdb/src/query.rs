use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;
use std::time::Instant;

use smallvec::SmallVec;

use bumbledb_core::encoding::{DecimalRaw, TimestampMicros};
use bumbledb_core::query_ir::{
    ComparisonOperator, Literal, TypedClause, TypedComparison, TypedFindTerm, TypedLiteral,
    TypedOperand, TypedQuery, TypedRelationAtom, TypedTerm,
};
use bumbledb_core::schema::{IndexKind, ValueType};

use crate::{
    AtomId, EncodedOwned, Error, FieldId, FreeJoinPlan, LinearIter, NodeId, OutputPlan, PlanNode,
    ProjectPlan, ReadTxn, RelationImage, Result, StorageSchema, TrieIter, Value, VarId,
};

use crate::QueryImageCacheDiagnostics;
use crate::allocation::{self, ALLOCATION_SIZE_CLASS_COUNT, AllocationDelta};
use crate::planner_stats::{PlannerIndexStats, PlannerRelationStats, PlannerStatsCacheDiagnostics};
use crate::query_image::QueryImageScope;

mod model;
pub use model::*;

mod metrics;
pub use metrics::*;

mod exec_state;
mod explain;
pub(in crate::query) use exec_state::*;

mod planner_types;
pub(in crate::query) use planner_types::*;

mod lftj_iter;
pub(in crate::query) use lftj_iter::*;

mod api;
mod hash;
pub(in crate::query) use hash::*;
mod lftj_prefix;
pub(in crate::query) use lftj_prefix::*;
mod lftj_runtime;
pub(in crate::query) use lftj_runtime::*;
mod timing;
pub(in crate::query) use timing::*;
mod lftj_leapfrog;
pub(in crate::query) use lftj_leapfrog::*;
mod lftj_access;
pub(in crate::query) use lftj_access::*;
mod planner;
pub(in crate::query) use planner::*;
mod planner_scoring;
pub(in crate::query) use planner_scoring::*;
mod comparison_eval;
pub(in crate::query) use comparison_eval::*;
mod normalize;
pub(in crate::query) use normalize::*;
mod sinks;
pub(in crate::query) use sinks::*;
mod values;
pub(in crate::query) use values::*;

#[cfg(test)]
#[path = "query_tests.rs"]
mod tests;
