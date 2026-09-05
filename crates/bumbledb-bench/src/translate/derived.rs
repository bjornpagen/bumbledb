//! No CTE after the rec —
use bumbledb::ir::{FindTerm, ProjectionRule, Rec, RecRule, RecStep};
use bumbledb::{AtomSource, InteriorId, ParamId, Query, Rule, Schema, Term, Value};

use super::query::{QueryShape, SharedParams, arm_body, rule_core};
use super::{Translated, VarCols, derived_cte_name};

/// # Errors
pub fn translate_query(
    query: &Query,
    schema: &Schema,
    sets: &[(ParamId, Vec<Value>)],
) -> Result<Translated, String> {
    refuse_interval_columns(query, schema)?;
    refuse_float_arithmetic(query, schema)?;
    match query {
        Query {
            interiors,
            rules,
            rec: None,
            ..
        } => translate_cq(interiors, rules, schema, sets),
        Query {
            interiors,
            rec: Some(rec),
            rules,
            ..
        } => translate_reach(interiors, rec, rules, schema, sets),
    }
}

/// The lossless F64 mirror stores ordered BLOBs. SQLite SUM would silently
/// coerce those bytes to unrelated numbers; it is never an arithmetic oracle.
pub(super) fn refuse_float_arithmetic(query: &Query, schema: &Schema) -> Result<(), String> {
    let mut float_columns: Vec<Vec<bool>> = Vec::new();
    for interior in &query.interiors {
        float_columns.push(float_head(
            &interior.rules[0].to_rule(),
            schema,
            &float_columns,
        ));
    }
    if let Some(rec) = &query.rec {
        float_columns.push(float_head(&rec.base[0].to_rule(), schema, &float_columns));
    }
    for rule in &query.rules {
        for find in &rule.finds {
            if let FindTerm::Aggregate {
                op: bumbledb::FoldOp::Sum | bumbledb::FoldOp::Mean,
                over,
            } = find
                && float_var(*over, rule, schema, &float_columns)
            {
                return Err(
                    "F64 arithmetic requires an exact numerical oracle, not SQLite BLOB SUM".into(),
                );
            }
        }
    }
    Ok(())
}

fn float_head(rule: &Rule, schema: &Schema, prior: &[Vec<bool>]) -> Vec<bool> {
    rule.finds
        .iter()
        .map(|find| match find {
            FindTerm::Var(var) => float_var(*var, rule, schema, prior),
            _ => false,
        })
        .collect()
}

fn float_var(var: bumbledb::VarId, rule: &Rule, schema: &Schema, prior: &[Vec<bool>]) -> bool {
    rule.atoms.iter().any(|atom| {
        atom.bindings.iter().any(|(field, term)| {
            matches!(term, Term::Var(v) if *v == var)
                && match atom.source {
                    AtomSource::Edb(relation) => {
                        schema.relation(relation).field(*field).value_type
                            == bumbledb::schema::ValueType::F64
                    }
                    AtomSource::Interior(InteriorId(id)) => prior
                        .get(id as usize)
                        .and_then(|row| row.get(usize::from(field.0)))
                        .copied()
                        .unwrap_or(false),
                }
        })
    })
}

fn translate_cq(
    interiors: &[bumbledb::Interior],
    rules: &[Rule],
    schema: &Schema,
    sets: &[(ParamId, Vec<Value>)],
) -> Result<Translated, String> {
    let mut params = SharedParams::default();
    params.shape = QueryShape::Cq;
    let mut ctes: Vec<String> = Vec::new();
    for (index, interior) in interiors.iter().enumerate() {
        ctes.push(cte_from_interior(
            index,
            interior,
            schema,
            sets,
            &mut params,
        )?);
    }
    let main = main_select(rules, schema, sets, &mut params)?;
    if ctes.is_empty() {
        return Ok(Translated {
            sql: main,
            params: params.params,
        });
    }
    Ok(Translated {
        sql: format!("WITH {} {main}", ctes.join(", ")),
        params: params.params,
    })
}

