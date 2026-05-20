//! Reusable query workloads.

use bumbledb_core::query_builder::{OperandRef, QueryBuildResult, QueryBuilder};
use bumbledb_core::query_ir::{AggregateFunction, ComparisonOperator, TypedQuery};
use bumbledb_core::schema::SchemaDescriptor;

/// Representative supported positive ledger queries.
pub fn ledger_queries(schema: &SchemaDescriptor) -> QueryBuildResult<Vec<TypedQuery>> {
    Ok(vec![
        QueryBuilder::new(schema)
            .rel("Account")?
            .var("id", "account")?
            .input("holder", "holder")?
            .done()
            .find_var("account")?
            .finish()?,
        QueryBuilder::new(schema)
            .rel("Account")?
            .var("id", "account")?
            .var("holder", "holder")?
            .done()
            .rel("Holder")?
            .var("id", "holder")?
            .var("name", "holder_name")?
            .done()
            .find_var("account")?
            .find_var("holder_name")?
            .finish()?,
        QueryBuilder::new(schema)
            .rel("Posting")?
            .var("id", "posting")?
            .var("account", "account")?
            .var("amount", "amount")?
            .var("at", "t")?
            .done()
            .rel("Account")?
            .var("id", "account")?
            .var("holder", "holder")?
            .done()
            .rel("Holder")?
            .var("id", "holder")?
            .var("name", "holder_name")?
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
            .find_var("account")?
            .find_var("holder_name")?
            .finish()?,
        QueryBuilder::new(schema)
            .rel("Posting")?
            .var("id", "posting")?
            .var("account", "account")?
            .var("amount", "amount")?
            .var("at", "t")?
            .done()
            .find_var("account")?
            .find_aggregate(AggregateFunction::Sum, "amount")?
            .find_aggregate(AggregateFunction::Count, "posting")?
            .finish()?,
    ])
}
