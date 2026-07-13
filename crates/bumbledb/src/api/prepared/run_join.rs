use super::{Bindings, EitherSink, Executor, FilterPredicate, Schema, ViewGeneration, ViewMemo};

use crate::error::Result;
use crate::image::cache::ImageCache;
use crate::image::view::apply;
use crate::obs;
use crate::storage::env::ReadTxn;

/// Resets the owned COLT sources against this execution's images and
/// views (buffer ping-pong: old survivor buffers recycle into the new
/// views), then runs the join into the sink.
#[expect(
    clippy::too_many_arguments,
    reason = "the split borrows and execution context are clearer unpacked"
)] // the prepared query's split borrows;
// bundling them into a struct would only rename the same ten things
pub(super) fn run_join<C: crate::exec::run::Counters>(
    plan: &crate::plan::fj::ValidatedPlan,
    schema: &Schema,
    txn: &ReadTxn<'_>,
    cache: &ImageCache,
    executor: &mut Executor,
    bindings: &mut Bindings,
    resolved_filters: &[Vec<FilterPredicate>],
    resolved_selections: &[Vec<Vec<u64>>],
    memo: &mut ViewMemo,
    sink: &mut EitherSink,
    counters: &mut C,
) -> Result<()> {
    let views_span = obs::span(obs::names::VIEWS, obs::Category::Execute);
    let txn_generation = txn.generation()?;
    memo.tick += 1;
    // Lowering routes every positive occurrence's Eq-constant into
    // selections; a leak here would silently resurrect the per-param
    // view scan (docs/architecture/40-execution.md). Negated occurrences
    // are exempt: their Eq-constants ARE view filters — the ordinary
    // filtered view their anti-probes run against, memoized per
    // (generation, resolved filters) like any occurrence
    // (docs/architecture/40-execution.md, § anti-probe filters).
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
                            op: crate::ir::CmpOp::Eq,
                            ..
                        }
                    )
                })
            }),
        "Eq-constant conditions never reach a positive occurrence's view filters"
    );
    for (occ_idx, occurrence) in plan.occurrences().iter().enumerate() {
        // A discharged occurrence (chase-eliminated or chase-folded) is
        // unreachable at execution — no subatom, no anti-probe — so it
        // earns no view and, above all, no image build
        // (`plan/chase.rs`: skipping this build is the rewrite's
        // payoff; for a fold, the sealed extension was already read at
        // prepare and nothing remains to bind).
        if occurrence.role.discharged() {
            continue;
        }
        // A closed relation's view binds to the theory identity rather
        // than a fabricated storage generation, so no commit can stale it.
        let generation = if schema.relation(occurrence.relation).is_closed() {
            ViewGeneration::Closed
        } else {
            ViewGeneration::Storage(txn_generation)
        };
        // Warm fast path: an active or parked binding for this exact
        // (generation, resolved residual filters) pair — the COLT's view
        // is still exactly right, and so are its forced tries (selections
        // live in the trie, not the view, so param churn never lands
        // here). No cache lock, no filter scan, no re-force.
        if memo.bind(occ_idx, generation, &resolved_filters[occ_idx]) {
            obs::event(
                obs::names::VIEW_MEMO_HIT,
                obs::Category::Execute,
                occ_idx as u64,
                0,
            );
            continue;
        }
        let mut build_span = obs::span_args(
            obs::names::VIEW_BUILD,
            obs::Category::Execute,
            occ_idx as u64,
            0,
        );
        let image = cache.get_or_build(txn, schema, occurrence.relation)?;
        let buffer = std::mem::take(&mut memo.spare_buffers[occ_idx]);
        let view = apply(&image, &resolved_filters[occ_idx], &[], buffer)?;
        build_span.set_args(occ_idx as u64, view.len() as u64);
        let old = memo.colts[occ_idx].reset(view);
        memo.spare_buffers[occ_idx] = old.recycle();
        memo.generation[occ_idx] = Some(generation);
        memo.filters[occ_idx].clone_from(&resolved_filters[occ_idx]);
    }
    views_span.end();
    // Selection probes (docs/architecture/40-execution.md): each occurrence's Eq constants
    // resolve to trie keys probed once per execution — set-bound levels
    // probe once per element and union survivors inside `select` — and a
    // miss means no fact matches, so the whole conjunctive query is
    // empty and the join never runs (the sink stays reset: a zero-emit
    // execution).
    for (occ_idx, keys) in resolved_selections.iter().enumerate() {
        if plan.occurrences()[occ_idx].role.discharged() {
            debug_assert!(
                keys.is_empty(),
                "discharged occurrences carry no selections"
            );
            continue;
        }
        let hit = memo.colts[occ_idx].select(keys).is_some();
        obs::event(
            obs::names::SELECT_PROBE,
            obs::Category::Execute,
            occ_idx as u64,
            u64::from(hit),
        );
        if !hit {
            return Ok(());
        }
    }
    let _join = obs::span(obs::names::JOIN, obs::Category::Execute);
    // One match per execution: the executor monomorphizes per concrete
    // sink type — no per-emit enum branch on the hot path.
    match sink {
        EitherSink::Projection(s) => {
            executor.execute(plan, &mut memo.colts, bindings, s, counters)?;
        }
        EitherSink::Aggregate(s) => {
            executor.execute(plan, &mut memo.colts, bindings, s.as_mut(), counters)?;
        }
    }
    Ok(())
}