fn translate_reach(
    interiors: &[bumbledb::Interior],
    rec: &Rec,
    rules: &[Rule],
    schema: &Schema,
    sets: &[(ParamId, Vec<Value>)],
) -> Result<Translated, String> {
    let mut params = SharedParams::default();
    params.shape = QueryShape::Reach {
        rec: InteriorId(u32::try_from(interiors.len()).expect("interior id fits u32")),
    };
    let mut ctes: Vec<String> = Vec::new();
    for (index, interior) in interiors.iter().enumerate() {
        ctes.push(cte_from_interior(
            index,
            interior,
            schema,
            sets,
            &mut params,
        )?);
    }
    ctes.push(rec_cte(interiors.len(), rec, schema, sets, &mut params)?);
    let main = main_select(rules, schema, sets, &mut params)?;
    Ok(Translated {
        sql: format!("WITH RECURSIVE {} {main}", ctes.join(", ")),
        params: params.params,
    })
}

fn cte_from_interior(
    index: usize,
    interior: &bumbledb::Interior,
    schema: &Schema,
    sets: &[(ParamId, Vec<Value>)],
    params: &mut SharedParams,
) -> Result<String, String> {
    let rules: Vec<Rule> = interior.rules.iter().map(ProjectionRule::to_rule).collect();
    let arms = rule_arms(&rules, schema, sets, params)?;
    let columns: Vec<String> = (0..interior.head().len())
        .map(|column| format!("c{column}"))
        .collect();
    Ok(format!(
        "{}({}) AS ({})",
        derived_cte_name(
            InteriorId(u32::try_from(index).expect("interior id fits u32")),
            params.shape
        ),
        columns.join(", "),
        arms.join(" UNION ")
    ))
}

fn rec_cte(
    rec_idx: usize,
    rec: &Rec,
    schema: &Schema,
    sets: &[(ParamId, Vec<Value>)],
    params: &mut SharedParams,
) -> Result<String, String> {
    let rec_id = InteriorId(u32::try_from(rec_idx).expect("interior id fits u32"));
    let base: Vec<Rule> = rec.base.iter().map(RecRule::to_rule).collect();
    let step: Vec<Rule> = rec
        .rec
        .iter()
        .map(|arm| RecStep::to_written_rule(arm, rec_id))
        .collect();
    let mut arms = rule_arms(&base, schema, sets, params)?;
    arms.extend(rule_arms(&step, schema, sets, params)?);
    let columns: Vec<String> = (0..rec.head().len())
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
                            "interval-typed derived column c{position} \
                             (the recursive lane is scalar-shaped)"
                        ));
                    }
                    None => return Err(format!("find variable {} unbound", var.0)),
                },
                FindTerm::Count | FindTerm::Aggregate { .. } | FindTerm::Pack { .. } => {
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

/// # Errors
pub fn refuse_interval_columns(query: &Query, schema: &Schema) -> Result<(), String> {
    match query {
        Query {
            interiors,
            rec: None,
            ..
        } => {
            refuse_interior_intervals(interiors, schema)?;
            Ok(())
        }
        Query {
            interiors,
            rec: Some(rec),
            ..
        } => {
            let flags = refuse_interior_intervals(interiors, schema)?;
            let base: Vec<Rule> = rec.base.iter().map(RecRule::to_rule).collect();
            let row = head_intervals(&rec.head(), &base, schema, &flags);
            if row.iter().any(|b| *b) {
                return Err(
                    "interval-typed derived column (the recursive lane is scalar-shaped)".into(),
                );
            }
            Ok(())
        }
    }
}

fn refuse_interior_intervals(
    interiors: &[bumbledb::Interior],
    schema: &Schema,
) -> Result<Vec<Vec<bool>>, String> {
    let mut flags: Vec<Vec<bool>> = Vec::new();
    for interior in interiors {
        let rules: Vec<Rule> = interior.rules.iter().map(ProjectionRule::to_rule).collect();
        flags.push(head_intervals(&interior.head(), &rules, schema, &flags));
        if flags.last().is_some_and(|row| row.iter().any(|b| *b)) {
            return Err(
                "interval-typed derived column (the recursive lane is scalar-shaped)".into(),
            );
        }
    }
    Ok(flags)
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
        AtomSource::Edb(relation) => schema.relation(relation).fields()[usize::from(field.0)]
            .value_type
            .is_interval(),
        AtomSource::Interior(InteriorId(id)) => prior
            .get(id as usize)
            .and_then(|row| row.get(usize::from(field.0)))
            .copied()
            .unwrap_or(false),
    }
}
