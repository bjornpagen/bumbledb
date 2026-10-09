//! The one rule runner: main rules, interior rules and rec arms resolve their
//! literals, aim a shared sink, bind their views and join through here.
use super::reach::OccImages;
use super::{
    Bindings, FilterPredicate, FreeJoinRule, PreparedRule, ResolutionState, Schema, ViewMemo,
};

use crate::error::Result;
use crate::exec::run::{Counters, Sink};
use crate::exec::sink::FindSpec;
use crate::image::intern::InternerHandle;
use crate::image::view::{Const, ResolvedWords, apply};
use crate::image::{SourceImages, ViewEpoch};
use crate::plan::fj::ScalarSetTraversal;

/// What one execution's rule runs read: sources and bound constants.
pub(super) struct RuleCtx<'a> {
    pub(super) schema: &'a Schema,
    pub(super) images: &'a SourceImages<'a>,
    pub(super) interner: &'a InternerHandle<'a>,
    pub(super) params: &'a [Const],
    pub(super) missed: &'a [bool],
}

/// The scratch one rule run borrows.
pub(super) struct RuleScratch<'a> {
    pub(super) bindings: &'a mut Bindings,
    pub(super) key_scratch: &'a mut ResolvedWords,
    pub(super) occ_images: &'a OccImages,
    pub(super) retired: &'a mut Vec<Vec<u32>>,
}

/// How a rule relates to the sink it emits into.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SinkUse {
    /// Several rules emit into the sink; it is aimed at each in turn.
    Shared,
    /// The only rule of an interior or the rec base.
    Sole,
    /// The only main rule: a scalar-set traversal may stand in for dedup.
    SoleMain,
}

/// A sink a rule can emit into.
pub(super) trait RuleSink: Sink {
    fn aim_rule(&mut self, finds: &[FindSpec], slot_count: usize, spans: &[(usize, usize)]);

    /// Accept a physical distinct-traversal proof for one invocation; returns
    /// the proof the sink can use.
    fn set_physical_distinct(
        &mut self,
        witness: Option<ScalarSetTraversal>,
    ) -> Option<ScalarSetTraversal> {
        let _ = witness;
        None
    }
}

impl RuleSink for crate::exec::sink::ProjectionSink {
    fn aim_rule(&mut self, finds: &[FindSpec], _slot_count: usize, _spans: &[(usize, usize)]) {
        self.aim(finds);
    }
}

impl RuleSink for super::EitherSink {
    fn aim_rule(&mut self, finds: &[FindSpec], slot_count: usize, spans: &[(usize, usize)]) {
        self.aim(finds, slot_count, spans);
    }

    fn set_physical_distinct(
        &mut self,
        witness: Option<ScalarSetTraversal>,
    ) -> Option<ScalarSetTraversal> {
        match self {
            Self::Aggregate(sink) => sink.set_physical_distinct(witness),
            Self::Computed(_) | Self::Projection(_) => None,
        }
    }
}

/// Run one prepared rule into `sink`. `Ok(false)` when a missed parameter
/// leaves the rule nothing to join.
pub(super) fn run_rule<S: RuleSink, C: Counters>(
    ctx: &RuleCtx<'_>,
    scratch: &mut RuleScratch<'_>,
    rule: &mut PreparedRule,
    sink_use: SinkUse,
    sink: &mut S,
    counters: &mut C,
) -> Result<bool> {
    match rule {
        PreparedRule::KeyProbe(rule) => {
            scratch.bindings.resize(rule.plan.slot_count());
            if sink_use == SinkUse::Shared {
                sink.aim_rule(&rule.finds, rule.plan.slot_count(), &rule.dedup_spans);
            }
            crate::exec::dispatch::execute_key_probe(
                &rule.plan,
                ctx.images.source(),
                ctx.schema,
                ctx.interner,
                ctx.params,
                &mut rule.row,
                scratch.key_scratch,
                scratch.bindings,
                sink,
                counters,
            )?;
            Ok(true)
        }
        PreparedRule::FreeJoin(rule) => run_free_join(ctx, scratch, rule, sink_use, sink, counters),
    }
}

