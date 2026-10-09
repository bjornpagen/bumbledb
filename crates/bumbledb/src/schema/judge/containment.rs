//! Containment judgment: every selected source projection has a selected
//! target, by tuple existence or, for a pointwise target, by coverage of the
//! source interval with the union of target intervals.
use std::ops::ControlFlow::{self, Break, Continue};

use super::grouped::GroupedMap;
use super::{
    DeltaFacts, Facts, Indexed, Judge, JudgedDirection, PendingViolation, encode_projection,
    encode_values, interval_order_words, mark_delta_groups, parse_span_key, satisfies, span_key,
};
use crate::Value;
use crate::WorkContext;
use crate::changes::DeltaShape;
use crate::error::Result;
use crate::schema::compiled::{CompiledProjection, CompiledTheory, ProjectionBinding};
use crate::schema::{ContainmentStatement, Enforcement, MemberSet, Schema, Side, StatementKind};

/// The target projection position judged by coverage, if any.
fn coverage_position(schema: &Schema, statement: &ContainmentStatement) -> Option<usize> {
    let fields = schema.relation(statement.target.relation).fields();
    statement
        .target
        .projection
        .iter()
        .position(|field| fields[usize::from(field.0)].value_type.is_interval())
}

impl Judge<'_, '_> {
    fn containment_pending(&self, statement: &ContainmentStatement) -> PendingViolation {
        self.pending(statement.id, StatementKind::Containment)
            .with_direction(JudgedDirection::SourceUnsatisfied)
    }

    pub(super) fn containment<S: Facts>(
        &mut self,
        state: &S,
        statement: &ContainmentStatement,
    ) -> Result<()> {
        let mut pending = self.containment_pending(statement);
        if let Enforcement::Closed { members } = &statement.enforcement {
            self.for_each_row(state, statement.source.relation, |judge, _rank, row| {
                judge.containment_closed_row(statement, members, row, &mut pending)?;
                Ok(Continue(()))
            })?;
        } else {
            match coverage_position(self.schema, statement) {
                None => self.containment_scalar(state, statement, &mut pending)?,
                Some(position) => {
                    self.containment_pointwise(state, statement, position, &mut pending)?;
                }
            }
        }
        self.finish(pending);
        Ok(())
    }

    /// A closed target's members already apply its selection, and its
    /// projection is the handle alone.
    fn containment_closed_row(
        &mut self,
        statement: &ContainmentStatement,
        members: &MemberSet,
        row: &[Value],
        pending: &mut PendingViolation,
    ) -> Result<()> {
        if !satisfies(&statement.source, row) {
            return Ok(());
        }
        let handle = &row[usize::from(statement.source.projection[0].0)];
        let witnessed = matches!(handle, Value::U64(word)
            if u8::try_from(*word).is_ok_and(|index| members.contains(index)));
        if !witnessed {
            pending.violated = true;
            self.offer(pending, statement.source.relation, row)?;
        }
        Ok(())
    }

    /// Tuple existence: every selected target projection goes in an exact
    /// set, then each selected source row probes it.
    fn containment_scalar<S: Facts>(
        &mut self,
        state: &S,
        statement: &ContainmentStatement,
        pending: &mut PendingViolation,
    ) -> Result<()> {
        let mut witnesses = GroupedMap::default();
        let mut key = Vec::new();
        self.for_each_row(state, statement.target.relation, |_judge, _rank, row| {
            if satisfies(&statement.target, row) {
                key.clear();
                encode_projection(&statement.target, row, None, &mut key);
                witnesses.put(&key, &[]);
            }
            Ok(Continue(()))
        })?;
        self.for_each_row(state, statement.source.relation, |judge, _rank, row| {
            if !satisfies(&statement.source, row) {
                return Ok(Continue(()));
            }
            key.clear();
            encode_projection(&statement.source, row, None, &mut key);
            if !witnesses.contains(&key) {
                pending.violated = true;
                judge.offer(pending, statement.source.relation, row)?;
            }
            Ok(Continue(()))
        })
    }

