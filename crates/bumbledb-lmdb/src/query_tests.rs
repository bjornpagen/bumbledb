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

include!("query_tests/basic.rs");

include!("query_tests/atom_cache.rs");

include!("query_tests/cache_and_planner.rs");

include!("query_tests/sinks_and_projection.rs");

include!("query_tests/differential.rs");

include!("query_tests/typed_ir_validation.rs");

include!("query_test_helpers.rs");
