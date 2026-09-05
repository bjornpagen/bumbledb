//! The linear-reach driver: one rec SCC, interiors then
//! rec then main. Round 0 runs the base arms through the ordinary rule
//! loop; rounds ≥ 1 run each rec arm against the watermark frontier
//! (the unique self-atom is the marked delta occurrence). An empty Δ ends the rec
//! Interiors-only never enters this module's loop.
//! (`lean/Bumbledb/Exec/Reach.lean: evalLinearReach_eq_lfp`).
use std::sync::Arc;

use super::run_join::run_join;
use super::{
    Bindings, EitherSink, FreeJoinRule, PreparedInterior, PreparedPipeline, PreparedQuery,
    PreparedRule, ProjectionSink, RecArm,
};
use crate::error::{Error, Result};
use crate::exec::run::Counters;
use crate::exec::sink::FindSpec;
use crate::image::SourceImages;
use crate::image::intern::InternerHandle;
use crate::image::view::Const;
use crate::image::{RelationImage, TransientImage};
use crate::schema::Schema;
use bumbledb_theory::schema::ValueType;

/// The one aim surface the derived rule loop needs: reach's sink stays a
/// plain projection (the recursive cycle is projection-only by type),
/// while an interior's stage sink is the full [`EitherSink`].
pub(super) trait StageSink: crate::exec::run::Sink {
    fn aim_stage(&mut self, finds: &[FindSpec], slot_count: usize, spans: &[(usize, usize)]);
}

impl StageSink for ProjectionSink {
    fn aim_stage(&mut self, finds: &[FindSpec], slot_count: usize, _spans: &[(usize, usize)]) {
        self.aim(finds, slot_count);
    }
}

impl StageSink for EitherSink {
    fn aim_stage(&mut self, finds: &[FindSpec], slot_count: usize, spans: &[(usize, usize)]) {
        self.aim(finds, slot_count, spans);
    }
}

/// The default rec-round budget. Generous by the safety theorem's own
/// measure: a real closure's round count is its graph's diameter.
pub const DEFAULT_REACH_ROUNDS: u32 = 1 << 16;

/// The default derived-tuple budget: the 10⁷-row scale axiom, applied
/// to every derived sink (each interior, then the rec table).
pub const DEFAULT_DERIVED_TUPLES: u64 = 10_000_000;

pub(crate) struct ReachDriver {
    pub(super) base: Vec<PreparedRule>,
    pub(super) rec: Vec<RecArm>,
    pub(super) field_types: Vec<ValueType>,
    pub(super) sink: crate::exec::sink::ProjectionSink,
    pub(super) units: usize,
    pub(super) scratch: RecPingPong,
}

#[derive(Default)]
pub(super) struct OccImages {
    slots: Vec<(usize, Arc<RelationImage>)>,
}

impl OccImages {
    pub(super) fn clear(&mut self) {
        self.slots.clear();
    }

    fn insert(&mut self, occ_idx: usize, image: Arc<RelationImage>) {
        self.slots.push((occ_idx, image));
    }

    pub(super) fn image(&self, occ_idx: usize) -> &Arc<RelationImage> {
        for (i, img) in &self.slots {
            if *i == occ_idx {
                return img;
            }
        }
        unreachable!("fill_plan_images wrote every live derived occurrence")
    }
}

#[derive(Clone, Copy)]
enum DerivedBind<'a> {
    Finished,
    Rec {
        delta: &'a Arc<RelationImage>,
        acc: &'a Arc<RelationImage>,
    },
}

/// One derived-image protocol: a working transient per derived id, a
/// seal-ordered published `Arc` grown as each table closes, and the
/// per-occurrence bind scratch `run_join` consumes.
#[derive(Default)]
pub(super) struct DerivedImages {
    working: Vec<TransientImage>,
    pub(super) published: Vec<Arc<RelationImage>>,
    pub(super) occ_images: OccImages,
    pub(super) retired: Vec<Vec<u32>>,
}

