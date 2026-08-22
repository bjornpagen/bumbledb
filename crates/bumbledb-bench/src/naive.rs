pub mod query;
mod tuple;

#[cfg(test)]
mod tests;

pub use query::ParamValue;
pub use tuple::Tuple;

use std::collections::BTreeSet;

use bumbledb::schema::{Bound, SchemaDescriptor, Side, StatementDescriptor, ValueType, Weight};
use bumbledb::{Direction, RelationId, StatementId, Value};

use tuple::{endpoints, overlaps};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NaiveDb {
    statements: Vec<StatementDescriptor>,

    field_types: Vec<Vec<ValueType>>,

    extensions: Vec<Option<Vec<Tuple>>>,
    relations: Vec<BTreeSet<Tuple>>,

    generation: u64,

    dict: Vec<Box<str>>,
}

#[derive(Debug, Clone, Default)]
pub struct Delta {
    pub deletes: Vec<(RelationId, Vec<Value>)>,
    pub inserts: Vec<(RelationId, Vec<Value>)>,
}

/// One citation of a refused write, identified exactly as the engine's commit
/// errors identify it: a statement the final state fails (the statement id,
/// plus the direction for a containment), or a delta operation naming a closed
/// relation — ground axioms are not data, and the refusal is typed identically
/// on both oracles (verdict parity including the typed identity, the
/// direction-divergence lesson applied at birth). A rejection is the COMPLETE
/// `Vec<Violation>` — every violated statement, once, in citation order
/// (statement id ascending, source before target within one statement) — the
/// same
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Violation {
    Functionality {
        statement: StatementId,
    },
    Containment {
        statement: StatementId,
        direction: Direction,
    },

    /// (`lean/Bumbledb/Capacity.lean: CapacityLaw`). The twin carries
    /// the WITNESSED measure (ruled 2026-07-24, C14 — measure parity in
    Capacity {
        statement: StatementId,

        measure: u128,
    },
    /// A delete or insert named a closed relation — refused before the
    ClosedRelationWrite {
        relation: RelationId,
    },

    CapacityRayMeasure {
        statement: StatementId,
    },
}

