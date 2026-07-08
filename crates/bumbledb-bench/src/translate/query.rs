use std::collections::BTreeMap;

use bumbledb::ir::FindTerm;
use bumbledb::{AggOp, Query, Schema};

use super::{Builder, Translated};

/// Translates one validated-shape query over the given schema.
///
/// # Errors
///
/// A message naming the untranslatable construct. Total over the ledger
/// grammar with one documented exception: a query whose every atom is a
/// gate (no bound columns exist to select from) — the query generator
/// never produces one.
pub fn translate(query: &Query, schema: &Schema) -> Result<Translated, String> {
    let mut b = Builder {
        schema,
        from: Vec::new(),
        predicates: Vec::new(),
        columns: BTreeMap::new(),
        param_index: BTreeMap::new(),
        params: Vec::new(),
    };
    for atom in &query.atoms {
        b.atom(atom)?;
    }
    for comparison in &query.predicates {
        b.comparison(comparison)?;
    }
    if b.from.is_empty() {
        return Err("no bound atoms: nothing to select from".to_owned());
    }

    let from = b.from.join(", ");
    let where_clause = if b.predicates.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", b.predicates.join(" AND "))
    };

    let has_aggregates = query
        .finds
        .iter()
        .any(|f| matches!(f, FindTerm::Aggregate { .. }));

    let sql = if has_aggregates {
        // The normative template: fold over the DISTINCT full binding set.
        let mut inner_cols: Vec<String> = Vec::new();
        for (var, column) in &b.columns {
            inner_cols.push(format!("{column} AS v{}", var.0));
        }
        let inner = format!(
            "SELECT DISTINCT {} FROM {from}{where_clause}",
            inner_cols.join(", ")
        );
        let group: Vec<String> = query
            .finds
            .iter()
            .filter_map(|f| match f {
                FindTerm::Var(var) => Some(format!("v{}", var.0)),
                FindTerm::Aggregate { .. } => None,
            })
            .collect();
        let outer_cols: Vec<String> = query
            .finds
            .iter()
            .map(|f| match f {
                FindTerm::Var(var) => format!("v{}", var.0),
                FindTerm::Aggregate { op, over } => {
                    let agg = match op {
                        AggOp::Sum => "SUM",
                        AggOp::Min => "MIN",
                        AggOp::Max => "MAX",
                        AggOp::Count => "COUNT",
                    };
                    match over {
                        Some(var) => format!("{agg}(v{})", var.0),
                        None => "COUNT(*)".to_owned(),
                    }
                }
            })
            .collect();
        let tail = if group.is_empty() {
            // Global aggregate: SQL yields one NULL row over empty input;
            // the engine yields the empty set. HAVING collapses them.
            " HAVING COUNT(*) > 0".to_owned()
        } else {
            format!(" GROUP BY {}", group.join(", "))
        };
        format!("SELECT {} FROM ({inner}){tail}", outer_cols.join(", "))
    } else {
        let cols: Vec<String> = query
            .finds
            .iter()
            .map(|f| match f {
                FindTerm::Var(var) => b
                    .columns
                    .get(var)
                    .cloned()
                    .ok_or_else(|| format!("find variable {} unbound", var.0)),
                FindTerm::Aggregate { .. } => unreachable!("no aggregates here"),
            })
            .collect::<Result<_, _>>()?;
        format!(
            "SELECT DISTINCT {} FROM {from}{where_clause}",
            cols.join(", ")
        )
    };

    Ok(Translated {
        sql,
        params: b.params,
    })
}
