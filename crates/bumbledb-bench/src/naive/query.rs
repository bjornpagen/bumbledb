//! Literal query semantics by nested loops
//! (`docs/architecture/20-query-ir.md`, normative). The model evaluates a
//! *validated* query — a union of conjunctive rules — **from the definition: the
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
//! empty-input global aggregates yielding the empty set).

use std::collections::{BTreeMap, BTreeSet};

use bumbledb::{
    Atom, AtomSource, Basic, CmpOp, Comparison, ConditionTree, FindTerm, FoldOp, HeadTerm,
    InteriorId, Query, RecRule, RecStep, Rule, Term, Value, VarId,
};

use super::tuple::{cmp_value, endpoints, point, point_in};
use super::{NaiveDb, Tuple};

/// One positional parameter, scalar or set — the model's mirror of the
/// engine's `ParamArg`, owned so op streams (and the family rotations)
/// can store it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamValue {
    Scalar(Value),
    Set(Vec<Value>),
}

/// The runtime query errors the semantics define: an aggregate's final
/// value out of its result type's range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryError {
    Overflow { find: usize },
}

/// One rule's DNF width, **from the definition**: the number of
/// conjunctive rules its predicate trees would distribute to — a leaf
/// is one disjunct, `And` multiplies its children's widths (the empty
/// conjunction is true: one disjunct), `Or` sums them (the empty
/// disjunction is false: zero), and the rule's conjoined trees
/// multiply. Deliberately independent of the engine's structural count
/// (`ir::normalize`): the verify error-parity lane compares the two —
/// the cap-exceeder verdict must carry the same `produced` on both
/// sides, typed identity included.
#[must_use]
pub fn dnf_width(rule: &Rule) -> usize {
    fn width(tree: &ConditionTree) -> usize {
        match tree {
            ConditionTree::Leaf(_) => 1,
            ConditionTree::And(children) => children.iter().map(width).product(),
            ConditionTree::Or(children) => children.iter().map(width).sum(),
        }
    }
    rule.conditions.iter().map(width).product()
}

/// `Pack` from the definition (`docs/architecture/20-query-ir.md`
/// § aggregation): the union of the claims' point sets as **maximal
/// disjoint half-open segments** — sort the endpoint pairs, then merge
/// while `next.start <= frontier` (equality merges: half-open segments
/// sharing a boundary leave no hole — the adjacency law). The model's
/// own arithmetic over logical endpoint values, deliberately independent
/// of the engine's word sweep (the differential oracle would otherwise
/// test a function against itself). A ray's `end` is the element
/// domain's `MAX`, so it is simply the frontier no later claim exceeds —
/// the packed ray is a ray, no case needed. Identical claims merge like
/// any overlap.
fn pack_segments(claims: &[&Value]) -> Vec<Value> {
    let mut segments: Vec<(i128, i128)> = claims.iter().map(|value| endpoints(value)).collect();
    segments.sort_unstable();
    let mut merged: Vec<(i128, i128)> = Vec::new();
    for segment in segments {
        match merged.last_mut() {
            Some(last) if segment.0 <= last.1 => last.1 = last.1.max(segment.1),
            _ => merged.push(segment),
        }
    }
    let rebuild = |(start, end): (i128, i128)| match claims[0] {
        Value::IntervalU64(..) => Value::IntervalU64(
            bumbledb::Interval::<u64>::new(
                u64::try_from(start).expect("u64 endpoints round-trip"),
                u64::try_from(end).expect("u64 endpoints round-trip"),
            )
            .expect("packing preserves nonempty intervals"),
        ),
        Value::IntervalI64(..) => Value::IntervalI64(
            bumbledb::Interval::<i64>::new(
                i64::try_from(start).expect("i64 endpoints round-trip"),
                i64::try_from(end).expect("i64 endpoints round-trip"),
            )
            .expect("packing preserves nonempty intervals"),
        ),
        other => panic!("validated: Pack takes an interval, got {other:?}"),
    };
    merged.into_iter().map(rebuild).collect()
}

