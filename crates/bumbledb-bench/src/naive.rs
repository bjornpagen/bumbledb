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
//! The comparison runner lives in [`differential`]; query evaluation in
//! [`query`]. Integration point: the verify command (PRDs 22–24) will feed
//! [`differential::run`] the corpus op streams — until that wiring lands,
//! the runner's only callers are this module's own tests.

pub mod differential;
pub mod query;
mod tuple;

#[cfg(test)]
mod tests;

pub use query::ParamValue;
pub use tuple::Tuple;

use std::collections::BTreeSet;

use bumbledb::schema::{LiteralValue, SchemaDescriptor, Side, StatementDescriptor, ValueType};
use bumbledb::{Direction, RelationId, StatementId, Value};

use tuple::{endpoints, overlaps};

/// The in-memory reference database: one set of decoded value vectors per
/// relation. Facts are `Vec<Value>` in field declaration order — the one
/// blessed shared representation (`bumbledb::ir::Value`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NaiveDb {
    schema: SchemaDescriptor,
    /// The materialized statement list; [`StatementId`] indexes it
    /// (serial auto-keys first, then declared statements — the same rule
    /// the engine pins in its fingerprint).
    statements: Vec<StatementDescriptor>,
    relations: Vec<BTreeSet<Tuple>>,
}

/// One write delta: facts to remove and facts to insert, as decoded value
/// vectors. Set arithmetic — a delete of an absent fact and an insert of a
/// present fact are no-ops, exactly as on the engine.
#[derive(Debug, Clone, Default)]
pub struct Delta {
    pub deletes: Vec<(RelationId, Vec<Value>)>,
    pub inserts: Vec<(RelationId, Vec<Value>)>,
}

/// A statement the final state fails, identified exactly as the engine's
/// commit errors identify it: the statement id, plus the direction for a
/// containment.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Violation {
    Functionality {
        statement: StatementId,
    },
    Containment {
        statement: StatementId,
        direction: Direction,
    },
}

impl NaiveDb {
    /// An empty model over a declared schema. The model consumes the raw
    /// descriptor — public data, no sealed accessors — and re-derives
    /// everything it needs from it.
    #[must_use]
    pub fn new(schema: &SchemaDescriptor) -> Self {
        Self {
            statements: schema.materialized_statements(),
            relations: vec![BTreeSet::new(); schema.relations.len()],
            schema: schema.clone(),
        }
    }

    /// The committed facts of one relation.
    #[must_use]
    pub fn relation(&self, rel: RelationId) -> &BTreeSet<Tuple> {
        &self.relations[rel.0 as usize]
    }

    /// Applies a write delta: remove the deletes, insert the inserts, then
    /// judge **every statement over the full final state**. Any violation
    /// returns without applying — the caller compares verdict and violator
    /// against the engine's commit result.
    ///
    /// # Errors
    ///
    /// The first violated statement, in the engine's phase order
    /// (functionality during inserts, then containment source side, then
    /// containment target side).
    pub fn apply(&mut self, delta: &Delta) -> Result<(), Violation> {
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
        self.relations = next;
        Ok(())
    }

    /// Judges every statement against a candidate final state, mirroring
    /// the engine's phase order at statement granularity (so the
    /// differential runner can compare violators, not just verdicts):
    /// functionality per inserted fact, then containment source-side per
    /// inserted fact, then containment target-side over surviving facts.
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
                if satisfies_selection(fact, &source.selection)
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

    /// Is one source fact's projected tuple contained in the target side?
    /// Scalar positions scan for an equal projected tuple among ψ-passing
    /// target facts; an interval position collects ALL matching target
    /// segments, sorts and merges them, and tests the source interval's
    /// containment in the merged union — never assuming the target keeps
    /// its own segments disjoint.
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
            None => state[target.relation.0 as usize].iter().any(|candidate| {
                satisfies_selection(candidate, &target.selection)
                    && target
                        .projection
                        .iter()
                        .zip(&projected)
                        .all(|(field, value)| &candidate.0[field.0 as usize] == *value)
            }),
            Some(index) => {
                let mut segments: Vec<(i128, i128)> = Vec::new();
                for candidate in &state[target.relation.0 as usize] {
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
        matches!(
            self.schema.relations[relation.0 as usize].fields[field.0 as usize].value_type,
            ValueType::Interval { .. }
        )
    }
}

/// Does the fact satisfy a side's σ — plain value equality per selected
/// field?
fn satisfies_selection(fact: &Tuple, selection: &[(bumbledb::FieldId, LiteralValue)]) -> bool {
    selection
        .iter()
        .all(|(field, literal)| fact.0[field.0 as usize] == literal_value(literal))
}

fn statement_id(index: usize) -> StatementId {
    StatementId(u16::try_from(index).expect("statement count fits u16"))
}

/// A selection literal as a plain [`Value`] — the model compares decoded
/// values, so σ literals convert once and compare structurally.
fn literal_value(literal: &LiteralValue) -> Value {
    match literal {
        LiteralValue::Bool(v) => Value::Bool(*v),
        LiteralValue::U64(v) => Value::U64(*v),
        LiteralValue::I64(v) => Value::I64(*v),
        LiteralValue::Enum(v) => Value::Enum(*v),
        LiteralValue::IntervalU64(start, end) => Value::IntervalU64(*start, *end),
        LiteralValue::IntervalI64(start, end) => Value::IntervalI64(*start, *end),
        LiteralValue::String(bytes) => Value::String(bytes.clone()),
        LiteralValue::Bytes(bytes) => Value::Bytes(bytes.clone()),
    }
}
