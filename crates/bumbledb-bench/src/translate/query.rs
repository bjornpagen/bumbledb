use std::collections::BTreeMap;

use bumbledb::ir::{FindTerm, Rule};
use bumbledb::{FoldOp, InteriorId, ParamId, Query, Schema, Value, VarId};

use super::{Builder, ParamSlot, Translated, VarCols, types};

/// # Errors
/// Total over the query grammar with two documented exceptions: a rule whose
/// every atom is a gate (no bound columns exist to select from) — the query
/// generator never produces one — and a `Pack` head, which is naive-only by
/// decision ([`super::sqlite_expressible`] routes it before translation).
pub fn translate(
    query: &Query,
    schema: &Schema,
    sets: &[(ParamId, Vec<Value>)],
) -> Result<Translated, String> {
    super::derived::translate_query(query, schema, sets)
}

pub(super) fn translate_rules(
    rules: &[Rule],
    schema: &Schema,
    sets: &[(ParamId, Vec<Value>)],
    params: &mut SharedParams,
) -> Result<String, String> {
    if let [rule] = rules {
        let b = rule_core(rule, schema, sets, params)?;
        return single_rule_sql(rule, &b);
    }
    let aggregated = rules[0].finds.iter().any(|f| {
        matches!(
            f,
            FindTerm::Count | FindTerm::Aggregate { .. } | FindTerm::Pack { .. }
        )
    });
    let mut arms: Vec<String> = Vec::new();
    for rule in rules {
        let b = rule_core(rule, schema, sets, params)?;
        arms.push(if aggregated {
            head_projection_sql(rule, &b)?
        } else {
            projection_sql(&rule.finds, &b)?
        });
    }
    if aggregated {
        union_fold_sql(&rules[0].finds, &arms)
    } else {
        Ok(arms.join(" UNION "))
    }
}

#[derive(Clone, Copy, Default)]
pub(super) enum QueryShape {
    #[default]
    Cq,
    Reach {
        rec: InteriorId,
    },
}

#[derive(Default)]
pub(super) struct SharedParams {
    index: BTreeMap<ParamSlot, usize>,
    pub(super) params: Vec<ParamSlot>,
    pub(super) shape: QueryShape,
}

pub(super) fn rule_core<'q>(
    rule: &'q Rule,
    schema: &'q Schema,
    sets: &'q [(ParamId, Vec<Value>)],
    params: &mut SharedParams,
) -> Result<Builder<'q>, String> {
    let mut b = Builder {
        schema,
        types: types::infer(rule, schema),
        sets,
        from: Vec::new(),
        conditions: Vec::new(),
        deferred: Vec::new(),
        columns: BTreeMap::new(),
        param_index: std::mem::take(&mut params.index),
        params: std::mem::take(&mut params.params),
        shape: params.shape,
    };
    for atom in &rule.atoms {
        b.render_atom(atom)?;
    }
    b.flush_deferred()?;
    for comparison in rule.conditions.iter().map(super::leaf) {
        b.comparison(comparison)?;
    }

    for (index, atom) in rule.negated.iter().enumerate() {
        b.negated_atom(index, atom)?;
    }
    if b.from.is_empty() {
        return Err("no bound atoms: nothing to select from".to_owned());
    }
    params.index = std::mem::take(&mut b.param_index);
    params.params = std::mem::take(&mut b.params);
    Ok(b)
}

pub(super) fn arm_body(b: &Builder) -> String {
    let (from, where_clause) = from_where(b);
    format!(" FROM {from}{where_clause}")
}

fn from_where(b: &Builder) -> (String, String) {
    let from = b.from.join(", ");
    let where_clause = if b.conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", b.conditions.join(" AND "))
    };
    (from, where_clause)
}

fn single_rule_sql(rule: &Rule, b: &Builder) -> Result<String, String> {
    if rule.finds.iter().any(|f| {
        matches!(
            f,
            FindTerm::Count | FindTerm::Aggregate { .. } | FindTerm::Pack { .. }
        )
    }) {
        let (from, where_clause) = from_where(b);
        fold_sql(&rule.finds, b, &from, &where_clause)
    } else {
        projection_sql(&rule.finds, b)
    }
}

fn projection_sql(finds: &[FindTerm], b: &Builder) -> Result<String, String> {
    let (from, where_clause) = from_where(b);
    let mut cols: Vec<String> = Vec::new();
    for find in finds {
        match find {
            FindTerm::Var(var) => match b.columns.get(var) {
                Some(VarCols::Scalar(column)) => cols.push(column.clone()),

                Some(VarCols::Interval { start, end }) => {
                    cols.push(start.clone());
                    cols.push(end.clone());
                }
                None => return Err(format!("find variable {} unbound", var.0)),
            },
            FindTerm::Count | FindTerm::Aggregate { .. } | FindTerm::Pack { .. } => {
                unreachable!("no aggregates here")
            }
        }
    }
    Ok(format!(
        "SELECT DISTINCT {} FROM {from}{where_clause}",
        cols.join(", ")
    ))
}

