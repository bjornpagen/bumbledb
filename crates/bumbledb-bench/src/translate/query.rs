use std::collections::BTreeMap;

use bumbledb::ir::{FindTerm, Rule};
use bumbledb::{AggOp, ParamId, Query, Schema, Value, VarId};

use super::{Builder, ParamSlot, Translated, VarCols, types};

/// Translates one validated-shape query over the given schema. `sets`
/// carries the bound element list of every set param: set params render
/// as literal `IN` lists (empty ⇒ `1 = 0`), so set-bound queries are
/// **re-rendered per execution** and prepared-statement parity is not
/// claimed for them (`docs/architecture/60-validation.md`).
///
/// **Rules → `UNION`** (the systematized form): a multi-rule projection
/// emits one `SELECT DISTINCT` per rule joined by `UNION` — `SQLite`'s
/// `UNION` is exactly ∪ under `DISTINCT` discipline. A multi-rule
/// aggregate head folds over the union of the rules' head-projected
/// `SELECT DISTINCT` rows (the union-fold, mirroring the rules-IR
/// definition); the single-rule fold domain stays the rule's distinct
/// **full** binding set, unchanged. Params are query-global: every rule
/// shares one positional `?N` space.
///
/// # Errors
///
/// A message naming the untranslatable construct. Total over the query
/// grammar with two documented exceptions: a rule whose every atom is a
/// gate (no bound columns exist to select from) — the query generator
/// never produces one — and a `Pack` head, which is naive-only by
/// decision ([`super::sqlite_expressible`] routes it before
/// translation). Dependency judgments are the enumerated inexpressible
/// set; no other *query* construct is inexpressible.
pub fn translate(
    query: &Query,
    schema: &Schema,
    sets: &[(ParamId, Vec<Value>)],
) -> Result<Translated, String> {
    let mut params = SharedParams::default();
    if let [rule] = query.rules.as_slice() {
        let b = rule_core(rule, schema, sets, &mut params)?;
        let sql = single_rule_sql(rule, &b)?;
        return Ok(Translated {
            sql,
            params: params.params,
        });
    }
    let aggregated = query.rules[0].finds.iter().any(|f| {
        matches!(
            f,
            FindTerm::Aggregate { .. } | FindTerm::AggregateDuration { .. }
        )
    });
    let mut arms: Vec<String> = Vec::new();
    for rule in &query.rules {
        let b = rule_core(rule, schema, sets, &mut params)?;
        arms.push(if aggregated {
            head_projection_sql(rule, &b)?
        } else {
            projection_sql(&rule.finds, &b)?
        });
    }
    let sql = if aggregated {
        union_fold_sql(&query.rules[0].finds, &arms)?
    } else {
        // One `SELECT DISTINCT` per rule joined by `UNION` — set union.
        arms.join(" UNION ")
    };
    Ok(Translated {
        sql,
        params: params.params,
    })
}

/// The query-global positional param space, threaded through the
/// per-rule builders (a param repeated across rules keeps one `?N`).
#[derive(Default)]
struct SharedParams {
    index: BTreeMap<ParamSlot, usize>,
    params: Vec<ParamSlot>,
}

/// Builds one rule's core (FROM entries, WHERE conjuncts, variable
/// columns) — the conjunctive walk every template selects from.
fn rule_core<'q>(
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
        predicates: Vec::new(),
        deferred: Vec::new(),
        columns: BTreeMap::new(),
        param_index: std::mem::take(&mut params.index),
        params: std::mem::take(&mut params.params),
    };
    for atom in &rule.atoms {
        b.render_atom(atom)?;
    }
    b.flush_deferred()?;
    for comparison in rule.predicates.iter().map(super::leaf) {
        b.comparison(comparison)?;
    }
    // Negation last: the NOT EXISTS subqueries append to the core's WHERE.
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

fn from_where(b: &Builder) -> (String, String) {
    let from = b.from.join(", ");
    let where_clause = if b.predicates.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", b.predicates.join(" AND "))
    };
    (from, where_clause)
}

/// The single-rule templates — the conjunctive query's three forms
/// (projection, fold, Arg join-back), unchanged from the pre-rules
/// translator.
fn single_rule_sql(rule: &Rule, b: &Builder) -> Result<String, String> {
    let (from, where_clause) = from_where(b);
    if let Some((key, is_max)) = arg_restriction(&rule.finds) {
        arg_sql(&rule.finds, b, &from, &where_clause, key, is_max)
    } else if rule.finds.iter().any(|f| {
        matches!(
            f,
            FindTerm::Aggregate { .. } | FindTerm::AggregateDuration { .. }
        )
    }) {
        fold_sql(&rule.finds, b, &from, &where_clause)
    } else {
        projection_sql(&rule.finds, b)
    }
}

