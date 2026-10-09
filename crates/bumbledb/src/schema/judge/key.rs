//! Key judgment: one row per scalar determinant, or disjoint intervals per
//! determinant for a pointwise key.
use std::ops::ControlFlow::{Break, Continue};

use super::grouped::{GroupedMap, ScalarKeyScratch};
use super::{
    DeltaFacts, Facts, Indexed, Judge, PendingViolation, interval_order_words, parse_span_key,
    span_key,
};
use crate::Value;
use crate::canonical::append_value;
use crate::error::Result;
use crate::schema::{KeyForm, KeyStatement, RelationId, StatementId, StatementKind};

/// A key's scalar determinant fields and, for a pointwise key, its final
/// interval field.
fn key_fields(statement: &KeyStatement) -> (Vec<usize>, Option<usize>) {
    let mut fields: Vec<usize> = statement
        .projection
        .iter()
        .map(|field| usize::from(field.0))
        .collect();
    let tail = match statement.form() {
        KeyForm::Scalar => None,
        KeyForm::Pointwise { .. } => fields.pop(),
    };
    (fields, tail)
}

/// What a key's group index says about one determinant.
enum Group {
    Unindexed,
    Lawful,
    Conflict,
}

/// The furthest end a pointwise sweep has seen in one group, and the row
/// that reaches it. Rows arrive sorted by start, so a row overlaps an earlier
/// row of its group exactly when it starts before the reach, and then it
/// overlaps the reach's owner; flagging both cites every row that overlaps
/// any other.
#[derive(Clone, Copy)]
struct Reach {
    token: u64,
    end: u64,
    seq: u64,
}

impl Reach {
    /// Advances the sweep by one span; returns the reach owner it overlaps.
    fn sweep(reach: &mut Option<Self>, token: u64, start: u64, end: u64, seq: u64) -> Option<u64> {
        let Some(current) = reach.filter(|current| current.token == token) else {
            *reach = Some(Self { token, end, seq });
            return None;
        };
        if end > current.end {
            *reach = Some(Self { token, end, seq });
        }
        (start < current.end).then_some(current.seq)
    }
}

impl Judge<'_, '_> {
    pub(super) fn key<S: Facts>(&mut self, state: &S, statement: &KeyStatement) -> Result<()> {
        let (scalar_fields, tail) = key_fields(statement);
        let mut pending = self.pending(statement.id, StatementKind::Functionality);
        match tail {
            None => self.key_scalar(state, statement.relation, &scalar_fields, &mut pending)?,
            Some(tail) => self.key_pointwise(
                state,
                statement.relation,
                &scalar_fields,
                tail,
                &mut pending,
            )?,
        }
        self.finish(pending);
        Ok(())
    }

    /// A determinant seen twice is a violation; every row of a violating
    /// group is offered.
    fn key_scalar<S: Facts>(
        &mut self,
        state: &S,
        relation: RelationId,
        scalar_fields: &[usize],
        pending: &mut PendingViolation,
    ) -> Result<()> {
        let mut seen = GroupedMap::default();
        let mut offending = GroupedMap::default();
        let mut det = Vec::new();
        self.for_each_row(state, relation, |_judge, _rank, row| {
            encode_determinant(row, scalar_fields, &mut det);
            if !seen.insert_if_absent(&det) {
                offending.put(&det, &[]);
            }
            Ok(Continue(()))
        })?;
        if offending.len() == 0 {
            return Ok(());
        }
        pending.violated = true;
        self.for_each_row(state, relation, |judge, _rank, row| {
            encode_determinant(row, scalar_fields, &mut det);
            if offending.contains(&det) {
                judge.offer(pending, relation, row)?;
            }
            Ok(Continue(()))
        })
    }

    /// Rows of one determinant coexist only with disjoint intervals. Spans
    /// are keyed `(group token, start, end, rank)` (see [`Reach`]).
    fn key_pointwise<S: Facts>(
        &mut self,
        state: &S,
        relation: RelationId,
        scalar_fields: &[usize],
        tail: usize,
        pending: &mut PendingViolation,
    ) -> Result<()> {
        let mut tokens = GroupedMap::default();
        let mut spans = GroupedMap::default();
        let mut det = Vec::new();
        self.for_each_row(state, relation, |_judge, rank, row| {
            encode_determinant(row, scalar_fields, &mut det);
            let token = tokens.token_of(&det);
            let (start, end) = interval_order_words(&row[tail])
                .expect("a projected interval position holds an interval value");
            spans.put(&span_key(token, start, end, rank), &[]);
            Ok(Continue(()))
        })?;
        let offending = self.sweep(&spans)?;
        if offending.len() == 0 {
            return Ok(());
        }
        pending.violated = true;
        self.for_each_row(state, relation, |judge, rank, row| {
            if offending.contains(&rank.to_be_bytes()) {
                judge.offer(pending, relation, row)?;
            }
            Ok(Continue(()))
        })
    }

    /// The ranks of every span that overlaps another of its group.
    fn sweep(&self, spans: &GroupedMap) -> Result<GroupedMap> {
        let mut offending = GroupedMap::default();
        let mut reach = None;
        spans.for_each(|key, _| {
            self.work.checkpoint()?;
            let (token, start, end, seq) = parse_span_key(key);
            if let Some(owner) = Reach::sweep(&mut reach, token, start, end, seq) {
                offending.put(&owner.to_be_bytes(), &[]);
                offending.put(&seq.to_be_bytes(), &[]);
            }
            Ok(Continue(()))
        })?;
        Ok(offending)
    }