/// Run one Free Join rule into `sink`.
pub(super) fn run_free_join<S: RuleSink, C: Counters>(
    ctx: &RuleCtx<'_>,
    scratch: &mut RuleScratch<'_>,
    rule: &mut FreeJoinRule,
    sink_use: SinkUse,
    sink: &mut S,
    counters: &mut C,
) -> Result<bool> {
    scratch.bindings.resize(rule.plan.slot_count());
    if !resolve(ctx, rule)? {
        return Ok(false);
    }
    if sink_use == SinkUse::Shared {
        sink.aim_rule(&rule.finds, rule.plan.slot_count(), &rule.dedup_spans);
    }
    let witness = (sink_use == SinkUse::SoleMain)
        .then(|| rule.plan.scalar_set_traversal())
        .flatten();
    let witness = sink.set_physical_distinct(witness);
    rule.executor.set_physical_distinct(witness);
    let joined = join(ctx, scratch, rule, sink, counters);
    // The proof belongs to this invocation; restore before propagating errors.
    rule.executor.set_physical_distinct(None);
    let _ = sink.set_physical_distinct(None);
    joined.map(|()| true)
}

/// Substitute this execution's constants into the rule's filters and
/// selections. A parameter-free rule reuses a completed resolution.
fn resolve(ctx: &RuleCtx<'_>, rule: &mut FreeJoinRule) -> Result<bool> {
    if ctx.params.is_empty() && rule.resolution == ResolutionState::Complete {
        return Ok(true);
    }
    let complete = super::bind::LiteralResolution {
        interner: ctx.interner,
        work: ctx.images.source().work(),
        params: ctx.params,
        missed: ctx.missed,
    }
    .filters(
        &rule.plan,
        &mut rule.resolved_filters,
        &mut rule.resolved_selections,
    )?;
    rule.resolution = if complete {
        ResolutionState::Complete
    } else {
        ResolutionState::Pending
    };
    Ok(complete)
}