/// One rule's `SELECT DISTINCT` over its find columns — the projection
/// template, and the multi-rule union's per-rule arm.
fn projection_sql(finds: &[FindTerm], b: &Builder) -> Result<String, String> {
    let (from, where_clause) = from_where(b);
    let mut cols: Vec<String> = Vec::new();
    for find in finds {
        match find {
            FindTerm::Var(var) => match b.columns.get(var) {
                Some(VarCols::Scalar(column)) => cols.push(column.clone()),
                // An interval find projects both halves; the decode
                // path reassembles the value (`crate::sqlmap`).
                Some(VarCols::Interval { start, end }) => {
                    cols.push(start.clone());
                    cols.push(end.clone());
                }
                None => return Err(format!("find variable {} unbound", var.0)),
            },
            // The measure: end − start arithmetic over the halves.
            FindTerm::Duration(var) => match b.columns.get(var) {
                Some(VarCols::Interval { start, end }) => {
                    cols.push(format!("({end} - {start})"));
                }
                Some(VarCols::Scalar(_)) => {
                    return Err(format!("Duration over scalar variable {}", var.0));
                }
                None => return Err(format!("find variable {} unbound", var.0)),
            },
            FindTerm::Aggregate { .. } | FindTerm::AggregateDuration { .. } => {
                unreachable!("no aggregates here")
            }
        }
    }
    Ok(format!(
        "SELECT DISTINCT {} FROM {from}{where_clause}",
        cols.join(", ")
    ))
}

/// One rule's head-projected `SELECT DISTINCT` — the multi-rule
/// union-fold's per-rule arm, columns aliased positionally (`hN`;
/// interval positions `hN_start`/`hN_end`): a variable position projects
/// its value, a measure position projects the measure, an aggregate
/// position projects its fold input (the nullary `Count` a constant
/// filler — positions stay stable), exactly the naive model's
/// union-fold domain rows.
fn head_projection_sql(rule: &Rule, b: &Builder) -> Result<String, String> {
    let (from, where_clause) = from_where(b);
    let mut cols: Vec<String> = Vec::new();
    for (position, find) in rule.finds.iter().enumerate() {
        match find {
            FindTerm::Var(var)
            | FindTerm::Aggregate {
                over: Some(var), ..
            } => match b.columns.get(var) {
                Some(VarCols::Scalar(column)) => cols.push(format!("{column} AS h{position}")),
                Some(VarCols::Interval { start, end }) => {
                    cols.push(format!("{start} AS h{position}_start"));
                    cols.push(format!("{end} AS h{position}_end"));
                }
                None => return Err(format!("find variable {} unbound", var.0)),
            },
            FindTerm::Duration(var) | FindTerm::AggregateDuration { over: var, .. } => {
                match b.columns.get(var) {
                    Some(VarCols::Interval { start, end }) => {
                        cols.push(format!("({end} - {start}) AS h{position}"));
                    }
                    _ => return Err(format!("Duration over non-interval variable {}", var.0)),
                }
            }
            FindTerm::Aggregate { over: None, .. } => cols.push(format!("0 AS h{position}")),
        }
    }
    Ok(format!(
        "SELECT DISTINCT {} FROM {from}{where_clause}",
        cols.join(", ")
    ))
}