impl DerivedImages {
    fn begin(&mut self, derived_count: usize) {
        self.working.resize_with(derived_count, Default::default);
        self.published.clear();
        self.occ_images.clear();
        self.retired.clear();
    }

    /// Seal one finished projection stage/rec table: drains the sink's
    /// distinct rows ACROSS BOTH TIERS (a spilled rec seen-set refills the
    /// sealed image from its scratch log), with the image slabs charged
    /// before allocation.
    fn stash_finished(
        &mut self,
        id: usize,
        field_types: &[ValueType],
        sink: &mut ProjectionSink,
        work: &crate::work::WorkContext,
    ) -> Result<Arc<RelationImage>> {
        debug_assert_eq!(
            self.published.len(),
            id,
            "derived tables seal in declaration order"
        );
        let rows = sink.len();
        let image =
            self.working[id].refill_drained(Some(work), field_types, rows, |_, write| {
                sink.drain_since(0, write)
            })?;
        self.published.push(image.clone());
        Ok(image)
    }

    /// Seal an aggregate/computed stage's finalized rows (flat, row-major
    /// in `scratch`, `row_words` words each). Slabs charged before
    /// allocation, like every other image build.
    fn stash_rows(
        &mut self,
        id: usize,
        field_types: &[ValueType],
        scratch: &[u64],
        row_words: usize,
        work: &crate::work::WorkContext,
    ) -> Result<Arc<RelationImage>> {
        debug_assert_eq!(
            self.published.len(),
            id,
            "derived tables seal in declaration order"
        );
        let count = if row_words == 0 {
            0
        } else {
            debug_assert_eq!(scratch.len() % row_words, 0, "whole finalized rows");
            scratch.len() / row_words
        };
        let image =
            self.working[id].refill_drained(Some(work), field_types, count, |_, write| {
                for row in scratch.chunks_exact(row_words) {
                    write(row);
                }
                Ok(())
            })?;
        self.published.push(image.clone());
        Ok(image)
    }
}

/// One stage's row width in image words: the same slot arithmetic the
/// binding layout uses (interval/Pack and Id128 columns are two words).
fn stage_row_words(field_types: &[ValueType]) -> usize {
    field_types
        .iter()
        .map(|ty| crate::ir::normalize::SlotWidth::of(ty).slots())
        .sum()
}

/// Seal one finished interior: a projection stage refills straight from
/// its sink; an aggregate/computed stage FINALIZES here — its overflow /
/// cardinality / scalar errors surface now, before any consumer runs
/// (the producer error boundary; a later filter cannot hide them).
/// Returns the sealed row count for the derived-tuples budget.
fn seal_interior(
    interior: &mut PreparedInterior,
    id: usize,
    derived: &mut DerivedImages,
    answer_scratch: &mut Vec<u64>,
    tuples_so_far: u64,
    tuples_budget: u64,
    work: &crate::work::WorkContext,
) -> Result<u64> {
    // The derived-tuples budget is judged BEFORE finalization/refill
    // materializes anything further (charge before growth, Q-BUDGET);
    // group counts and projection lengths are already known.
    let over = |rows: u64| -> Result<()> {
        let total = tuples_so_far.saturating_add(rows);
        if total > tuples_budget {
            return Err(Error::DerivedBudgetExceeded {
                rounds: 0,
                tuples: total,
            });
        }
        Ok(())
    };
    let row_words = stage_row_words(&interior.field_types);
    let field_types = &interior.field_types;
    let scratch = &mut interior.row_scratch;
    let mut seal_aggregate = |sink: &mut crate::exec::sink::AggregateSink| -> Result<u64> {
        over(sink.group_count() as u64)?;
        scratch.clear();
        sink.finalize_into(answer_scratch, |row| {
            scratch.extend_from_slice(row);
            Ok(())
        })?;
        let image = derived.stash_rows(id, field_types, scratch, row_words, work)?;
        Ok(image.row_count() as u64)
    };
    match &mut interior.sink {
        EitherSink::Projection(sink) => {
            let rows = sink.len() as u64;
            over(rows)?;
            derived.stash_finished(id, field_types, sink, work)?;
            Ok(rows)
        }
        EitherSink::Computed(computed) => {
            if let Some(error) = &computed.error {
                return Err(error.clone());
            }
            match &mut computed.inner {
                EitherSink::Projection(sink) => {
                    let rows = sink.len() as u64;
                    over(rows)?;
                    derived.stash_finished(id, field_types, sink, work)?;
                    Ok(rows)
                }
                EitherSink::Aggregate(sink) => seal_aggregate(sink),
                EitherSink::Computed(_) => {
                    unreachable!("the computed adapter never nests itself")
                }
            }
        }
        EitherSink::Aggregate(sink) => seal_aggregate(sink),
    }
}

