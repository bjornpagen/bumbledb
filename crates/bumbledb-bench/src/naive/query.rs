//! Literal query semantics by nested loops
//! (`docs/architecture/20-query-ir.md`, normative). The model evaluates a
//! *validated* query: params substituted first, then the cross product of
//! the positive atoms enumerated fact by fact, bindings built from scalar
//! occurrences, membership evaluated as a per-binding test (a point value
//! must lie in the fact's interval), predicates via the endpoint formulas,
//! negated atoms as plain anti-joins, full bindings deduplicated into a
//! `BTreeSet`, and finds projected or folded per the aggregation rules
//! (Sum in i128, `CountDistinct` via `BTreeSet`, Arg terms as literal
//! restrict-then-project with ties surviving, empty-input global
//! aggregates yielding the empty set).

use std::collections::{BTreeMap, BTreeSet};

use bumbledb::schema::ValueType;
use bumbledb::{AggOp, Atom, CmpOp, Comparison, FindTerm, Query, Term, Value, VarId};

use super::tuple::{cmp_value, contains_point, endpoints, overlaps, point};
use super::{NaiveDb, Tuple};

/// One positional parameter, scalar or set — the model's mirror of the
/// engine's `ParamArg`, owned so op streams (and the family rotations)
/// can store it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamValue {
    Scalar(Value),
    Set(Vec<Value>),
}

/// The one runtime query error the semantics define: an aggregate's final
/// value out of its result type's range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryError {
    Overflow { find: usize },
}

/// A term after parameter substitution.
#[derive(Debug, Clone)]
enum Substituted {
    Var(usize),
    Lit(Value),
    Set(Vec<Value>),
}

/// One atom over substituted terms, each binding pre-tagged with whether
/// its field is interval-typed (the membership rule's trigger).
struct FlatAtom {
    relation: usize,
    bindings: Vec<(usize, bool, Substituted)>,
}

/// Everything enumeration reads.
struct Env<'a> {
    relations: &'a [BTreeSet<Tuple>],
    atoms: Vec<FlatAtom>,
    negated: Vec<FlatAtom>,
    predicates: Vec<(CmpOp, Substituted, Substituted)>,
    /// Per variable: bound on some non-interval field of a positive atom,
    /// hence a scalar (an occurrence on an interval field is then point
    /// membership; without a scalar anchor the variable is interval-typed
    /// and interval occurrences are value equality).
    scalar_anchored: Vec<bool>,
    var_count: usize,
}

impl NaiveDb {
    /// Evaluates a validated query with positional parameters: the set of
    /// distinct full bindings, projected and folded per the find list.
    ///
    /// # Errors
    ///
    /// [`QueryError::Overflow`] when an aggregate's final value exceeds
    /// its result type.
    ///
    /// # Panics
    ///
    /// On malformed input — the model evaluates queries the engine's
    /// validation boundary has accepted, with matching parameters.
    pub fn query(
        &self,
        query: &Query,
        params: &[ParamValue],
    ) -> Result<BTreeSet<Tuple>, QueryError> {
        let var_count = count_vars(query);
        let mut scalar_anchored = vec![false; var_count];
        for atom in &query.atoms {
            for (field, term) in &atom.bindings {
                if let Term::Var(var) = term {
                    if !self.atom_field_is_interval(atom, *field) {
                        scalar_anchored[usize::from(var.0)] = true;
                    }
                }
            }
        }
        let env = Env {
            relations: &self.relations,
            atoms: query
                .atoms
                .iter()
                .map(|atom| self.flatten(atom, params))
                .collect(),
            negated: query
                .negated
                .iter()
                .map(|atom| self.flatten(atom, params))
                .collect(),
            predicates: query
                .predicates
                .iter()
                .map(|Comparison { op, lhs, rhs }| {
                    (*op, substitute(lhs, params), substitute(rhs, params))
                })
                .collect(),
            scalar_anchored,
            var_count,
        };
        let mut bindings = BTreeSet::new();
        let mut assignment = vec![None; var_count];
        let mut pending = Vec::new();
        enumerate(&env, 0, &mut assignment, &mut pending, &mut bindings);
        project(&query.finds, &bindings)
    }

    fn flatten(&self, atom: &Atom, params: &[ParamValue]) -> FlatAtom {
        FlatAtom {
            relation: atom.relation.0 as usize,
            bindings: atom
                .bindings
                .iter()
                .map(|(field, term)| {
                    (
                        usize::from(field.0),
                        self.atom_field_is_interval(atom, *field),
                        substitute(term, params),
                    )
                })
                .collect(),
        }
    }

