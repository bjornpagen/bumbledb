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
    AggOp, Atom, AtomSource, Basic, CmpOp, Comparison, ConditionTree, FindTerm, HeadTerm, MaskTerm,
    Program, Query, Rule, Term, Value, VarId,
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
/// value out of its result type's range, and the measure of a ray —
/// `Duration` over `[s, ∞)` (the engine's one runtime type error,
/// `docs/architecture/10-data-model.md`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryError {
    Overflow { find: usize },
    MeasureOfRay,
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

/// The measure, from the definition: `|[s, e)| = e − s` over the logical
/// element values — the model's own arithmetic, deliberately independent
/// of the engine's encoded-word subtraction (the differential oracle
/// would otherwise test a function against itself). A ray (`end` at the
/// element domain's MAX) has no finite measure.
fn measure_value(value: &Value) -> Result<u64, QueryError> {
    match value {
        Value::IntervalU64(interval) => {
            if interval.is_ray() {
                Err(QueryError::MeasureOfRay)
            } else {
                Ok(interval.end() - interval.start())
            }
        }
        Value::IntervalI64(interval) => {
            if interval.is_ray() {
                Err(QueryError::MeasureOfRay)
            } else {
                Ok(
                    u64::try_from(i128::from(interval.end()) - i128::from(interval.start()))
                        .expect("constructor: end > start, difference below 2^64"),
                )
            }
        }
        other => panic!("validated: Duration takes an interval, got {other:?}"),
    }
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
        if let FindTerm::Aggregate {
            op: AggOp::Pack,
            over,
        } = find
        {
            Some((index, over.expect("validated: Pack carries a variable")))
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
    /// The measure of an interval variable (`Term::Measure`).
    Measure(usize),
}

/// A predicate tree after parameter substitution — the input grammar's
/// shape, kept: the model evaluates it recursively, exactly as written.
enum SubstitutedTree {
    Leaf(CmpOp, Substituted, Substituted),
    And(Vec<SubstitutedTree>),
    Or(Vec<SubstitutedTree>),
}

/// The predicate reading of one program evaluation — the fixpoint's
/// working sets beside their column typing. A predicate's facts ARE its
/// answer tuples, read positionally: `FieldId(i)` is head position `i`
/// (`lean/Bumbledb/Exec/Fixpoint.lean: tupleFact` — the positional
/// addressing the program cut promised). A plain query reads no
/// predicates: the empty world.
pub(super) struct PredWorld<'a> {
    /// Per predicate, the accumulated answer-tuple set.
    sets: &'a [BTreeSet<Tuple>],
    /// Per predicate, per head position: interval-typed? — the
    /// membership typing rule read through predicate columns.
    interval: &'a [Vec<bool>],
}

impl PredWorld<'static> {
    /// The query world: no predicates exist.
    const EMPTY: PredWorld<'static> = PredWorld {
        sets: &[],
        interval: &[],
    };
}

/// A resolved atom source: an index into the stored relations or into
/// the predicate sets — the model's plain-data twin of the Lean even/odd
/// source coding (a device there, an enum here).
enum Src {
    Edb(usize),
    Idb(usize),
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
    /// The predicate sets an `Idb` occurrence reads (empty for queries).
    predicates: &'a [BTreeSet<Tuple>],
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
    /// The measure poison: a predicate's `Duration` reached a ray — the
    /// rule's answer is [`QueryError::MeasureOfRay`], checked after
    /// enumeration (the model's twin of the engine's poison flag).
    ray: std::cell::Cell<bool>,
}

impl Env<'_> {
    /// The fact set one source reads: a stored relation, or a
    /// predicate's accumulated answers.
    fn facts(&self, src: &Src) -> &BTreeSet<Tuple> {
        match src {
            Src::Edb(relation) => &self.relations[*relation],
            Src::Idb(pred) => &self.predicates[*pred],
        }
    }
}