/// The multi-rule union fold: the aggregate applied over the `UNION` of
/// the rules' head-projected distinct rows, grouped by the variable and
/// measure positions — the SQL form of the naive model's union fold
/// (per-rule dedup at head granularity, one set union, then the fold).
fn union_fold_sql(finds: &[FindTerm], arms: &[String]) -> Result<String, String> {
    let union = arms.join(" UNION ");
    let mut group: Vec<String> = Vec::new();
    let mut outer: Vec<String> = Vec::new();
    for (position, find) in finds.iter().enumerate() {
        match find {
            FindTerm::Var(_) | FindTerm::Duration(_) => {
                // Interval group positions carry two columns; the pinned
                // head row names which (validation aligns rules).
                let names = if matches!(find, FindTerm::Var(_)) {
                    // The arm aliased scalar vars `hN` and interval vars
                    // `hN_start`/`hN_end`; group by whichever exists —
                    // rendered from the first arm's alias shape.
                    head_group_names(arms, position)
                } else {
                    vec![format!("h{position}")]
                };
                group.extend(names.iter().cloned());
                outer.extend(names);
            }
            FindTerm::AggregateDuration { op, .. } => outer.push({
                let agg = match op {
                    AggOp::Sum => "SUM",
                    AggOp::Min => "MIN",
                    AggOp::Max => "MAX",
                    _ => return Err("measure folds are Sum/Min/Max".to_owned()),
                };
                format!("{agg}(h{position})")
            }),
            FindTerm::Aggregate { op, .. } => outer.push(match op {
                AggOp::Sum => format!("SUM(h{position})"),
                AggOp::Min => format!("MIN(h{position})"),
                AggOp::Max => format!("MAX(h{position})"),
                AggOp::Count => "COUNT(*)".to_owned(),
                AggOp::CountDistinct => {
                    if arms
                        .first()
                        .is_some_and(|arm| arm.contains(&format!("h{position}_start")))
                    {
                        // COUNT(DISTINCT ...) takes one expression: an
                        // interval's halves concatenate through an
                        // injective decimal rendering.
                        format!("COUNT(DISTINCT h{position}_start || ',' || h{position}_end)")
                    } else {
                        format!("COUNT(DISTINCT h{position})")
                    }
                }
                AggOp::ArgMax { .. } | AggOp::ArgMin { .. } => {
                    unreachable!("validation refuses Arg-restriction across rules")
                }
                // The expressibility gate (`super::sqlite_expressible`)
                // routes Pack heads to the naive lane before translation.
                AggOp::Pack => return Err("Pack is naive-only (no SQL coalesce)".to_owned()),
            }),
        }
    }
    let tail = if group.is_empty() {
        // Global aggregate: SQL yields one NULL row over empty input; the
        // engine yields the empty set. HAVING collapses them — the
        // documented translation rule, not a comparison patch.
        " HAVING COUNT(*) > 0".to_owned()
    } else {
        format!(" GROUP BY {}", group.join(", "))
    };
    Ok(format!("SELECT {} FROM ({union}){tail}", outer.join(", ")))
}

/// A group position's alias name(s), read off the first arm's rendering
/// (scalar `hN` vs the interval halves) — rules align positionally by
/// validation, so every arm shares the shape.
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

/// The Arg key and direction, if any find term is an Arg-restriction
/// (validation guarantees all Arg terms share one key and direction,
/// that no fold aggregate mixes in, and that Arg heads are single-rule).
fn arg_restriction(finds: &[FindTerm]) -> Option<(VarId, bool)> {
    finds.iter().find_map(|find| match find {
        FindTerm::Aggregate {
            op: AggOp::ArgMax { key },
            ..
        } => Some((*key, true)),
        FindTerm::Aggregate {
            op: AggOp::ArgMin { key },
            ..
        } => Some((*key, false)),
        _ => None,
    })
}

/// The distinct-subquery column list — every bound variable, interval
/// variables as their two halves (`vN_start`, `vN_end`).
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

/// A variable's column name(s) inside the distinct subquery, prefixed
/// (`""` inside it, `"d."` from the join-back outer).
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

/// The normative single-rule fold template: the aggregate applied over
/// the `SELECT DISTINCT <all bound variables>` subquery, grouped by the
/// non-aggregated finds — never a bare GROUP BY over the joined bag
/// (which folds witness multiplicity).
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
            // The measure as a group-key expression: end − start over the
            // subquery's halves.
            FindTerm::Duration(var) => {
                let expr = format!("(v{0}_end - v{0}_start)", var.0);
                group.push(expr.clone());
                outer.push(expr);
            }
            FindTerm::AggregateDuration { op, over } => outer.push({
                let agg = match op {
                    AggOp::Sum => "SUM",
                    AggOp::Min => "MIN",
                    AggOp::Max => "MAX",
                    _ => return Err("measure folds are Sum/Min/Max".to_owned()),
                };
                format!("{agg}(v{0}_end - v{0}_start)", over.0)
            }),
            FindTerm::Aggregate { op, over } => outer.push(match op {
                AggOp::Sum | AggOp::Min | AggOp::Max => {
                    let var = over.ok_or("fold aggregate without a variable")?;
                    let agg = match op {
                        AggOp::Sum => "SUM",
                        AggOp::Min => "MIN",
                        _ => "MAX",
                    };
                    format!("{agg}(v{})", var.0)
                }
                AggOp::Count => "COUNT(*)".to_owned(),
                AggOp::CountDistinct => {
                    let var = over.ok_or("CountDistinct without a variable")?;
                    match b.columns.get(&var) {
                        // COUNT(DISTINCT ...) takes one expression: an
                        // interval's halves concatenate through an
                        // injective decimal rendering.
                        Some(VarCols::Interval { .. }) => {
                            format!("COUNT(DISTINCT v{0}_start || ',' || v{0}_end)", var.0)
                        }
                        Some(VarCols::Scalar(_)) => format!("COUNT(DISTINCT v{})", var.0),
                        None => return Err(format!("find variable {} unbound", var.0)),
                    }
                }
                AggOp::ArgMax { .. } | AggOp::ArgMin { .. } => {
                    unreachable!("Arg terms take the join-back template")
                }
                // The expressibility gate (`super::sqlite_expressible`)
                // routes Pack heads to the naive lane before translation.
                AggOp::Pack => return Err("Pack is naive-only (no SQL coalesce)".to_owned()),
            }),
        }
    }
    let tail = if group.is_empty() {
        // Global aggregate: SQL yields one NULL row over empty input; the
        // engine yields the empty set. HAVING collapses them — the
        // documented translation rule, not a comparison patch.
        " HAVING COUNT(*) > 0".to_owned()
    } else {
        format!(" GROUP BY {}", group.join(", "))
    };
    Ok(format!("SELECT {} FROM ({inner}){tail}", outer.join(", ")))
}