    fn atom_field_is_interval(&self, atom: &Atom, field: bumbledb::FieldId) -> bool {
        matches!(
            self.schema.relations[atom.relation.0 as usize].fields[usize::from(field.0)].value_type,
            ValueType::Interval { .. }
        )
    }
}

fn count_vars(query: &Query) -> usize {
    fn see(count: &mut usize, var: VarId) {
        *count = (*count).max(usize::from(var.0) + 1);
    }
    fn see_term(count: &mut usize, term: &Term) {
        if let Term::Var(var) = term {
            see(count, *var);
        }
    }
    let mut count = 0;
    for atom in query.atoms.iter().chain(&query.negated) {
        for (_, term) in &atom.bindings {
            see_term(&mut count, term);
        }
    }
    for Comparison { lhs, rhs, .. } in &query.predicates {
        see_term(&mut count, lhs);
        see_term(&mut count, rhs);
    }
    for find in &query.finds {
        match find {
            FindTerm::Var(var) => see(&mut count, *var),
            FindTerm::Aggregate { op, over } => {
                if let Some(var) = over {
                    see(&mut count, *var);
                }
                if let AggOp::ArgMax { key } | AggOp::ArgMin { key } = op {
                    see(&mut count, *key);
                }
            }
        }
    }
    count
}

fn substitute(term: &Term, params: &[ParamValue]) -> Substituted {
    match term {
        Term::Var(var) => Substituted::Var(usize::from(var.0)),
        Term::Literal(value) => Substituted::Lit(value.clone()),
        Term::Param(id) => match &params[usize::from(id.0)] {
            ParamValue::Scalar(value) => Substituted::Lit(value.clone()),
            ParamValue::Set(_) => panic!("param {} bound as a set, used as a scalar", id.0),
        },
        Term::ParamSet(id) => match &params[usize::from(id.0)] {
            ParamValue::Set(values) => Substituted::Set(values.clone()),
            ParamValue::Scalar(_) => panic!("param {} bound as a scalar, used as a set", id.0),
        },
    }
}

/// Nested loops over the positive atoms: place a fact for the atom at
/// `index`, extend the assignment, recurse; at the leaf check the deferred
/// membership tests, the predicates, and the negated atoms, then record
/// the full binding.
fn enumerate(
    env: &Env<'_>,
    index: usize,
    assignment: &mut Vec<Option<Value>>,
    pending: &mut Vec<(usize, Value)>,
    out: &mut BTreeSet<Tuple>,
) {
    if index == env.atoms.len() {
        if leaf_admits(env, assignment, pending) {
            out.insert(Tuple(
                (0..env.var_count)
                    .map(|var| match &assignment[var] {
                        Some(value) => value.clone(),
                        // An id below the maximum that no term uses: a
                        // constant filler keeps positions stable and is
                        // never projected (an unused id occurs nowhere).
                        None => Value::Bool(false),
                    })
                    .collect(),
            ));
        }
        return;
    }
    let atom = &env.atoms[index];
    for fact in &env.relations[atom.relation] {
        let pending_before = pending.len();
        let mut bound_here = Vec::new();
        let mut admitted = true;
        for (field, field_is_interval, term) in &atom.bindings {
            if !admit(
                env,
                &fact.0[*field],
                *field_is_interval,
                term,
                assignment,
                pending,
                &mut bound_here,
            ) {
                admitted = false;
                break;
            }
        }
        if admitted {
            enumerate(env, index + 1, assignment, pending, out);
        }
        for var in bound_here {
            assignment[var] = None;
        }
        pending.truncate(pending_before);
    }
}

/// One binding position against one fact value: literals and set elements
/// by the membership-or-equality rule; variables bind scalar occurrences,
/// equality-check repeat occurrences, and defer membership occurrences
/// until their scalar anchor binds them.
fn admit(
    env: &Env<'_>,
    fact_value: &Value,
    field_is_interval: bool,
    term: &Substituted,
    assignment: &mut [Option<Value>],
    pending: &mut Vec<(usize, Value)>,
    bound_here: &mut Vec<usize>,
) -> bool {
    match term {
        Substituted::Lit(value) => constrains(fact_value, field_is_interval, value),
        Substituted::Set(values) => values
            .iter()
            .any(|value| constrains(fact_value, field_is_interval, value)),
        Substituted::Var(var) => {
            if field_is_interval && env.scalar_anchored[*var] {
                if let Some(bound) = &assignment[*var] {
                    contains_point(
                        endpoints(fact_value),
                        point(bound).expect("a scalar-anchored variable holds a scalar"),
                    )
                } else {
                    pending.push((*var, fact_value.clone()));
                    true
                }
            } else if let Some(bound) = &assignment[*var] {
                bound == fact_value
            } else {
                assignment[*var] = Some(fact_value.clone());
                bound_here.push(*var);
                true
            }
        }
    }
}