#[derive(Default)]
pub(super) struct RecPingPong {
    delta_a: TransientImage,
    delta_b: TransientImage,
    acc_a: TransientImage,
    acc_b: TransientImage,
    acc_filled: [usize; 2],
    flip: bool,
    watermark: usize,
}

impl RecPingPong {
    fn begin(&mut self) {
        self.acc_filled = [0; 2];
        self.flip = false;
        self.watermark = 0;
    }

    fn delta_mut(&mut self) -> &mut TransientImage {
        if self.flip {
            &mut self.delta_b
        } else {
            &mut self.delta_a
        }
    }

    fn acc_mut(&mut self) -> &mut TransientImage {
        if self.flip {
            &mut self.acc_b
        } else {
            &mut self.acc_a
        }
    }
}

#[derive(Clone, Copy)]
struct RunCtx<'a> {
    schema: &'a Schema,
    images: &'a SourceImages<'a>,
    interner: &'a InternerHandle<'a>,
    resolved_params: &'a [Const],
    missed_params: &'a [bool],
    fast_eligible: bool,
    /// Route Free Join rules through the cursor fallback (Q-FALLBACK
    /// forcing, or the one bounded restart after reservation refusal).
    fallback: bool,
}

/// The fallback's derived-source resolver over sealed stage tables
/// (interiors and the finished rec read `Finished(id)`).
pub(super) fn finished_resolver(
    published: &[Arc<RelationImage>],
) -> impl Fn(crate::ir::normalize::OccBind) -> Option<Arc<RelationImage>> + '_ {
    |bind| match bind {
        crate::ir::normalize::OccBind::Finished(id) => published.get(id.index()).cloned(),
        crate::ir::normalize::OccBind::Edb(_)
        | crate::ir::normalize::OccBind::RecDelta(_)
        | crate::ir::normalize::OccBind::RecAcc(_) => None,
    }
}

/// As [`finished_resolver`], plus the current round's Δ/accumulated rec
/// tables for rec arms.
fn rec_resolver<'a>(
    published: &'a [Arc<RelationImage>],
    delta: &'a Arc<RelationImage>,
    acc: &'a Arc<RelationImage>,
) -> impl Fn(crate::ir::normalize::OccBind) -> Option<Arc<RelationImage>> + 'a {
    move |bind| match bind {
        crate::ir::normalize::OccBind::Finished(id) => published.get(id.index()).cloned(),
        crate::ir::normalize::OccBind::RecDelta(_) => Some(Arc::clone(delta)),
        crate::ir::normalize::OccBind::RecAcc(_) => Some(Arc::clone(acc)),
        crate::ir::normalize::OccBind::Edb(_) => None,
    }
}

