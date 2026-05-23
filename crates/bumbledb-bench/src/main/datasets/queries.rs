use bumbledb_core::query_builder::{OperandRef, QueryBuildResult, QueryBuilder};
use bumbledb_core::query_ir::{ComparisonOperator, TypedQuery};
use bumbledb_core::schema::SchemaDescriptor;

pub(crate) fn build_ledger_postings_for_holder_range(
    schema: &SchemaDescriptor,
) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
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

pub(crate) fn build_ledger_balances_by_instrument(
    schema: &SchemaDescriptor,
) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("Posting")?
        .var("id", "posting")?
        .var("account", "account")?
        .var("instrument", "instrument")?
        .var("amount", "amount")?
        .var("at", "t")?
        .done()
        .rel("Account")?
        .var("id", "account")?
        .input("holder", "holder")?
        .done()
        .find_var("instrument")?
        .find_var("amount")?
        .finish()
}

pub(crate) fn build_ledger_tag_lookup_join(
    schema: &SchemaDescriptor,
) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("PostingTag")?
        .var("posting", "posting")?
        .input("tag", "tag")?
        .done()
        .rel("Posting")?
        .var("id", "posting")?
        .var("account", "account")?
        .done()
        .find_var("posting")?
        .find_var("account")?
        .finish()
}

pub(crate) fn build_sailors_red_boat_sailors(
    schema: &SchemaDescriptor,
) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("Reserve")?
        .var("sailor", "sailor")?
        .var("boat", "boat")?
        .done()
        .rel("Boat")?
        .var("id", "boat")?
        .input("color", "color")?
        .done()
        .rel("Sailor")?
        .var("id", "sailor")?
        .var("rating", "rating")?
        .done()
        .find_var("sailor")?
        .find_var("rating")?
        .finish()
}

pub(crate) fn build_sailors_sailor_range_reserves(
    schema: &SchemaDescriptor,
) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("Reserve")?
        .input("sailor", "sailor")?
        .var("boat", "boat")?
        .var("day", "day")?
        .done()
        .cmp(
            OperandRef::var("day"),
            ComparisonOperator::Gte,
            OperandRef::input("start"),
        )?
        .cmp(
            OperandRef::var("day"),
            ComparisonOperator::Lt,
            OperandRef::input("end"),
        )?
        .find_var("boat")?
        .find_var("day")?
        .finish()
}

pub(crate) fn build_sailors_high_rating_red_boats(
    schema: &SchemaDescriptor,
) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("Sailor")?
        .var("id", "sailor")?
        .var("rating", "rating")?
        .done()
        .rel("Reserve")?
        .var("sailor", "sailor")?
        .var("boat", "boat")?
        .done()
        .rel("Boat")?
        .var("id", "boat")?
        .input("color", "color")?
        .done()
        .cmp(
            OperandRef::var("rating"),
            ComparisonOperator::Gte,
            OperandRef::input("min_rating"),
        )?
        .find_var("sailor")?
        .find_var("boat")?
        .finish()
}

pub(crate) fn build_joinstress_chain4_from_a(
    schema: &SchemaDescriptor,
) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("A")?
        .input("id", "a")?
        .done()
        .rel("B")?
        .var("id", "b")?
        .input("a", "a")?
        .done()
        .rel("C")?
        .var("id", "c")?
        .var("b", "b")?
        .done()
        .rel("D")?
        .var("id", "d")?
        .var("c", "c")?
        .done()
        .find_var("d")?
        .finish()
}

pub(crate) fn build_joinstress_triangle_count(
    schema: &SchemaDescriptor,
) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("EdgeAB")?
        .var("a", "a")?
        .var("b", "b")?
        .done()
        .rel("EdgeAC")?
        .var("a", "a")?
        .var("c", "c")?
        .done()
        .rel("EdgeBC")?
        .var("b", "b")?
        .var("c", "c")?
        .done()
        .find_var("a")?
        .finish()
}

pub(crate) fn build_tpch_revenue_by_customer_range(
    schema: &SchemaDescriptor,
) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("Customer")?
        .var("id", "customer")?
        .input("nation", "nation")?
        .done()
        .rel("Orders")?
        .var("id", "order")?
        .var("customer", "customer")?
        .done()
        .rel("LineItem")?
        .var("id", "line")?
        .var("order", "order")?
        .var("extended_price", "price")?
        .var("ship_date", "ship")?
        .done()
        .cmp(
            OperandRef::var("ship"),
            ComparisonOperator::Gte,
            OperandRef::input("start"),
        )?
        .cmp(
            OperandRef::var("ship"),
            ComparisonOperator::Lt,
            OperandRef::input("end"),
        )?
        .find_var("customer")?
        .find_var("price")?
        .finish()
}

pub(crate) fn build_tpch_supplier_nation_orders(
    schema: &SchemaDescriptor,
) -> QueryBuildResult<TypedQuery> {
    let mut query = QueryBuilder::new(schema);
    query
        .rel("Supplier")?
        .var("id", "supplier")?
        .input("nation", "nation")?
        .done()
        .rel("LineItem")?
        .var("id", "line")?
        .var("order", "order")?
        .var("supplier", "supplier")?
        .done()
        .rel("Orders")?
        .var("id", "order")?
        .var("customer", "customer")?
        .done()
        .find_var("line")?
        .find_var("order")?
        .finish()
}