/// The head's one `Pack` position, if any (validation: at most one).
fn pack_position(finds: &[FindTerm]) -> Option<(usize, VarId)> {
    finds.iter().enumerate().find_map(|(index, find)| {
        if let FindTerm::Pack { over } = find {
            Some((index, *over))
        } else {
            None
        }
    })
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

/// Finished derived tables — interiors in declaration order, then the
/// rec's accumulating table — beside their column typing. A derived
/// table's facts ARE its answer tuples, read positionally: `FieldId(i)`
/// is head position `i`
/// (`lean/Bumbledb/Query/Denotation.lean: tupleFact` — the positional
/// addressing interiors and rec share). A plain query reads no
/// derived tables: the empty world.
pub(super) struct DerivedWorld<'a> {
    /// Per derived table, the accumulated answer-tuple set.
    sets: &'a [BTreeSet<Tuple>],
    /// Per derived table, per head position: interval-typed? — the
    /// membership typing rule read through sealed interior columns.
    interval: &'a [Vec<bool>],
}

/// A resolved atom source: an index into the stored relations or into
/// the finished derived tables.
enum Src {
    Edb(usize),
    Derived(usize),
}

/// One atom over substituted terms, each binding pre-tagged with whether
/// its column is interval-typed (the membership rule's trigger).
struct FlatAtom {
    src: Src,
    bindings: Vec<(usize, bool, Substituted)>,
}

/// Everything enumeration reads.
struct Env<'a> {
    relations: &'a [BTreeSet<Tuple>],
    /// The derived tables an `Interior` occurrence reads (empty for a plain query).
    interiors: &'a [BTreeSet<Tuple>],
    atoms: Vec<FlatAtom>,
    negated: Vec<FlatAtom>,
    /// The rule's predicate trees, conjoined — evaluated directly.
    conditions: Vec<SubstitutedTree>,
    /// Per variable: bound on some non-interval field of a positive atom,
    /// hence a scalar (an occurrence on an interval field is then point
    /// membership; without a scalar anchor the variable is interval-typed
    /// and interval occurrences are value equality).
    scalar_anchored: Vec<bool>,
    var_count: usize,
}

impl Env<'_> {
    /// The fact set one source reads: a stored relation, or a
    /// derived table's accumulated answers.
    fn facts(&self, src: &Src) -> &BTreeSet<Tuple> {
        match src {
            Src::Edb(relation) => &self.relations[*relation],
            Src::Derived(id) => &self.interiors[*id],
        }
    }
}