impl<S> PreparedQuery<S> {
    #[expect(
        clippy::too_many_lines,
        reason = "the derived phase reads as one protocol: interiors, then rec"
    )]
    pub(super) fn run_derived<Cnt: Counters>(
        &mut self,
        images: &SourceImages<'_>,
        counters: &mut Cnt,
    ) -> Result<bool> {
        let derived_count = match &self.pipeline {
            PreparedPipeline::PointProbe { .. } => 0,
            PreparedPipeline::Cq { interiors, .. } => interiors.len(),
            PreparedPipeline::Reach { derived_count, .. } => {
                usize::try_from(*derived_count).expect("derived_count stored at validate")
            }
        };
        self.derived.begin(derived_count);

        // Arcs; drop them before refill so TransientImage can get_mut.
        {
            let retired = &mut self.derived.retired;
            for rule in self.pipeline.main_rules_mut() {
                if let PreparedRule::FreeJoin(fj) = rule {
                    unbind_interior_rule(fj, retired);
                }
            }
        }
        let fast_eligible = self.latch.is_latched() && self.params.is_empty();
        let mut latched = 0u32;
        let mut ran = false;
        let mut derived_tuples: u64 = 0;
        let interner = InternerHandle::new(self.cache.interner(), images.source().work());

        let n_interiors = self.pipeline.interiors().len();
        if n_interiors > 0 {
            let mut interiors_span = crate::obs::span(crate::obs::names::INTERIORS);
            let mut interior_emits: u64 = 0;
            for i in 0..n_interiors {
                {
                    let interiors = self.pipeline.interiors_mut();
                    unbind_interior_views(&mut interiors[i], &mut self.derived.retired);
                    interiors[i].sink.reset();
                }
                let ctx = RunCtx {
                    schema: self.schema.as_ref(),
                    images,
                    interner: &interner,
                    resolved_params: &self.resolved_params,
                    missed_params: &self.missed_params,
                    fast_eligible,
                    fallback: self.forced_fallback,
                };
                let rule_count = self.pipeline.interiors()[i].rules.len();
                for rule_idx in 0..rule_count {
                    fill_finished_images(
                        &self.pipeline.interiors()[i].rules[rule_idx],
                        &mut self.derived,
                    );
                    let occ_images = std::mem::take(&mut self.derived.occ_images);
                    let mut retired = std::mem::take(&mut self.derived.retired);
                    let interiors = self.pipeline.interiors_mut();
                    let units = interiors[i].units;
                    let interior = &mut interiors[i];
                    let resolver = finished_resolver(&self.derived.published);
                    ran |= run_into_projection(
                        &ctx,
                        &resolver,
                        &mut interior.rules,
                        rule_idx,
                        units,
                        &occ_images,
                        &mut retired,
                        &mut interior.sink,
                        &mut self.bindings,
                        &mut self.key_scratch,
                        &mut latched,
                        counters,
                    )?;
                    self.derived.occ_images = occ_images;
                    self.derived.retired = retired;
                }
                // Seal the stage: aggregate/computed stages finalize HERE,
                // so a required producer error (overflow, cardinality,
                // scalar failure) fails the query before any consumer
                // could discard it (the stage error boundary, Q-IR).
                let sealed = {
                    let tuples_budget = self.tuples_budget;
                    let interiors = self.pipeline.interiors_mut();
                    seal_interior(
                        &mut interiors[i],
                        i,
                        &mut self.derived,
                        &mut self.answer_scratch,
                        derived_tuples,
                        tuples_budget,
                        images.source().work(),
                    )?
                };
                derived_tuples += sealed;
                interior_emits += sealed;
            }
            interiors_span.set_pair(n_interiors as u64, interior_emits);
        }

        let rec_ran = match &mut self.pipeline {
            PreparedPipeline::Reach {
                driver,
                rec_id,
                rounds_budget,
                ..
            } => {
                let rec_id = usize::try_from(rec_id.0).expect("rec_id stored at validate");
                run_reach(
                    driver,
                    rec_id,
                    *rounds_budget,
                    self.tuples_budget,
                    self.sink_ram,
                    &mut self.derived,
                    &mut self.bindings,
                    &mut self.key_scratch,
                    self.schema.as_ref(),
                    images,
                    &interner,
                    &self.resolved_params,
                    &self.missed_params,
                    fast_eligible,
                    self.forced_fallback,
                    counters,
                    &mut latched,
                    derived_tuples,
                )?
            }
            PreparedPipeline::Cq { .. } | PreparedPipeline::PointProbe { .. } => false,
        };
        ran |= rec_ran;

        self.latch = self.latch.credit(latched);
        Ok(ran)
    }

    pub(super) fn fill_main_images(&mut self, rule_idx: usize) {
        let Some(plan) = main_plan(self.pipeline.main_rules(), rule_idx) else {
            self.derived.occ_images.clear();
            return;
        };
        fill_plan_images(plan, &mut self.derived, DerivedBind::Finished);
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "the prepared query's split borrows are clearer unpacked"
)]
#[expect(
    clippy::too_many_lines,
    reason = "the driver reads as one protocol: reset, round 0, Δ loop, budget"
)]
fn run_reach<Cnt: Counters>(
    driver: &mut ReachDriver,
    rec_id: usize,
    rounds_budget: u32,
    tuples_budget: u64,
    sink_ram: usize,
    derived: &mut DerivedImages,
    bindings: &mut Bindings,
    key_scratch: &mut Vec<u64>,
    schema: &Schema,
    images: &SourceImages<'_>,
    interner: &InternerHandle<'_>,
    resolved_params: &[Const],
    missed_params: &[bool],
    fast_eligible: bool,
    forced_fallback: bool,
    counters: &mut Cnt,
    latched: &mut u32,
    mut derived_tuples: u64,
) -> Result<bool> {
    let mut ran = false;
    let mut reach_span = crate::obs::span(crate::obs::names::REACH);

    driver.sink.reset();
    // The rec seen/frontier state is charged like the main sink's distinct
    // state: past this execution's RAM allowance it continues in the one
    // scratch map, and the watermark drains below read across both tiers.
    driver.sink.begin(Some(crate::exec::sink::SinkBudget {
        work: images.source().work().clone(),
        ram_bytes: sink_ram,
    }));
    driver.scratch.begin();
    for rule in &mut driver.base {
        if let PreparedRule::FreeJoin(fj) = rule {
            unbind_interior_rule(fj, &mut derived.retired);
        }
    }
    for arm in &mut driver.rec {
        unbind_interior_rule(&mut arm.rule, &mut derived.retired);
    }

    let ctx = RunCtx {
        schema,
        images,
        interner,
        resolved_params,
        missed_params,
        fast_eligible,
        fallback: forced_fallback,
    };

    let mut round_span = Some(crate::obs::span(crate::obs::names::FIXPOINT_ROUND));
    let mut round_emits_before = counters.emits();

    for rule_idx in 0..driver.base.len() {
        fill_finished_images(&driver.base[rule_idx], derived);
        let resolver = finished_resolver(&derived.published);
        ran |= run_into_projection(
            &ctx,
            &resolver,
            &mut driver.base,
            rule_idx,
            driver.units,
            &derived.occ_images,
            &mut derived.retired,
            &mut driver.sink,
            bindings,
            key_scratch,
            latched,
            counters,
        )?;
    }
    let emitted = counters.emits() - round_emits_before;
    let newly = driver.sink.len() as u64;
    counters.fixpoint_round(emitted, emitted.saturating_sub(newly));
    if let Some(mut span) = round_span.take() {
        span.set_pair(emitted, emitted.saturating_sub(newly));
    }

    let mut rounds: u32 = 0;
    loop {
        let len = driver.sink.len();
        let tuples = derived_tuples + len as u64;
        let any_delta = len > driver.scratch.watermark;
        if !any_delta {
            let ReachDriver {
                sink, field_types, ..
            } = &mut *driver;
            derived.stash_finished(rec_id, field_types, sink, images.source().work())?;
            reach_span.set_pair(u64::from(rounds), tuples);
            break;
        }
        if rounds >= rounds_budget || tuples > tuples_budget {
            return Err(Error::DerivedBudgetExceeded { rounds, tuples });
        }
        rounds += 1;
        round_span = Some(crate::obs::span(crate::obs::names::FIXPOINT_ROUND));
        round_emits_before = counters.emits();
        let flip = usize::from(driver.scratch.flip);
        let since = driver.scratch.watermark;
        counters.fixpoint_delta((len - since) as u64);
        // Δ/accumulated images drain the seen-set across both tiers (the
        // frontier keeps its watermark contract after a spill) and charge
        // their slabs before allocation.
        let (round_delta, round_acc) = {
            let ReachDriver {
                sink,
                scratch,
                field_types,
                ..
            } = &mut *driver;
            let work = images.source().work();
            let round_delta = scratch.delta_mut().refill_drained(
                Some(work),
                field_types,
                len - since,
                |_, write| sink.drain_since(since, write),
            )?;
            let filled = scratch.acc_filled[flip];
            let round_acc = scratch.acc_mut().append_drained(
                Some(work),
                field_types,
                filled,
                len,
                |from, write| sink.drain_since(from, write),
            )?;
            (round_delta, round_acc)
        };
        driver.scratch.acc_filled[flip] = len;
        driver.scratch.flip = !driver.scratch.flip;
        driver.scratch.watermark = len;

        for arm_idx in 0..driver.rec.len() {
            fill_plan_images(
                &driver.rec[arm_idx].rule.plan,
                derived,
                DerivedBind::Rec {
                    delta: &round_delta,
                    acc: &round_acc,
                },
            );
            let resolver = rec_resolver(&derived.published, &round_delta, &round_acc);
            ran |= run_free_join_into_projection(
                &ctx,
                &resolver,
                &mut driver.rec[arm_idx].rule,
                driver.units,
                &derived.occ_images,
                &mut derived.retired,
                &mut driver.sink,
                bindings,
                latched,
                counters,
            )?;
        }
        let emitted = counters.emits() - round_emits_before;
        let newly = (driver.sink.len() - driver.scratch.watermark) as u64;
        counters.fixpoint_round(emitted, emitted.saturating_sub(newly));
        if let Some(mut span) = round_span.take() {
            span.set_pair(emitted, emitted.saturating_sub(newly));
        }
        derived_tuples = tuples;
    }
    Ok(ran)
}

