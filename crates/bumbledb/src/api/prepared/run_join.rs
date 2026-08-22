use super::{Bindings, Executor, FilterPredicate, Schema, ViewMemo};

use crate::error::Result;
use crate::image::ImageBind;
use crate::image::ViewEpoch;
use crate::image::view::apply;
use crate::obs;

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
    executor: &mut Executor,
    bindings: &mut Bindings,
    resolved_filters: &[Vec<FilterPredicate>],
    resolved_selections: &[Vec<Vec<u64>>],
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
    let views_span = obs::span(obs::names::VIEWS);
    memo.tick += 1;

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
            let mut build_span = obs::span_args(
                obs::names::VIEW_BUILD,
                obs::TraceArgs::Count(occ_idx as u64),
            );
            let mut buffer = std::mem::take(memo.spare_mut(occ_idx));
            if buffer.capacity() == 0
                && let Some(pooled) = derived_retired.pop()
            {
                buffer = pooled;
            }
            let view = apply(image, &resolved_filters[occ_idx], &[], buffer);
            build_span.set_pair(occ_idx as u64, view.len() as u64);
            let old = memo.colts[occ_idx].reset(view);
            *memo.spare_mut(occ_idx) = old.recycle();
            debug_assert!(
                memo.is_derived(occ_idx),
                "an Interior occurrence is Derived and never enters the memo"
            );
            continue;
        }
        let relation = match occurrence.source() {
            crate::ir::AtomSource::Edb(relation) => relation,
            crate::ir::AtomSource::Interior(_) => {
                unreachable!("Interior continued above")
            }
        };

        let epoch = images.epoch(schema, relation)?;

        if memo.bind(occ_idx, epoch, &resolved_filters[occ_idx]) {
            obs::event(
                obs::names::VIEW_MEMO_HIT,
                obs::TraceArgs::Count(occ_idx as u64),
            );
            continue;
        }

        if let Some(canon) = dedup_source(plan, memo, occ_idx, epoch, resolved_filters) {
            let buffer = std::mem::take(memo.spare_mut(occ_idx));
            let [canon_colt, colt] = memo
                .colts
                .get_disjoint_mut([canon, occ_idx])
                .expect("dedup source is a distinct occurrence");
            canon_colt.force_root();
            let old = colt.clone_bound_from(canon_colt, buffer);
            obs::event(
                obs::names::VIEW_DEDUP,
                obs::TraceArgs::Pair(occ_idx as u64, canon as u64),
            );
            *memo.spare_mut(occ_idx) = old.recycle();
            memo.set_bound(occ_idx, epoch, &resolved_filters[occ_idx]);
            continue;
        }
        let mut build_span = obs::span_args(
            obs::names::VIEW_BUILD,
            obs::TraceArgs::Count(occ_idx as u64),
        );
        let image = images.image(schema, relation)?;
        let buffer = std::mem::take(memo.spare_mut(occ_idx));
        let view = apply(&image, &resolved_filters[occ_idx], &[], buffer);
        build_span.set_pair(occ_idx as u64, view.len() as u64);
        let old = memo.colts[occ_idx].reset(view);
        *memo.spare_mut(occ_idx) = old.recycle();
        memo.set_bound(occ_idx, epoch, &resolved_filters[occ_idx]);
    }
    views_span.end();

    let mut selections_span = obs::span(obs::names::SELECTIONS);
    let mut probed = 0u64;
    for (occ_idx, keys) in resolved_selections.iter().enumerate() {
        if plan.occurrences()[occ_idx].role.discharged() {
            debug_assert!(
                keys.is_empty(),
                "discharged occurrences carry no selections"
            );
            continue;
        }
        let hit = memo.colts[occ_idx].select(keys).is_some();
        probed += 1;
        obs::event(
            obs::names::SELECT_PROBE,
            obs::TraceArgs::Pair(occ_idx as u64, u64::from(hit)),
        );
        if !hit {
            selections_span.set_count(probed);
            return Ok(());
        }
    }
    selections_span.set_pair(probed, 1);
    selections_span.end();
    let _join = obs::span(obs::names::JOIN);

    // their sink enum once per execution BEFORE this call (`run_rule`'s

    executor.execute(plan, &mut memo.colts, bindings, sink, counters)?;
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
