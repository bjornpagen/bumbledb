//! The naive model: the dependency-semantics oracle
//! (docs/architecture/60-validation.md § the two oracles).
//!
//! An obviously-correct in-memory implementation of the data model, both
//! judgments, and the full query semantics — nested loops and `BTreeSet`s,
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

use bumbledb::schema::{SchemaDescriptor, Side, StatementDescriptor, ValueType};
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

/// A refused write, identified exactly as the engine's commit errors
/// identify it: a statement the final state fails (the statement id,
/// plus the direction for a containment), or a delta operation naming a
/// closed relation — ground axioms are not data, and the refusal is
/// typed identically on both oracles (verdict parity including the
/// typed identity, the direction-divergence lesson applied at birth).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Violation {
    Functionality {
        statement: StatementId,
    },
    Containment {
        statement: StatementId,
        direction: Direction,
    },
    /// A delete or insert named a closed relation — refused before the
    /// delta, exactly the engine's `Error::ClosedRelationWrite`.
    ClosedRelationWrite {
        relation: RelationId,
    },
}

/// A conditional write's abort cause ([`NaiveDb::apply_from`]): the
/// witness compare failed, or the final state fails a statement — the
/// model twin of the engine's `GenerationMoved` / commit violations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConditionalAbort {
    /// The witnessed generation is no longer current (payload: the two
    /// generations, exactly the engine's error payload).
    Moved { witnessed: u64, current: u64 },
    /// The witness held; the judgment aborted.
    Violation(Violation),
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
        self.apply(delta).map_err(ConditionalAbort::Violation)
    }

    /// Applies a write delta: remove the deletes, insert the inserts, then
    /// judge **every statement over the full final state**. Any violation
    /// returns without applying — the caller compares verdict and violator
    /// against the engine's commit result.
    ///
    /// # Errors
    ///
    /// [`Violation::ClosedRelationWrite`] for the first delta operation
    /// (deletes, then inserts — the replay order) naming a closed
    /// relation, refused before anything applies; otherwise the first
    /// violated statement, in the engine's phase order (functionality
    /// during inserts, then containment source side, then containment
    /// target side).
    pub fn apply(&mut self, delta: &Delta) -> Result<(), Violation> {
        for (relation, _) in delta.deletes.iter().chain(&delta.inserts) {
            if self.extensions[relation.0 as usize].is_some() {
                return Err(Violation::ClosedRelationWrite {
                    relation: *relation,
                });
            }
        }
        let mut next = self.relations.clone();
        for (rel, fact) in &delta.deletes {
            next[rel.0 as usize].remove(&Tuple(fact.clone()));
        }
        // The facts this delta genuinely establishes (absent before): the
        // set that separates the two containment directions, exactly as
        // the engine's no-op-insert rule does.
        let mut inserted: Vec<BTreeSet<Tuple>> = vec![BTreeSet::new(); next.len()];
        for (rel, fact) in &delta.inserts {
            let tuple = Tuple(fact.clone());
            if !self.relations[rel.0 as usize].contains(&tuple) {
                inserted[rel.0 as usize].insert(tuple.clone());
            }
            next[rel.0 as usize].insert(tuple);
        }
        if let Some(violation) = self.judge(&next, &inserted) {
            return Err(violation);
        }
        // State-changing commits only advance the generation — a no-op
        // delta (deletes of absent facts, re-inserts of present ones)
        // leaves it alone, exactly as the engine's tx id does.
        if next != self.relations {
            self.generation += 1;
        }
        self.relations = next;
        Ok(())
    }

    /// Judges every statement against a candidate final state, mirroring
    /// the engine's phase order at statement granularity (so the
    /// differential runner can compare violators, not just verdicts):
    /// functionality per inserted fact, then containment source-side per
    /// inserted fact, then containment target-side over surviving facts.
    /// A **closed source** (domain quantification) needs no case of its
    /// own: its "surviving facts" are the seeded extension tuples, φ is
    /// the same [`satisfies_selection`] value comparison, and the
    /// ordinary set-containment judgment runs unchanged against the
    /// mutable target — the A-side tuples ARE φ over the extension.
    fn judge(&self, state: &[BTreeSet<Tuple>], inserted: &[BTreeSet<Tuple>]) -> Option<Violation> {
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
                        return Some(Violation::Functionality {
                            statement: statement_id(sid),
                        });
                    }
                }
            }
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
                        return Some(Violation::Containment {
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
                    continue; // an inserted source was pass-two work
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
                    return Some(Violation::Containment {
                        statement: statement_id(sid),
                        direction: Direction::TargetRequired,
                    });
                }
            }
        }
        None
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

/// Does the fact satisfy a side's σ — plain value equality per selected
/// field? σ literals *are* decoded values (the one shared [`Value`] sum),
/// so the comparison is structural, no conversion anywhere.
fn satisfies_selection(fact: &Tuple, selection: &[(bumbledb::FieldId, Value)]) -> bool {
    selection
        .iter()
        .all(|(field, literal)| fact.0[field.0 as usize] == *literal)
}

fn statement_id(index: usize) -> StatementId {
    StatementId(u16::try_from(index).expect("statement count fits u16"))
}
