use std::collections::{BTreeMap, BTreeSet};

use bumbledb::{
    Atom, AtomSource, Basic, CmpOp, Comparison, ConditionTree, FindTerm, FoldOp, HeadTerm,
    InteriorId, Query, RecRule, RecStep, Rule, Term, Value, VarId,
};

use super::tuple::{cmp_value, endpoints, point, point_in};
use super::{NaiveDb, Tuple};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParamValue {
    Scalar(Value),
    Set(Vec<Value>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryError {
    Overflow { find: usize },
}

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

/// A predicate tree after parameter substitution — the input grammar's shape,
/// kept: the model evaluates it recursively, exactly as written.
enum SubstitutedTree {
    Leaf(CmpOp, Substituted, Substituted),
    And(Vec<SubstitutedTree>),
    Or(Vec<SubstitutedTree>),
}

/// A derived table's facts ARE its answer tuples, read positionally:
/// `FieldId(i)` is head position `i` (`lean/Bumbledb/Query/Denotation.lean:
/// tupleFact` — the positional addressing interiors and rec share).
pub(super) struct DerivedWorld<'a> {
    sets: &'a [BTreeSet<Tuple>],

    interval: &'a [Vec<bool>],
}

enum Src {
    Edb(usize),
    Derived(usize),
}

struct FlatAtom {
    src: Src,
    bindings: Vec<(usize, bool, Substituted)>,
}

struct Env<'a> {
    relations: &'a [BTreeSet<Tuple>],

    interiors: &'a [BTreeSet<Tuple>],
    atoms: Vec<FlatAtom>,
    negated: Vec<FlatAtom>,

    conditions: Vec<SubstitutedTree>,

    scalar_anchored: Vec<bool>,
    var_count: usize,
}

impl Env<'_> {
    fn facts(&self, src: &Src) -> &BTreeSet<Tuple> {
        match src {
            Src::Edb(relation) => &self.relations[*relation],
            Src::Derived(id) => &self.interiors[*id],
        }
    }
}

impl NaiveDb {
    /// # Errors

    /// # Panics

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

        let mut rows = BTreeSet::new();
        for rule in rules {
            let bindings = self.rule_bindings(rule, params, derived);
            rows.extend(project(&rule.finds, &bindings)?);
        }
        Ok(rows)
    }

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

                        FindTerm::Count => Ok(Value::Bool(false)),
                    })
                    .collect();
                domain.insert(Tuple(row?));
            }
        }

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

fn constrains(fact_value: &Value, field_is_interval: bool, term_value: &Value) -> bool {
    if field_is_interval && let Some(t) = point(term_value) {
        return point_in(endpoints(fact_value), t);
    }
    term_value == fact_value
}

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
