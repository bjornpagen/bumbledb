use super::*;
use crate::{Environment, ExecuteError, Fact, QueryError};
use bumbledb_core::query_builder::{OperandRef, QueryBuildResult, QueryBuilder};
use bumbledb_core::schema::{
    ConstraintDescriptor, FieldDescriptor, IndexDescriptor, RelationDescriptor,
};

type TestResult = std::result::Result<(), Box<dyn std::error::Error>>;

fn typed_query(
    schema: &StorageSchema,
    build: impl FnOnce(&mut QueryBuilder<'_>) -> QueryBuildResult<()>,
) -> QueryBuildResult<TypedQuery> {
    let mut builder = QueryBuilder::new(schema.descriptor());
    build(&mut builder)?;
    builder.finish()
}

#[path = "query_tests/atom_cache.rs"]
mod atom_cache;
#[path = "query_tests/basic.rs"]
mod basic;
#[path = "query_tests/cache_and_planner.rs"]
mod cache_and_planner;
#[path = "query_tests/differential.rs"]
mod differential;
#[path = "query_tests/sinks_and_projection.rs"]
mod sinks_and_projection;
#[path = "query_tests/typed_ir_validation.rs"]
mod typed_ir_validation;

#[path = "query_test_helpers.rs"]
mod query_test_helpers;

use query_test_helpers::*;