/// The Arg-restriction join-back template
/// (`docs/architecture/60-validation.md`, normative): the distinct
/// binding set `d` joined against its per-group key extreme, with
/// `SELECT DISTINCT` on the outer — ties survive set-honestly on both
/// sides by construction. The global variant omits the group columns.
fn arg_sql(
    finds: &[FindTerm],
    b: &Builder,
    from: &str,
    where_clause: &str,
    key: VarId,
    is_max: bool,
) -> Result<String, String> {
    let inner = format!(
        "SELECT DISTINCT {} FROM {from}{where_clause}",
        inner_columns(b).join(", ")
    );
    // Per group position: (the m-subquery's select entry with alias, the
    // GROUP BY expression, the join equality). A plain column aliases to
    // itself; a measure aliases its end − start expression so the
    // join-back can name it.
    let mut group: Vec<(String, String, String)> = Vec::new();
    let mut outer: Vec<String> = Vec::new();
    for find in finds {
        match find {
            FindTerm::Var(var) => {
                for name in var_names(b, *var, "")? {
                    group.push((name.clone(), name.clone(), format!("d.{name} = m.{name}")));
                }
                outer.extend(var_names(b, *var, "d.")?);
            }
            // The measure as an aliased group-key expression on both
            // sides of the join-back.
            FindTerm::Duration(var) => {
                let expr = format!("(v{0}_end - v{0}_start)", var.0);
                group.push((
                    format!("{expr} AS dur{}", var.0),
                    expr,
                    format!("(d.v{0}_end - d.v{0}_start) = m.dur{0}", var.0),
                ));
                outer.push(format!("(d.v{0}_end - d.v{0}_start)", var.0));
            }
            FindTerm::Aggregate { over, .. } => {
                let carry = over.ok_or("Arg term without a carried variable")?;
                outer.extend(var_names(b, carry, "d.")?);
            }
            FindTerm::AggregateDuration { .. } => {
                return Err("Arg terms and measure folds never mix".to_owned());
            }
        }
    }
    let extreme = if is_max { "MAX" } else { "MIN" };
    // The key is orderable (U64/I64) by validation — always one column.
    let key_col = format!("v{}", key.0);
    let outer = outer.join(", ");
    if group.is_empty() {
        return Ok(format!(
            "WITH d AS ({inner}) SELECT DISTINCT {outer} FROM d \
             JOIN (SELECT {extreme}({key_col}) AS mk FROM d) m ON d.{key_col} = m.mk"
        ));
    }
    let group_eq = group
        .iter()
        .map(|(_, _, eq)| eq.clone())
        .collect::<Vec<_>>()
        .join(" AND ");
    let select = group
        .iter()
        .map(|(select, _, _)| select.clone())
        .collect::<Vec<_>>()
        .join(", ");
    let group_by = group
        .iter()
        .map(|(_, by, _)| by.clone())
        .collect::<Vec<_>>()
        .join(", ");
    Ok(format!(
        "WITH d AS ({inner}) SELECT DISTINCT {outer} FROM d \
         JOIN (SELECT {select}, {extreme}({key_col}) AS mk FROM d GROUP BY {group_by}) m \
         ON {group_eq} AND d.{key_col} = m.mk"
    ))
}
