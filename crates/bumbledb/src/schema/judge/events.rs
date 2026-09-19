//! Exact Event admission over distinct canonical facts in a proposed final state.
//! Summaries use spillable scalar groups and canonical Event values. Rebuilding
//! from surviving contributors handles deletion without subtracting overlapping
//! coverage. Empty contributions retain their namespace and still participate
//! in the independent scalar statements.

use super::grouped::{GroupedMap, encode_value};
use super::{CandidateFacts, Judge, JudgeError, PendingViolation, encode_projection};
use crate::event::{BoolOp4, Event};
use crate::schema::coverage::{Coverage, covers};
use crate::schema::{ContainmentStatement, RelationId};
use crate::{Value, WorkContext};

fn read<E>(
    map: &mut GroupedMap<E>,
    key: &[u8],
    bytes: &mut Vec<u8>,
    work: &WorkContext,
) -> Result<Option<Event>, JudgeError<E>> {
    if map.get(key, bytes)? {
        Event::from_bytes(bytes, work).map(Some).map_err(Into::into)
    } else {
        Ok(None)
    }
}

fn union<E>(
    map: &mut GroupedMap<E>,
    key: &[u8],
    value: &Event,
    bytes: &mut Vec<u8>,
    work: &WorkContext,
) -> Result<(), JudgeError<E>> {
    let combined = if let Some(previous) = read(map, key, bytes, work)? {
        // Alignment precedes all empty/full shortcuts, even for an empty row.
        let value = value.align_to(&previous.space(), work)?;
        previous.apply(BoolOp4::OR, &value, work)?
    } else {
        value.clone()
    };
    map.put(key, &combined.to_bytes(work)?)
}

fn event(value: &Value) -> &Event {
    let Value::Event(event) = value else {
        unreachable!("sealed Event projection and checked canonical row")
    };
    event
}

/// A full projection has no stored owner. It denotes the full set in every
/// admitted context, and is instantiated when an Event operand supplies one.
const CONTEXTUAL_FULL: &[u8] = &[0];

fn read_coverage<E>(
    map: &mut GroupedMap<E>,
    key: &[u8],
    bytes: &mut Vec<u8>,
    work: &WorkContext,
) -> Result<Option<Coverage>, JudgeError<E>> {
    if !map.get(key, bytes)? {
        return Ok(None);
    }
    if bytes.as_slice() == CONTEXTUAL_FULL {
        return Ok(Some(Coverage::ContextualFull));
    }
    Ok(Some(Coverage::Region(Event::from_bytes(bytes, work)?)))
}

impl<E> Judge<'_, '_, E> {
    pub(super) fn key_event<S: CandidateFacts<Error = E>>(
        &mut self,
        state: &S,
        relation: RelationId,
        scalars: &[usize],
        tail: usize,
        pending: &mut PendingViolation,
    ) -> Result<(), JudgeError<E>> {
        let mut coverage = self.grouped();
        let mut conflicts = self.grouped();
        let mut determinant = Vec::new();
        let mut bytes = Vec::new();
        self.for_each_row(state, relation, |judge, _, row| {
            determinant.clear();
            for &at in scalars {
                encode_value(&row[at], &mut determinant);
            }
            let value = event(&row[tail]);
            if let Some(previous) = read(&mut coverage, &determinant, &mut bytes, judge.work)? {
                let value = value.align_to(&previous.space(), judge.work)?;
                let overlap = previous.apply(BoolOp4::AND, &value, judge.work)?;
                if !overlap.is_empty() {
                    union(
                        &mut conflicts,
                        &determinant,
                        &overlap,
                        &mut bytes,
                        judge.work,
                    )?;
                }
                let combined = previous.apply(BoolOp4::OR, &value, judge.work)?;
                coverage.put(&determinant, &combined.to_bytes(judge.work)?)?;
            } else {
                coverage.put(&determinant, &value.to_bytes(judge.work)?)?;
            }
            Ok(true)
        })?;
        if conflicts.len() == 0 {
            return Ok(());
        }
        pending.violated = true;
        // Cite actual participating facts, selected by canonical whole-fact
        // bytes. Empty or disjoint rows in a bad group are not conflict witnesses.
        self.for_each_row(state, relation, |judge, _, row| {
            determinant.clear();
            for &at in scalars {
                encode_value(&row[at], &mut determinant);
            }
            if let Some(conflict) = read(&mut conflicts, &determinant, &mut bytes, judge.work)? {
                let value = event(&row[tail]).align_to(&conflict.space(), judge.work)?;
                if !conflict.apply(BoolOp4::AND, &value, judge.work)?.is_empty() {
                    judge.offer(pending, relation, row)?;
                }
            }
            Ok(true)
        })
    }

