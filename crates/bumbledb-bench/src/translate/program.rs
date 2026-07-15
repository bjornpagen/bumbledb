//! The recursive lane: `Program` → `WITH RECURSIVE`
//! (`docs/architecture/60-validation.md` § the translation rules; the
//! shipping law's `SQLite` row). One CTE per predicate (`p{id}`,
//! positional columns `c{i}`), emitted reads-first; a **linear
//! self-recursive** predicate — every recursive rule reading its own
//! predicate through exactly one atom — is a recursive CTE under
//! `UNION`, which is ∪ under the `DISTINCT` discipline and exactly the
//! set-semantics fixpoint (`SQLite`'s recursive-CTE queue enqueues new
//! rows only). Base rules render before recursive rules — the
//! initial-select law — and the final statement reads the output
//! predicate's CTE whole.
//!
//! **The gate is the enumerated set** ([`sqlite_program_expressible`]):
//! non-linear rules (two recursive atoms — `SQLite` admits exactly one
//! reference to the recursive table per arm), mutual recursion (no
//! mutually recursive CTE form), and folds anywhere in a program
//! (aggregation over recursive strata; the degenerate no-`Idb`
//! aggregate program is the plain query lane's) route to the naive+Lean
//! side — counted, reported, never silent: the ψ-subset
//! division-of-labor precedent, verbatim. Negation OF a lower stratum
//! translates: the `NOT EXISTS` subquery references a *finished* CTE,
//! never the recursive table (stratification already refuses the
//! in-cycle shape).
//!
//! Two documented translator limits beyond the gate (errors naming the
//! construct, generator-unreachable): an interval-typed predicate
//! column (the recursive lane is scalar-shaped; carrying intervals
//! through a closure is the chain-window fence's neighborhood,
//! `docs/architecture/20-query-ir.md` § engine recursion, the
//! chain-window fence), and a recursive predicate
//! with no base rule (its fixpoint is empty; `SQLite` refuses the
//! CTE shape and the generator never emits one).

use bumbledb::ir::FindTerm;
use bumbledb::{AtomSource, ParamId, Program, Rule, Schema, Term, Value};

use super::query::{SharedParams, arm_body, rule_core};
use super::{Inexpressible, Translated, VarCols};

/// The recursive lane's expressibility gate — the enumerated set, so
/// nothing is silently skipped (module doc). Everything that passes
/// translates through [`translate_program`].
///
/// # Errors
///
/// The [`Inexpressible`] class: [`Inexpressible::MutualRecursion`],
/// [`Inexpressible::NonLinearRecursion`],
/// [`Inexpressible::RecursiveFold`], or
/// [`Inexpressible::SelfNegation`] (the gate mirrors the engine's
/// stratification fence, so a raw un-validated `Program` routes as a
/// typed verdict instead of emitting a self-referencing `NOT EXISTS`).
pub fn sqlite_program_expressible(program: &Program) -> Result<(), Inexpressible> {
    for def in &program.predicates {
        for rule in &def.rules {
            let fold = rule.finds.iter().any(|find| {
                matches!(
                    find,
                    FindTerm::Aggregate { .. } | FindTerm::AggregateMeasure { .. }
                )
            });
            if fold {
                return Err(Inexpressible::RecursiveFold);
            }
        }
    }
    let reaches = reachability(program);
    for (index, row) in reaches.iter().enumerate() {
        for (target, reached) in row.iter().enumerate() {
            if index != target && *reached && reaches[target][index] {
                return Err(Inexpressible::MutualRecursion);
            }
        }
    }
    for (index, def) in program.predicates.iter().enumerate() {
        for rule in &def.rules {
            let this = AtomSource::Idb(bumbledb::PredId(pred_id(index)));
            if rule.negated.iter().any(|atom| atom.source == this) {
                return Err(Inexpressible::SelfNegation);
            }
            let self_atoms = rule.atoms.iter().filter(|atom| atom.source == this).count();
            if self_atoms > 1 {
                return Err(Inexpressible::NonLinearRecursion);
            }
        }
    }
    Ok(())
}

fn pred_id(index: usize) -> u16 {
    u16::try_from(index).expect("predicate count capped at MAX_PREDICATES")
}

