//! Literal query semantics by nested loops
//! (`docs/architecture/20-query-ir.md`, normative). The model evaluates a
//! *validated* query — a program of rules — **from the definition: the
//! query denotes the set union of its rules' denotations.** Per rule:
//! params substituted first (params are query-global; variables are
//! rule-scoped), then the cross product of the positive atoms enumerated
//! fact by fact, bindings built from scalar occurrences, membership
//! evaluated as a per-binding test (a point value must lie in the fact's
//! interval), predicate trees evaluated **directly from the definition**
//! (`And` = every child, `Or` = any child, a leaf via the endpoint
//! formulas — the model never distributes to DNF; the engine's lowering
//! is proven *against* this evaluation), negated atoms as
//! plain anti-joins, full bindings deduplicated into a `BTreeSet`, and
//! finds projected or folded per the aggregation rules (Sum in i128,
//! `CountDistinct` via `BTreeSet`, Arg terms as literal
//! restrict-then-project with ties surviving, empty-input global
//! aggregates yielding the empty set).

use std::collections::{BTreeMap, BTreeSet};

use bumbledb::schema::ValueType;
use bumbledb::{
    AggOp, Atom, Basic, CmpOp, Comparison, FindTerm, MaskTerm, PredicateTree, Query, Rule, Term,
    Value, VarId,
};

use super::tuple::{cmp_value, contains_point, endpoints, point};
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

/// A predicate tree after parameter substitution — the input grammar's
/// shape, kept: the model evaluates it recursively, exactly as written.
enum SubstitutedTree {
    Leaf(CmpOp, Substituted, Substituted),
    And(Vec<SubstitutedTree>),
    Or(Vec<SubstitutedTree>),
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
    /// The rule's predicate trees, conjoined — evaluated directly.
    predicates: Vec<SubstitutedTree>,
    /// Per variable: bound on some non-interval field of a positive atom,
    /// hence a scalar (an occurrence on an interval field is then point
    /// membership; without a scalar anchor the variable is interval-typed
    /// and interval occurrences are value equality).
    scalar_anchored: Vec<bool>,
    var_count: usize,
}