    /// Judges a key through the state's group index: only the groups the
    /// delta adds to. A scalar group is probed for a second row; a pointwise
    /// group is swept whole. `Unindexed` asks for complete judgment.
    pub(super) fn key_delta_local<S: DeltaFacts>(
        &mut self,
        state: &S,
        statement: &KeyStatement,
    ) -> Result<Indexed> {
        let (scalar_fields, tail) = key_fields(statement);
        let Some(tail) = tail else {
            return self.key_scalar_delta_local(state, statement, &scalar_fields);
        };
        let mut seen = GroupedMap::default();
        let mut groups: Vec<Vec<Value>> = Vec::new();
        let mut det = Vec::new();
        let work = self.work;
        state.visit_added_rows(statement.relation, &mut |row| {
            work.checkpoint()?;
            encode_determinant(row, &scalar_fields, &mut det);
            if seen.insert_if_absent(&det) {
                groups.push(scalar_fields.iter().map(|&at| row[at].clone()).collect());
            }
            Ok(Continue(()))
        })?;
        let mut pending = self.pending(statement.id, StatementKind::Functionality);
        for determinant in &groups {
            if self.key_group_pointwise(state, statement, determinant, tail, &mut pending)?
                == Indexed::Unindexed
            {
                return Ok(Indexed::Unindexed);
            }
        }
        self.finish(pending);
        Ok(Indexed::Walked)
    }

    /// Scalar keys need at most two final rows to convict a group; only
    /// violating groups are remembered and enumerated for citations. A group
    /// the index declines discards every provisional citation.
    fn key_scalar_delta_local<S: DeltaFacts>(
        &mut self,
        state: &S,
        statement: &KeyStatement,
        scalar_fields: &[usize],
    ) -> Result<Indexed> {
        if state.scalar_key_preserved(statement.id) {
            return Ok(Indexed::Walked);
        }
        let mut determinant = ScalarKeyScratch::new(scalar_fields.len());
        let mut offending = GroupedMap::default();
        let mut pending = self.pending(statement.id, StatementKind::Functionality);
        let mut indexed = Indexed::Walked;
        state.visit_added_rows(statement.relation, &mut |row| {
            self.work.checkpoint()?;
            determinant.project(row, scalar_fields);
            // Known violating groups are checked first: many additions to one
            // group must not repeat its citation walk.
            let have_bad_groups = offending.len() != 0;
            if have_bad_groups && offending.contains(determinant.encode()) {
                return Ok(Continue(()));
            }
            match self.scalar_key_conflict(state, statement.id, determinant.values())? {
                Group::Unindexed => {
                    indexed = Indexed::Unindexed;
                    return Ok(Break(()));
                }
                Group::Lawful => return Ok(Continue(())),
                Group::Conflict => {}
            }
            if !have_bad_groups {
                determinant.encode();
            }
            offending.insert_if_absent(determinant.key());
            pending.violated = true;
            indexed = self.offer_key_group(
                state,
                statement.id,
                statement.relation,
                determinant.values(),
                &mut pending,
                None,
            )?;
            Ok(match indexed {
                Indexed::Walked => Continue(()),
                Indexed::Unindexed => Break(()),
            })
        })?;
        if indexed == Indexed::Walked {
            self.finish(pending);
        }
        Ok(indexed)
    }

    fn scalar_key_conflict<S: DeltaFacts>(
        &self,
        state: &S,
        statement: StatementId,
        determinant: &[Value],
    ) -> Result<Group> {
        let mut count = 0u8;
        let indexed = state.visit_key_competitors(statement, determinant, &mut |_| {
            self.work.checkpoint()?;
            count += 1;
            Ok(if count < 2 { Continue(()) } else { Break(()) })
        })?;
        Ok(match indexed {
            Indexed::Unindexed => Group::Unindexed,
            Indexed::Walked if count == 2 => Group::Conflict,
            Indexed::Walked => Group::Lawful,
        })
    }

    fn key_group_pointwise<S: DeltaFacts>(
        &mut self,
        state: &S,
        statement: &KeyStatement,
        determinant: &[Value],
        tail: usize,
        pending: &mut PendingViolation,
    ) -> Result<Indexed> {
        let mut spans = GroupedMap::default();
        let mut seq = 0u64;
        let work = self.work;
        let indexed = state.visit_key_competitors(statement.id, determinant, &mut |values| {
            work.checkpoint()?;
            let (start, end) = interval_order_words(&values[tail])
                .expect("a projected interval position holds an interval value");
            spans.put(&span_key(0, start, end, seq), &[]);
            seq += 1;
            Ok(Continue(()))
        })?;
        if indexed == Indexed::Unindexed {
            return Ok(Indexed::Unindexed);
        }
        let offending = self.sweep(&spans)?;
        if offending.len() == 0 {
            return Ok(Indexed::Walked);
        }
        pending.violated = true;
        self.offer_key_group(
            state,
            statement.id,
            statement.relation,
            determinant,
            pending,
            Some(&offending),
        )
    }

    /// Offers a key group's rows: every row, or only those whose encounter
    /// position `only` flags.
    fn offer_key_group<S: DeltaFacts>(
        &mut self,
        state: &S,
        statement: StatementId,
        relation: RelationId,
        determinant: &[Value],
        pending: &mut PendingViolation,
        only: Option<&GroupedMap>,
    ) -> Result<Indexed> {
        let mut seq = 0u64;
        state.visit_key_competitors(statement, determinant, &mut |values| {
            let at = seq;
            seq += 1;
            if only.is_none_or(|flagged| flagged.contains(&at.to_be_bytes())) {
                self.offer(pending, relation, values)?;
            }
            Ok(Continue(()))
        })
    }
}

fn encode_determinant(row: &[Value], scalar_fields: &[usize], out: &mut Vec<u8>) {
    out.clear();
    for &at in scalar_fields {
        append_value(out, &row[at]);
    }
}
