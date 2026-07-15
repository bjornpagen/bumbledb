//! The naive model: the dependency-semantics oracle
//! (docs/architecture/60-validation.md § the two oracles).
//!
//! An obviously-correct in-memory implementation of the data model, both
//! judgments, and the full query semantics — programs included: the
//! naive stratified fixpoint ([`NaiveDb::program`], the shipping
//! law's naive oracle —
//! `docs/architecture/60-validation.md` § the two oracles) — nested
//! loops and `BTreeSet`s,
//! zero cleverness. It shares the engine's *types* (`bumbledb::ir`,
//! `bumbledb::schema`) and none of its *algorithms*: a shared bug would be
//! an invisible bug, so everything here is re-derived from the semantics
//! chapters (`docs/architecture/30-dependencies.md`, `20-query-ir.md`) by
//! brute force. The model assumes nothing the engine enforces — containment
//! collects and merges every matching target segment rather than trusting
//! the target's own key to keep them disjoint.
//!
//! The comparison runner lives beside this module (`crate::differential`
//! — it drives the engine, which nothing under `naive/` may import);
//! query evaluation in [`query`].

pub mod query;
mod tuple;

#[cfg(test)]
mod tests;

pub use query::ParamValue;
pub use tuple::Tuple;

use std::collections::BTreeSet;

use bumbledb::schema::{RankChain, SchemaDescriptor, Side, StatementDescriptor, ValueType};
use bumbledb::{Direction, RelationId, StatementId, Value};

use tuple::{endpoints, overlaps};

/// The in-memory reference database: one set of decoded value vectors per
/// relation. Facts are `Vec<Value>` in field declaration order — the one
/// blessed shared representation (`bumbledb::ir::Value`). A **closed**
/// relation's facts sit in the sealed field space (the synthetic id at
/// position 0, then the declared columns), seeded from the descriptor
/// extension at construction — the model may materialize the axioms (it
/// is a model, not the engine), but it never compiles them: closed
/// judgments are σ over the extension rows by value comparison, and the
/// independence law keeps the engine's compiled member sets out of here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NaiveDb {
    /// The materialized statement list; [`StatementId`] indexes it
    /// (fresh auto-keys first, then the closed auto-keys, then declared
    /// statements — the same rule the engine pins in its fingerprint).
    statements: Vec<StatementDescriptor>,
    /// Per relation, per **sealed** field position, the value type (a
    /// closed relation's list opens with the synthetic `U64` id).
    field_types: Vec<Vec<ValueType>>,
    /// A closed relation's extension as decoded tuples in the sealed
    /// field space (`[U64(row id), declared values...]`), in declaration
    /// order; `None` = ordinary. The closed-target membership judgment
    /// reads THIS list — the σ-over-extension definition, never the
    /// engine's compiled word set.
    extensions: Vec<Option<Vec<Tuple>>>,
    relations: Vec<BTreeSet<Tuple>>,
    /// The state-changing generation: bumped iff an applied delta
    /// changed committed state — never by a no-op — mirroring the
    /// engine's storage tx id (the number the image cache keys on).
    generation: u64,
}

/// One write delta: facts to remove and facts to insert, as decoded value
/// vectors. Set arithmetic — a delete of an absent fact and an insert of a
/// present fact are no-ops, exactly as on the engine.
#[derive(Debug, Clone, Default)]
pub struct Delta {
    pub deletes: Vec<(RelationId, Vec<Value>)>,
    pub inserts: Vec<(RelationId, Vec<Value>)>,
}