impl NaiveDb {
    /// Evaluates a validated query with positional parameters, from the
    /// definition: the **set union of the rules' denotations**. Per rule,
    /// the set of distinct full bindings is projected and folded per its
    /// find list; a one-rule program is exactly the conjunctive query.
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
        self.rows_for(&query.head, &query.rules, params, &PredWorld::EMPTY)
    }

    /// Evaluates a validated program with positional parameters, from
    /// the definition — **the naive stratified fixpoint** (the shipping
    /// law's naive oracle; the Lean truth is
    /// `lean/Bumbledb/Exec/Fixpoint.lean: evalProgram` under
    /// `program_eval_sound`, and the stratified denotation it lists is
    /// `programDen`). Per stratum in condensation order, loop: evaluate
    /// EVERY rule of every stratum predicate against the current
    /// predicate sets with the same nested-loop evaluator queries use,
    /// union the answers in, stop on no change. Naive, never semi-naive,
    /// deliberately — no deltas, no frontier, no watermark: the model's
    /// correctness is definitional and its independence is the recorded
    /// trust root (the independence law).
    ///
    /// # Errors
    ///
    /// [`QueryError::Overflow`] / [`QueryError::MeasureOfRay`] exactly as
    /// [`NaiveDb::query`] raises them — reachable only from
    /// non-recursive predicates (the strata roster keeps measures and
    /// folds out of recursive heads), so the verdict never depends on
    /// iteration order.
    ///
    /// # Panics
    ///
    /// On malformed input — the model evaluates programs the engine's
    /// validation roster has accepted (stratified, safe, aligned).
    pub fn program(
        &self,
        program: &Program,
        params: &[ParamValue],
    ) -> Result<BTreeSet<Tuple>, QueryError> {
        let strata = model_strata(program);
        let interval = self.predicate_intervals(program);
        let mut sets: Vec<BTreeSet<Tuple>> = vec![BTreeSet::new(); program.predicates.len()];
        let top = strata.iter().copied().max().unwrap_or(0);
        for stratum in 0..=top {
            let members: Vec<usize> = (0..program.predicates.len())
                .filter(|index| strata[*index] == stratum)
                .collect();
            loop {
                // One round: every stratum rule against the CURRENT
                // sets (the Lean `stratumStep`), then one union — the
                // rounds are simultaneous, re-derivation is absorbed by
                // set semantics, never re-counted.
                let mut derived: Vec<(usize, BTreeSet<Tuple>)> = Vec::new();
                for index in &members {
                    let def = &program.predicates[*index];
                    let preds = PredWorld {
                        sets: &sets,
                        interval: &interval,
                    };
                    derived.push((
                        *index,
                        self.rows_for(&def.head, &def.rules, params, &preds)?,
                    ));
                }
                let mut changed = false;
                for (index, rows) in derived {
                    for row in rows {
                        changed |= sets[index].insert(row);
                    }
                }
                if !changed {
                    break;
                }
            }
        }
        Ok(std::mem::take(&mut sets[usize::from(program.output.0)]))
    }

    /// One predicate's denotation against a predicate world — the
    /// query dispatch (single rule / union fold / union of
    /// projections), source-generalized. [`NaiveDb::query`] is the
    /// empty-world reading; the fixpoint calls it per round.
    fn rows_for(
        &self,
        head: &[HeadTerm],
        rules: &[Rule],
        params: &[ParamValue],
        preds: &PredWorld<'_>,
    ) -> Result<BTreeSet<Tuple>, QueryError> {
        if let [rule] = rules {
            let bindings = self.rule_bindings(rule, params, preds)?;
            return project(&rule.finds, &bindings);
        }
        let aggregated = head
            .iter()
            .any(|term| matches!(term, HeadTerm::Aggregate(_)));
        if aggregated {
            return self.union_fold(rules, params, preds);
        }
        // Projection head: the union of the per-rule projected sets —
        // one union, set semantics.
        let mut rows = BTreeSet::new();
        for rule in rules {
            let bindings = self.rule_bindings(rule, params, preds)?;
            rows.extend(project(&rule.finds, &bindings)?);
        }
        Ok(rows)
    }

    /// Per predicate, per head position: interval-typed? — the
    /// membership typing rule read through predicate columns (an `Idb`
    /// occurrence on an interval-typed column participates in point
    /// membership exactly as an interval field does). Re-derived from
    /// the rules to a monotone fixpoint: a projected variable is
    /// interval-typed when no positive occurrence anchors it on a
    /// scalar column, `Pack` and Arg-carried interval variables type
    /// their positions, every other head shape is scalar. Rules align
    /// per predicate (validation), so the first rule speaks for all.
    fn predicate_intervals(&self, program: &Program) -> Vec<Vec<bool>> {
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
                let col_is_interval = |atom: &Atom, field: bumbledb::FieldId| match atom.source {
                    AtomSource::Edb(_) => self.atom_field_is_interval(atom, field),
                    AtomSource::Idb(pred) => interval[usize::from(pred.0)]
                        .get(usize::from(field.0))
                        .copied()
                        .unwrap_or(false),
                };
                let var_is_interval = |var: VarId| {
                    !rule.atoms.iter().any(|atom| {
                        atom.bindings.iter().any(|(field, term)| {
                            matches!(term, Term::Var(v) if *v == var)
                                && !col_is_interval(atom, *field)
                        })
                    })
                };
                let flags: Vec<bool> = rule
                    .finds
                    .iter()
                    .map(|find| match find {
                        // A projected variable and an Arg-carried one
                        // type their positions identically: the value
                        // is the variable's.
                        FindTerm::Var(var)
                        | FindTerm::Aggregate {
                            op: AggOp::ArgMax { .. } | AggOp::ArgMin { .. },
                            over: Some(var),
                        } => var_is_interval(*var),
                        FindTerm::Aggregate {
                            op: AggOp::Pack, ..
                        } => true,
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
                return interval;
            }
        }
    }

    /// One rule's distinct full binding set — the conjunctive semantics
    /// over the rule's own variable scope, occurrences read through the
    /// source world (stored relations, plus the predicate sets when a
    /// fixpoint is running).
    fn rule_bindings(
        &self,
        rule: &Rule,
        params: &[ParamValue],
        preds: &PredWorld<'_>,
    ) -> Result<BTreeSet<Tuple>, QueryError> {
        let var_count = count_vars(rule);
        let mut scalar_anchored = vec![false; var_count];
        for atom in &rule.atoms {
            for (field, term) in &atom.bindings {
                if let Term::Var(var) = term
                    && !self.source_field_is_interval(atom, *field, preds)
                {
                    scalar_anchored[usize::from(var.0)] = true;
                }
            }
        }
        let env = Env {
            relations: &self.relations,
            predicates: preds.sets,
            atoms: rule
                .atoms
                .iter()
                .map(|atom| self.flatten(atom, params, preds))
                .collect(),
            negated: rule
                .negated
                .iter()
                .map(|atom| self.flatten(atom, params, preds))
                .collect(),
            conditions: rule
                .conditions
                .iter()
                .map(|tree| substitute_tree(tree, params))
                .collect(),
            scalar_anchored,
            var_count,
            ray: std::cell::Cell::new(false),
        };
        let mut bindings = BTreeSet::new();
        let mut assignment = vec![None; var_count];
        let mut pending = Vec::new();
        enumerate(&env, 0, &mut assignment, &mut pending, &mut bindings);
        if env.ray.get() {
            return Err(QueryError::MeasureOfRay);
        }
        Ok(bindings)
    }

    /// The multi-rule aggregate fold: each rule's binding set projected
    /// to the head (per position: the variable's value, or the
    /// aggregate's fold-input value — the nullary `Count` contributes a
    /// constant filler), unioned as a set, then grouped and folded per
    /// position. Arg terms are single-rule-only — validation refuses
    /// them across rules (their key is a rule variable the head
    /// projection does not carry, so the union's extreme is undefined —
    /// `20-query-ir.md` § aggregation).
    fn union_fold(
        &self,
        rules: &[Rule],
        params: &[ParamValue],
        preds: &PredWorld<'_>,
    ) -> Result<BTreeSet<Tuple>, QueryError> {
        let head = &rules[0].finds;
        assert!(
            !head.iter().any(|term| matches!(
                term,
                FindTerm::Aggregate {
                    op: AggOp::ArgMax { .. } | AggOp::ArgMin { .. },
                    ..
                }
            )),
            "validation refuses Arg-restriction across rules"
        );
        let mut domain: BTreeSet<Tuple> = BTreeSet::new();
        for rule in rules {
            for binding in &self.rule_bindings(rule, params, preds)? {
                let row: Result<Vec<Value>, QueryError> = rule
                    .finds
                    .iter()
                    .map(|term| match term {
                        FindTerm::Var(var)
                        | FindTerm::Aggregate {
                            over: Some(var), ..
                        } => Ok(binding.0[usize::from(var.0)].clone()),
                        // The measure positions project the measure — from
                        // the definition, ray included.
                        FindTerm::Measure(var) | FindTerm::AggregateMeasure { over: var, .. } => {
                            measure_value(&binding.0[usize::from(var.0)]).map(Value::U64)
                        }
                        // Nullary Count: no fold input — a constant
                        // filler keeps positions stable.
                        FindTerm::Aggregate { over: None, .. } => Ok(Value::Bool(false)),
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
                    .filter(|(term, _)| matches!(term, FindTerm::Var(_) | FindTerm::Measure(_)))
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
                            FindTerm::Var(_) | FindTerm::Measure(_) => {
                                Ok(group[0].0[index].clone())
                            }
                            FindTerm::Aggregate { .. } if index == position => Ok(segment.clone()),
                            FindTerm::Aggregate { .. } | FindTerm::AggregateMeasure { .. } => {
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
                    FindTerm::Var(_) | FindTerm::Measure(_) => Ok(group[0].0[index].clone()),
                    FindTerm::Aggregate { op, .. } | FindTerm::AggregateMeasure { op, .. } => {
                        fold_position(*op, index, group)
                    }
                })
                .collect();
            rows.insert(Tuple(row?));
        }
        Ok(rows)
    }

    fn flatten(&self, atom: &Atom, params: &[ParamValue], preds: &PredWorld<'_>) -> FlatAtom {
        FlatAtom {
            src: match atom.source {
                AtomSource::Edb(relation) => Src::Edb(relation.0 as usize),
                AtomSource::Idb(pred) => Src::Idb(usize::from(pred.0)),
            },
            bindings: atom
                .bindings
                .iter()
                .map(|(field, term)| {
                    (
                        usize::from(field.0),
                        self.source_field_is_interval(atom, *field, preds),
                        substitute(term, params),
                    )
                })
                .collect(),
        }
    }

    fn atom_field_is_interval(&self, atom: &Atom, field: bumbledb::FieldId) -> bool {
        matches!(
            self.field_type(atom.relation().0 as usize, usize::from(field.0)),
            ValueType::Interval { .. }
        )
    }

    /// The membership trigger per source: a stored field's declared
    /// type, or a predicate column's derived one.
    fn source_field_is_interval(
        &self,
        atom: &Atom,
        field: bumbledb::FieldId,
        preds: &PredWorld<'_>,
    ) -> bool {
        match atom.source {
            AtomSource::Edb(_) => self.atom_field_is_interval(atom, field),
            AtomSource::Idb(pred) => preds.interval[usize::from(pred.0)]
                .get(usize::from(field.0))
                .copied()
                .unwrap_or(false),
        }
    }
}

/// The model's stratification witness, by relaxation from the
/// definition (`lean/Bumbledb/Query/Syntax.lean: Program.StratifiedBy`):
/// sweep until every positive edge is non-increasing and every negated
/// edge strictly decreasing — plus, mirroring the engine's SCC
/// condensation (a fold rule's reads always sit in a strictly lower
/// component, `AggregationThroughCycle`), a fold rule's `Idb` reads sit
/// strictly below, so a fold never folds a still-growing set. The
/// denotation is witness-independent (the recorded classical narrowing,
/// `lean/Bumbledb/Exec/Fixpoint.lean` module doc), so any witness the
/// definition accepts serves. Deliberately NOT Tarjan: the model
/// re-derives from the definition, never shares the judge's algorithm.
///
/// # Panics
///
/// When no witness exists — the model evaluates programs the strata
/// judge has accepted.
///
/// `pub(crate)` for one reader beside the fixpoint: the conformance
/// serializer records this witness in each program case, and the Lean
/// evaluator (`evalProgram`) evaluates under the witness it is handed
/// (the denotation is witness-independent — the recorded narrowing).
pub(crate) fn model_strata(program: &Program) -> Vec<usize> {
    let count = program.predicates.len();
    let mut strata = vec![0usize; count];
    // A stratified program's strata are bounded by the predicate count,
    // and each changing sweep raises at least one stratum — so a
    // legitimate relaxation settles within count² sweeps.
    for _ in 0..=count * count {
        let mut changed = false;
        for (index, def) in program.predicates.iter().enumerate() {
            for rule in &def.rules {
                let fold = rule.finds.iter().any(|find| {
                    matches!(
                        find,
                        FindTerm::Aggregate { .. } | FindTerm::AggregateMeasure { .. }
                    )
                });
                let occurrences = rule
                    .atoms
                    .iter()
                    .map(|atom| (atom, fold))
                    .chain(rule.negated.iter().map(|atom| (atom, true)));
                for (atom, strict) in occurrences {
                    let AtomSource::Idb(pred) = atom.source else {
                        continue;
                    };
                    let floor = strata[usize::from(pred.0)] + usize::from(strict);
                    if strata[index] < floor {
                        strata[index] = floor;
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            return strata;
        }
    }
    panic!("validated: programs are stratified (the strata judge accepted this program)")
}

fn count_vars(rule: &Rule) -> usize {
    fn see(count: &mut usize, var: VarId) {
        *count = (*count).max(usize::from(var.0) + 1);
    }
    fn see_term(count: &mut usize, term: &Term) {
        if let Term::Var(var) | Term::Measure(var) = term {
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
            FindTerm::Var(var) | FindTerm::Measure(var) => see(&mut count, *var),
            FindTerm::AggregateMeasure { over, .. } => see(&mut count, *over),
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
fn substitute_tree(tree: &ConditionTree, params: &[ParamValue]) -> SubstitutedTree {
    match tree {
        ConditionTree::Leaf(Comparison { op, lhs, rhs }) => {
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
        Term::Measure(var) => Substituted::Measure(usize::from(var.0)),
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
        Substituted::Measure(_) => unreachable!("validated: no measure in bindings"),
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

fn leaf_admits(
    env: &Env<'_>,
    assignment: &mut [Option<Value>],
    pending: &[(usize, Value)],
) -> bool {
    for (var, interval) in pending {
        let bound = assignment[*var]
            .as_ref()
            .expect("validated: every point variable has a scalar anchor");
        if !point_in(
            endpoints(interval),
            point(bound).expect("a scalar-anchored variable holds a scalar"),
        ) {
            return false;
        }
    }
    for tree in &env.conditions {
        if !tree_holds(tree, assignment, &env.ray) {
            return false;
        }
    }
    for atom in &env.negated {
        let matched = env
            .facts(&atom.src)
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
fn tree_holds(
    tree: &SubstitutedTree,
    assignment: &[Option<Value>],
    ray: &std::cell::Cell<bool>,
) -> bool {
    match tree {
        SubstitutedTree::Leaf(op, lhs, rhs) => predicate_holds(*op, lhs, rhs, assignment, ray),
        SubstitutedTree::And(children) => children
            .iter()
            .all(|child| tree_holds(child, assignment, ray)),
        SubstitutedTree::Or(children) => children
            .iter()
            .any(|child| tree_holds(child, assignment, ray)),
    }
}

fn predicate_holds(
    op: CmpOp,
    lhs: &Substituted,
    rhs: &Substituted,
    assignment: &[Option<Value>],
    ray: &std::cell::Cell<bool>,
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
            // The measure, from the definition — a ray poisons the rule
            // (the enumeration's caller raises `MeasureOfRay`) and the
            // binding is dropped.
            Substituted::Measure(var) => {
                let interval = assignment[*var]
                    .clone()
                    .expect("validated: predicate variables are bound");
                match measure_value(&interval) {
                    Ok(duration) => Some(Value::U64(duration)),
                    Err(QueryError::MeasureOfRay) => {
                        ray.set(true);
                        None
                    }
                    Err(other) => panic!("measure raises only MeasureOfRay: {other:?}"),
                }
            }
        }
    };
    // A poisoned measure side: reject the binding — the rule's answer is
    // the error, checked after enumeration.
    if matches!(lhs, Substituted::Measure(_)) || matches!(rhs, Substituted::Measure(_)) {
        let (Some(left), Some(right)) = (resolve(lhs), resolve(rhs)) else {
            return false;
        };
        let a = point(&left).expect("the measure and its bound are integers");
        let b = point(&right).expect("the measure and its bound are integers");
        return match op {
            CmpOp::Lt => a < b,
            CmpOp::Le => a <= b,
            CmpOp::Gt => a > b,
            CmpOp::Ge => a >= b,
            _ => unreachable!("validated: measures compare under order operators"),
        };
    }
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
        CmpOp::PointIn => {
            let t = point(&right).expect("validated: PointIn's right side is a point");
            point_in(endpoints(&left), t)
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
                FindTerm::Measure(var) => {
                    measure_value(&group[0].0[usize::from(var.0)]).map(Value::U64)
                }
                FindTerm::Aggregate { .. } if index == position => Ok(segment.clone()),
                FindTerm::Aggregate { .. } | FindTerm::AggregateMeasure { .. } => {
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
                // A measure find is a group-key position: the projected
                // value is the measure, from the definition.
                FindTerm::Measure(var) => {
                    key.push(Value::U64(measure_value(&binding.0[usize::from(var.0)])?));
                }
                FindTerm::Aggregate { .. } | FindTerm::AggregateMeasure { .. } => {}
            }
        }
        groups.entry(Tuple(key)).or_default().push(binding);
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
    let pack = pack_position(finds);
    let mut rows = BTreeSet::new();
    for group in groups.values() {
        if let Some((position, over)) = pack {
            pack_group_rows(finds, position, over, group, &mut rows)?;
        } else if let Some((key_var, is_max)) = arg {
            // Arg-restriction: restrict the group to the bindings
            // attaining the key's extreme, then project every survivor —
            // a tie yields every attaining row.
            let extreme = group
                .iter()
                .map(|binding| &binding.0[key_var])
                .max_by(|a, b| {
                    let ordering = cmp_value(a, b);
                    if is_max { ordering } else { ordering.reverse() }
                })
                .expect("groups are nonempty by construction");
            for binding in group {
                if binding.0[key_var] != *extreme {
                    continue;
                }
                let row: Result<Vec<Value>, QueryError> = finds
                    .iter()
                    .map(|find| match find {
                        FindTerm::Var(var) => Ok(binding.0[usize::from(var.0)].clone()),
                        FindTerm::Measure(var) => {
                            measure_value(&binding.0[usize::from(var.0)]).map(Value::U64)
                        }
                        FindTerm::Aggregate { over, .. } => Ok(binding.0
                            [usize::from(over.expect("Arg terms carry a variable").0)]
                        .clone()),
                        FindTerm::AggregateMeasure { .. } => {
                            unreachable!("validated: Arg terms and folds never mix")
                        }
                    })
                    .collect();
                rows.insert(Tuple(row?));
            }
        } else {
            let row: Result<Vec<Value>, QueryError> = finds
                .iter()
                .enumerate()
                .map(|(index, find)| match find {
                    FindTerm::Var(var) => Ok(group[0].0[usize::from(var.0)].clone()),
                    FindTerm::Measure(var) => {
                        measure_value(&group[0].0[usize::from(var.0)]).map(Value::U64)
                    }
                    FindTerm::Aggregate { op, over } => fold(*op, *over, group, index),
                    FindTerm::AggregateMeasure { op, over } => {
                        fold_duration(*op, *over, group, index)
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
        AggOp::Pack => unreachable!("Pack heads take the segment path"),
    }
}

/// One measure fold over a group's binding set: measures computed from
/// the definition (a ray raises), then folded exactly as `Sum`/`Min`/
/// `Max` over u64 values — Sum in i128 with the one finalize range check.
fn fold_duration(
    op: AggOp,
    over: VarId,
    group: &[&Tuple],
    find: usize,
) -> Result<Value, QueryError> {
    let measures: Result<Vec<u64>, QueryError> = group
        .iter()
        .map(|binding| measure_value(&binding.0[usize::from(over.0)]))
        .collect();
    let measures = measures?;
    match op {
        AggOp::Sum => {
            let total: i128 = measures.iter().map(|m| i128::from(*m)).sum();
            u64::try_from(total)
                .map(Value::U64)
                .map_err(|_| QueryError::Overflow { find })
        }
        AggOp::Min => Ok(Value::U64(
            measures.iter().copied().min().expect("groups are nonempty"),
        )),
        AggOp::Max => Ok(Value::U64(
            measures.iter().copied().max().expect("groups are nonempty"),
        )),
        _ => unreachable!("validated: measure folds are Sum/Min/Max"),
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
        AggOp::Pack => unreachable!("Pack heads take the segment path"),
    }
}
