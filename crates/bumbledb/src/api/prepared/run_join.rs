use super::{Bindings, Executor, FilterPredicate, Schema, ViewMemo};

use crate::error::Result;
use crate::image::ImageBind;
use crate::image::ViewEpoch;
use crate::image::view::apply;
use crate::obs;

/// Resets the owned COLT sources against this execution's images and
/// views (buffer ping-pong: old survivor buffers recycle into the new
/// views), then runs the join into the sink.
#[expect(
    clippy::too_many_arguments,
    reason = "the split borrows and execution context are clearer unpacked"
)]
#[expect(
    clippy::too_many_lines,
    reason = "the bind-then-probe-then-join protocol reads as one pass"
)] // the prepared query's split borrows;
// bundling them into a struct would only rename the same ten things
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
    // Lowering routes every positive occurrence's Eq-constant into
    // selections; a leak here would silently resurrect the per-param
    // view scan (docs/architecture/40-execution.md). Two exemptions:
    // negated occurrences, whose Eq-constants ARE view filters — the
    // ordinary filtered view their anti-probes run against, memoized
    // per (generation, resolved filters) like any occurrence
    // (docs/architecture/40-execution.md, § anti-probe filters) — and
    // measured positive occurrences, whose whole filter list
    // `split_filters` pins residual so the Eq runs before the
    // subtraction (the filter-order law,
    // docs/architecture/20-query-ir.md § the measure).
    debug_assert!(
        resolved_filters
            .iter()
            .enumerate()
            .all(|(occ_idx, filters)| {
                plan.is_negated(crate::ir::normalize::OccId(
                    u16::try_from(occ_idx).expect("occurrence ids fit u16"),
                )) || filters.iter().any(|f| {
                    matches!(
                        f,
                        FilterPredicate::DurationCompare { .. }
                            | FilterPredicate::DurationFieldsCompare { .. }
                    )
                }) || filters.iter().all(|f| {
                    !matches!(
                        f,
                        FilterPredicate::Compare {
                            op: crate::ir::WordCmp::Eq,
                            ..
                        }
                    )
                })
            }),
        "an Eq-constant reaches a positive occurrence's view filters only under a measure predicate"
    );
    for (occ_idx, occurrence) in plan.occurrences().iter().enumerate() {
        // A discharged occurrence (grounding-eliminated or grounding-folded) is
        // unreachable at execution — no subatom, no anti-probe — so it
        // earns no view and, above all, no image build
        // (`plan/ground.rs`: skipping this build is the rewrite's
        // payoff; for a fold, the sealed extension was already read at
        // prepare and nothing remains to bind).
        if occurrence.role.discharged() {
            continue;
        }
        // The Interior bind (40-execution.md § the linear reach driver): a transient image is
        // valid for ONE ROUND of ONE EXECUTION — a lifetime the
        // generation vocabulary cannot express — so it lives entirely
        // outside the view-memo axiom's machinery: never
        // `ImageCache::get_or_build`, never `memo.bind`, never parked,
        // never pinned by staleness. The bind is the ordinary miss path
        // — `apply` over the driver-supplied image into a per-round
        // `Colt::reset`, survivor buffers recycled through the existing
        // `spare_buffers` ping-pong — and every generation-keyed
        // mechanism never learns recursion exists
        // (`docs/architecture/40-execution.md` § the linear reach driver).
        if occurrence.bind.edb().is_none() {
            let image = derived_images.image(occ_idx);
            let mut build_span = obs::span_args(
                obs::names::VIEW_BUILD,
                obs::TraceArgs::Count(occ_idx as u64),
            );
            let mut buffer = std::mem::take(&mut memo.spare_buffers[occ_idx]);
            if buffer.capacity() == 0
                && let Some(pooled) = derived_retired.pop()
            {
                // The entry unbind parked the second circulating
                // survivor buffer (one spare slot, two buffers); the
                // first spare-starved rebind takes it back.
                buffer = pooled;
            }
            let view = apply(image, &resolved_filters[occ_idx], &[], buffer);
            build_span.set_pair(occ_idx as u64, view.len() as u64);
            let old = memo.colts[occ_idx].reset(view);
            memo.spare_buffers[occ_idx] = old.recycle();
            debug_assert!(
                memo.epoch[occ_idx].is_none(),
                "an Interior occurrence never enters the memo's epoch table"
            );
            continue;
        }
        let relation = match occurrence.source() {
            crate::ir::AtomSource::Edb(relation) => relation,
            crate::ir::AtomSource::Interior(_) => {
                unreachable!("Interior continued above")
            }
        };
        // Closed → theory identity; frozen → this owned instance;
        // store → the snapshot generation. Identity is checked before
        // the memo uses the epoch, so Frozen cannot alias another owner.
        let epoch = images.epoch(schema, relation)?;
        // Warm fast path: an active or parked binding for this exact
        // (epoch, resolved residual filters) pair — the COLT's view
        // is still exactly right, and so are its forced tries (selections
        // live in the trie, not the view, so param churn never lands
        // here). No cache lock, no filter scan, no re-force.
        if memo.bind(occ_idx, epoch, &resolved_filters[occ_idx]) {
            obs::event(
                obs::names::VIEW_MEMO_HIT,
                obs::TraceArgs::Count(occ_idx as u64),
            );
            continue;
        }
        // The occurrence dedup (docs/architecture/40-execution.md):
        // another occurrence whose ACTIVE binding is this exact
        // (epoch, resolved residual filters) over the same relation
        // with the same trie orientation holds a byte-identical view and
        // byte-identical forced state — a cyclic self-join was scanning
        // and re-forcing the same 428k-row view once per occurrence. The
        // canonical root forces eagerly first (the one force the join
        // was about to pay lazily anyway), then the rebuild is a pool
        // copy instead of an image scan plus a per-occurrence re-force.
        if let Some(canon) = dedup_source(plan, memo, occ_idx, epoch, resolved_filters) {
            let buffer = std::mem::take(&mut memo.spare_buffers[occ_idx]);
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
            memo.spare_buffers[occ_idx] = old.recycle();
            memo.epoch[occ_idx] = Some(epoch);
            memo.filters[occ_idx].clone_from(&resolved_filters[occ_idx]);
            continue;
        }
        let mut build_span = obs::span_args(
            obs::names::VIEW_BUILD,
            obs::TraceArgs::Count(occ_idx as u64),
        );
        let image = images.image(schema, relation)?;
        let buffer = std::mem::take(&mut memo.spare_buffers[occ_idx]);
        let view = apply(&image, &resolved_filters[occ_idx], &[], buffer);
        build_span.set_pair(occ_idx as u64, view.len() as u64);
        let old = memo.colts[occ_idx].reset(view);
        memo.spare_buffers[occ_idx] = old.recycle();
        memo.epoch[occ_idx] = Some(epoch);
        memo.filters[occ_idx].clone_from(&resolved_filters[occ_idx]);
    }
    views_span.end();
    // Selection probes (docs/architecture/40-execution.md): each occurrence's Eq constants
    // resolve to trie keys probed once per execution — set-bound levels
    // probe once per element and union survivors inside `select` — and a
    // miss means no fact matches, so the whole conjunctive query is
    // empty and the join never runs (the sink stays reset: a zero-emit
    // execution).
    // One batched span over the whole loop (Gap A): the probes force
    // selection levels lazily, and without this span that dominant cold
    // cost masqueraded as rule self-time. Zero-cost off, batch
    // granularity — never a span per occurrence, and the per-occurrence
    // probe stays the existing point event.
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
    // The executor monomorphizes per concrete sink type — callers match
    // their sink enum once per execution BEFORE this call (`run_rule`'s
    // `EitherSink` match; the reach driver's rec and interior sinks), so
    // no per-emit enum branch exists on the hot path.
    executor.execute(plan, &mut memo.colts, bindings, sink, counters)?;
    Ok(())
}

/// The occurrence-dedup scan: an occurrence other than `occ` whose
/// *active* binding is exactly (`epoch`, occ's resolved residual
/// filters) over the same relation with the same trie orientation
/// ([`crate::exec::colt::Colt::same_shape`]). Derived and discharged
/// occurrences never bind an epoch, so the epoch check excludes them
/// for free. O(occurrences) compares, only inside the sanctioned
/// rebuild window — the warm path never gets here.
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
                && memo.epoch[other] == Some(epoch)
                && memo.filters[other] == resolved_filters[occ]
                && memo.colts[other].same_shape(&memo.colts[occ])
        })
}