impl Violation {
    /// before source (1) before target (2). `ClosedRelationWrite` is
    /// refused before any judgment and never sorts beside statement
    fn citation(self) -> (u16, u8, u32) {
        match self {
            Self::Functionality { statement } | Self::Capacity { statement, .. } => {
                (statement.0, 0, 0)
            }
            Self::Containment {
                statement,
                direction,
            } => (
                statement.0,
                match direction {
                    Direction::SourceUnsatisfied => 1,
                    Direction::TargetRequired => 2,
                },
                0,
            ),
            Self::ClosedRelationWrite { relation } => (u16::MAX, u8::MAX, relation.0),
            // A refusal, never sorted beside statement citations (the
            Self::CapacityRayMeasure { statement } => (statement.0, 3, 0),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionalAbort {
    Moved { witnessed: u64, current: u64 },

    Violations(Vec<Violation>),
}

impl NaiveDb {
    #[must_use]
    pub fn new(schema: &SchemaDescriptor) -> Self {
        let field_types: Vec<Vec<ValueType>> = schema
            .relations
            .iter()
            .map(|relation| {
                let declared = relation.fields.iter().map(|field| field.value_type);
                if relation.extension.is_some() {
                    std::iter::once(ValueType::U64).chain(declared).collect()
                } else {
                    declared.collect()
                }
            })
            .collect();
        let extensions: Vec<Option<Vec<Tuple>>> = schema
            .relations
            .iter()
            .map(|relation| {
                relation.extension.as_ref().map(|rows| {
                    rows.iter()
                        .enumerate()
                        .map(|(row, axiom)| {
                            let mut fact = vec![Value::U64(row as u64)];
                            fact.extend(axiom.values.iter().cloned());
                            Tuple(fact)
                        })
                        .collect()
                })
            })
            .collect();

        // seeded once, write-refused forever, so queries and judgments

        let relations = extensions
            .iter()
            .map(|extension| match extension {
                Some(rows) => rows.iter().cloned().collect(),
                None => BTreeSet::new(),
            })
            .collect();

        let mut dict: Vec<Box<str>> = Vec::new();
        for extension in extensions.iter().flatten() {
            for row in extension {
                for value in &row.0 {
                    if let Value::String(raw) = value
                        && !dict.contains(raw)
                    {
                        dict.push(raw.clone());
                    }
                }
            }
        }
        Self {
            statements: schema.materialized_statements(),
            field_types,
            extensions,
            relations,
            generation: 0,
            dict,
        }
    }

    /// refuses those before any final state is formed).

    /// # Panics
    pub fn load_candidate(&mut self, facts: &[(RelationId, Vec<Value>)]) {
        for (rel, fact) in facts {
            assert!(
                self.extensions[rel.0 as usize].is_none(),
                "complete admission stages ordinary facts only"
            );
            for value in fact {
                if let Value::String(raw) = value
                    && !self.dict.contains(raw)
                {
                    self.dict.push(raw.clone());
                }
            }
            self.relations[rel.0 as usize].insert(Tuple(fact.clone()));
        }
    }

    /// incremental `judge` cannot see without a holds-before premise.
    #[must_use]
    pub fn judge_complete(&self) -> Vec<Violation> {
        let state = &self.relations;
        let minted = &self.dict;
        let mut found: Vec<Violation> = Vec::new();
        for (sid, statement) in self.statements.iter().enumerate() {
            let StatementDescriptor::Functionality {
                relation,
                projection,
            } = statement
            else {
                continue;
            };
            for fact in &state[relation.0 as usize] {
                if self.functionality_violated(state, *relation, projection, fact) {
                    found.push(Violation::Functionality {
                        statement: statement_id(sid),
                    });
                    break;
                }
            }
        }
        if !found.is_empty() {
            return sealed(found);
        }
        for (sid, statement) in self.statements.iter().enumerate() {
            match statement {
                StatementDescriptor::Containment { source, target } => {
                    for fact in &state[source.relation.0 as usize] {
                        if satisfies_selection(fact, &source.selection)
                            && !self.contained(state, source, target, fact)
                        {
                            found.push(Violation::Containment {
                                statement: statement_id(sid),
                                direction: Direction::SourceUnsatisfied,
                            });
                            break;
                        }
                    }
                }
                StatementDescriptor::Capacity {
                    target,
                    weight,
                    lo,
                    hi,
                    source,
                } => {
                    if let Some(Bound::TargetDuration(field)) = hi
                        && self.target_facts(state, target).any(|parent| {
                            satisfies_selection(parent, &target.selection)
                                && is_ray(&parent.0[field.0 as usize])
                        })
                    {
                        return vec![Violation::CapacityRayMeasure {
                            statement: statement_id(sid),
                        }];
                    }
                    if let Some(measure) =
                        self.capacity_violated(state, minted, target, *weight, *lo, *hi, source)
                    {
                        found.push(Violation::Capacity {
                            statement: statement_id(sid),
                            measure,
                        });
                    }
                }
                StatementDescriptor::Functionality { .. } => {}
            }
        }
        sealed(found)
    }

    #[must_use]
    pub fn relation(&self, rel: RelationId) -> &BTreeSet<Tuple> {
        &self.relations[rel.0 as usize]
    }

    /// never a claim — the recorded refusal).
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// # Errors
    pub fn apply_from(&mut self, witnessed: u64, delta: &Delta) -> Result<(), ConditionalAbort> {
        if witnessed != self.generation {
            return Err(ConditionalAbort::Moved {
                witnessed,
                current: self.generation,
            });
        }
        self.apply(delta).map_err(ConditionalAbort::Violations)
    }

    /// # Errors
    pub fn apply(&mut self, delta: &Delta) -> Result<(), Vec<Violation>> {
        let (next, minted) = self.judged(delta)?;

        if next != self.relations {
            self.generation += 1;
        }
        self.relations = next;

        self.dict.extend(minted);
        Ok(())
    }

    /// order; source before target within one statement), deduplicated.

    /// states): a delta op naming a closed relation is refused before
    #[must_use]
    pub fn violations(&self, delta: &Delta) -> Vec<Violation> {
        self.judged(delta).err().unwrap_or_default()
    }

    #[expect(
        clippy::type_complexity,
        reason = "the pair is this function's two-halves contract: the staged state and its mints"
    )]
    fn judged(
        &self,
        delta: &Delta,
    ) -> Result<(Vec<BTreeSet<Tuple>>, Vec<Box<str>>), Vec<Violation>> {
        for (relation, _) in delta.deletes.iter().chain(&delta.inserts) {
            if self.extensions[relation.0 as usize].is_some() {
                return Err(vec![Violation::ClosedRelationWrite {
                    relation: *relation,
                }]);
            }
        }
        if let Some(refusal) = self.ray_weight_refusal(delta) {
            return Err(vec![refusal]);
        }
        let (next, inserted) = self.staged(delta);
        let minted = self.minted(delta);
        let violations = self.judge(&next, &inserted, &minted);
        if violations.is_empty() {
            Ok((next, minted))
        } else {
            Err(violations)
        }
    }

    /// The plan-phase ray refusal (C17's write-time strengthening of

    /// weight position is a ray refuses the whole commit before a
    fn ray_weight_refusal(&self, delta: &Delta) -> Option<Violation> {
        for (rel, fact) in &delta.inserts {
            let fact = Tuple(fact.clone());
            for (sid, statement) in self.statements.iter().enumerate() {
                let StatementDescriptor::Capacity {
                    weight: Weight::DurationOf(field),
                    source,
                    ..
                } = statement
                else {
                    continue;
                };
                if source.relation == *rel
                    && satisfies_selection(&fact, &source.selection)
                    && is_ray(&fact.0[field.0 as usize])
                {
                    return Some(Violation::CapacityRayMeasure {
                        statement: statement_id(sid),
                    });
                }
            }
        }
        None
    }

    fn minted(&self, delta: &Delta) -> Vec<Box<str>> {
        let mut minted: Vec<Box<str>> = Vec::new();
        for (_, fact) in &delta.inserts {
            for value in fact {
                if let Value::String(raw) = value
                    && !self.dict.contains(raw)
                    && !minted.contains(raw)
                {
                    minted.push(raw.clone());
                }
            }
        }
        minted
    }

    fn staged(&self, delta: &Delta) -> (Vec<BTreeSet<Tuple>>, Vec<BTreeSet<Tuple>>) {
        let mut next = self.relations.clone();
        for (rel, fact) in &delta.deletes {
            next[rel.0 as usize].remove(&Tuple(fact.clone()));
        }
        let mut inserted: Vec<BTreeSet<Tuple>> = vec![BTreeSet::new(); next.len()];
        for (rel, fact) in &delta.inserts {
            let tuple = Tuple(fact.clone());
            if !self.relations[rel.0 as usize].contains(&tuple) {
                inserted[rel.0 as usize].insert(tuple.clone());
            }
            next[rel.0 as usize].insert(tuple);
        }
        (next, inserted)
    }

    /// (materialized statement order; source before target within one
    fn judge(
        &self,
        state: &[BTreeSet<Tuple>],
        inserted: &[BTreeSet<Tuple>],
        minted: &[Box<str>],
    ) -> Vec<Violation> {
        let mut found: Vec<Violation> = Vec::new();
        for (rel, facts) in inserted.iter().enumerate() {
            for fact in facts {
                for (sid, statement) in self.statements.iter().enumerate() {
                    let StatementDescriptor::Functionality {
                        relation,
                        projection,
                    } = statement
                    else {
                        continue;
                    };
                    if relation.0 as usize == rel
                        && self.functionality_violated(state, *relation, projection, fact)
                    {
                        found.push(Violation::Functionality {
                            statement: statement_id(sid),
                        });
                    }
                }
            }
        }
        if !found.is_empty() {
            return sealed(found);
        }
        for (rel, facts) in inserted.iter().enumerate() {
            for fact in facts {
                for (sid, statement) in self.statements.iter().enumerate() {
                    let StatementDescriptor::Containment { source, target } = statement else {
                        continue;
                    };
                    if source.relation.0 as usize == rel
                        && satisfies_selection(fact, &source.selection)
                        && !self.contained(state, source, target, fact)
                    {
                        found.push(Violation::Containment {
                            statement: statement_id(sid),
                            direction: Direction::SourceUnsatisfied,
                        });
                    }
                }
            }
        }
        for (sid, statement) in self.statements.iter().enumerate() {
            let StatementDescriptor::Containment { source, target } = statement else {
                continue;
            };
            for fact in &state[source.relation.0 as usize] {
                if inserted[source.relation.0 as usize].contains(fact) {
                    continue;
                }

                // instance that held before and fails after. This is the

                if satisfies_selection(fact, &source.selection)
                    && self.contained(&self.relations, source, target, fact)
                    && !self.contained(state, source, target, fact)
                {
                    found.push(Violation::Containment {
                        statement: statement_id(sid),
                        direction: Direction::TargetRequired,
                    });
                }
            }
        }

        // (`lean/Bumbledb/Txn.lean` — the statement-phase violation set

        // (`lean/Bumbledb/Txn/DeltaRestriction.lean:

        for (sid, statement) in self.statements.iter().enumerate() {
            match statement {
                StatementDescriptor::Capacity {
                    target,
                    weight,
                    lo,
                    hi,
                    source,
                } => {
                    // every walked parent's own row BEFORE its window

                    if let Some(Bound::TargetDuration(field)) = hi
                        && self.target_facts(state, target).any(|parent| {
                            satisfies_selection(parent, &target.selection)
                                && is_ray(&parent.0[field.0 as usize])
                        })
                    {
                        return vec![Violation::CapacityRayMeasure {
                            statement: statement_id(sid),
                        }];
                    }
                    if let Some(measure) =
                        self.capacity_violated(state, minted, target, *weight, *lo, *hi, source)
                    {
                        found.push(Violation::Capacity {
                            statement: statement_id(sid),
                            measure,
                        });
                    }
                }
                StatementDescriptor::Functionality { .. }
                | StatementDescriptor::Containment { .. } => {}
            }
        }
        sealed(found)
    }

    /// (`lean/Bumbledb/Capacity.lean: CapacityLaw`). Returns the
    #[expect(
        clippy::too_many_arguments,
        reason = "the parameter list IS the capacity statement's descriptor, spelled flat"
    )]
    fn capacity_violated(
        &self,
        state: &[BTreeSet<Tuple>],
        minted: &[Box<str>],
        target: &Side,
        weight: Weight,
        lo: u64,
        hi: Option<Bound>,
        source: &Side,
    ) -> Option<u128> {
        let order = self.determinant_order(target);
        let mut witnessed: Option<(Tuple, u128)> = None;
        for parent in self.target_facts(state, target) {
            if !satisfies_selection(parent, &target.selection) {
                continue;
            }
            let measure: u128 = self
                .target_facts(state, source)
                .filter(|child| {
                    satisfies_selection(child, &source.selection)
                        && source
                            .projection
                            .iter()
                            .zip(target.projection.iter())
                            .all(|(s, t)| child.0[s.0 as usize] == parent.0[t.0 as usize])
                })
                .map(|child| child_weight(weight, child))
                .sum();
            let ceiling = hi.map(|bound| resolve_bound(bound, parent));
            if measure < u128::from(lo) || ceiling.is_some_and(|hi| measure > hi) {
                let key = self.encoded_key(minted, parent, order);
                if witnessed.as_ref().is_none_or(|(least, _)| key < *least) {
                    witnessed = Some((key, measure));
                }
            }
        }
        witnessed.map(|(_, measure)| measure)
    }

    fn determinant_order<'a>(&'a self, target: &'a Side) -> &'a [bumbledb::FieldId] {
        let wanted: BTreeSet<u16> = target.projection.iter().map(|field| field.0).collect();
        self.statements
            .iter()
            .find_map(|statement| match statement {
                StatementDescriptor::Functionality {
                    relation,
                    projection,
                } if *relation == target.relation
                    && projection
                        .iter()
                        .map(|field| field.0)
                        .collect::<BTreeSet<u16>>()
                        == wanted =>
                {
                    Some(projection.as_ref())
                }
                StatementDescriptor::Functionality { .. }
                | StatementDescriptor::Containment { .. }
                | StatementDescriptor::Capacity { .. } => None,
            })
            .unwrap_or(&target.projection)
    }

    fn encoded_key(
        &self,
        minted: &[Box<str>],
        parent: &Tuple,
        order: &[bumbledb::FieldId],
    ) -> Tuple {
        Tuple(
            order
                .iter()
                .map(|field| match &parent.0[field.0 as usize] {
                    Value::String(raw) => Value::U64(self.intern_rank(minted, raw)),
                    other => other.clone(),
                })
                .collect(),
        )
    }

    fn intern_rank(&self, minted: &[Box<str>], raw: &str) -> u64 {
        let rank = self
            .dict
            .iter()
            .position(|entry| entry.as_ref() == raw)
            .or_else(|| {
                minted
                    .iter()
                    .position(|entry| entry.as_ref() == raw)
                    .map(|rank| self.dict.len() + rank)
            })
            .expect("a stored str is interned: committed dictionary or this delta's mints");
        u64::try_from(rank).expect("intern rank fits u64")
    }

    fn functionality_violated(
        &self,
        state: &[BTreeSet<Tuple>],
        relation: RelationId,
        projection: &[bumbledb::FieldId],
        fact: &Tuple,
    ) -> bool {
        let interval = projection
            .iter()
            .position(|field| self.is_interval(relation, *field));
        for other in &state[relation.0 as usize] {
            if other == fact {
                continue;
            }
            let scalars_agree = projection.iter().enumerate().all(|(index, field)| {
                interval == Some(index) || other.0[field.0 as usize] == fact.0[field.0 as usize]
            });
            if !scalars_agree {
                continue;
            }
            match interval {
                None => return true,
                Some(index) => {
                    let field = projection[index].0 as usize;
                    if overlaps(endpoints(&fact.0[field]), endpoints(&other.0[field])) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// engine's compiled member set — the model must not share the
    fn target_facts<'a>(
        &'a self,
        state: &'a [BTreeSet<Tuple>],
        target: &Side,
    ) -> Box<dyn Iterator<Item = &'a Tuple> + 'a> {
        match &self.extensions[target.relation.0 as usize] {
            Some(rows) => Box::new(rows.iter()),
            None => Box::new(state[target.relation.0 as usize].iter()),
        }
    }

    fn contained(
        &self,
        state: &[BTreeSet<Tuple>],
        source: &Side,
        target: &Side,
        fact: &Tuple,
    ) -> bool {
        let interval = source
            .projection
            .iter()
            .position(|field| self.is_interval(source.relation, *field));
        let projected: Vec<&Value> = source
            .projection
            .iter()
            .map(|field| &fact.0[field.0 as usize])
            .collect();
        match interval {
            None => self.target_facts(state, target).any(|candidate| {
                satisfies_selection(candidate, &target.selection)
                    && target
                        .projection
                        .iter()
                        .zip(&projected)
                        .all(|(field, value)| &candidate.0[field.0 as usize] == *value)
            }),
            Some(index) => {
                let mut segments: Vec<(i128, i128)> = Vec::new();
                for candidate in self.target_facts(state, target) {
                    if !satisfies_selection(candidate, &target.selection) {
                        continue;
                    }
                    let scalars_match =
                        target
                            .projection
                            .iter()
                            .enumerate()
                            .all(|(position, field)| {
                                position == index
                                    || candidate.0[field.0 as usize] == *projected[position]
                            });
                    if scalars_match {
                        segments.push(endpoints(&candidate.0[target.projection[index].0 as usize]));
                    }
                }
                segments.sort_unstable();
                let mut merged: Vec<(i128, i128)> = Vec::new();
                for segment in segments {
                    match merged.last_mut() {
                        Some(last) if segment.0 <= last.1 => last.1 = last.1.max(segment.1),
                        _ => merged.push(segment),
                    }
                }
                let (start, end) = endpoints(projected[index]);
                merged.iter().any(|(covered_start, covered_end)| {
                    *covered_start <= start && end <= *covered_end
                })
            }
        }
    }

    fn is_interval(&self, relation: RelationId, field: bumbledb::FieldId) -> bool {
        self.field_types[relation.0 as usize][field.0 as usize].is_interval()
    }

    pub(crate) fn field_type(&self, relation: usize, field: usize) -> &ValueType {
        &self.field_types[relation][field]
    }
}

/// Does the fact satisfy a side's σ — per selected field, membership in the
/// binding's literal set (a singleton set is plain equality —
/// `lean/Bumbledb/Schema.lean: Selection.singleton_satisfies_iff`)?
fn satisfies_selection(
    fact: &Tuple,
    selection: &[(bumbledb::FieldId, bumbledb::schema::LiteralSet)],
) -> bool {
    selection.iter().all(|(field, literals)| {
        literals
            .literals()
            .iter()
            .any(|literal| fact.0[field.0 as usize] == *literal)
    })
}

/// One source fact's measure under a capacity weight: 1 for the unit (count)
/// instance, the u64 field value for `[field]`, and the interval measure `end −
/// start` for `[Duration(field)]` — validation guarantees the encodings (a
/// signed weight field is gate-refused; polarity), so a mismatch is a fixture
/// bug, panicked not tolerated. A ray has no finite measure (C10 — the R6
/// precedent): the model refuses it the way the engine's typed commit refusal
/// does, loudly.
fn child_weight(weight: Weight, child: &Tuple) -> u128 {
    match weight {
        Weight::Unit => 1,
        Weight::Field(field) => match &child.0[field.0 as usize] {
            Value::U64(w) => u128::from(*w),
            other => panic!("a capacity weight field must be u64-encoded, got {other:?}"),
        },
        Weight::DurationOf(field) => duration_measure(&child.0[field.0 as usize]),
    }
}

fn resolve_bound(bound: Bound, parent: &Tuple) -> u128 {
    match bound {
        Bound::Lit(n) => u128::from(n),
        Bound::TargetField(field) => match &parent.0[field.0 as usize] {
            Value::U64(n) => u128::from(*n),
            other => panic!("a dependent capacity bound must be u64-encoded, got {other:?}"),
        },
        Bound::TargetDuration(field) => duration_measure(&parent.0[field.0 as usize]),
    }
}

fn is_ray(value: &Value) -> bool {
    match value {
        Value::IntervalU64(interval) => interval.is_ray(),
        Value::IntervalI64(interval) => interval.is_ray(),
        other => panic!("a Duration weight/bound must be interval-encoded, got {other:?}"),
    }
}

/// A ray has no finite measure — the typed refusal is
/// [`Violation::CapacityRayMeasure`], raised BEFORE any measure folds (weight
/// rays at the plan phase, bound rays ahead of the statement's walk), so a ray
/// reaching this fold is a model bug, panicked not tolerated.
fn duration_measure(value: &Value) -> u128 {
    assert!(!is_ray(value), "a ray has no finite measure (C10)");
    let (start, end) = endpoints(value);
    u128::try_from(end - start).expect("interval measure is non-negative")
}

fn statement_id(index: usize) -> StatementId {
    StatementId(u16::try_from(index).expect("statement count fits u16"))
}

/// Seals a raw citation list: sorted by the explicit citation key (materialized
/// statement order, source before target within one statement —
/// [`Violation::citation`], the engine's own sort key) and deduplicated.
fn sealed(mut found: Vec<Violation>) -> Vec<Violation> {
    found.sort_unstable_by_key(|violation| violation.citation());
    found.dedup();
    found
}
