//! The linear-reach driver (`docs/architecture/40-execution.md` § the
//! linear reach driver): one rec SCC, interiors then
//! rec then main. Round 0 runs the base arms through the ordinary rule
//! loop; rounds ≥ 1 run each rec arm's one delta variant against the
//! watermark frontier. An empty Δ ends the rec
//! (`lean/Bumbledb/Exec/Reach.lean: evalLinearReach_eq_lfp`).
//! Interiors-only never enters this module's loop.

use std::sync::Arc;

use super::run_join::run_join;
use super::{
    Bindings, EitherSink, PreparedBody, PreparedInterior, PreparedQuery, PreparedRule,
    ProjectionSink,
};
use crate::error::{Error, Result};
use crate::exec::run::Counters;
use crate::image::cache::ImageCache;
use crate::image::view::Const;
use crate::image::{RelationImage, TransientImage};
use crate::ir::normalize::OccId;
use crate::schema::Schema;
use crate::storage::env::ReadTxn;
use bumbledb_theory::TypeDesc;

/// The default rec-round budget. Generous by the safety theorem's own
/// measure: a real closure's round count is its graph's diameter.
pub const DEFAULT_REACH_ROUNDS: u32 = 1 << 16;

/// The default derived-tuple budget: the 10⁷-row scale axiom, applied
/// to every derived sink (each interior, then the rec table).
pub const DEFAULT_DERIVED_TUPLES: u64 = 10_000_000;

/// The rec SCC's prepared artifact: base arms (round 0), rec arms
/// (rounds ≥ 1, each one [`super::DeltaVariant`]), and the rec's
/// projection sink. Budget lives on [`PreparedQuery`].
pub(crate) struct ReachDriver {
    pub(super) base: Vec<PreparedRule>,
    pub(super) rec: Vec<PreparedRule>,
    /// Main/answer rules, run after the rec closes. The rec is never
    /// the answer predicate.
    pub(super) main: Vec<PreparedRule>,
    pub(super) field_types: Vec<TypeDesc>,
    pub(super) sink: ProjectionSink,
    pub(super) units: usize,
    pub(super) scratch: ReachScratch,
}

/// Finished derived images (interiors then rec) plus the per-occurrence
/// bind scratch `run_join`'s Interior arm consumes. Interior count is
/// data — not a 16-slot array.
#[derive(Default)]
pub(super) struct DerivedScratch {
    finished_slot: Vec<TransientImage>,
    finished: Vec<Option<Arc<RelationImage>>>,
    pub(super) occ_images: Vec<Option<Arc<RelationImage>>>,
    pub(super) retired: Vec<Vec<u32>>,
}

impl DerivedScratch {
    fn begin(&mut self, derived_count: usize) {
        self.finished_slot
            .resize_with(derived_count, Default::default);
        self.finished.clear();
        self.finished.resize(derived_count, None);
        self.occ_images.clear();
        self.retired.clear();
    }

    fn stash_finished(
        &mut self,
        id: usize,
        field_types: &[TypeDesc],
        sink: &ProjectionSink,
    ) -> Arc<RelationImage> {
        let image = self.finished_slot[id].refill(field_types, sink.len(), sink.answers_since(0));
        self.finished[id] = Some(image.clone());
        image
    }
}

/// Rec ping-pong: delta vs accumulated of the one SCC. Size 1.
#[derive(Default)]
pub(super) struct ReachScratch {
    delta: [TransientImage; 2],
    acc: [TransientImage; 2],
    acc_filled: [usize; 2],
    flip: bool,
    watermark: usize,
    round_delta: Option<Arc<RelationImage>>,
    round_acc: Option<Arc<RelationImage>>,
}

impl ReachScratch {
    fn begin(&mut self) {
        self.acc_filled = [0; 2];
        self.flip = false;
        self.watermark = 0;
        self.round_delta = None;
        self.round_acc = None;
    }
}

/// Shared per-run context (split borrows of the prepared query).
#[derive(Clone, Copy)]
struct RunCtx<'a> {
    schema: &'a Schema,
    txn: &'a ReadTxn<'a>,
    cache: &'a ImageCache,
    resolved_params: &'a [Const],
    missed_params: &'a [bool],
    fast_eligible: bool,
}