/// One citation of a refused write, identified exactly as the engine's
/// commit errors identify it: a statement the final state fails (the
/// statement id, plus the direction for a containment), or a delta
/// operation naming a closed relation — ground axioms are not data, and
/// the refusal is typed identically on both oracles (verdict parity
/// including the typed identity, the direction-divergence lesson
/// applied at birth). A rejection is the COMPLETE `Vec<Violation>` —
/// every violated statement, once, in citation order (statement id
/// ascending, source before target within one statement) — the same
/// total object as the engine's sealed `Violations`.
///
/// The statement phase can mix containment, cardinality, and order
/// citations in one rejection, so [`sealed`] sorts by the explicit
/// citation key ([`Violation::citation`]) — the engine's own sort key —
/// never the derived variant order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Violation {
    Functionality {
        statement: StatementId,
    },
    Containment {
        statement: StatementId,
        direction: Direction,
    },
    /// A cardinality window failed: some ψ-selected parent's child-group
    /// count falls outside the window
    /// (`lean/Bumbledb/Cardinality.lean: CardinalityWindow`).
    Cardinality {
        statement: StatementId,
    },
    /// An order mark failed: some group's positions are not exactly
    /// `1..k`, or — ranked — a smaller rank sits later
    /// (`lean/Bumbledb/Order.lean: OrderMark` / `RankedOrderMark`).
    Order {
        statement: StatementId,
    },
    /// A delete or insert named a closed relation — refused before the
    /// delta, exactly the engine's `Error::ClosedRelationWrite`.
    ClosedRelationWrite {
        relation: RelationId,
    },
}

impl Violation {
    /// The engine's citation key (`bumbledb::Violation`'s sort and dedup
    /// key, mirrored): statement id, then direction rank — none (0)
    /// before source (1) before target (2). `ClosedRelationWrite` is
    /// refused before any judgment and never sorts beside statement
    /// citations; its key only has to be total.
    fn citation(self) -> (u16, u8, u32) {
        match self {
            Self::Functionality { statement }
            | Self::Cardinality { statement }
            | Self::Order { statement } => (statement.0, 0, 0),
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
        }
    }
}

/// A conditional write's abort cause ([`NaiveDb::apply_from`]): the
/// witness compare failed, or the final state fails the judgment — the
/// model twin of the engine's `GenerationMoved` / `CommitRejected`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConditionalAbort {
    /// The witnessed generation is no longer current (payload: the two
    /// generations, exactly the engine's error payload).
    Moved { witnessed: u64, current: u64 },
    /// The witness held; the judgment aborted with the complete
    /// violation set.
    Violations(Vec<Violation>),
}