fn main_plan(rules: &[PreparedRule], rule_idx: usize) -> Option<&crate::plan::fj::ValidatedPlan> {
    match rules.get(rule_idx)? {
        PreparedRule::FreeJoin(rule) => Some(&rule.plan),
        PreparedRule::KeyProbe(_) => None,
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
        if occurrence.role.discharged() || occurrence.bind.edb().is_some() {
            continue;
        }
        let old = rule.memo.colts[occ_idx].reset(crate::image::view::View::Unbound);
        let recycled = old.recycle();
        let spare = rule.memo.spare_mut(occ_idx);
        if spare.capacity() == 0 {
            *spare = recycled;
        } else if recycled.capacity() > 0 {
            retired.push(recycled);
        }
    }
}

fn fill_finished_images(rule: &PreparedRule, derived: &mut DerivedImages) {
    let plan = match rule {
        PreparedRule::FreeJoin(rule) => &rule.plan,
        PreparedRule::KeyProbe(_) => {
            derived.occ_images.clear();
            return;
        }
    };
    fill_plan_images(plan, derived, DerivedBind::Finished);
}

fn fill_plan_images(
    plan: &crate::plan::fj::ValidatedPlan,
    derived: &mut DerivedImages,
    bind: DerivedBind<'_>,
) {
    derived.occ_images.clear();
    for (occ_idx, occurrence) in plan.occurrences().iter().enumerate() {
        if occurrence.role.discharged() {
            continue;
        }
        let image = match occurrence.bind {
            crate::plan::fj::OccBind::Edb(_) => continue,
            crate::plan::fj::OccBind::Finished(id) => derived.published[id.index()].clone(),
            crate::plan::fj::OccBind::RecDelta(_) => match bind {
                DerivedBind::Rec { delta, .. } => delta.clone(),
                DerivedBind::Finished => {
                    unreachable!("RecDelta is stamped only on rec arms")
                }
            },
            crate::plan::fj::OccBind::RecAcc(_) => match bind {
                DerivedBind::Rec { acc, .. } => acc.clone(),
                DerivedBind::Finished => {
                    unreachable!("RecAcc is stamped only on rec arms")
                }
            },
        };
        derived.occ_images.insert(occ_idx, image);
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "the prepared query's split borrows are clearer unpacked"
)]
fn run_into_projection<S: StageSink, Cnt: Counters>(
    ctx: &RunCtx<'_>,
    derived_resolver: &dyn Fn(crate::ir::normalize::OccBind) -> Option<Arc<RelationImage>>,
    rules: &mut [PreparedRule],
    rule_idx: usize,
    units: usize,
    occ_images: &OccImages,
    retired: &mut Vec<Vec<u32>>,
    sink: &mut S,
    bindings: &mut Bindings,
    key_scratch: &mut Vec<u64>,
    latched: &mut u32,
    counters: &mut Cnt,
) -> Result<bool> {
    let multi_unit = units > 1;
    match &mut rules[rule_idx] {
        PreparedRule::KeyProbe(rule) => {
            bindings.resize(rule.plan.slot_count());
            if multi_unit {
                sink.aim_stage(&rule.finds, rule.plan.slot_count(), &rule.dedup_spans);
            }
            crate::exec::dispatch::execute_key_probe(
                &rule.plan,
                ctx.images.source(),
                ctx.schema,
                ctx.interner,
                ctx.resolved_params,
                key_scratch,
                bindings,
                sink,
                counters,
            )?;
            Ok(true)
        }
        PreparedRule::FreeJoin(rule) => run_free_join_into_projection(
            ctx,
            derived_resolver,
            rule,
            units,
            occ_images,
            retired,
            sink,
            bindings,
            latched,
            counters,
        ),
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "the prepared query's split borrows are clearer unpacked"
)]
fn run_free_join_into_projection<S: StageSink, Cnt: Counters>(
    ctx: &RunCtx<'_>,
    derived_resolver: &dyn Fn(crate::ir::normalize::OccBind) -> Option<Arc<RelationImage>>,
    rule: &mut FreeJoinRule,
    units: usize,
    occ_images: &OccImages,
    retired: &mut Vec<Vec<u32>>,
    sink: &mut S,
    bindings: &mut Bindings,
    latched: &mut u32,
    counters: &mut Cnt,
) -> Result<bool> {
    let multi_unit = units > 1;
    bindings.resize(rule.plan.slot_count());
    if ctx.fallback {
        if multi_unit {
            sink.aim_stage(&rule.finds, rule.plan.slot_count(), &rule.dedup_spans);
        }
        let fallback_ctx = super::fallback::FallbackCtx {
            source: ctx.images.source(),
            schema: ctx.schema,
            interner: ctx.interner,
            params: ctx.resolved_params,
            missed: ctx.missed_params,
            derived: derived_resolver,
        };
        super::fallback::run_fallback(&mut rule.fallback, &fallback_ctx, bindings, sink, latched)?;
        return Ok(true);
    }
    let resolved = if ctx.fast_eligible && rule.resolution == super::ResolutionState::Complete {
        true
    } else {
        let complete = super::bind::resolve_filters(
            ctx.interner,
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
        sink.aim_stage(&rule.finds, rule.plan.slot_count(), &rule.dedup_spans);
    }
    run_join(
        &rule.plan,
        ctx.schema,
        ctx.images,
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
