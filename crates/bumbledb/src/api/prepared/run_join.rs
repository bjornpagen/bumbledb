use super::{Bindings, Executor, FilterPredicate, Schema, ViewMemo};

use crate::error::Result;
use crate::image::ImageBind;
use crate::image::ViewEpoch;
use crate::image::view::apply;

#[expect(
    clippy::too_many_arguments,
    reason = "the split borrows and execution context are clearer unpacked"
)]
#[expect(
    clippy::too_many_lines,
    reason = "the bind-then-probe-then-join protocol reads as one pass"
)]
pub(super) fn run_join<S, C, I>(
    plan: &crate::plan::fj::ValidatedPlan,
    schema: &Schema,
    images: &I,
    work: &crate::work::WorkContext,
    executor: &mut Executor,
    bindings: &mut Bindings,
    resolved_filters: &[Vec<FilterPredicate>],
    resolved_selections: &[Vec<crate::image::view::ResolvedWords>],
    memo: &mut ViewMemo,
    derived_images: &super::reach::OccImages,
    derived_retired: &mut Vec<Vec<u32>>,
    sink: &mut S,
    counters: &mut C,
) -> Result<()>
where
    S: crate::exec::run::Sink,
    C: crate::exec::run::Counters,
    I: ImageBind,
{
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

        // Scratch-backed derived occurrences never reach this arm:
        // `rule_uses_scratch_derived` selects fallback before COLT build,
        // so join/negation walks the sealed stage instead of rematerializing.
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