/// The membership typing rule for a constant against a field value: an
/// element-typed constant on an interval field is point membership;
/// everything else is value equality.
fn constrains(fact_value: &Value, field_is_interval: bool, term_value: &Value) -> bool {
    if field_is_interval {
        if let Some(t) = point(term_value) {
            return contains_point(endpoints(fact_value), t);
        }
    }
    term_value == fact_value
}

fn leaf_admits(
    env: &Env<'_>,
    assignment: &mut [Option<Value>],
    pending: &[(usize, Value)],
) -> bool {
    for (var, interval) in pending {
        let bound = assignment[*var]
            .as_ref()
            .expect("validated: every point variable has a scalar anchor");
        if !contains_point(
            endpoints(interval),
            point(bound).expect("a scalar-anchored variable holds a scalar"),
        ) {
            return false;
        }
    }
    for (op, lhs, rhs) in &env.predicates {
        if !predicate_holds(*op, lhs, rhs, assignment) {
            return false;
        }
    }
    for atom in &env.negated {
        let matched = env.relations[atom.relation]
            .iter()
            .any(|fact| negated_matches(env, atom, fact, assignment));
        if matched {
            return false;
        }
    }
    true
}

/// Does a fact match a negated atom under a complete assignment? One
/// matching rule serves both polarities: every negated-atom variable is
/// positively bound (the safety rule), so [`admit`] can only take its
/// already-bound arms here — it binds nothing and defers nothing.
fn negated_matches(
    env: &Env<'_>,
    atom: &FlatAtom,
    fact: &Tuple,
    assignment: &mut [Option<Value>],
) -> bool {
    let mut pending = Vec::new();
    let mut bound_here = Vec::new();
    let matched = atom
        .bindings
        .iter()
        .all(|(field, field_is_interval, term)| {
            admit(
                env,
                &fact.0[*field],
                *field_is_interval,
                term,
                assignment,
                &mut pending,
                &mut bound_here,
            )
        });
    assert!(
        pending.is_empty() && bound_here.is_empty(),
        "validated: negated-atom variables are positively bound"
    );
    matched
}

fn predicate_holds(
    op: CmpOp,
    lhs: &Substituted,
    rhs: &Substituted,
    assignment: &[Option<Value>],
) -> bool {
    let resolve = |term: &Substituted| -> Option<Value> {
        match term {
            Substituted::Var(var) => Some(
                assignment[*var]
                    .clone()
                    .expect("validated: predicate variables are bound"),
            ),
            Substituted::Lit(value) => Some(value.clone()),
            Substituted::Set(_) => None,
        }
    };
    // A set is legal on one side of Eq only: "any element" — value in set.
    if let (CmpOp::Eq, Substituted::Set(values), other)
    | (CmpOp::Eq, other, Substituted::Set(values)) = (op, lhs, rhs)
    {
        let value = resolve(other).expect("validated: one side of a set Eq is scalar");
        return values.contains(&value);
    }
    let left = resolve(lhs).expect("validated: sets appear only under Eq");
    let right = resolve(rhs).expect("validated: sets appear only under Eq");
    match op {
        CmpOp::Eq => left == right,
        CmpOp::Ne => left != right,
        CmpOp::Lt | CmpOp::Le | CmpOp::Gt | CmpOp::Ge => {
            let a = point(&left).expect("validated: order operators take integers");
            let b = point(&right).expect("validated: order operators take integers");
            match op {
                CmpOp::Lt => a < b,
                CmpOp::Le => a <= b,
                CmpOp::Gt => a > b,
                CmpOp::Ge => a >= b,
                _ => unreachable!(),
            }
        }
        CmpOp::Overlaps => overlaps(endpoints(&left), endpoints(&right)),
        CmpOp::Contains => {
            let (start, end) = endpoints(&left);
            if let Some(t) = point(&right) {
                contains_point((start, end), t)
            } else {
                let (inner_start, inner_end) = endpoints(&right);
                start <= inner_start && inner_end <= end
            }
        }
    }
}