    /// Coverage: the scalar prefix matches and the source span lies inside
    /// one maximal run of matching target spans (adjacent spans connect).
    fn containment_pointwise<S: Facts>(
        &mut self,
        state: &S,
        statement: &ContainmentStatement,
        position: usize,
        pending: &mut PendingViolation,
    ) -> Result<()> {
        let mut tokens = GroupedMap::default();
        let mut spans = GroupedMap::default();
        let mut prefix = Vec::new();
        self.for_each_row(state, statement.target.relation, |_judge, _rank, row| {
            if satisfies(&statement.target, row) {
                prefix.clear();
                encode_projection(&statement.target, row, Some(position), &mut prefix);
                let token = tokens.token_of(&prefix);
                put_span(&mut spans, token, &statement.target, position, row);
            }
            Ok(Continue(()))
        })?;
        let runs = coverage_runs(&spans, self.work)?;
        self.for_each_row(state, statement.source.relation, |judge, _rank, row| {
            if !satisfies(&statement.source, row) {
                return Ok(Continue(()));
            }
            prefix.clear();
            encode_projection(&statement.source, row, Some(position), &mut prefix);
            let span = &row[usize::from(statement.source.projection[position].0)];
            let witnessed = tokens
                .lookup_token(&prefix)
                .is_some_and(|token| run_covers(token, span, &runs));
            if !witnessed {
                pending.violated = true;
                judge.offer(pending, statement.source.relation, row)?;
            }
            Ok(Continue(()))
        })
    }