    pub(super) fn containment_event<S: CandidateFacts<Error = E>>(
        &mut self,
        state: &S,
        statement: &ContainmentStatement,
        position: usize,
        pending: &mut PendingViolation,
    ) -> Result<(), JudgeError<E>> {
        let mut coverage = self.grouped();
        let mut determinant = Vec::new();
        let mut bytes = Vec::new();
        self.for_each_row(state, statement.target.relation, |judge, _, row| {
            if judge.satisfies(&statement.target, row)? {
                determinant.clear();
                encode_projection(&statement.target, row, Some(position), &mut determinant);
                if statement.target.projection.is_event_full() {
                    coverage.put(&determinant, CONTEXTUAL_FULL)?;
                } else {
                    let at = usize::from(statement.target.projection.fields()[position].0);
                    union(
                        &mut coverage,
                        &determinant,
                        event(&row[at]),
                        &mut bytes,
                        judge.work,
                    )?;
                }
            }
            Ok(true)
        })?;
        self.for_each_row(state, statement.source.relation, |judge, _, row| {
            if !judge.satisfies(&statement.source, row)? {
                return Ok(true);
            }
            determinant.clear();
            encode_projection(&statement.source, row, Some(position), &mut determinant);
            let mut target = read_coverage(&mut coverage, &determinant, &mut bytes, judge.work)?;
            let source = if statement.source.projection.is_event_full() {
                None
            } else {
                let at = usize::from(statement.source.projection.fields()[position].0);
                Some(event(&row[at]))
            };
            let contextual = !matches!(target, Some(Coverage::Region(_)));
            let covered = covers(&mut target, source, judge.work)?;
            if contextual && let Some(Coverage::Region(target)) = target {
                coverage.put(&determinant, &target.to_bytes(judge.work)?)?;
            }
            if !covered {
                pending.violated = true;
                judge.offer(pending, statement.source.relation, row)?;
            }
            Ok(true)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{Space, SpaceId};
    use crate::schema::judge::{JudgeScratch, store_fault};
    use crate::storage::store::StoreError;

    #[test]
    fn event_summaries_survive_spill_and_check_context_before_shortcuts() {
        let work = WorkContext::new();
        let space = Space::new(SpaceId([71; 32]), 3, &work).unwrap();
        let other_owner = Space::new(SpaceId([71; 32]), 3, &work).unwrap();
        let mut map =
            GroupedMap::<StoreError>::new(&work, JudgeScratch::channel(store_fault).channel);
        let mut bytes = Vec::new();
        // Oversized keys exercise exact reconstruction in scratch hash buckets.
        let group = vec![5; 1200];
        union(
            &mut map,
            &group,
            &space.coordinate(0, &work).unwrap(),
            &mut bytes,
            &work,
        )
        .unwrap();
        map.put(b"full", CONTEXTUAL_FULL).unwrap();
        map.force_spill().unwrap();
        let path = map.scratch_path().unwrap();
        assert!(matches!(
            read_coverage(&mut map, b"full", &mut bytes, &work).unwrap(),
            Some(Coverage::ContextualFull)
        ));
        union(
            &mut map,
            &group,
            &other_owner.coordinate(1, &work).unwrap(),
            &mut bytes,
            &work,
        )
        .unwrap();
        union(&mut map, b"empty", &space.empty(), &mut bytes, &work).unwrap();
        assert_eq!(
            read(&mut map, &group, &mut bytes, &work)
                .unwrap()
                .unwrap()
                .count(&work)
                .unwrap(),
            6
        );
        assert!(
            read(&mut map, b"empty", &mut bytes, &work)
                .unwrap()
                .unwrap()
                .is_empty()
        );
        assert!(
            read(&mut map, b"absent", &mut bytes, &work)
                .unwrap()
                .is_none()
        );
        let foreign = Space::new(SpaceId([72; 32]), 3, &work).unwrap();
        assert!(matches!(
            union(&mut map, b"empty", &foreign.empty(), &mut bytes, &work),
            Err(JudgeError::Event(crate::event::Error::SpaceMismatch))
        ));
        work.cancel();
        assert!(matches!(
            read(&mut map, &group, &mut bytes, &work),
            Err(JudgeError::Work(_))
        ));
        drop(map);
        assert!(!path.exists());
    }
}
