//! Reproducible benchmark fixtures for the normalized ledger workload.

use bumbledb_core::query_builder::QueryBuildResult;
use bumbledb_core::query_ir::TypedQuery;
use bumbledb_core::schema::SchemaDescriptor;

mod facts;
mod queries;
mod schema;

pub use facts::benchmark_facts;
pub use queries::benchmark_queries;
pub use schema::benchmark_schema;

/// Builds a typed benchmark query for a schema descriptor.
pub type BenchmarkQueryBuilder = fn(&SchemaDescriptor) -> QueryBuildResult<TypedQuery>;

/// A named benchmark query with equivalent typed Bumbledb and SQLite SQL.
#[derive(Clone, Debug)]
pub struct BenchmarkQuery {
    /// Stable query name.
    pub name: &'static str,
    /// Typed query builder.
    pub build: BenchmarkQueryBuilder,
    /// SQLite SQL query text.
    pub sqlite: &'static str,
}

/// Benchmark run output summary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BenchmarkComparison {
    /// Query name.
    pub query: String,
    /// Number of Bumbledb output facts.
    pub bumbledb_facts: usize,
    /// Number of SQLite output facts.
    pub sqlite_facts: usize,
    /// Bumbledb explain plan text.
    pub explain: String,
}

#[cfg(test)]
#[path = "benchmark/tests.rs"]
mod tests;