fn head_projection_sql(rule: &Rule, b: &Builder) -> Result<String, String> {
    let (from, where_clause) = from_where(b);
    let mut cols: Vec<String> = Vec::new();
    for (position, find) in rule.finds.iter().enumerate() {
        match find {
            FindTerm::Var(var)
            | FindTerm::Aggregate { over: var, .. }
            | FindTerm::Pack { over: var } => match b.columns.get(var) {
                Some(VarCols::Scalar(column)) => cols.push(format!("{column} AS h{position}")),
                Some(VarCols::Interval { start, end }) => {
                    cols.push(format!("{start} AS h{position}_start"));
                    cols.push(format!("{end} AS h{position}_end"));
                }
                None => return Err(format!("find variable {} unbound", var.0)),
            },
            FindTerm::Count => cols.push(format!("0 AS h{position}")),
        }
    }
    Ok(format!(
        "SELECT DISTINCT {} FROM {from}{where_clause}",
        cols.join(", ")
    ))
}

fn union_fold_sql(finds: &[FindTerm], arms: &[String]) -> Result<String, String> {
    let union = arms.join(" UNION ");
    let mut group: Vec<String> = Vec::new();
    let mut outer: Vec<String> = Vec::new();
    for (position, find) in finds.iter().enumerate() {
        match find {
            FindTerm::Var(_) => {
                let names = if matches!(find, FindTerm::Var(_)) {
                    head_group_names(arms, position)
                } else {
                    vec![format!("h{position}")]
                };
                group.extend(names.iter().cloned());
                outer.extend(names);
            }
            FindTerm::Count => outer.push("COUNT(*)".to_owned()),
            FindTerm::Aggregate { op, .. } => outer.push(match op {
                FoldOp::Sum => format!("SUM(h{position})"),
                FoldOp::Mean => return Err("exact F64 Mean has no SQLite numerical oracle".into()),
                FoldOp::Min => format!("MIN(h{position})"),
                FoldOp::Max => format!("MAX(h{position})"),
            }),
            FindTerm::Pack { .. } => {
                // routes Pack heads to the naive lane before translation.
                return Err("Pack is naive-only (no SQL coalesce)".to_owned());
            }
        }
    }
    let tail = if group.is_empty() {
        " HAVING COUNT(*) > 0".to_owned()
    } else {
        format!(" GROUP BY {}", group.join(", "))
    };
    Ok(format!("SELECT {} FROM ({union}){tail}", outer.join(", ")))
}

fn head_group_names(arms: &[String], position: usize) -> Vec<String> {
    if arms
        .first()
        .is_some_and(|arm| arm.contains(&format!("h{position}_start")))
    {
        vec![format!("h{position}_start"), format!("h{position}_end")]
    } else {
        vec![format!("h{position}")]
    }
}

fn inner_columns(b: &Builder) -> Vec<String> {
    let mut cols = Vec::new();
    for (var, columns) in &b.columns {
        match columns {
            VarCols::Scalar(column) => cols.push(format!("{column} AS v{}", var.0)),
            VarCols::Interval { start, end } => {
                cols.push(format!("{start} AS v{}_start", var.0));
                cols.push(format!("{end} AS v{}_end", var.0));
            }
        }
    }
    cols
}

fn var_names(b: &Builder, var: VarId, prefix: &str) -> Result<Vec<String>, String> {
    match b.columns.get(&var) {
        Some(VarCols::Scalar(_)) => Ok(vec![format!("{prefix}v{}", var.0)]),
        Some(VarCols::Interval { .. }) => Ok(vec![
            format!("{prefix}v{}_start", var.0),
            format!("{prefix}v{}_end", var.0),
        ]),
        None => Err(format!("find variable {} unbound", var.0)),
    }
}

fn fold_sql(
    finds: &[FindTerm],
    b: &Builder,
    from: &str,
    where_clause: &str,
) -> Result<String, String> {
    let inner = format!(
        "SELECT DISTINCT {} FROM {from}{where_clause}",
        inner_columns(b).join(", ")
    );
    let mut group: Vec<String> = Vec::new();
    let mut outer: Vec<String> = Vec::new();
    for find in finds {
        match find {
            FindTerm::Var(var) => {
                let names = var_names(b, *var, "")?;
                group.extend(names.iter().cloned());
                outer.extend(names);
            }
            FindTerm::Count => outer.push("COUNT(*)".to_owned()),
            FindTerm::Aggregate { op, over } => outer.push({
                let agg = match op {
                    FoldOp::Sum => "SUM",
                    FoldOp::Mean => return Err("exact F64 Mean has no SQLite numerical oracle".into()),
                    FoldOp::Min => "MIN",
                    FoldOp::Max => "MAX",
                };
                format!("{agg}(v{})", over.0)
            }),
            FindTerm::Pack { .. } => {
                return Err("Pack is naive-only (no SQL coalesce)".to_owned());
            }
        }
    }
    let tail = if group.is_empty() {
        " HAVING COUNT(*) > 0".to_owned()
    } else {
        format!(" GROUP BY {}", group.join(", "))
    };
    Ok(format!("SELECT {} FROM ({inner}){tail}", outer.join(", ")))
}
