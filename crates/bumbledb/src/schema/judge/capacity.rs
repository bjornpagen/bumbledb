//! Capacity judgment: each selected target row's group measures the
//! distinct selected source rows of its group, and the measure must fall in
//! the statement's window.
use std::ops::ControlFlow::{self, Break, Continue};

use super::containment::Sides;
use super::grouped::{FLAG_OVERFLOW, FLAG_RAY, GroupedMap};
use super::{
    DeltaFacts, Facts, Indexed, Judge, PendingViolation, encode_projection, encode_values,
    mark_delta_groups, satisfies,
};
use crate::Value;
use crate::changes::DeltaShape;
use crate::error::{Error, Result};
use crate::schema::compiled::CompiledTheory;
use crate::schema::{CapacityStatement, SealedBound, SealedWeight, StatementKind};

impl Judge<'_, '_> {
    /// Accumulates every group's widened measure from the source rows, then
    /// opens each selected target row's window, then cites the sources of
    /// violating groups. A group's ray or overflow is sticky and refuses
    /// only when a target row reads the group.
    pub(super) fn capacity<S: Facts>(
        &mut self,
        state: &S,
        statement: &CapacityStatement,
    ) -> Result<()> {
        let mut pending = self.pending(statement.id, StatementKind::Capacity);
        let mut totals = GroupedMap::default();
        let mut group = Vec::new();
        self.for_each_row(state, statement.source.relation, |_judge, _rank, row| {
            if satisfies(&statement.source, row) {
                group.clear();
                encode_projection(&statement.source, row, None, &mut group);
                accumulate_capacity(&mut totals, statement, row, &group);
            }
            Ok(Continue(()))
        })?;
        let mut violating = GroupedMap::default();
        self.for_each_row(state, statement.target.relation, |judge, rank, row| {
            if satisfies(&statement.target, row) {
                group.clear();
                encode_projection(&statement.target, row, None, &mut group);
                judge.note_capacity_target(
                    statement,
                    (rank, row),
                    &group,
                    &totals,
                    &mut violating,
                    &mut pending,
                )?;
            }
            Ok(Continue(()))
        })?;
        if pending.violated {
            self.for_each_row(state, statement.source.relation, |judge, _rank, row| {
                if satisfies(&statement.source, row) {
                    group.clear();
                    encode_projection(&statement.source, row, None, &mut group);
                    if violating.contains(&group) {
                        judge.offer(&mut pending, statement.source.relation, row)?;
                    }
                }
                Ok(Continue(()))
            })?;
        }
        self.finish(pending);
        Ok(())
    }

    /// Recomputes only the groups the delta can change: touched source
    /// groups and added targets.
    pub(super) fn capacity_delta_local<S: DeltaFacts>(
        &mut self,
        state: &S,
        theory: &CompiledTheory,
        statement: &CapacityStatement,
    ) -> Result<()> {
        let (Some(source_binding), Some(target_binding)) = (
            theory.source_binding(statement.id),
            theory.target_binding(statement.id),
        ) else {
            return self.capacity(state, statement);
        };
        let mut affected = GroupedMap::default();
        mark_delta_groups(
            state,
            &statement.source,
            source_binding,
            DeltaShape {
                adds: true,
                removes: true,
            },
            &mut affected,
        )?;
        mark_delta_groups(
            state,
            &statement.target,
            target_binding,
            DeltaShape {
                adds: true,
                removes: false,
            },
            &mut affected,
        )?;
        if affected.len() == 0 {
            return Ok(());
        }
        let sides = Sides {
            theory,
            source_compiled: theory.source_projection(statement.id),
            target_compiled: theory.target_projection(statement.id),
            source_binding,
            target_binding,
        };
        let mut pending = self.pending(statement.id, StatementKind::Capacity);
        if self.capacity_compiled(state, statement, &sides, &affected, &mut pending)?
            == Indexed::Unindexed
        {
            // A declined index invalidates the provisional pass.
            pending = self.pending(statement.id, StatementKind::Capacity);
            self.capacity_affected(state, statement, &sides, &affected, &mut pending)?;
        }
        self.finish(pending);
        Ok(())
    }

