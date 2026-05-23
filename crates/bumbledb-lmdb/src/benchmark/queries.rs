use bumbledb_core::query_builder::{OperandRef, QueryBuildResult, QueryBuilder};
use bumbledb_core::query_ir::{ComparisonOperator, TypedQuery};
use bumbledb_core::schema::SchemaDescriptor;

use super::BenchmarkQuery;

/// Returns the benchmark query set.
pub fn benchmark_queries() -> Vec<BenchmarkQuery> {
    vec![BenchmarkQuery {
        name: "postings_for_holder_range",
        build: postings_for_holder_range_query,
        sqlite: r#"
            SELECT p.id, p.amount
            FROM posting p
            JOIN account a ON a.id = p.account
            WHERE a.holder = ?1 AND p.at >= ?2 AND p.at < ?3
        "#,
    }]
}

fn postings_for_holder_range_query(schema: &SchemaDescriptor) -> QueryBuildResult<TypedQuery> {
    QueryBuilder::new(schema)
        .rel("Posting")?
        .var("id", "posting")?
        .var("account", "account")?
        .var("amount", "amount")?
        .var("at", "t")?
        .done()
        .rel("Account")?
        .var("id", "account")?
        .input("holder", "holder")?
        .done()
        .cmp(
            OperandRef::var("t"),
            ComparisonOperator::Gte,
            OperandRef::input("start"),
        )?
        .cmp(
            OperandRef::var("t"),
            ComparisonOperator::Lt,
            OperandRef::input("end"),
        )?
        .find_var("posting")?
        .find_var("amount")?
        .finish()
}
