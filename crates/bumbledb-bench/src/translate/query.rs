use std::collections::BTreeMap;

use bumbledb::ir::FindTerm;
use bumbledb::{AggOp, ParamId, Query, Schema, Value, VarId};

use super::{types, Builder, Translated, VarCols};

/// Translates one validated-shape query over the given schema. `sets`
/// carries the bound element list of every set param: set params render
/// as literal `IN` lists (empty ⇒ `1 = 0`), so set-bound queries are
/// **re-rendered per execution** and prepared-statement parity is not
/// claimed for them (`docs/architecture/60-validation.md`).
///
/// # Errors
///
/// A message naming the untranslatable construct. Total over the query
/// grammar with one documented exception: a query whose every atom is a
/// gate (no bound columns exist to select from) — the query generator
/// never produces one. Dependency judgments are the enumerated
/// inexpressible set ([`super::sqlite_expressible`]); no *query*
/// construct is inexpressible.
pub fn translate(
    query: &Query,
    schema: &Schema,
    sets: &[(ParamId, Vec<Value>)],
) -> Result<Translated, String> {
    let mut b = Builder {
        schema,
        types: types::infer(query, schema),
        sets,
        from: Vec::new(),
        predicates: Vec::new(),
        deferred: Vec::new(),
        columns: BTreeMap::new(),
        param_index: BTreeMap::new(),
        params: Vec::new(),
    };
    for atom in &query.rules[0].atoms {
        b.atom(atom)?;
    }
    b.flush_deferred()?;
    for comparison in query.rules[0].predicates.iter().map(super::leaf) {
        b.comparison(comparison)?;
    }
    // Negation last: the NOT EXISTS subqueries append to the core's WHERE.
    for (index, atom) in query.rules[0].negated.iter().enumerate() {
        b.negated_atom(index, atom)?;
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

    let sql = if let Some((key, is_max)) = arg_restriction(query) {
        arg_sql(query, &b, &from, &where_clause, key, is_max)?
    } else if query.rules[0]
        .finds
        .iter()
        .any(|f| matches!(f, FindTerm::Aggregate { .. }))
    {
        fold_sql(query, &b, &from, &where_clause)?
    } else {
        let mut cols: Vec<String> = Vec::new();
        for find in &query.rules[0].finds {
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
                        return Err(format!("Duration over scalar variable {}", var.0))
                    }
                    None => return Err(format!("find variable {} unbound", var.0)),
                },
                FindTerm::Aggregate { .. } | FindTerm::AggregateDuration { .. } => {
                    unreachable!("no aggregates here")
                }
            }
        }
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

/// The Arg key and direction, if any find term is an Arg-restriction
/// (validation guarantees all Arg terms share one key and direction, and
/// that no fold aggregate mixes in).
fn arg_restriction(query: &Query) -> Option<(VarId, bool)> {
    query.rules[0].finds.iter().find_map(|find| match find {
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

/// The normative fold template: the aggregate applied over the
/// `SELECT DISTINCT <all bound variables>` subquery, grouped by the
/// non-aggregated finds — never a bare GROUP BY over the joined bag
/// (which folds witness multiplicity).
fn fold_sql(query: &Query, b: &Builder, from: &str, where_clause: &str) -> Result<String, String> {
    let inner = format!(
        "SELECT DISTINCT {} FROM {from}{where_clause}",
        inner_columns(b).join(", ")
    );
    let mut group: Vec<String> = Vec::new();
    let mut outer: Vec<String> = Vec::new();
    for find in &query.rules[0].finds {
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
    query: &Query,
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
    for find in &query.rules[0].finds {
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
                return Err("Arg terms and measure folds never mix".to_owned())
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