/// The predicate-reads reachability closure (positive and negated
/// edges alike — mutuality through negation is still mutuality), by
/// brute-force sweeps: the matrix is bounded by `MAX_PREDICATES`².
fn reachability(program: &Program) -> Vec<Vec<bool>> {
    let count = program.predicates.len();
    let mut reaches = vec![vec![false; count]; count];
    for (index, def) in program.predicates.iter().enumerate() {
        for rule in &def.rules {
            for atom in rule.atoms.iter().chain(&rule.negated) {
                if let AtomSource::Idb(pred) = atom.source {
                    reaches[index][usize::from(pred.0)] = true;
                }
            }
        }
    }
    loop {
        let mut changed = false;
        for from in 0..count {
            for via in 0..count {
                if !reaches[from][via] {
                    continue;
                }
                let via_row = reaches[via].clone();
                for (to, reachable) in via_row.iter().enumerate() {
                    if *reachable && !reaches[from][to] {
                        reaches[from][to] = true;
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            return reaches;
        }
    }
}

/// Whether a rule reads its own predicate — the recursive-arm test.
fn reads_self(rule: &Rule, index: usize) -> bool {
    rule.atoms
        .iter()
        .any(|atom| atom.source == AtomSource::Idb(bumbledb::PredId(pred_id(index))))
}

/// CTE emission order: reads before readers (Kahn over the predicate
/// dependency graph, self-loops dropped — the gate refused mutual
/// recursion, so the condensation is the predicates themselves).
fn emission_order(program: &Program) -> Result<Vec<usize>, String> {
    let count = program.predicates.len();
    let mut reads: Vec<Vec<usize>> = vec![Vec::new(); count];
    for (index, def) in program.predicates.iter().enumerate() {
        for rule in &def.rules {
            for atom in rule.atoms.iter().chain(&rule.negated) {
                if let AtomSource::Idb(pred) = atom.source {
                    let target = usize::from(pred.0);
                    if target != index && !reads[index].contains(&target) {
                        reads[index].push(target);
                    }
                }
            }
        }
    }
    let mut emitted = vec![false; count];
    let mut order = Vec::with_capacity(count);
    while order.len() < count {
        let next = (0..count)
            .find(|index| !emitted[*index] && reads[*index].iter().all(|target| emitted[*target]));
        let Some(index) = next else {
            return Err("cyclic predicate reads past the mutual-recursion gate".to_owned());
        };
        emitted[index] = true;
        order.push(index);
    }
    Ok(order)
}

/// Translates one gate-passing program over the given schema —
/// `WITH RECURSIVE p0(...) AS (...), ... SELECT DISTINCT ... FROM
/// p{output}` (module doc; the hand-written goldens are
/// [`super::goldens`]' recursive rows).
///
/// # Errors
///
/// A message naming the untranslatable construct — the two documented
/// limits (module doc), plus everything [`super::translate`] names.
pub fn translate_program(
    program: &Program,
    schema: &Schema,
    sets: &[(ParamId, Vec<Value>)],
) -> Result<Translated, String> {
    refuse_interval_columns(program, schema)?;
    let mut params = SharedParams::default();
    let mut ctes: Vec<String> = Vec::new();
    for index in emission_order(program)? {
        let def = &program.predicates[index];
        let (base, recursive): (Vec<&Rule>, Vec<&Rule>) =
            def.rules.iter().partition(|rule| !reads_self(rule, index));
        if base.is_empty() && !recursive.is_empty() {
            return Err(format!(
                "recursive predicate p{index} has no base rule (its fixpoint is empty)"
            ));
        }
        let mut arms: Vec<String> = Vec::new();
        for rule in base.iter().chain(&recursive) {
            let b = rule_core(rule, schema, sets, &mut params)?;
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
                        return Err("program folds are naive-only (the gate routes them)".into());
                    }
                };
                cols.push(format!("{expr} AS c{position}"));
            }
            // Plain SELECT per arm: `UNION` is the set — and SQLite's
            // recursive-select refuses DISTINCT; the final statement's
            // DISTINCT covers the single-arm bag.
            arms.push(format!("SELECT {}{}", cols.join(", "), arm_body(&b)));
        }
        let arity = program.predicates[index].head.len();
        let columns: Vec<String> = (0..arity).map(|column| format!("c{column}")).collect();
        ctes.push(format!(
            "p{index}({}) AS ({})",
            columns.join(", "),
            arms.join(" UNION ")
        ));
    }
    let output = usize::from(program.output.0);
    let out_cols: Vec<String> = (0..program.predicates[output].head.len())
        .map(|column| format!("c{column}"))
        .collect();
    Ok(Translated {
        sql: format!(
            "WITH RECURSIVE {} SELECT DISTINCT {} FROM p{output}",
            ctes.join(", "),
            out_cols.join(", ")
        ),
        params: params.params,
    })
}

/// The scalar-shape check: no predicate head position may be
/// interval-typed (module doc, the first documented limit). The flags
/// re-derive from the rules to a monotone fixpoint, exactly as the
/// naive model derives its predicate signatures: a projected variable
/// is interval-typed when no positive occurrence anchors it on a
/// scalar column.
fn refuse_interval_columns(program: &Program, schema: &Schema) -> Result<(), String> {
    let mut interval: Vec<Vec<bool>> = program
        .predicates
        .iter()
        .map(|def| vec![false; def.head.len()])
        .collect();
    loop {
        let mut changed = false;
        for (index, def) in program.predicates.iter().enumerate() {
            let Some(rule) = def.rules.first() else {
                continue;
            };
            let flags: Vec<bool> = rule
                .finds
                .iter()
                .map(|find| match find {
                    FindTerm::Var(var) => !rule.atoms.iter().any(|atom| {
                        atom.bindings.iter().any(|(field, term)| {
                            matches!(term, Term::Var(v) if v == var)
                                && !match atom.source {
                                    AtomSource::Edb(relation) => matches!(
                                        schema.relation(relation).fields()[usize::from(field.0)]
                                            .value_type,
                                        bumbledb::schema::ValueType::Interval { .. }
                                    ),
                                    AtomSource::Idb(pred) => {
                                        interval[usize::from(pred.0)][usize::from(field.0)]
                                    }
                                }
                        })
                    }),
                    FindTerm::Measure(_)
                    | FindTerm::Aggregate { .. }
                    | FindTerm::AggregateMeasure { .. } => false,
                })
                .collect();
            if flags != interval[index] {
                interval[index] = flags;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    if interval.iter().flatten().any(|flag| *flag) {
        return Err(
            "interval-typed predicate column (the recursive lane is scalar-shaped)".to_owned(),
        );
    }
    Ok(())
}
