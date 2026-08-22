//! No CTE after the rec.
use std::collections::BTreeMap;

use bumbledb::schema::{KeyStatement, StatementDescriptor};
use bumbledb::{InteriorId, ParamId, Query, RelationId, Schema, Value, VarId};

mod builder;
mod derived;
mod query;
#[cfg(test)]
mod tests;
mod types;

pub use query::translate;

fn leaf(tree: &bumbledb::ConditionTree) -> &bumbledb::Comparison {
    match tree {
        bumbledb::ConditionTree::Leaf(comparison) => comparison,
        bumbledb::ConditionTree::And(_) | bumbledb::ConditionTree::Or(_) => {
            unreachable!("the SQL translation consumes flat conjunctions only")
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ParamSlot {
    Whole(ParamId),
    Start(ParamId),
    End(ParamId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Translated {
    pub sql: String,
    pub params: Vec<ParamSlot>,
}

/// # Panics
#[must_use]
pub fn keyed_get(schema: &Schema, relation: RelationId, statement: &KeyStatement) -> Translated {
    let rel = schema.relation(relation);
    let select: Vec<String> = rel
        .fields()
        .iter()
        .flat_map(|field| {
            crate::sqlmap::field_columns(field)
                .into_iter()
                .map(|(name, _)| format!("\"{name}\""))
        })
        .collect();
    let mut params: Vec<ParamSlot> = Vec::new();
    let mut conjuncts: Vec<String> = Vec::new();
    for (position, &field) in statement.projection.iter().enumerate() {
        let param = ParamId(u16::try_from(position).expect("a projection fits u16"));
        let columns = crate::sqlmap::field_columns(&rel.fields()[usize::from(field.0)]);
        if let [(start, _), (end, _)] = columns.as_slice() {
            conjuncts.push(format!("\"{start}\" = ?{}", params.len() + 1));
            params.push(ParamSlot::Start(param));
            conjuncts.push(format!("\"{end}\" = ?{}", params.len() + 1));
            params.push(ParamSlot::End(param));
        } else {
            let (name, _) = &columns[0];
            conjuncts.push(format!("\"{name}\" = ?{}", params.len() + 1));
            params.push(ParamSlot::Whole(param));
        }
    }
    Translated {
        sql: format!(
            "SELECT {} FROM \"{}\" WHERE {}",
            select.join(", "),
            rel.name(),
            conjuncts.join(" AND ")
        ),
        params,
    }
}

#[derive(Debug, Clone)]
enum VarCols {
    Scalar(String),
    Interval { start: String, end: String },
}

struct Builder<'q> {
    schema: &'q Schema,

    types: types::TermTypes,

    sets: &'q [(ParamId, Vec<Value>)],

    from: Vec<String>,

    conditions: Vec<String>,

    deferred: Vec<(String, String, VarId)>,

    columns: BTreeMap<VarId, VarCols>,

    param_index: BTreeMap<ParamSlot, usize>,
    params: Vec<ParamSlot>,

    shape: query::QueryShape,
}

#[must_use]
fn derived_cte_name(id: InteriorId, shape: query::QueryShape) -> String {
    match shape {
        query::QueryShape::Reach { rec } if rec == id => "rec".to_owned(),
        query::QueryShape::Cq | query::QueryShape::Reach { .. } => format!("interior{}", id.0),
    }
}

#[derive(Debug, Clone, Copy)]
pub enum LaneCase<'a> {
    Query(&'a Query),
    Judgment(&'a StatementDescriptor),
}

/// Trigger emulation is refused by decision, not deferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Inexpressible {
    FunctionalityJudgment,

    ContainmentJudgment,

    PackAggregate,

    /// (a `SUM` is a query, not a typed refusal citing a statement).
    CapacityJudgment,

    IntervalDerivedColumn,
}

/// Callers do not choose a gate by shape — one function,
/// # Errors
pub fn sqlite_expressible(case: &LaneCase<'_>) -> Result<(), Inexpressible> {
    sqlite_expressible_on(case, crate::querygen::target::schema())
}

/// # Errors
pub fn sqlite_expressible_on(case: &LaneCase<'_>, schema: &Schema) -> Result<(), Inexpressible> {
    match case {
        LaneCase::Query(query) => {
            if query
                .head()
                .iter()
                .any(|term| matches!(term, bumbledb::HeadTerm::Aggregate(bumbledb::HeadOp::Pack)))
            {
                Err(Inexpressible::PackAggregate)
            } else {
                derived::refuse_interval_columns(query, schema)
                    .map_err(|_| Inexpressible::IntervalDerivedColumn)
            }
        }
        LaneCase::Judgment(StatementDescriptor::Functionality { .. }) => {
            Err(Inexpressible::FunctionalityJudgment)
        }
        LaneCase::Judgment(StatementDescriptor::Containment { .. }) => {
            Err(Inexpressible::ContainmentJudgment)
        }
        LaneCase::Judgment(StatementDescriptor::Capacity { .. }) => {
            Err(Inexpressible::CapacityJudgment)
        }
    }
}

pub mod goldens;