/// Bind every occurrence's view, select, and run the executor.
fn join<S: Sink, C: Counters>(
    ctx: &RuleCtx<'_>,
    scratch: &mut RuleScratch<'_>,
    rule: &mut FreeJoinRule,
    sink: &mut S,
    counters: &mut C,
) -> Result<()> {
    let FreeJoinRule {
        plan,
        executor,
        resolved_filters,
        resolved_selections,
        memo,
        ..
    } = rule;
    let plan = &*plan;
    let resolved_filters = &resolved_filters[..];
    let resolved_selections = &resolved_selections[..];
    let schema = ctx.schema;
    let images = ctx.images;
    let work = images.source().work();
    let bindings = &mut *scratch.bindings;
    let derived_images = scratch.occ_images;
    let derived_retired = &mut *scratch.retired;
    memo.tick += 1;

    // Bind the current operation on every COLT before any view reset,
    // force_root, select, or execute. Rebind installs this ledger so a
    // prior execution's refusal cannot poison this one.
    executor.begin_work(work, &mut memo.colts);
    let mut pending_steps = 0u32;

    debug_assert!(
        resolved_filters
            .iter()
            .enumerate()
            .all(|(occ_idx, filters)| {
                plan.is_negated(crate::ir::normalize::OccId(
                    u16::try_from(occ_idx).expect("occurrence ids fit u16"),
                )) || filters.iter().all(|f| {
                    !matches!(
                        f,
                        FilterPredicate::Compare {
                            op: crate::ir::WordCmp::Eq,
                            ..
                        }
                    )
                })
            }),
        "an Eq-constant does not reach a positive occurrence's view filters"
    );
    for (occ_idx, occurrence) in plan.occurrences().iter().enumerate() {
        if occurrence.role.discharged() {
            continue;
        }

        if occurrence.bind.edb().is_none() {
            let image = derived_images.image(occ_idx);
            let mut buffer = std::mem::take(memo.spare_mut(occ_idx));
            if buffer.capacity() == 0
                && let Some(pooled) = derived_retired.pop()
            {
                buffer = pooled;
            }
            let eq = image.generation().text_eq();
            let view = apply(image, &resolved_filters[occ_idx], &[], buffer, eq)?;
            let old = memo.colts[occ_idx].reset(view);
            *memo.spare_mut(occ_idx) = old.recycle();
            debug_assert!(
                memo.is_derived(occ_idx),
                "an Interior occurrence is Derived and never enters the memo"
            );
            checkpoint_join_work(work, &mut pending_steps)?;
            continue;
        }
        let relation = match occurrence.source() {
            crate::ir::AtomSource::Edb(relation) => relation,
            crate::ir::AtomSource::Interior(_) => {
                unreachable!("Interior continued above")
            }
        };

        let epoch = images.epoch(schema, relation)?;

        if memo.bind(
            occ_idx,
            epoch,
            &resolved_filters[occ_idx],
            &resolved_selections[occ_idx],
        ) {
            checkpoint_join_work(work, &mut pending_steps)?;
            continue;
        }

        if let Some(canon) = dedup_source(plan, memo, occ_idx, epoch, resolved_filters) {
            let buffer = std::mem::take(memo.spare_mut(occ_idx));
            let [canon_colt, colt] = memo
                .colts
                .get_disjoint_mut([canon, occ_idx])
                .expect("dedup source is a distinct occurrence");
            canon_colt
                .force_root()
                .map_err(crate::api::prepared::source::work_error)?;
            let old = colt
                .clone_bound_from(canon_colt, buffer)
                .map_err(crate::api::prepared::source::work_error)?;
            *memo.spare_mut(occ_idx) = old.recycle();
            memo.set_bound(occ_idx, epoch, &resolved_filters[occ_idx], None);
            checkpoint_join_work(work, &mut pending_steps)?;
            continue;
        }
        // Prefer an already shared full image. Otherwise an indexed bucket
        // can feed the same COLT, but its coverage belongs to this query's
        // selection-keyed memo, never the relation cache or source dedup.
        let (image, selected) = if let Some(image) = images.peek(schema, relation)? {
            (image, false)
        } else if memo.partial_capacity_exhausted(occ_idx, epoch) {
            (images.image(schema, relation)?, false)
        } else if let Some(image) = images.selection_image(
            schema,
            relation,
            &occurrence.selections,
            &resolved_selections[occ_idx],
        )? {
            (image, true)
        } else {
            (images.image(schema, relation)?, false)
        };
        let buffer = std::mem::take(memo.spare_mut(occ_idx));
        let eq = image.generation().text_eq();
        let view = apply(&image, &resolved_filters[occ_idx], &[], buffer, eq)?;
        let old = memo.colts[occ_idx].reset(view);
        *memo.spare_mut(occ_idx) = old.recycle();
        memo.set_bound(
            occ_idx,
            epoch,
            &resolved_filters[occ_idx],
            selected.then_some(&resolved_selections[occ_idx]),
        );
        checkpoint_join_work(work, &mut pending_steps)?;
    }

    for (occ_idx, keys) in resolved_selections.iter().enumerate() {
        if plan.occurrences()[occ_idx].role.discharged() {
            debug_assert!(
                keys.is_empty(),
                "discharged occurrences carry no selections"
            );
            continue;
        }
        checkpoint_join_work(work, &mut pending_steps)?;
        let selected = memo.colts[occ_idx]
            .select(keys)
            .map_err(crate::api::prepared::source::work_error)?;
        let hit = selected.is_some();
        if !hit {
            return Ok(());
        }
    }
    flush_join_work(work, &mut pending_steps)?;

    executor.execute(plan, &mut memo.colts, bindings, sink, counters)?;
    flush_join_work(work, &mut pending_steps)?;
    Ok(())
}

fn checkpoint_join_work(
    work: &crate::work::WorkContext,
    pending: &mut u32,
) -> crate::error::Result<()> {
    *pending = pending.saturating_add(1);
    if *pending >= crate::exec::sink::STEP_QUANTUM {
        flush_join_work(work, pending)?;
    }
    Ok(())
}

fn flush_join_work(work: &crate::work::WorkContext, pending: &mut u32) -> crate::error::Result<()> {
    if *pending == 0 {
        return Ok(());
    }
    work.checkpoint()
        .map_err(crate::api::prepared::source::work_error)?;
    *pending = 0;
    Ok(())
}

fn dedup_source(
    plan: &crate::plan::fj::ValidatedPlan,
    memo: &ViewMemo,
    occ: usize,
    epoch: ViewEpoch,
    resolved_filters: &[Vec<FilterPredicate>],
) -> Option<usize> {
    let crate::ir::AtomSource::Edb(relation) = plan.occurrences()[occ].source() else {
        return None;
    };
    plan.occurrences()
        .iter()
        .enumerate()
        .position(|(other, occurrence)| {
            other != occ
                && occurrence.source().edb() == Some(relation)
                && memo.active_matches(other, epoch, &resolved_filters[occ])
                && memo.colts[other].same_shape(&memo.colts[occ])
        })
}