impl NaiveDb {
    /// Evaluates a validated query with positional parameters, from the
    /// definition: the **set union of the rules' denotations**. Per rule,
    /// the set of distinct full bindings is projected and folded per its
    /// find list; a one-rule query is exactly the conjunctive query.
    ///
    /// A multi-rule aggregate head folds over the union of the rules'
    /// binding sets projected to the head (the rules-IR definition; the
    /// executor's spanning seen-set implements the same dedup —
    /// `docs/architecture/40-execution.md` § the rule loop). The
    /// single-rule fold domain stays the rule's distinct **full**
    /// binding set — the normative aggregation rule, unchanged.
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
        let mut sets: Vec<BTreeSet<Tuple>> = Vec::new();
        let mut interval: Vec<Vec<bool>> = Vec::new();
        let (interiors, rec, head, rules) = match query {
            Query {
                interiors,
                head,
                rules,
                rec: None,
            } => (
                interiors.as_slice(),
                None,
                head.as_slice(),
                rules.as_slice(),
            ),
            Query {
                interiors,
                rec: Some(rec),
                head,
                rules,
            } => (
                interiors.as_slice(),
                Some(rec),
                head.as_slice(),
                rules.as_slice(),
            ),
        };
        for interior in interiors {
            let derived = DerivedWorld {
                sets: &sets,
                interval: &interval,
            };
            let head = interior.head();
            let rules: Vec<Rule> = interior
                .rules
                .iter()
                .map(bumbledb::ProjectionRule::to_rule)
                .collect();
            let rows = self.rows_for(&head, &rules, params, &derived)?;
            interval.push(self.seal_intervals(&head, &rules, &interval));
            sets.push(rows);
        }
        if let Some(rec) = rec {
            self.rec_lfp(rec, params, &mut sets, &mut interval)?;
        }
        let derived = DerivedWorld {
            sets: &sets,
            interval: &interval,
        };
        self.rows_for(head, rules, params, &derived)
    }

    /// Naive full-T(I) least fixpoint: re-evaluate base ∪ rec each
    /// iteration until the rec table stops growing. The derived-table
    /// index is assigned here before the empty table is pushed.
    fn rec_lfp(
        &self,
        rec: &bumbledb::Rec,
        params: &[ParamValue],
        sets: &mut Vec<BTreeSet<Tuple>>,
        interval: &mut Vec<Vec<bool>>,
    ) -> Result<(), QueryError> {
        let table_idx = sets.len();
        let iid = InteriorId(u32::try_from(table_idx).expect("interior id fits u32"));
        let head = rec.head();
        let base: Vec<Rule> = rec.base.iter().map(RecRule::to_rule).collect();
        let step: Vec<Rule> = rec
            .rec
            .iter()
            .map(|arm| RecStep::to_rule(arm, iid))
            .collect();
        interval.push(self.seal_intervals(&head, &base, interval));
        sets.push(BTreeSet::new());
        loop {
            let derived = DerivedWorld { sets, interval };
            let mut next = self.rows_for(&head, &base, params, &derived)?;
            next.extend(self.rows_for(&head, &step, params, &derived)?);
            if next == sets[table_idx] {
                break;
            }
            sets[table_idx] = next;
        }
        Ok(())
    }

    /// One named interior / rec's column interval flags, sealed from
    /// the first rule against already-sealed prior tables. Rec seals
    /// from `base` (base never reads the rec).
    fn seal_intervals(&self, head: &[HeadTerm], rules: &[Rule], prior: &[Vec<bool>]) -> Vec<bool> {
        let Some(rule) = rules.first() else {
            return vec![false; head.len()];
        };
        let col_is_interval = |atom: &Atom, field: bumbledb::FieldId| match atom.source {
            AtomSource::Edb(relation) => self.edb_field_is_interval(relation, field),
            AtomSource::Interior(id) => prior
                .get(id.0 as usize)
                .and_then(|row| row.get(usize::from(field.0)))
                .copied()
                .unwrap_or(false),
        };
        let var_is_interval = |var: VarId| {
            !rule.atoms.iter().any(|atom| {
                atom.bindings.iter().any(|(field, term)| {
                    matches!(term, Term::Var(v) if *v == var) && !col_is_interval(atom, *field)
                })
            })
        };
        rule.finds
            .iter()
            .map(|find| match find {
                FindTerm::Var(var) => var_is_interval(*var),
                FindTerm::Pack { .. } => true,
                FindTerm::Count | FindTerm::Aggregate { .. } => false,
            })
            .collect()
    }

    /// One derived table's denotation against a derived world — the
    /// query dispatch (single rule / union fold / union of
    /// projections), source-generalized. [`NaiveDb::query`] is the
    /// empty-world reading; the fixpoint calls it per round.
    fn rows_for(
        &self,
        head: &[HeadTerm],
        rules: &[Rule],
        params: &[ParamValue],
        derived: &DerivedWorld<'_>,
    ) -> Result<BTreeSet<Tuple>, QueryError> {
        if let [rule] = rules {
            let bindings = self.rule_bindings(rule, params, derived);
            return project(&rule.finds, &bindings);
        }
        let aggregated = head
            .iter()
            .any(|term| matches!(term, HeadTerm::Aggregate(_)));
        if aggregated {
            return self.union_fold(rules, params, derived);
        }
        // Projection head: the union of the per-rule projected sets —
        // one union, set semantics.
        let mut rows = BTreeSet::new();
        for rule in rules {
            let bindings = self.rule_bindings(rule, params, derived);
            rows.extend(project(&rule.finds, &bindings)?);
        }
        Ok(rows)
    }

    /// One rule's distinct full binding set — the conjunctive semantics
    /// over the rule's own variable scope, occurrences read through the
    /// source world (stored relations, plus the derived tables when a
    /// fixpoint is running).
    fn rule_bindings(
        &self,
        rule: &Rule,
        params: &[ParamValue],
        derived: &DerivedWorld<'_>,
    ) -> BTreeSet<Tuple> {
        let var_count = count_vars(rule);
        let mut scalar_anchored = vec![false; var_count];
        for atom in &rule.atoms {
            for (field, term) in &atom.bindings {
                if let Term::Var(var) = term
                    && !self.source_field_is_interval(atom, *field, derived)
                {
                    scalar_anchored[usize::from(var.0)] = true;
                }
            }
        }
        let env = Env {
            relations: &self.relations,
            interiors: derived.sets,
            atoms: rule
                .atoms
                .iter()
                .map(|atom| self.flatten(atom, params, derived))
                .collect(),
            negated: rule
                .negated
                .iter()
                .map(|atom| self.flatten(atom, params, derived))
                .collect(),
            conditions: rule
                .conditions
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
    /// position. Pack is relation-shaped and stays on its own path.
    fn union_fold(
        &self,
        rules: &[Rule],
        params: &[ParamValue],
        derived: &DerivedWorld<'_>,
    ) -> Result<BTreeSet<Tuple>, QueryError> {
        let head = &rules[0].finds;
        let mut domain: BTreeSet<Tuple> = BTreeSet::new();
        for rule in rules {
            for binding in &self.rule_bindings(rule, params, derived) {
                let row: Result<Vec<Value>, QueryError> = rule
                    .finds
                    .iter()
                    .map(|term| match term {
                        FindTerm::Var(var)
                        | FindTerm::Aggregate { over: var, .. }
                        | FindTerm::Pack { over: var } => Ok(binding.0[usize::from(var.0)].clone()),
                        // Nullary Count: no fold input — a constant
                        // filler keeps positions stable.
                        FindTerm::Count => Ok(Value::Bool(false)),
                    })
                    .collect();
                domain.insert(Tuple(row?));
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
        let pack = pack_position(head);
        let mut rows = BTreeSet::new();
        for group in groups.values() {
            // A Pack head folds the union: the domain rows carry the raw
            // claims at the Pack position (per rule, deduplicated as a
            // set above), and the group coalesces them — ∪ then maximal
            // segments, one row per segment. Every other position is a
            // group-key position (validation).
            if let Some((position, _)) = pack {
                let claims: Vec<&Value> = group.iter().map(|row| &row.0[position]).collect();
                for segment in pack_segments(&claims) {
                    let row: Result<Vec<Value>, QueryError> = head
                        .iter()
                        .enumerate()
                        .map(|(index, term)| match term {
                            FindTerm::Var(_) => Ok(group[0].0[index].clone()),
                            FindTerm::Pack { .. } if index == position => Ok(segment.clone()),
                            FindTerm::Count
                            | FindTerm::Aggregate { .. }
                            | FindTerm::Pack { .. } => {
                                unreachable!("validated: Pack mixes with no other aggregate")
                            }
                        })
                        .collect();
                    rows.insert(Tuple(row?));
                }
                continue;
            }
            let row: Result<Vec<Value>, QueryError> = head
                .iter()
                .enumerate()
                .map(|(index, term)| match term {
                    // The domain rows already hold measure values at the
                    // measure positions, so the union fold reads them
                    // exactly like plain positions.
                    FindTerm::Var(_) => Ok(group[0].0[index].clone()),
                    FindTerm::Count => Ok(Value::U64(
                        u64::try_from(group.len()).expect("group sizes fit u64"),
                    )),
                    FindTerm::Aggregate { op, .. } => fold_position(*op, index, group),
                    FindTerm::Pack { .. } => {
                        unreachable!("validated: Pack heads take the segment path")
                    }
                })
                .collect();
            rows.insert(Tuple(row?));
        }
        Ok(rows)
    }

    fn flatten(&self, atom: &Atom, params: &[ParamValue], derived: &DerivedWorld<'_>) -> FlatAtom {
        FlatAtom {
            src: match atom.source {
                AtomSource::Edb(relation) => Src::Edb(relation.0 as usize),
                AtomSource::Interior(id) => Src::Derived(id.0 as usize),
            },
            bindings: atom
                .bindings
                .iter()
                .map(|(field, term)| {
                    (
                        usize::from(field.0),
                        self.source_field_is_interval(atom, *field, derived),
                        substitute(term, params),
                    )
                })
                .collect(),
        }
    }

    fn edb_field_is_interval(
        &self,
        relation: bumbledb::RelationId,
        field: bumbledb::FieldId,
    ) -> bool {
        self.field_type(relation.0 as usize, usize::from(field.0))
            .is_interval()
    }

    /// The membership trigger per source: a stored field's declared
    /// type, or a derived column's.
    fn source_field_is_interval(
        &self,
        atom: &Atom,
        field: bumbledb::FieldId,
        derived: &DerivedWorld<'_>,
    ) -> bool {
        match atom.source {
            AtomSource::Edb(relation) => self.edb_field_is_interval(relation, field),
            AtomSource::Interior(id) => derived
                .interval
                .get(id.0 as usize)
                .and_then(|row| row.get(usize::from(field.0)))
                .copied()
                .unwrap_or(false),
        }
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
    fn see_tree(count: &mut usize, tree: &ConditionTree) {
        match tree {
            ConditionTree::Leaf(Comparison { lhs, rhs, .. }) => {
                see_term(count, lhs);
                see_term(count, rhs);
            }
            ConditionTree::And(children) | ConditionTree::Or(children) => {
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
    for tree in &rule.conditions {
        see_tree(&mut count, tree);
    }
    for find in &rule.finds {
        match find {
            FindTerm::Var(var) => see(&mut count, *var),
            FindTerm::Aggregate { over, .. } | FindTerm::Pack { over } => see(&mut count, *over),
            FindTerm::Count => {}
        }
    }
    count
}

/// Substitutes params through a predicate tree, keeping its shape. A
/// param mask substitutes like any param — the model sees only literal
/// masks past this point.
fn substitute_tree(tree: &ConditionTree, params: &[ParamValue]) -> SubstitutedTree {
    match tree {
        ConditionTree::Leaf(Comparison { op, lhs, rhs }) => {
            SubstitutedTree::Leaf(*op, substitute(lhs, params), substitute(rhs, params))
        }
        ConditionTree::And(children) => SubstitutedTree::And(
            children
                .iter()
                .map(|child| substitute_tree(child, params))
                .collect(),
        ),
        ConditionTree::Or(children) => SubstitutedTree::Or(
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
/// `index`, extend the assignment, recurse; at the leaf judge the deferred
/// membership tests, the predicates, and the negated atoms — `Holds`
/// records the full binding, `Fails` drops.
fn enumerate(
    env: &Env<'_>,
    index: usize,
    assignment: &mut Vec<Option<Value>>,
    pending: &mut Vec<(usize, Value)>,
    out: &mut BTreeSet<Tuple>,
) {
    if index == env.atoms.len() {
        match leaf_verdict(env, assignment, pending) {
            Verdict3::Holds => {
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
            Verdict3::Fails => {}
        }
        return;
    }
    let atom = &env.atoms[index];
    for fact in env.facts(&atom.src) {
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
                    point_in(
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
    if field_is_interval && let Some(t) = point(term_value) {
        return point_in(endpoints(fact_value), t);
    }
    term_value == fact_value
}

/// One complete binding's verdict: the deferred membership tests, the
/// predicate trees, and the negated atoms conjoined in the Kleene
/// lattice. Memberships and negations are two-valued (no measure can
/// reach them), so their `Fails` absorbs any `Ray` a condition tree
/// renders — exactly `andFold`'s law.
fn leaf_verdict(
    env: &Env<'_>,
    assignment: &mut [Option<Value>],
    pending: &[(usize, Value)],
) -> Verdict3 {
    for (var, interval) in pending {
        let bound = assignment[*var]
            .as_ref()
            .expect("validated: every point variable has a scalar anchor");
        if !point_in(
            endpoints(interval),
            point(bound).expect("a scalar-anchored variable holds a scalar"),
        ) {
            return Verdict3::Fails;
        }
    }
    for atom in &env.negated {
        let matched = env
            .facts(&atom.src)
            .iter()
            .any(|fact| negated_matches(env, atom, fact, assignment));
        if matched {
            return Verdict3::Fails;
        }
    }
    env.conditions
        .iter()
        .fold(Verdict3::Holds, |verdict, tree| {
            verdict.and(tree_verdict(tree, assignment))
        })
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

/// Two-valued verdict of one condition evaluation: `Fails` absorbs
/// `and`, `Holds` absorbs `or`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Verdict3 {
    Holds,
    Fails,
}

impl Verdict3 {
    fn of(holds: bool) -> Self {
        if holds { Self::Holds } else { Self::Fails }
    }

    fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::Fails, _) | (_, Self::Fails) => Self::Fails,
            (Self::Holds, Self::Holds) => Self::Holds,
        }
    }

    fn or(self, other: Self) -> Self {
        match (self, other) {
            (Self::Holds, _) | (_, Self::Holds) => Self::Holds,
            (Self::Fails, Self::Fails) => Self::Fails,
        }
    }
}

/// One predicate tree under a complete assignment, from the definition:
/// a leaf is its comparison's verdict, `And` folds children with
/// `Verdict3::and` (the empty conjunction holds), `Or` with
/// `Verdict3::or` (the empty disjunction fails). No DNF, no
/// distribution, no short circuit — the tree is the semantics and the
/// lattice makes child order unobservable.
fn tree_verdict(tree: &SubstitutedTree, assignment: &[Option<Value>]) -> Verdict3 {
    match tree {
        SubstitutedTree::Leaf(op, lhs, rhs) => leaf_comparison(*op, lhs, rhs, assignment),
        SubstitutedTree::And(children) => {
            children.iter().fold(Verdict3::Holds, |verdict, child| {
                verdict.and(tree_verdict(child, assignment))
            })
        }
        SubstitutedTree::Or(children) => children.iter().fold(Verdict3::Fails, |verdict, child| {
            verdict.or(tree_verdict(child, assignment))
        }),
    }
}

fn leaf_comparison(
    op: CmpOp,
    lhs: &Substituted,
    rhs: &Substituted,
    assignment: &[Option<Value>],
) -> Verdict3 {
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
        return Verdict3::of(values.contains(&value));
    }
    let left = resolve(lhs).expect("validated: sets appear only under Eq");
    let right = resolve(rhs).expect("validated: sets appear only under Eq");
    Verdict3::of(match op {
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
            let (a, b) = (endpoints(&left), endpoints(&right));
            Basic::ALL
                .iter()
                .any(|basic| mask.contains(*basic) && basic_holds(*basic, a, b))
        }
        CmpOp::PointIn => {
            let t = point(&right).expect("validated: PointIn's right side is a point");
            point_in(endpoints(&left), t)
        }
    })
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

/// One group's `Pack` rows: relation-shaped — one row per maximal
/// segment of the group's claim union ([`pack_segments`], the point-set
/// definition); every other position is a group-key position
/// (validation: Pack mixes with no other aggregate).
fn pack_group_rows(
    finds: &[FindTerm],
    position: usize,
    over: VarId,
    group: &[&Tuple],
    rows: &mut BTreeSet<Tuple>,
) -> Result<(), QueryError> {
    let claims: Vec<&Value> = group
        .iter()
        .map(|binding| &binding.0[usize::from(over.0)])
        .collect();
    for segment in pack_segments(&claims) {
        let row: Result<Vec<Value>, QueryError> = finds
            .iter()
            .enumerate()
            .map(|(index, find)| match find {
                FindTerm::Var(var) => Ok(group[0].0[usize::from(var.0)].clone()),
                FindTerm::Pack { .. } if index == position => Ok(segment.clone()),
                FindTerm::Count | FindTerm::Aggregate { .. } | FindTerm::Pack { .. } => {
                    unreachable!("validated: Pack mixes with no other aggregate")
                }
            })
            .collect();
        rows.insert(Tuple(row?));
    }
    Ok(())
}

/// Projects and folds the distinct full bindings per the find list: group
/// key = the values of the plain-variable finds; every aggregate folds
/// over its group's binding set. No bindings means no groups — the empty
/// set, global aggregates included.
fn project(finds: &[FindTerm], bindings: &BTreeSet<Tuple>) -> Result<BTreeSet<Tuple>, QueryError> {
    let mut groups: BTreeMap<Tuple, Vec<&Tuple>> = BTreeMap::new();
    for binding in bindings {
        let mut key = Vec::new();
        for find in finds {
            match find {
                FindTerm::Var(var) => key.push(binding.0[usize::from(var.0)].clone()),
                FindTerm::Count | FindTerm::Aggregate { .. } | FindTerm::Pack { .. } => {}
            }
        }
        groups.entry(Tuple(key)).or_default().push(binding);
    }
    let pack = pack_position(finds);
    let mut rows = BTreeSet::new();
    for group in groups.values() {
        if let Some((position, over)) = pack {
            pack_group_rows(finds, position, over, group, &mut rows)?;
        } else {
            let row: Result<Vec<Value>, QueryError> = finds
                .iter()
                .enumerate()
                .map(|(index, find)| match find {
                    FindTerm::Var(var) => Ok(group[0].0[usize::from(var.0)].clone()),
                    FindTerm::Count => Ok(Value::U64(
                        u64::try_from(group.len()).expect("group sizes fit u64"),
                    )),
                    FindTerm::Aggregate { op, over } => fold(*op, *over, group, index),
                    FindTerm::Pack { .. } => {
                        unreachable!("validated: Pack heads take the segment path")
                    }
                })
                .collect();
            rows.insert(Tuple(row?));
        }
    }
    Ok(rows)
}

/// One fold aggregate over a group of head-projected tuples (the
/// multi-rule union fold): the position's values are the fold inputs.
fn fold_position(op: FoldOp, index: usize, group: &[&Tuple]) -> Result<Value, QueryError> {
    let values = || group.iter().map(move |row| &row.0[index]);
    match op {
        FoldOp::Sum => {
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
        FoldOp::Min | FoldOp::Max => {
            let picked = values()
                .max_by(|a, b| {
                    let ordering = cmp_value(a, b);
                    if matches!(op, FoldOp::Max) {
                        ordering
                    } else {
                        ordering.reverse()
                    }
                })
                .expect("groups are nonempty");
            Ok(picked.clone())
        }
    }
}

/// One fold aggregate over a group's binding set.
fn fold(op: FoldOp, over: VarId, group: &[&Tuple], find: usize) -> Result<Value, QueryError> {
    let values = || group.iter().map(move |b| &b.0[usize::from(over.0)]);
    match op {
        FoldOp::Sum => {
            let total: i128 = values()
                .map(|value| point(value).expect("validated: Sum takes integers"))
                .sum();
            match values().next().expect("groups are nonempty") {
                Value::U64(_) => u64::try_from(total)
                    .map(Value::U64)
                    .map_err(|_| QueryError::Overflow { find }),
                Value::I64(_) => i64::try_from(total)
                    .map(Value::I64)
                    .map_err(|_| QueryError::Overflow { find }),
                other => panic!("validated: Sum takes integers, got {other:?}"),
            }
        }
        FoldOp::Min | FoldOp::Max => {
            let picked = values()
                .max_by(|a, b| {
                    let ordering = cmp_value(a, b);
                    if matches!(op, FoldOp::Max) {
                        ordering
                    } else {
                        ordering.reverse()
                    }
                })
                .expect("groups are nonempty");
            Ok(picked.clone())
        }
    }
}