impl<S> PreparedQuery<'_, S> {
    /// Amends this prepared query's derived-tuples / rec-rounds budget.
    /// The rounds axis is inert when `rec` is `None` (rounds never
    /// advance); the tuples axis judges every query — interiors-only
    /// included. Hosts copy-paste.
    pub fn set_derived_budget(&mut self, rounds: u32, tuples: u64) {
        self.rounds_budget = rounds;
        self.tuples_budget = tuples;
    }

    /// Interiors in declaration order, then rec, then main. One derived
    /// tuples ledger. Interiors-only never enters [`Self::run_reach`].
    pub(super) fn run_derived<C: Counters>(
        &mut self,
        txn: &ReadTxn<'_>,
        cache: &ImageCache,
        counters: &mut C,
    ) -> Result<bool> {
        let derived_count =
            self.interiors.len() + usize::from(matches!(self.body, PreparedBody::Reach(_)));
        self.derived.begin(derived_count);
        let fast_eligible = self.unresolved_literals == 0 && self.params.is_empty();
        let mut latched = 0u32;
        let mut ran = false;
        let mut derived_tuples: u64 = 0;

        if !self.interiors.is_empty() {
            let mut interiors_span =
                crate::obs::span(crate::obs::names::INTERIORS, crate::obs::Category::Execute);
            let mut interior_emits: u64 = 0;
            for i in 0..self.interiors.len() {
                unbind_interior_views(&mut self.interiors[i], &mut self.derived.retired);
                self.interiors[i].sink.reset();
                let ctx = RunCtx {
                    schema: self.schema,
                    txn,
                    cache,
                    resolved_params: &self.resolved_params,
                    missed_params: &self.missed_params,
                    fast_eligible,
                };
                for rule_idx in 0..self.interiors[i].rules.len() {
                    fill_finished_images(
                        &self.interiors[i].rules[rule_idx],
                        None,
                        &mut self.derived,
                    );
                    let occ_images = std::mem::take(&mut self.derived.occ_images);
                    let mut retired = std::mem::take(&mut self.derived.retired);
                    let units = self.interiors[i].units;
                    let interior = &mut self.interiors[i];
                    ran |= run_into_projection(
                        ctx,
                        &mut interior.rules,
                        rule_idx,
                        None,
                        units,
                        &occ_images,
                        &mut retired,
                        &mut interior.sink,
                        &mut self.bindings,
                        &mut self.determinant_key,
                        &mut latched,
                        counters,
                    )?;
                    self.derived.occ_images = occ_images;
                    self.derived.retired = retired;
                }
                derived_tuples += self.interiors[i].sink.len() as u64;
                interior_emits += self.interiors[i].sink.len() as u64;
                if derived_tuples > self.tuples_budget {
                    return Err(Error::DerivedBudgetExceeded {
                        rounds: 0,
                        tuples: derived_tuples,
                    });
                }
                let types = self.interiors[i].field_types.clone();
                self.derived
                    .stash_finished(i, &types, &self.interiors[i].sink);
                if ran {
                    self.run_interior_ray_probes(i, txn, cache, counters)?;
                }
            }
            interiors_span.set_args(self.interiors.len() as u64, interior_emits);
        }

        let rec_ran = if matches!(self.body, PreparedBody::Reach(_)) {
            self.run_reach(txn, cache, counters, &mut latched, derived_tuples)?
        } else {
            false
        };
        ran |= rec_ran;

        self.unresolved_literals = self.unresolved_literals.saturating_sub(latched);
        Ok(ran)
    }

    fn run_interior_ray_probes<C: Counters>(
        &mut self,
        interior: usize,
        txn: &ReadTxn<'_>,
        cache: &ImageCache,
        counters: &mut C,
    ) -> Result<()> {
        let fast_eligible = self.unresolved_literals == 0 && self.params.is_empty();
        let mut latched = 0u32;
        // Split the probes out so we can fill images against the plan.
        let mut sets = std::mem::take(&mut self.interiors[interior].ray_probes);
        for set in &mut sets {
            set.verdict.resolve_interns(txn)?;
            for probe in &mut set.probes {
                fill_plan_images(&probe.rule.plan, &mut self.derived, None, None, None);
                let resolved =
                    if fast_eligible && probe.rule.resolution == super::ResolutionState::Complete {
                        true
                    } else {
                        let complete = super::bind::resolve_filters(
                            txn,
                            &mut probe.rule.plan,
                            &self.resolved_params,
                            &self.missed_params,
                            &mut probe.rule.resolved_filters,
                            &mut probe.rule.resolved_selections,
                            &mut latched,
                        )?;
                        probe.rule.resolution = if complete {
                            super::ResolutionState::Complete
                        } else {
                            super::ResolutionState::Pending
                        };
                        complete
                    };
                if !resolved {
                    continue;
                }
                self.bindings.resize(probe.rule.plan.slot_count());
                let mut arbiter = crate::exec::verdict::RayArbiter::new(
                    &set.verdict,
                    &self.resolved_params,
                    probe.measured_slot,
                );
                run_join(
                    &probe.rule.plan,
                    self.schema,
                    txn,
                    cache,
                    &mut probe.rule.executor,
                    &mut self.bindings,
                    &probe.rule.resolved_filters,
                    &probe.rule.resolved_selections,
                    &mut probe.rule.memo,
                    &self.derived.occ_images,
                    &mut self.derived.retired,
                    &mut arbiter,
                    counters,
                )?;
                if let Some([start, end]) = arbiter.measure_of_ray() {
                    self.interiors[interior].ray_probes = sets;
                    return Err(crate::error::Error::MeasureOfRay { start, end });
                }
            }
        }
        self.interiors[interior].ray_probes = sets;
        self.unresolved_literals = self.unresolved_literals.saturating_sub(latched);
        Ok(())
    }

    /// Rec least fixpoint. Interiors-only never calls this.
    ///
    /// # Panics
    ///
    /// On a programmer-invariant violation: dispatched only when body
    /// is [`PreparedBody::Reach`].
    #[expect(
        clippy::too_many_lines,
        reason = "the driver reads as one protocol: reset, round 0, Δ loop, budget"
    )]
    fn run_reach<C: Counters>(
        &mut self,
        txn: &ReadTxn<'_>,
        cache: &ImageCache,
        counters: &mut C,
        latched: &mut u32,
        mut derived_tuples: u64,
    ) -> Result<bool> {
        let PreparedBody::Reach(_) = &self.body else {
            unreachable!("run_reach is dispatched only on PreparedBody::Reach")
        };
        let rec_id = self.interiors.len();
        let fast_eligible = self.unresolved_literals == 0 && self.params.is_empty();
        let mut ran = false;
        let mut reach_span =
            crate::obs::span(crate::obs::names::REACH, crate::obs::Category::Execute);

        {
            let PreparedBody::Reach(driver) = &mut self.body else {
                unreachable!("matched above")
            };
            driver.sink.reset();
            driver.scratch.begin();
            for rule in &mut driver.base {
                if let PreparedRule::FreeJoin(fj) = rule {
                    unbind_interior_rule(fj, &mut self.derived.retired);
                }
            }
            for rule in &mut driver.rec {
                if let PreparedRule::Recursive(rec) = rule {
                    unbind_interior_rule(&mut rec.variant.rule, &mut self.derived.retired);
                }
            }
        }

        let ctx = RunCtx {
            schema: self.schema,
            txn,
            cache,
            resolved_params: &self.resolved_params,
            missed_params: &self.missed_params,
            fast_eligible,
        };

        let mut round_span = Some(crate::obs::span(
            crate::obs::names::FIXPOINT_ROUND,
            crate::obs::Category::Execute,
        ));
        let mut round_emits_before = counters.emits();

        // Round 0: base arms into the rec sink.
        {
            let PreparedBody::Reach(driver) = &mut self.body else {
                unreachable!("matched above")
            };
            for rule_idx in 0..driver.base.len() {
                fill_finished_images(&driver.base[rule_idx], None, &mut self.derived);
                ran |= run_into_projection(
                    ctx,
                    &mut driver.base,
                    rule_idx,
                    None,
                    driver.units,
                    &self.derived.occ_images,
                    &mut self.derived.retired,
                    &mut driver.sink,
                    &mut self.bindings,
                    &mut self.determinant_key,
                    latched,
                    counters,
                )?;
            }
            let emitted = counters.emits() - round_emits_before;
            let newly = driver.sink.len() as u64;
            counters.fixpoint_round(emitted, emitted.saturating_sub(newly));
            if let Some(mut span) = round_span.take() {
                span.set_args(emitted, emitted.saturating_sub(newly));
            }
        }

        let mut rounds: u32 = 0;
        loop {
            let PreparedBody::Reach(driver) = &mut self.body else {
                unreachable!("matched above")
            };
            let len = driver.sink.len();
            let tuples = derived_tuples + len as u64;
            let any_delta = len > driver.scratch.watermark;
            if !any_delta {
                self.derived
                    .stash_finished(rec_id, &driver.field_types, &driver.sink);
                reach_span.set_args(u64::from(rounds), tuples);
                break;
            }
            if rounds >= self.rounds_budget || tuples > self.tuples_budget {
                return Err(Error::DerivedBudgetExceeded { rounds, tuples });
            }
            rounds += 1;
            round_span = Some(crate::obs::span(
                crate::obs::names::FIXPOINT_ROUND,
                crate::obs::Category::Execute,
            ));
            round_emits_before = counters.emits();
            let flip = usize::from(driver.scratch.flip);
            let since = driver.scratch.watermark;
            counters.fixpoint_delta((len - since) as u64);
            driver.scratch.round_delta = Some(driver.scratch.delta[flip].refill(
                &driver.field_types,
                len - since,
                driver.sink.answers_since(since),
            ));
            let filled = driver.scratch.acc_filled[flip];
            driver.scratch.round_acc = Some(driver.scratch.acc[flip].append(
                &driver.field_types,
                filled,
                len,
                |from| driver.sink.answers_since(from),
            ));
            driver.scratch.acc_filled[flip] = len;
            driver.scratch.flip = !driver.scratch.flip;
            driver.scratch.watermark = len;

            let rec_delta = driver.scratch.round_delta.clone();
            let rec_acc = driver.scratch.round_acc.clone();
            for rule_idx in 0..driver.rec.len() {
                let PreparedRule::Recursive(rule) = &driver.rec[rule_idx] else {
                    continue;
                };
                let delta = rule.variant.delta;
                fill_plan_images(
                    &rule.variant.rule.plan,
                    &mut self.derived,
                    Some(rec_id),
                    rec_delta.as_ref(),
                    rec_acc.as_ref(),
                );
                ran |= run_into_projection(
                    ctx,
                    &mut driver.rec,
                    rule_idx,
                    Some(delta),
                    driver.units,
                    &self.derived.occ_images,
                    &mut self.derived.retired,
                    &mut driver.sink,
                    &mut self.bindings,
                    &mut self.determinant_key,
                    latched,
                    counters,
                )?;
            }
            let emitted = counters.emits() - round_emits_before;
            let newly = (driver.sink.len() - driver.scratch.watermark) as u64;
            counters.fixpoint_round(emitted, emitted.saturating_sub(newly));
            if let Some(mut span) = round_span.take() {
                span.set_args(emitted, emitted.saturating_sub(newly));
            }
            derived_tuples = tuples;
        }
        Ok(ran)
    }

    /// Fills `derived.occ_images` for a main-rule plan from finished
    /// interiors (and finished rec).
    pub(super) fn fill_main_images(&mut self, rule_idx: usize) {
        let Some(plan) = main_plan(&self.body, rule_idx) else {
            self.derived.occ_images.clear();
            return;
        };
        fill_plan_images(plan, &mut self.derived, None, None, None);
    }
}