    fn capacity_compiled<S: DeltaFacts>(
        &mut self,
        state: &S,
        statement: &CapacityStatement,
        sides: &Sides<'_>,
        affected: &GroupedMap,
        pending: &mut PendingViolation,
    ) -> Result<Indexed> {
        let fields = self.schema.relation(statement.source.relation).fields();
        let scalars = &sides.source_binding.logical_scalars;
        let mut first = true;
        let mut indexed = Indexed::Walked;
        let mut totals = GroupedMap::default();
        affected.for_each_determinant(fields, scalars, self.work, |group, det| {
            if std::mem::take(&mut first) && sides.live(state, det)? == Indexed::Unindexed {
                indexed = Indexed::Unindexed;
                return Ok(Break(()));
            }
            if let Some(compiled) = sides.source_compiled {
                let walked = self.visit_indexed_group(
                    state,
                    compiled,
                    sides.source_binding,
                    det,
                    |_judge, row| {
                        if satisfies(&statement.source, row) {
                            accumulate_capacity(&mut totals, statement, row, group);
                        }
                        Ok(Continue(()))
                    },
                )?;
                if walked == Indexed::Unindexed {
                    indexed = Indexed::Unindexed;
                    return Ok(Break(()));
                }
            } else {
                self.for_each_row(state, statement.source.relation, |_judge, _rank, row| {
                    if satisfies(&statement.source, row)
                        && CompiledTheory::group_key(sides.source_binding, row).as_slice() == det
                    {
                        accumulate_capacity(&mut totals, statement, row, group);
                    }
                    Ok(Continue(()))
                })?;
            }
            Ok(Continue(()))
        })?;
        if indexed == Indexed::Unindexed {
            return Ok(indexed);
        }
        let mut violating = GroupedMap::default();
        affected.for_each_determinant(fields, scalars, self.work, |group, det| {
            let mut note =
                |judge: &mut Self, rank: u64, row: &[Value]| -> Result<ControlFlow<()>> {
                    if satisfies(&statement.target, row) {
                        judge.note_capacity_target(
                            statement,
                            (rank, row),
                            group,
                            &totals,
                            &mut violating,
                            pending,
                        )?;
                    }
                    Ok(Continue(()))
                };
            if let Some(compiled) = sides.target_compiled {
                let walked = self.visit_ranked_indexed_group(
                    state,
                    compiled,
                    sides.target_binding,
                    det,
                    &mut note,
                )?;
                if walked == Indexed::Unindexed {
                    indexed = Indexed::Unindexed;
                    return Ok(Break(()));
                }
            } else {
                self.for_each_row(state, statement.target.relation, |judge, rank, row| {
                    if CompiledTheory::group_key(sides.target_binding, row).as_slice() == det {
                        return note(judge, rank, row);
                    }
                    Ok(Continue(()))
                })?;
            }
            Ok(Continue(()))
        })?;
        if indexed == Indexed::Unindexed || !pending.violated {
            return Ok(indexed);
        }
        // The violating groups are exactly the groups whose sources are cited.
        violating.for_each_determinant(fields, scalars, self.work, |_group, det| {
            let mut offer = |judge: &mut Self, row: &[Value]| -> Result<ControlFlow<()>> {
                if satisfies(&statement.source, row) {
                    judge.offer(pending, statement.source.relation, row)?;
                }
                Ok(Continue(()))
            };
            if let Some(compiled) = sides.source_compiled {
                let walked = self.visit_indexed_group(
                    state,
                    compiled,
                    sides.source_binding,
                    det,
                    &mut offer,
                )?;
                if walked == Indexed::Unindexed {
                    indexed = Indexed::Unindexed;
                    return Ok(Break(()));
                }
            } else {
                self.for_each_row(state, statement.source.relation, |judge, _rank, row| {
                    if CompiledTheory::group_key(sides.source_binding, row).as_slice() == det {
                        return offer(judge, row);
                    }
                    Ok(Continue(()))
                })?;
            }
            Ok(Continue(()))
        })?;
        Ok(indexed)
    }