/// Projects and folds the distinct full bindings per the find list: group
/// key = the values of the plain-variable finds; every aggregate folds
/// over its group's binding set. No bindings means no groups — the empty
/// set, global aggregates included.
fn project(finds: &[FindTerm], bindings: &BTreeSet<Tuple>) -> Result<BTreeSet<Tuple>, QueryError> {
    let mut groups: BTreeMap<Tuple, Vec<&Tuple>> = BTreeMap::new();
    for binding in bindings {
        let key = Tuple(
            finds
                .iter()
                .filter_map(|find| match find {
                    FindTerm::Var(var) => Some(binding.0[usize::from(var.0)].clone()),
                    FindTerm::Aggregate { .. } => None,
                })
                .collect(),
        );
        groups.entry(key).or_default().push(binding);
    }
    let arg = finds.iter().find_map(|find| match find {
        FindTerm::Aggregate {
            op: AggOp::ArgMax { key },
            ..
        } => Some((usize::from(key.0), true)),
        FindTerm::Aggregate {
            op: AggOp::ArgMin { key },
            ..
        } => Some((usize::from(key.0), false)),
        _ => None,
    });
    let mut rows = BTreeSet::new();
    for group in groups.values() {
        if let Some((key_var, is_max)) = arg {
            // Arg-restriction: restrict the group to the bindings
            // attaining the key's extreme, then project every survivor —
            // a tie yields every attaining row.
            let extreme = group
                .iter()
                .map(|binding| &binding.0[key_var])
                .max_by(|a, b| {
                    let ordering = cmp_value(a, b);
                    if is_max {
                        ordering
                    } else {
                        ordering.reverse()
                    }
                })
                .expect("groups are nonempty by construction");
            for binding in group {
                if binding.0[key_var] != *extreme {
                    continue;
                }
                rows.insert(Tuple(
                    finds
                        .iter()
                        .map(|find| match find {
                            FindTerm::Var(var) => binding.0[usize::from(var.0)].clone(),
                            FindTerm::Aggregate { over, .. } => binding.0
                                [usize::from(over.expect("Arg terms carry a variable").0)]
                            .clone(),
                        })
                        .collect(),
                ));
            }
        } else {
            let row: Result<Vec<Value>, QueryError> = finds
                .iter()
                .enumerate()
                .map(|(index, find)| match find {
                    FindTerm::Var(var) => Ok(group[0].0[usize::from(var.0)].clone()),
                    FindTerm::Aggregate { op, over } => fold(*op, *over, group, index),
                })
                .collect();
            rows.insert(Tuple(row?));
        }
    }
    Ok(rows)
}

/// One fold aggregate over a group's binding set.
fn fold(
    op: AggOp,
    over: Option<VarId>,
    group: &[&Tuple],
    find: usize,
) -> Result<Value, QueryError> {
    let values = |var: VarId| group.iter().map(move |b| &b.0[usize::from(var.0)]);
    match op {
        AggOp::Count => Ok(Value::U64(
            u64::try_from(group.len()).expect("group sizes fit u64"),
        )),
        AggOp::CountDistinct => {
            let var = over.expect("validated: CountDistinct carries a variable");
            let distinct: BTreeSet<Tuple> = values(var)
                .map(|value| Tuple(vec![value.clone()]))
                .collect();
            Ok(Value::U64(
                u64::try_from(distinct.len()).expect("group sizes fit u64"),
            ))
        }
        AggOp::Sum => {
            let var = over.expect("validated: Sum carries a variable");
            let total: i128 = values(var)
                .map(|value| point(value).expect("validated: Sum takes integers"))
                .sum();
            match values(var).next().expect("groups are nonempty") {
                Value::U64(_) => u64::try_from(total)
                    .map(Value::U64)
                    .map_err(|_| QueryError::Overflow { find }),
                Value::I64(_) => i64::try_from(total)
                    .map(Value::I64)
                    .map_err(|_| QueryError::Overflow { find }),
                other => panic!("validated: Sum takes integers, got {other:?}"),
            }
        }
        AggOp::Min | AggOp::Max => {
            let var = over.expect("validated: Min/Max carry a variable");
            let picked = values(var)
                .max_by(|a, b| {
                    let ordering = cmp_value(a, b);
                    if matches!(op, AggOp::Max) {
                        ordering
                    } else {
                        ordering.reverse()
                    }
                })
                .expect("groups are nonempty");
            Ok(picked.clone())
        }
        AggOp::ArgMax { .. } | AggOp::ArgMin { .. } => {
            unreachable!("Arg terms take the restriction path")
        }
    }
}