fn main_plan(body: &PreparedBody, rule_idx: usize) -> Option<&crate::plan::fj::ValidatedPlan> {
    let rules = body.rules();
    match rules.get(rule_idx)? {
        PreparedRule::FreeJoin(rule) => Some(&rule.plan),
        PreparedRule::KeyProbe(_) | PreparedRule::Recursive(_) => None,
    }
}

fn unbind_interior_views(interior: &mut PreparedInterior, retired: &mut Vec<Vec<u32>>) {
    for rule in &mut interior.rules {
        if let PreparedRule::FreeJoin(fj) = rule {
            unbind_interior_rule(fj, retired);
        }
    }
}

fn unbind_interior_rule(rule: &mut super::FreeJoinRule, retired: &mut Vec<Vec<u32>>) {
    for (occ_idx, occurrence) in rule.plan.occurrences().iter().enumerate() {
        if occurrence.role.discharged() || occurrence.source.edb().is_some() {
            continue;
        }
        let old = rule.memo.colts[occ_idx].reset(crate::image::view::View::Unbound);
        let recycled = old.recycle();
        let spare = &mut rule.memo.spare_buffers[occ_idx];
        if spare.capacity() == 0 {
            *spare = recycled;
        } else if recycled.capacity() > 0 {
            retired.push(recycled);
        }
    }
}