    /// Judges only the groups the delta can affect: added sources and
    /// removed targets, through compiled group visits when the state has
    /// them.
    pub(super) fn containment_delta_local<S: DeltaFacts>(
        &mut self,
        state: &S,
        theory: &CompiledTheory,
        statement: &ContainmentStatement,
    ) -> Result<()> {
        if let Enforcement::Closed { members } = &statement.enforcement {
            return self.containment_closed_delta(state, statement, members);
        }
        let (Some(source_binding), Some(target_binding)) = (
            theory.source_binding(statement.id),
            theory.target_binding(statement.id),
        ) else {
            return self.containment(state, statement);
        };
        let mut affected = GroupedMap::default();
        mark_delta_groups(
            state,
            &statement.source,
            source_binding,
            DeltaShape {
                adds: true,
                removes: false,
            },
            &mut affected,
        )?;
        mark_delta_groups(
            state,
            &statement.target,
            target_binding,
            DeltaShape {
                adds: false,
                removes: true,
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
        let position = coverage_position(self.schema, statement);
        let mut pending = self.containment_pending(statement);
        let indexed = match position {
            None => {
                self.containment_scalar_compiled(state, statement, &sides, &affected, &mut pending)?
            }
            Some(position) => self.containment_pointwise_compiled(
                state,
                statement,
                position,
                &sides,
                &affected,
                &mut pending,
            )?,
        };
        if indexed == Indexed::Unindexed {
            // A declined group invalidates every provisional citation before
            // the affected groups are walked in full.
            pending = self.containment_pending(statement);
            match position {
                None => self.containment_scalar_affected(
                    state,
                    statement,
                    &sides,
                    &affected,
                    &mut pending,
                )?,
                Some(position) => self.containment_pointwise_affected(
                    state,
                    statement,
                    position,
                    &sides,
                    &affected,
                    &mut pending,
                )?,
            }
        }
        self.finish(pending);
        Ok(())
    }

    /// A lawful parent already satisfies the immutable closed target, and
    /// removing sources cannot violate it: only selected additions can.
    fn containment_closed_delta<S: DeltaFacts>(
        &mut self,
        state: &S,
        statement: &ContainmentStatement,
        members: &MemberSet,
    ) -> Result<()> {
        let mut pending = self.containment_pending(statement);
        state.visit_added_rows(statement.source.relation, &mut |row| {
            self.work.checkpoint()?;
            self.containment_closed_row(statement, members, row, &mut pending)?;
            Ok(Continue(()))
        })?;
        self.finish(pending);
        Ok(())
    }

    fn containment_scalar_compiled<S: DeltaFacts>(
        &mut self,
        state: &S,
        statement: &ContainmentStatement,
        sides: &Sides<'_>,
        affected: &GroupedMap,
        pending: &mut PendingViolation,
    ) -> Result<Indexed> {
        let fields = self.schema.relation(statement.source.relation).fields();
        let mut first = true;
        let mut indexed = Indexed::Walked;
        affected.for_each_determinant(
            fields,
            &sides.source_binding.logical_scalars,
            self.work,
            |_group, det| {
                if std::mem::take(&mut first) && sides.live(state, det)? == Indexed::Unindexed {
                    indexed = Indexed::Unindexed;
                    return Ok(Break(()));
                }
                let mut witnessed = false;
                if let Some(compiled) = sides.target_compiled {
                    let walked = self.visit_indexed_group(
                        state,
                        compiled,
                        sides.target_binding,
                        det,
                        |_judge, row| {
                            if satisfies(&statement.target, row) {
                                witnessed = true;
                                return Ok(Break(()));
                            }
                            Ok(Continue(()))
                        },
                    )?;
                    if walked == Indexed::Unindexed {
                        indexed = Indexed::Unindexed;
                        return Ok(Break(()));
                    }
                } else {
                    witnessed = self.unindexed_matches(
                        state,
                        &statement.target,
                        sides.target_binding,
                        det,
                    )?;
                }
                // One selected target witnesses every source of the group, so
                // an admitted insertion never walks the group's sources.
                if witnessed {
                    return Ok(Continue(()));
                }
                if let Some(compiled) = sides.source_compiled {
                    let walked = self.visit_indexed_group(
                        state,
                        compiled,
                        sides.source_binding,
                        det,
                        |judge, row| {
                            if satisfies(&statement.source, row) {
                                pending.violated = true;
                                judge.offer(pending, statement.source.relation, row)?;
                            }
                            Ok(Continue(()))
                        },
                    )?;
                    if walked == Indexed::Unindexed {
                        indexed = Indexed::Unindexed;
                        return Ok(Break(()));
                    }
                } else {
                    self.offer_unindexed_unsatisfied(
                        state,
                        &statement.source,
                        sides.source_binding,
                        det,
                        pending,
                    )?;
                }
                Ok(Continue(()))
            },
        )?;
        Ok(indexed)
    }

    fn containment_pointwise_compiled<S: DeltaFacts>(
        &mut self,
        state: &S,
        statement: &ContainmentStatement,
        position: usize,
        sides: &Sides<'_>,
        affected: &GroupedMap,
        pending: &mut PendingViolation,
    ) -> Result<Indexed> {
        let fields = self.schema.relation(statement.source.relation).fields();
        let source_field = usize::from(statement.source.projection[position].0);
        let mut first = true;
        let mut indexed = Indexed::Walked;
        affected.for_each_determinant(
            fields,
            &sides.source_binding.logical_scalars,
            self.work,
            |_group, det| {
                if std::mem::take(&mut first) && sides.live(state, det)? == Indexed::Unindexed {
                    indexed = Indexed::Unindexed;
                    return Ok(Break(()));
                }
                let Some(runs) = self.target_coverage(state, statement, position, sides, det)?
                else {
                    indexed = Indexed::Unindexed;
                    return Ok(Break(()));
                };
                let mut offer_uncovered =
                    |judge: &mut Self, row: &[Value]| -> Result<ControlFlow<()>> {
                        if satisfies(&statement.source, row)
                            && !run_covers(0, &row[source_field], &runs)
                        {
                            pending.violated = true;
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
                        &mut offer_uncovered,
                    )?;
                    if walked == Indexed::Unindexed {
                        indexed = Indexed::Unindexed;
                        return Ok(Break(()));
                    }
                } else {
                    self.for_each_row(state, statement.source.relation, |judge, _rank, row| {
                        if CompiledTheory::group_key(sides.source_binding, row).as_slice() == det {
                            return offer_uncovered(judge, row);
                        }
                        Ok(Continue(()))
                    })?;
                }
                Ok(Continue(()))
            },
        )?;
        Ok(indexed)
    }

    /// Coverage runs of one target determinant; `None` when the state's
    /// index declines the group.
    fn target_coverage<S: DeltaFacts>(
        &mut self,
        state: &S,
        statement: &ContainmentStatement,
        position: usize,
        sides: &Sides<'_>,
        determinant: &[Value],
    ) -> Result<Option<GroupedMap>> {
        let mut spans = GroupedMap::default();
        if let Some(compiled) = sides.target_compiled {
            let walked = self.visit_indexed_group(
                state,
                compiled,
                sides.target_binding,
                determinant,
                |_judge, row| {
                    if satisfies(&statement.target, row) {
                        put_span(&mut spans, 0, &statement.target, position, row);
                    }
                    Ok(Continue(()))
                },
            )?;
            if walked == Indexed::Unindexed {
                return Ok(None);
            }
        } else {
            self.for_each_row(state, statement.target.relation, |_judge, _rank, row| {
                if satisfies(&statement.target, row)
                    && CompiledTheory::group_key(sides.target_binding, row).as_slice()
                        == determinant
                {
                    put_span(&mut spans, 0, &statement.target, position, row);
                }
                Ok(Continue(()))
            })?;
        }
        coverage_runs(&spans, self.work).map(Some)
    }

    fn containment_scalar_affected<S: DeltaFacts>(
        &mut self,
        state: &S,
        statement: &ContainmentStatement,
        sides: &Sides<'_>,
        affected: &GroupedMap,
        pending: &mut PendingViolation,
    ) -> Result<()> {
        let mut witnesses = GroupedMap::default();
        let mut key = Vec::new();
        self.for_each_row(state, statement.target.relation, |_judge, _rank, row| {
            if satisfies(&statement.target, row) {
                key.clear();
                encode_values(
                    &CompiledTheory::group_key(sides.target_binding, row),
                    &mut key,
                );
                if affected.contains(&key) {
                    witnesses.put(&key, &[]);
                }
            }
            Ok(Continue(()))
        })?;
        self.for_each_row(state, statement.source.relation, |judge, _rank, row| {
            if !satisfies(&statement.source, row) {
                return Ok(Continue(()));
            }
            key.clear();
            encode_values(
                &CompiledTheory::group_key(sides.source_binding, row),
                &mut key,
            );
            if affected.contains(&key) && !witnesses.contains(&key) {
                pending.violated = true;
                judge.offer(pending, statement.source.relation, row)?;
            }
            Ok(Continue(()))
        })
    }

    fn containment_pointwise_affected<S: DeltaFacts>(
        &mut self,
        state: &S,
        statement: &ContainmentStatement,
        position: usize,
        sides: &Sides<'_>,
        affected: &GroupedMap,
        pending: &mut PendingViolation,
    ) -> Result<()> {
        let mut tokens = GroupedMap::default();
        let mut spans = GroupedMap::default();
        let mut prefix = Vec::new();
        self.for_each_row(state, statement.target.relation, |_judge, _rank, row| {
            if !satisfies(&statement.target, row) {
                return Ok(Continue(()));
            }
            prefix.clear();
            encode_values(
                &CompiledTheory::group_key(sides.target_binding, row),
                &mut prefix,
            );
            if affected.contains(&prefix) {
                let token = tokens.token_of(&prefix);
                put_span(&mut spans, token, &statement.target, position, row);
            }
            Ok(Continue(()))
        })?;
        let runs = coverage_runs(&spans, self.work)?;
        self.for_each_row(state, statement.source.relation, |judge, _rank, row| {
            if !satisfies(&statement.source, row) {
                return Ok(Continue(()));
            }
            prefix.clear();
            encode_values(
                &CompiledTheory::group_key(sides.source_binding, row),
                &mut prefix,
            );
            if !affected.contains(&prefix) {
                return Ok(Continue(()));
            }
            let span = &row[usize::from(statement.source.projection[position].0)];
            let witnessed = tokens
                .lookup_token(&prefix)
                .is_some_and(|token| run_covers(token, span, &runs));
            if !witnessed {
                pending.violated = true;
                judge.offer(pending, statement.source.relation, row)?;
            }
            Ok(Continue(()))
        })
    }

    fn unindexed_matches<S: Facts>(
        &mut self,
        state: &S,
        side: &Side,
        binding: &ProjectionBinding,
        det: &[Value],
    ) -> Result<bool> {
        let mut found = false;
        self.for_each_row(state, side.relation, |_judge, _rank, row| {
            found =
                satisfies(side, row) && CompiledTheory::group_key(binding, row).as_slice() == det;
            Ok(if found { Break(()) } else { Continue(()) })
        })?;
        Ok(found)
    }

    fn offer_unindexed_unsatisfied<S: Facts>(
        &mut self,
        state: &S,
        side: &Side,
        binding: &ProjectionBinding,
        det: &[Value],
        pending: &mut PendingViolation,
    ) -> Result<()> {
        self.for_each_row(state, side.relation, |judge, _rank, row| {
            if satisfies(side, row) && CompiledTheory::group_key(binding, row).as_slice() == det {
                pending.violated = true;
                judge.offer(pending, side.relation, row)?;
            }
            Ok(Continue(()))
        })
    }
}

/// A statement's compiled access to both sides.
pub(super) struct Sides<'t> {
    pub(super) theory: &'t CompiledTheory,
    pub(super) source_compiled: Option<&'t CompiledProjection>,
    pub(super) target_compiled: Option<&'t CompiledProjection>,
    pub(super) source_binding: &'t ProjectionBinding,
    pub(super) target_binding: &'t ProjectionBinding,
}

impl Sides<'_> {
    /// Whether the state indexes both sides, probed with one determinant.
    pub(super) fn live<S: DeltaFacts>(&self, state: &S, sample: &[Value]) -> Result<Indexed> {
        Judge::compiled_indexes_live(
            state,
            self.theory,
            self.source_binding,
            self.target_binding,
            sample,
        )
    }
}