impl NaiveDb {
    /// An empty model over a declared schema. The model consumes the raw
    /// descriptor — public data, no sealed accessors — and re-derives
    /// everything it needs from it: the sealed field space (a closed
    /// relation's fields open with the synthetic `U64` id) and the
    /// extension tuples the closed relations are seeded with — row id =
    /// declaration index, then the declared values, plain [`Value`]s.
    #[must_use]
    pub fn new(schema: &SchemaDescriptor) -> Self {
        let field_types: Vec<Vec<ValueType>> = schema
            .relations
            .iter()
            .map(|relation| {
                let declared = relation.fields.iter().map(|field| field.value_type.clone());
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
        // The committed view of a closed relation IS its extension —
        // seeded once, write-refused forever, so queries and judgments
        // read one consistent state.
        let relations = extensions
            .iter()
            .map(|extension| match extension {
                Some(rows) => rows.iter().cloned().collect(),
                None => BTreeSet::new(),
            })
            .collect();
        Self {
            statements: schema.materialized_statements(),
            field_types,
            extensions,
            relations,
            generation: 0,
        }
    }

    /// The committed facts of one relation.
    #[must_use]
    pub fn relation(&self, rel: RelationId) -> &BTreeSet<Tuple> {
        &self.relations[rel.0 as usize]
    }

    /// The current state-changing generation — the model's witness. The
    /// model hands out the integer because the model *is* the semantics;
    /// the engine's API refuses it and takes the snapshot (evidence,
    /// never a claim — the recorded refusal).
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The generation witness, naively: one integer compare, then
    /// [`NaiveDb::apply`] — the semantics is these two lines, which is
    /// the point (PRD 18).
    ///
    /// # Errors
    ///
    /// [`ConditionalAbort::Moved`] when the witness is stale (the state
    /// is untouched — the compare runs first); otherwise `apply`'s
    /// verdict, wrapped.
    pub fn apply_from(&mut self, witnessed: u64, delta: &Delta) -> Result<(), ConditionalAbort> {
        if witnessed != self.generation {
            return Err(ConditionalAbort::Moved {
                witnessed,
                current: self.generation,
            });
        }
        self.apply(delta).map_err(ConditionalAbort::Violations)
    }

    /// Applies a write delta: remove the deletes, insert the inserts, then
    /// judge the statements over the full final state. Any violation
    /// returns without applying — the caller compares verdict and
    /// citation set against the engine's commit result.
    ///
    /// # Errors
    ///
    /// The rejection IS [`NaiveDb::violations`]' complete set — one
    /// derivation, the same total object the engine's `CommitRejected`
    /// carries. Nonempty by construction.
    pub fn apply(&mut self, delta: &Delta) -> Result<(), Vec<Violation>> {
        let violations = self.violations(delta);
        if !violations.is_empty() {
            return Err(violations);
        }
        let (next, _) = self.staged(delta);
        // State-changing commits only advance the generation — a no-op
        // delta (deletes of absent facts, re-inserts of present ones)
        // leaves it alone, exactly as the engine's tx id does.
        if next != self.relations {
            self.generation += 1;
        }
        self.relations = next;
        Ok(())
    }

    /// The COMPLETE violation set of one delta against the committed
    /// state — [`NaiveDb::apply`]'s rejection is exactly this list, one
    /// derivation: every violated statement, cited once (per direction
    /// for a containment), sorted ascending (materialized statement
    /// order; source before target within one statement), deduplicated.
    /// Preemption mirrors the phase structure the engine pins
    /// (`docs/architecture/30-dependencies.md` § judged on final
    /// states): a delta op naming a closed relation is refused before
    /// any judgment (the singleton set), and key (functionality)
    /// violations preempt the containment judgment — the containment
    /// probes are defined over the keyed final state.
    ///
    /// The `ops` fuzz oracle (the crucible packet (git ecec1dc3)) compares
    /// this set against the engine's sealed `Violations` by STRICT
    /// EQUALITY, order included.
    #[must_use]
    pub fn violations(&self, delta: &Delta) -> Vec<Violation> {
        for (relation, _) in delta.deletes.iter().chain(&delta.inserts) {
            if self.extensions[relation.0 as usize].is_some() {
                return vec![Violation::ClosedRelationWrite {
                    relation: *relation,
                }];
            }
        }
        let (next, inserted) = self.staged(delta);
        self.judge(&next, &inserted)
    }

    /// The delta's candidate final state beside the facts it genuinely
    /// establishes (absent before) — the set that separates the two
    /// containment directions, exactly as the engine's no-op-insert rule
    /// does. Pure staging; nothing is applied.
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

    /// Judges every statement against a candidate final state, mirroring
    /// the engine's phase structure at statement granularity (so the
    /// differential runners can compare complete citation sets, not just
    /// verdicts): functionality per inserted fact — and if any key
    /// statement fails, that IS the rejection (the containment judgment
    /// is defined over the keyed final state, so the engine never
    /// reaches it) — otherwise containment source-side per inserted
    /// fact, then containment target-side over pre-existing surviving
    /// facts. Returns the COMPLETE violation list, sorted ascending
    /// (materialized statement order; source before target within one
    /// statement) and deduplicated — the whole list is `apply`'s
    /// rejection and [`NaiveDb::violations`]' value.
    /// A **closed source** (domain quantification) needs no case of its
    /// own: its "surviving facts" are the seeded extension tuples, φ is
    /// the same [`satisfies_selection`] value comparison, and the
    /// ordinary set-containment judgment runs unchanged against the
    /// mutable target — the A-side tuples ARE φ over the extension.
    fn judge(&self, state: &[BTreeSet<Tuple>], inserted: &[BTreeSet<Tuple>]) -> Vec<Violation> {
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
            // Key violations preempt the containment judgment — the
            // rejection is the complete set of violated key statements.
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
                    // An inserted source was pass-two work — the sides
                    // partition the final state's sources, so one
                    // statement is never convicted twice through one
                    // fact (the engine's target scan skips inserted
                    // survivors identically).
                    continue;
                }
                // The target side judges what the delta BROKE: an
                // instance that held before and fails after. This is the
                // model twin of the delta-restricted judgment
                // (`30-dependencies.md` § enforcement: per
                // deleted-and-not-reestablished target tuple) — for
                // ordinary theories it equals the plain full-state check
                // by the clean-prestate induction, and for a closed
                // SOURCE (domain quantification) it is the recorded
                // semantics: the empty store violates the statement
                // until the targets land, and commits that never touch
                // the target cannot observe that (the offline sweeper's
                // division of authority).
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
        // The extension forms join the statement phase whole
        // (`lean/Bumbledb/Txn.lean` — the statement-phase violation set
        // carries containment, cardinality, and order citations): the
        // model judges every parent and every group of the FINAL state —
        // the full judgment the engine's delta-restricted checks are
        // provably equal to over a clean pre-state
        // (`lean/Bumbledb/Txn/DeltaRestriction.lean:
        // delta_restricted_commit_sound`).
        for (sid, statement) in self.statements.iter().enumerate() {
            match statement {
                StatementDescriptor::Cardinality {
                    source,
                    lo,
                    hi,
                    target,
                } => {
                    if self.window_violated(state, source, *lo, *hi, target) {
                        found.push(Violation::Cardinality {
                            statement: statement_id(sid),
                        });
                    }
                }
                StatementDescriptor::Order {
                    relation,
                    position,
                    grouping,
                    ranking,
                } => {
                    if self.order_violated(state, *relation, *position, grouping, ranking.as_ref())
                    {
                        found.push(Violation::Order {
                            statement: statement_id(sid),
                        });
                    }
                }
                StatementDescriptor::Functionality { .. }
                | StatementDescriptor::Containment { .. } => {}
            }
        }
        sealed(found)
    }

    /// Does some ψ-selected parent's child-group count fall outside the
    /// window? Per parent, the children are the φ-selected source facts
    /// whose projected tuple equals the parent's — O(parents × children)
    /// value comparison is the point
    /// (`lean/Bumbledb/Cardinality.lean: CardinalityWindow`).
    fn window_violated(
        &self,
        state: &[BTreeSet<Tuple>],
        source: &Side,
        lo: u64,
        hi: Option<u64>,
        target: &Side,
    ) -> bool {
        self.target_facts(state, target).any(|parent| {
            if !satisfies_selection(parent, &target.selection) {
                return false;
            }
            let count = self
                .target_facts(state, source)
                .filter(|child| {
                    satisfies_selection(child, &source.selection)
                        && source
                            .projection
                            .iter()
                            .zip(target.projection.iter())
                            .all(|(s, t)| child.0[s.0 as usize] == parent.0[t.0 as usize])
                })
                .count();
            let count = u64::try_from(count).expect("fact count fits u64");
            count < lo || hi.is_some_and(|hi| count > hi)
        })
    }

    /// Does some group break the ordinal discipline — positions not
    /// exactly `1..k` — or, ranked, the rank monotonicity
    /// (`lean/Bumbledb/Order.lean: OrderMark` / `RankedOrderMark`)?
    fn order_violated(
        &self,
        state: &[BTreeSet<Tuple>],
        relation: RelationId,
        position: bumbledb::FieldId,
        grouping: &[bumbledb::FieldId],
        ranking: Option<&RankChain>,
    ) -> bool {
        let mut groups: std::collections::BTreeMap<Tuple, Vec<&Tuple>> =
            std::collections::BTreeMap::new();
        for fact in &state[relation.0 as usize] {
            let key = Tuple(
                grouping
                    .iter()
                    .map(|field| fact.0[field.0 as usize].clone())
                    .collect(),
            );
            groups.entry(key).or_default().push(fact);
        }
        for members in groups.values() {
            // (position ordinal, rank) per member, position-sorted — the
            // ordinal reading is total (`lean/Bumbledb/Order.lean:
            // Value.ordinal`: a u64 reads its numeral, junk reads 0).
            let mut ordered: Vec<(u64, Option<u64>)> = members
                .iter()
                .map(|fact| {
                    let ordinal = match &fact.0[position.0 as usize] {
                        Value::U64(v) => *v,
                        _ => 0,
                    };
                    let rank = ranking.and_then(|chain| self.rank_of(state, chain, fact));
                    (ordinal, rank)
                })
                .collect();
            ordered.sort_unstable_by_key(|(ordinal, _)| *ordinal);
            let contiguous = ordered.iter().enumerate().all(|(index, (ordinal, _))| {
                *ordinal == u64::try_from(index).expect("fact count fits u64") + 1
            });
            if !contiguous {
                return true;
            }
            // Rank monotonicity in position order, among rank-carrying
            // members (a hop miss means no rank, imposing nothing —
            // `lean/Bumbledb/Order.lean: RankChain.rankOf` is
            // relational).
            let mut prev: Option<u64> = None;
            for (_, rank) in ordered {
                let Some(rank) = rank else { continue };
                if prev.is_some_and(|prev| rank < prev) {
                    return true;
                }
                prev = Some(rank);
            }
        }
        false
    }

    /// One fact's rank under a `by` chain, relationally: per hop, find
    /// the (key-backed, hence unique) fact of the hop relation carrying
    /// the running value at the key field, read the payload; the final
    /// value's ordinal is the rank. `None` when a hop misses — the fact
    /// has no rank (`lean/Bumbledb/Order.lean: RankChain.rankOf`).
    fn rank_of(&self, state: &[BTreeSet<Tuple>], chain: &RankChain, fact: &Tuple) -> Option<u64> {
        let mut running: Value = fact.0[chain.link.0 as usize].clone();
        for hop in &chain.hops {
            let candidate = match &self.extensions[hop.relation.0 as usize] {
                Some(rows) => rows.iter().find(|row| row.0[hop.key.0 as usize] == running),
                None => state[hop.relation.0 as usize]
                    .iter()
                    .find(|row| row.0[hop.key.0 as usize] == running),
            }?;
            running = candidate.0[hop.read.0 as usize].clone();
        }
        match running {
            Value::U64(v) => Some(v),
            _ => Some(0),
        }
    }

    /// Does inserting `fact` leave two distinct facts agreeing on the
    /// projection? Scalar positions by value equality; the interval
    /// position (if any) by the pointwise overlap test
    /// `a.start < b.end && b.start < a.end` — O(n²) is the point.
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

    /// The target-side candidate facts of a containment: the committed
    /// state for an ordinary target — or, for a **closed** target, the
    /// extension rows themselves, from the σ-over-extension *definition*
    /// (`docs/architecture/30-dependencies.md` § IND into a closed
    /// target): ψ is applied to the ground axioms by plain value
    /// comparison on the shared [`Value`] sum. Deliberately NOT the
    /// engine's compiled member set — the model must not share the
    /// engine's representation (the independence law).
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

    /// Is one source fact's projected tuple contained in the target side?
    /// Scalar positions scan for an equal projected tuple among ψ-passing
    /// target facts (the extension rows when the target is closed —
    /// [`NaiveDb::target_facts`]); an interval position collects ALL
    /// matching target segments, sorts and merges them, and tests the
    /// source interval's containment in the merged union — never assuming
    /// the target keeps its own segments disjoint.
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

    /// Whether a **sealed** field position is interval-typed (a closed
    /// relation's position 0 is the synthetic `U64` id).
    fn is_interval(&self, relation: RelationId, field: bumbledb::FieldId) -> bool {
        matches!(
            self.field_types[relation.0 as usize][field.0 as usize],
            ValueType::Interval { .. }
        )
    }

    /// The sealed field-type table, for the query evaluator's membership
    /// typing rule ([`query`]).
    pub(crate) fn field_type(&self, relation: usize, field: usize) -> &ValueType {
        &self.field_types[relation][field]
    }
}

/// Does the fact satisfy a side's σ — per selected field, membership in
/// the binding's literal set (a singleton set is plain equality —
/// `lean/Bumbledb/Schema.lean: Selection.singleton_satisfies_iff`)? σ
/// literals *are* decoded values (the one shared [`Value`] sum), so the
/// comparison is structural, no conversion anywhere.
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

fn statement_id(index: usize) -> StatementId {
    StatementId(u16::try_from(index).expect("statement count fits u16"))
}

/// Seals a raw citation list: sorted by the explicit citation key
/// (materialized statement order, source before target within one
/// statement — [`Violation::citation`], the engine's own sort key) and
/// deduplicated. The model twin of the engine's `Violations::seal`.
fn sealed(mut found: Vec<Violation>) -> Vec<Violation> {
    found.sort_unstable_by_key(|violation| violation.citation());
    found.dedup();
    found
}