impl NaiveDb {
    /// Evaluates a validated query with positional parameters, from the
    /// definition: the **set union of the rules' denotations**. Per rule,
    /// the set of distinct full bindings is projected and folded per its
    /// find list; a one-rule program is exactly the conjunctive query.
    ///
    /// A multi-rule aggregate head folds over the union of the rules'
    /// binding sets projected to the head (the rules-IR definition; PRD
    /// ALG-07 owns the executor's dedup semantics). The single-rule fold
    /// domain stays the rule's distinct **full** binding set — the
    /// normative aggregation rule, unchanged.
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
        if let [rule] = query.rules.as_slice() {
            let bindings = self.rule_bindings(rule, params);
            return project(&rule.finds, &bindings);
        }
        let aggregated = query
            .head
            .iter()
            .any(|term| matches!(term, bumbledb::HeadTerm::Aggregate(_)));
        if aggregated {
            return self.union_fold(query, params);
        }
        // Projection head: the union of the per-rule projected sets —
        // one union, set semantics.
        let mut rows = BTreeSet::new();
        for rule in &query.rules {
            let bindings = self.rule_bindings(rule, params);
            rows.extend(project(&rule.finds, &bindings)?);
        }
        Ok(rows)
    }

    /// One rule's distinct full binding set — the conjunctive semantics
    /// over the rule's own variable scope.
    fn rule_bindings(&self, rule: &Rule, params: &[ParamValue]) -> BTreeSet<Tuple> {
        let var_count = count_vars(rule);
        let mut scalar_anchored = vec![false; var_count];
        for atom in &rule.atoms {
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
            atoms: rule
                .atoms
                .iter()
                .map(|atom| self.flatten(atom, params))
                .collect(),
            negated: rule
                .negated
                .iter()
                .map(|atom| self.flatten(atom, params))
                .collect(),
            predicates: rule
                .predicates
                .iter()
                .map(|tree| substitute_tree(tree, params))
                .collect(),
            scalar_anchored,
            var_count,
        };
        let mut bindings = BTreeSet::new();
        let mut assignment = vec![None; var_count];
        let mut pending = Vec::new();
        enumerate(&env, 0, &mut assignment, &mut pending, &mut bindings);
        bindings
    }

    /// The multi-rule aggregate fold: each rule's binding set projected
    /// to the head (per position: the variable's value, or the
    /// aggregate's fold-input value — the nullary `Count` contributes a
    /// constant filler), unioned as a set, then grouped and folded per
    /// position. Arg terms are single-rule-only until PRD ALG-07 defines
    /// their cross-rule restriction (their key is a rule variable the
    /// head projection does not carry).
    fn union_fold(
        &self,
        query: &Query,
        params: &[ParamValue],
    ) -> Result<BTreeSet<Tuple>, QueryError> {
        let head = &query.rules[0].finds;
        assert!(
            !head.iter().any(|term| matches!(
                term,
                FindTerm::Aggregate {
                    op: AggOp::ArgMax { .. } | AggOp::ArgMin { .. },
                    ..
                }
            )),
            "multi-rule Arg restriction is undefined until PRD ALG-07"
        );
        let mut domain: BTreeSet<Tuple> = BTreeSet::new();
        for rule in &query.rules {
            for binding in &self.rule_bindings(rule, params) {
                domain.insert(Tuple(
                    rule.finds
                        .iter()
                        .map(|term| match term {
                            FindTerm::Var(var)
                            | FindTerm::Aggregate {
                                over: Some(var), ..
                            } => binding.0[usize::from(var.0)].clone(),
                            // Nullary Count: no fold input — a constant
                            // filler keeps positions stable.
                            FindTerm::Aggregate { over: None, .. } => Value::Bool(false),
                        })
                        .collect(),
                ));
            }
        }
        // Group by the variable positions; fold each aggregate position
        // over its group's projected tuples.
        let mut groups: BTreeMap<Tuple, Vec<&Tuple>> = BTreeMap::new();
        for row in &domain {
            let key = Tuple(
                head.iter()
                    .zip(&row.0)
                    .filter(|(term, _)| matches!(term, FindTerm::Var(_)))
                    .map(|(_, value)| value.clone())
                    .collect(),
            );
            groups.entry(key).or_default().push(row);
        }
        let mut rows = BTreeSet::new();
        for group in groups.values() {
            let row: Result<Vec<Value>, QueryError> = head
                .iter()
                .enumerate()
                .map(|(index, term)| match term {
                    FindTerm::Var(_) => Ok(group[0].0[index].clone()),
                    FindTerm::Aggregate { op, .. } => fold_position(*op, index, group),
                })
                .collect();
            rows.insert(Tuple(row?));
        }
        Ok(rows)
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

fn count_vars(rule: &Rule) -> usize {
    fn see(count: &mut usize, var: VarId) {
        *count = (*count).max(usize::from(var.0) + 1);
    }
    fn see_term(count: &mut usize, term: &Term) {
        if let Term::Var(var) = term {
            see(count, *var);
        }
    }
    fn see_tree(count: &mut usize, tree: &PredicateTree) {
        match tree {
            PredicateTree::Leaf(Comparison { lhs, rhs, .. }) => {
                see_term(count, lhs);
                see_term(count, rhs);
            }
            PredicateTree::And(children) | PredicateTree::Or(children) => {
                for child in children {
                    see_tree(count, child);
                }
            }
        }
    }
    let mut count = 0;
    for atom in rule.atoms.iter().chain(&rule.negated) {
        for (_, term) in &atom.bindings {
            see_term(&mut count, term);
        }
    }
    for tree in &rule.predicates {
        see_tree(&mut count, tree);
    }
    for find in &rule.finds {
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

/// Substitutes params through a predicate tree, keeping its shape. A
/// param mask substitutes like any param — the model sees only literal
/// masks past this point.
fn substitute_tree(tree: &PredicateTree, params: &[ParamValue]) -> SubstitutedTree {
    match tree {
        PredicateTree::Leaf(Comparison { op, lhs, rhs }) => {
            let op = match op {
                CmpOp::Allen {
                    mask: MaskTerm::Param(param),
                } => {
                    let ParamValue::Scalar(Value::AllenMask(mask)) = &params[usize::from(param.0)]
                    else {
                        panic!("validated: a mask param binds an Allen mask")
                    };
                    CmpOp::Allen {
                        mask: MaskTerm::Literal(*mask),
                    }
                }
                op => *op,
            };
            SubstitutedTree::Leaf(op, substitute(lhs, params), substitute(rhs, params))
        }
        PredicateTree::And(children) => SubstitutedTree::And(
            children
                .iter()
                .map(|child| substitute_tree(child, params))
                .collect(),
        ),
        PredicateTree::Or(children) => SubstitutedTree::Or(
            children
                .iter()
                .map(|child| substitute_tree(child, params))
                .collect(),
        ),
    }
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
    for tree in &env.predicates {
        if !tree_holds(tree, assignment) {
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

/// One predicate tree under a complete assignment, from the definition:
/// a leaf is its comparison, `And` holds iff every child holds (the
/// empty conjunction is true), `Or` iff any child holds (the empty
/// disjunction is false). No DNF, no distribution — the tree is the
/// semantics.
fn tree_holds(tree: &SubstitutedTree, assignment: &[Option<Value>]) -> bool {
    match tree {
        SubstitutedTree::Leaf(op, lhs, rhs) => predicate_holds(*op, lhs, rhs, assignment),
        SubstitutedTree::And(children) => {
            children.iter().all(|child| tree_holds(child, assignment))
        }
        SubstitutedTree::Or(children) => children.iter().any(|child| tree_holds(child, assignment)),
    }
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
        CmpOp::Allen { mask } => {
            let MaskTerm::Literal(mask) = mask else {
                panic!("param masks substitute before evaluation")
            };
            let (a, b) = (endpoints(&left), endpoints(&right));
            Basic::ALL
                .iter()
                .any(|basic| mask.contains(*basic) && basic_holds(*basic, a, b))
        }
        CmpOp::Contains => {
            let t = point(&right).expect("validated: Contains' right side is a point");
            contains_point(endpoints(&left), t)
        }
    }
}

/// One Allen basic's point-set definition over half-open intervals,
/// written directly as its endpoint characterization — the model's own
/// arithmetic, deliberately **independent** of the engine's classifier
/// (the differential oracle would otherwise test a function against
/// itself).
fn basic_holds(basic: Basic, a: (i128, i128), b: (i128, i128)) -> bool {
    let ((a_s, a_e), (b_s, b_e)) = (a, b);
    match basic {
        Basic::Before => a_e < b_s,
        Basic::Meets => a_e == b_s,
        Basic::Overlaps => a_s < b_s && b_s < a_e && a_e < b_e,
        Basic::Starts => a_s == b_s && a_e < b_e,
        Basic::During => b_s < a_s && a_e < b_e,
        Basic::Finishes => b_s < a_s && a_e == b_e,
        Basic::Equals => a_s == b_s && a_e == b_e,
        Basic::FinishedBy => a_s < b_s && a_e == b_e,
        Basic::Contains => a_s < b_s && b_e < a_e,
        Basic::StartedBy => a_s == b_s && b_e < a_e,
        Basic::OverlappedBy => b_s < a_s && a_s < b_e && b_e < a_e,
        Basic::MetBy => b_e == a_s,
        Basic::After => b_e < a_s,
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

/// One fold aggregate over a group of head-projected tuples (the
/// multi-rule union fold): the position's values are the fold inputs.
fn fold_position(op: AggOp, index: usize, group: &[&Tuple]) -> Result<Value, QueryError> {
    let values = || group.iter().map(move |row| &row.0[index]);
    match op {
        AggOp::Count => Ok(Value::U64(
            u64::try_from(group.len()).expect("group sizes fit u64"),
        )),
        AggOp::CountDistinct => {
            let distinct: BTreeSet<Tuple> =
                values().map(|value| Tuple(vec![value.clone()])).collect();
            Ok(Value::U64(
                u64::try_from(distinct.len()).expect("group sizes fit u64"),
            ))
        }
        AggOp::Sum => {
            let total: i128 = values()
                .map(|value| point(value).expect("validated: Sum takes integers"))
                .sum();
            match values().next().expect("groups are nonempty") {
                Value::U64(_) => u64::try_from(total)
                    .map(Value::U64)
                    .map_err(|_| QueryError::Overflow { find: index }),
                Value::I64(_) => i64::try_from(total)
                    .map(Value::I64)
                    .map_err(|_| QueryError::Overflow { find: index }),
                other => panic!("validated: Sum takes integers, got {other:?}"),
            }
        }
        AggOp::Min | AggOp::Max => {
            let picked = values()
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
            unreachable!("multi-rule Arg terms are rejected by the caller")
        }
    }
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