fn fill_finished_images(
    rule: &PreparedRule,
    variant_delta: Option<OccId>,
    derived: &mut DerivedScratch,
) {
    let plan = match rule {
        PreparedRule::FreeJoin(rule) => &rule.plan,
        PreparedRule::Recursive(rule) => &rule.variant.rule.plan,
        PreparedRule::KeyProbe(_) => {
            derived.occ_images.clear();
            return;
        }
    };
    let _ = variant_delta;
    fill_plan_images(plan, derived, None, None, None);
}

fn fill_plan_images(
    plan: &crate::plan::fj::ValidatedPlan,
    derived: &mut DerivedScratch,
    rec_id: Option<usize>,
    rec_delta: Option<&Arc<RelationImage>>,
    rec_acc: Option<&Arc<RelationImage>>,
) {
    derived.occ_images.clear();
    derived.occ_images.resize(plan.occurrences().len(), None);
    for (occ_idx, occurrence) in plan.occurrences().iter().enumerate() {
        if occurrence.role.discharged() {
            continue;
        }
        let Some(target) = occurrence.source.interior() else {
            continue;
        };
        let q = target.index();
        let image = if rec_id == Some(q) {
            if rec_delta.is_some() && rec_acc.is_some() {
                // Rec self: the caller marks the delta occurrence by
                // filling after this helper for the delta occ. Here we
                // default to accumulated; the delta occ is overwritten
                // below when rec_delta is present for that occ_id.
                rec_acc.cloned()
            } else {
                derived.finished[q].clone()
            }
        } else {
            derived.finished[q].clone()
        };
        derived.occ_images[occ_idx] = image;
    }
    if let (Some(rec_id), Some(delta_img)) = (rec_id, rec_delta) {
        for (occ_idx, occurrence) in plan.occurrences().iter().enumerate() {
            if occurrence.role.discharged() {
                continue;
            }
            let Some(target) = occurrence.source.interior() else {
                continue;
            };
            if target.index() == rec_id {
                // Unique self-atom is the delta — overwrite acc default.
                derived.occ_images[occ_idx] = Some(delta_img.clone());
            }
        }
        let _ = rec_acc;
    }
}