fn put_span(spans: &mut GroupedMap, token: u64, side: &Side, position: usize, row: &[Value]) {
    let (start, end) = interval_order_words(&row[usize::from(side.projection[position].0)])
        .expect("positional typing pairs interval positions");
    spans.put(&span_key(token, start, end, 0), &[]);
}

/// Merges each token's spans into maximal runs keyed `(token, run start)`
/// with the run end as value; a span starting at or before the current run
/// end extends it.
fn coverage_runs(spans: &GroupedMap, work: &WorkContext) -> Result<GroupedMap> {
    let mut runs = GroupedMap::default();
    let mut current: Option<(u64, u64, u64)> = None;
    spans.for_each(|key, _| {
        work.checkpoint()?;
        let (token, start, end, _) = parse_span_key(key);
        match current {
            Some((run_token, run_start, run_end)) if run_token == token && start <= run_end => {
                current = Some((run_token, run_start, run_end.max(end)));
            }
            _ => {
                if let Some((run_token, run_start, run_end)) = current {
                    runs.put(&run_key(run_token, run_start), &run_end.to_be_bytes());
                }
                current = Some((token, start, end));
            }
        }
        Ok(Continue(()))
    })?;
    if let Some((run_token, run_start, run_end)) = current {
        runs.put(&run_key(run_token, run_start), &run_end.to_be_bytes());
    }
    Ok(runs)
}

/// Whether the run of `token` starting at or before the span's start
/// reaches its end.
fn run_covers(token: u64, span: &Value, runs: &GroupedMap) -> bool {
    let (span_start, span_end) =
        interval_order_words(span).expect("positional typing pairs interval positions");
    runs.last_at_or_before(&run_key(token, span_start))
        .is_some_and(|(key, end)| {
            key[..8] == token.to_be_bytes()
                && u64::from_be_bytes(end.try_into().expect("8-byte run end")) >= span_end
        })
}

fn run_key(token: u64, start: u64) -> [u8; 16] {
    let mut key = [0u8; 16];
    key[..8].copy_from_slice(&token.to_be_bytes());
    key[8..].copy_from_slice(&start.to_be_bytes());
    key
}