    fn capacity_affected<S: DeltaFacts>(
        &mut self,
        state: &S,
        statement: &CapacityStatement,
        sides: &Sides<'_>,
        affected: &GroupedMap,
        pending: &mut PendingViolation,
    ) -> Result<()> {
        let mut totals = GroupedMap::default();
        let mut group = Vec::new();
        self.for_each_row(state, statement.source.relation, |_judge, _rank, row| {
            if satisfies(&statement.source, row) {
                group.clear();
                encode_values(
                    &CompiledTheory::group_key(sides.source_binding, row),
                    &mut group,
                );
                if affected.contains(&group) {
                    accumulate_capacity(&mut totals, statement, row, &group);
                }
            }
            Ok(Continue(()))
        })?;
        let mut violating = GroupedMap::default();
        self.for_each_row(state, statement.target.relation, |judge, rank, row| {
            if satisfies(&statement.target, row) {
                group.clear();
                encode_values(
                    &CompiledTheory::group_key(sides.target_binding, row),
                    &mut group,
                );
                if affected.contains(&group) {
                    judge.note_capacity_target(
                        statement,
                        (rank, row),
                        &group,
                        &totals,
                        &mut violating,
                        pending,
                    )?;
                }
            }
            Ok(Continue(()))
        })?;
        if pending.violated {
            self.for_each_row(state, statement.source.relation, |judge, _rank, row| {
                if satisfies(&statement.source, row) {
                    group.clear();
                    encode_values(
                        &CompiledTheory::group_key(sides.source_binding, row),
                        &mut group,
                    );
                    if violating.contains(&group) {
                        judge.offer(pending, statement.source.relation, row)?;
                    }
                }
                Ok(Continue(()))
            })?;
        }
        Ok(())
    }

    /// Opens one selected target row's window over its group's measure. The
    /// witnessed measure is the last violating group's in logical target
    /// order.
    fn note_capacity_target(
        &mut self,
        statement: &CapacityStatement,
        (rank, row): (u64, &[Value]),
        group: &[u8],
        totals: &GroupedMap,
        violating: &mut GroupedMap,
        pending: &mut PendingViolation,
    ) -> Result<()> {
        let (total, flag) = totals.group_total(group);
        if flag == FLAG_RAY {
            return Err(Error::CapacityRayMeasure {
                statement: statement.id,
            });
        }
        if flag == FLAG_OVERFLOW {
            return Err(Error::MeasureOverflow {
                statement: statement.id,
            });
        }
        let ceiling: Option<u128> = match statement.hi {
            SealedBound::Unbounded => None,
            SealedBound::Lit(hi) => Some(u128::from(hi)),
            SealedBound::TargetField(field) => match &row[usize::from(field.0)] {
                Value::U64(hi) => Some(u128::from(*hi)),
                _ => unreachable!("validation types a dependent bound as u64"),
            },
            SealedBound::Duration { field, .. } => Some(u128::from(
                duration(&row[usize::from(field.0)]).ok_or(Error::CapacityRayMeasure {
                    statement: statement.id,
                })?,
            )),
        };
        if total < u128::from(statement.lo) || ceiling.is_some_and(|hi| total > hi) {
            pending.violated = true;
            pending.record_measure_at(rank, total);
            violating.put(group, &[]);
            self.offer(pending, statement.target.relation, row)?;
        }
        Ok(())
    }
}

/// Adds one source row's weight to its group's widened total; a ray weight
/// or an overflow marks the group instead.
fn accumulate_capacity(
    totals: &mut GroupedMap,
    statement: &CapacityStatement,
    row: &[Value],
    group: &[u8],
) {
    let (mut total, mut flag) = totals.group_total(group);
    if flag != 0 {
        return;
    }
    let weight = match statement.weight {
        SealedWeight::Unit => Some(1u128),
        SealedWeight::Field(field) => match &row[usize::from(field.0)] {
            Value::U64(weight) => Some(u128::from(*weight)),
            _ => unreachable!("validation types a [field] weight as u64"),
        },
        SealedWeight::Duration { field, .. } => {
            duration(&row[usize::from(field.0)]).map(u128::from)
        }
    };
    match weight {
        None => flag = FLAG_RAY,
        Some(weight) => match total.checked_add(weight) {
            Some(next) => total = next,
            None => flag = FLAG_OVERFLOW,
        },
    }
    totals.put_group_total(group, total, flag);
}

/// A discrete interval's exact duration; `None` for a ray.
fn duration(value: &Value) -> Option<u64> {
    match value {
        Value::IntervalU64(interval) => interval.duration(),
        Value::IntervalI64(interval) => interval.duration(),
        _ => unreachable!("validation types duration positions as discrete intervals"),
    }
}