/// Runs one plan unit into a projection sink (interior or rec).
#[expect(
    clippy::too_many_arguments,
    reason = "the prepared query's split borrows are clearer unpacked"
)]
fn run_into_projection<C: Counters>(
    ctx: RunCtx<'_>,
    rules: &mut [PreparedRule],
    rule_idx: usize,
    rec_delta: Option<OccId>,
    units: usize,
    occ_images: &[Option<Arc<RelationImage>>],
    retired: &mut Vec<Vec<u32>>,
    sink: &mut ProjectionSink,
    bindings: &mut Bindings,
    determinant_key: &mut Vec<u8>,
    latched: &mut u32,
    counters: &mut C,
) -> Result<bool> {
    let _ = rec_delta;
    let multi_unit = units > 1;
    if let PreparedRule::KeyProbe(rule) = &rules[rule_idx] {
        bindings.resize(rule.plan.slot_count());
        if multi_unit {
            sink.aim(&rule.finds, rule.plan.slot_count());
        }
        crate::exec::dispatch::execute_key_probe(
            &rule.plan,
            ctx.txn,
            ctx.schema,
            ctx.resolved_params,
            determinant_key,
            bindings,
            sink,
            counters,
        )?;
        return Ok(true);
    }
    let rule = match &mut rules[rule_idx] {
        PreparedRule::FreeJoin(rule) => rule,
        PreparedRule::Recursive(rule) => &mut rule.variant.rule,
        PreparedRule::KeyProbe(_) => unreachable!("handled above"),
    };
    bindings.resize(rule.plan.slot_count());
    let resolved = if ctx.fast_eligible && rule.resolution == super::ResolutionState::Complete {
        true
    } else {
        let complete = super::bind::resolve_filters(
            ctx.txn,
            &mut rule.plan,
            ctx.resolved_params,
            ctx.missed_params,
            &mut rule.resolved_filters,
            &mut rule.resolved_selections,
            latched,
        )?;
        rule.resolution = if complete {
            super::ResolutionState::Complete
        } else {
            super::ResolutionState::Pending
        };
        complete
    };
    if !resolved {
        return Ok(false);
    }
    rule.executor.bind_allen_masks(ctx.resolved_params);
    if multi_unit {
        sink.aim(&rule.finds, rule.plan.slot_count());
    }
    run_join(
        &rule.plan,
        ctx.schema,
        ctx.txn,
        ctx.cache,
        &mut rule.executor,
        bindings,
        &rule.resolved_filters,
        &rule.resolved_selections,
        &mut rule.memo,
        occ_images,
        retired,
        sink,
        counters,
    )?;
    Ok(true)
}

/// Suppress unused-import warning if EitherSink is only needed by execute.
#[allow(dead_code)]
fn _either_sink_marker(_: &EitherSink) {}
