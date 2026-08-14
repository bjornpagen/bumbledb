//! Derived tables → `WITH [RECURSIVE]` (lossy SQLite image of this
//! cut). Interiors emit as `interior{id}` CTEs in declaration order;
//! rec is `rec`. Main is the SELECT. Zero CTEs is a plain query — the
//! same SQL as the old CQ path. No UNION ALL. No CTE after the rec —
//! `CLOSURE_ROOTS` inlines the anti-join into main.

use bumbledb::ir::{FindTerm, Rec};
use bumbledb::{AtomSource, InteriorId, ParamId, Query, Rule, Schema, Term, Value};

use super::query::{SharedParams, arm_body, rule_core};
use super::{Translated, VarCols, derived_cte_name};

/// Translate a Query: derived tables as CTEs in declaration order, then
/// main. Zero CTEs → no `WITH` clause (the CQ path).
///
/// # Errors
///
/// Interval-typed derived columns, or anything [`super::query::translate_rules`]
/// names on a rule.
pub fn translate_query(
    query: &Query,
    schema: &Schema,
    sets: &[(ParamId, Vec<Value>)],
) -> Result<Translated, String> {
    refuse_interval_columns(query, schema)?;
    let mut params = SharedParams::default();
    params.rec = rec_id(query);
    let mut ctes: Vec<String> = Vec::new();
    for (index, interior) in query.interiors.iter().enumerate() {
        ctes.push(cte_from_rules(
            index,
            &interior.head,
            &interior.rules,
            schema,
            sets,
            &mut params,
        )?);
    }
    if let Some(rec) = &query.rec {
        ctes.push(rec_cte(
            query.interiors.len(),
            rec,
            schema,
            sets,
            &mut params,
        )?);
    }
    let main = main_select(&query.rules, schema, sets, &mut params)?;
    if ctes.is_empty() {
        return Ok(Translated {
            sql: main,
            params: params.params,
        });
    }
    let with = if query.rec.is_some() {
        format!("WITH RECURSIVE {}", ctes.join(", "))
    } else {
        format!("WITH {}", ctes.join(", "))
    };
    Ok(Translated {
        sql: format!("{with} {main}"),
        params: params.params,
    })
}

fn cte_from_rules(
    index: usize,
    head: &[bumbledb::HeadTerm],
    rules: &[Rule],
    schema: &Schema,
    sets: &[(ParamId, Vec<Value>)],
    params: &mut SharedParams,
) -> Result<String, String> {
    let arms = rule_arms(rules, schema, sets, params)?;
    let columns: Vec<String> = (0..head.len()).map(|column| format!("c{column}")).collect();
    Ok(format!(
        "{}({}) AS ({})",
        derived_cte_name(
            InteriorId(u32::try_from(index).expect("interior id fits u32")),
            params.rec
        ),
        columns.join(", "),
        arms.join(" UNION ")
    ))
}

fn rec_cte(
    _rec_id: usize,
    rec: &Rec,
    schema: &Schema,
    sets: &[(ParamId, Vec<Value>)],
    params: &mut SharedParams,
) -> Result<String, String> {
    if rec.base.is_empty() && !rec.rec.is_empty() {
        return Err("recursive CTE rec has no base rule (its fixpoint is empty)".into());
    }
    let mut arms = rule_arms(&rec.base, schema, sets, params)?;
    arms.extend(rule_arms(&rec.rec, schema, sets, params)?);
    let columns: Vec<String> = (0..rec.head.len())
        .map(|column| format!("c{column}"))
        .collect();
    Ok(format!(
        "rec({}) AS ({})",
        columns.join(", "),
        arms.join(" UNION ")
    ))
}

fn rule_arms(
    rules: &[Rule],
    schema: &Schema,
    sets: &[(ParamId, Vec<Value>)],
    params: &mut SharedParams,
) -> Result<Vec<String>, String> {
    let mut arms = Vec::new();
    for rule in rules {
        let b = rule_core(rule, schema, sets, params)?;
        let mut cols: Vec<String> = Vec::new();
        for (position, find) in rule.finds.iter().enumerate() {
            let expr = match find {
                FindTerm::Var(var) => match b.columns.get(var) {
                    Some(VarCols::Scalar(column)) => column.clone(),
                    Some(VarCols::Interval { .. }) => {
                        return Err(format!(
                            "interval-typed predicate column c{position} \
                             (the recursive lane is scalar-shaped)"
                        ));
                    }
                    None => return Err(format!("find variable {} unbound", var.0)),
                },
                FindTerm::Measure(var) => match b.columns.get(var) {
                    Some(VarCols::Interval { start, end }) => format!("({end} - {start})"),
                    _ => return Err(format!("Duration over non-interval variable {}", var.0)),
                },
                FindTerm::Aggregate { .. } | FindTerm::AggregateMeasure { .. } => {
                    return Err("folds on interiors/rec arms are refused".into());
                }
            };
            cols.push(format!("{expr} AS c{position}"));
        }
        arms.push(format!("SELECT {}{}", cols.join(", "), arm_body(&b)));
    }
    Ok(arms)
}

fn main_select(
    rules: &[Rule],
    schema: &Schema,
    sets: &[(ParamId, Vec<Value>)],
    params: &mut SharedParams,
) -> Result<String, String> {
    super::query::translate_rules(rules, schema, sets, params)
}

fn rec_id(query: &Query) -> Option<InteriorId> {
    query
        .rec
        .is_some()
        .then(|| InteriorId(u32::try_from(query.interiors.len()).expect("interior id fits u32")))
}

/// Interval-typed derived columns are the remaining translator limit.
///
/// # Errors
///
/// An interior or rec head column whose type is interval.
pub fn refuse_interval_columns(query: &Query, schema: &Schema) -> Result<(), String> {
    let mut flags: Vec<Vec<bool>> = Vec::new();
    for interior in &query.interiors {
        flags.push(head_intervals(
            &interior.head,
            &interior.rules,
            schema,
            &flags,
        ));
        if flags.last().is_some_and(|row| row.iter().any(|b| *b)) {
            return Err(
                "interval-typed derived column (the recursive lane is scalar-shaped)".into(),
            );
        }
    }
    if let Some(rec) = &query.rec {
        let row = head_intervals(&rec.head, &rec.base, schema, &flags);
        if row.iter().any(|b| *b) {
            return Err(
                "interval-typed derived column (the recursive lane is scalar-shaped)".into(),
            );
        }
    }
    Ok(())
}

fn head_intervals(
    _head: &[bumbledb::HeadTerm],
    rules: &[Rule],
    schema: &Schema,
    prior: &[Vec<bool>],
) -> Vec<bool> {
    let Some(rule) = rules.first() else {
        return Vec::new();
    };
    rule.finds
        .iter()
        .map(|find| match find {
            FindTerm::Var(var) => rule.atoms.iter().all(|atom| {
                !atom.bindings.iter().any(|(field, term)| {
                    matches!(term, Term::Var(v) if *v == *var)
                        && !col_interval(atom.source, *field, schema, prior)
                })
            }),
            _ => false,
        })
        .collect()
}

fn col_interval(
    source: AtomSource,
    field: bumbledb::FieldId,
    schema: &Schema,
    prior: &[Vec<bool>],
) -> bool {
    match source {
        AtomSource::Edb(relation) => matches!(
            schema.relation(relation).fields()[usize::from(field.0)].value_type,
            bumbledb::schema::ValueType::Interval { .. }
        ),
        AtomSource::Interior(InteriorId(id)) => prior
            .get(id as usize)
            .and_then(|row| row.get(usize::from(field.0)))
            .copied()
            .unwrap_or(false),
    }
}
