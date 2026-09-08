//! The linear-reach driver: one rec SCC, interiors then
//! rec then main. Round 0 runs the base arms through the ordinary rule
//! loop; rounds ≥ 1 run each rec arm against the watermark frontier
//! (the unique self-atom is the marked delta occurrence). An empty Δ ends the rec
//! Interiors-only never enters this module's loop.
//! (`lean/Bumbledb/Exec/Reach.lean: evalLinearReach_eq_lfp`).
use std::sync::Arc;

use super::derived::{ScratchStage, SealedStage};
use super::run_join::run_join;
use super::{
    Bindings, EitherSink, FreeJoinRule, PreparedInterior, PreparedPipeline, PreparedQuery,
    PreparedRule, ProjectionSink,
};
use crate::error::Result;
use crate::exec::run::Counters;
use crate::exec::scratch::ScratchRelation;
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
    fn aim_stage(&mut self, finds: &[FindSpec], _slot_count: usize, _spans: &[(usize, usize)]) {
        self.aim(finds);
    }
}

impl StageSink for EitherSink {
    fn aim_stage(&mut self, finds: &[FindSpec], slot_count: usize, spans: &[(usize, usize)]) {
        self.aim(finds, slot_count, spans);
    }
}

pub(crate) struct ReachDriver {
    pub(super) base: Vec<PreparedRule>,
    pub(super) rec: Vec<FreeJoinRule>,
    pub(super) field_types: Vec<ValueType>,
    pub(super) sink: crate::exec::sink::ProjectionSink,
    pub(super) units: usize,
    pub(super) frontier: TransientImage,
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

/// One derived-image protocol: a working transient per derived id, a
/// seal-ordered published `Arc` grown as each table closes, and the
/// per-occurrence bind scratch `run_join` consumes.
#[derive(Default)]
pub(super) struct DerivedImages {
    working: Vec<TransientImage>,
    pub(super) published: Vec<SealedStage>,
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
    /// distinct rows across both representations. A scratch-backed seen-set
    /// is consumed through its ordered row visitor.
    fn stash_finished(
        &mut self,
        id: usize,
        field_types: &[ValueType],
        sink: &mut ProjectionSink,
        work: &crate::work::WorkContext,
        generation: &crate::work::GenerationHandle,
    ) -> Result<u64> {
        debug_assert_eq!(
            self.published.len(),
            id,
            "derived tables seal in declaration order"
        );
        let stage = if sink.spilled() || u32::try_from(sink.len()).is_err() {
            seal_scratch_range(sink, work, field_types, generation, 0)?
        } else {
            let rows = sink.len();
            let image = self.working[id].refill_drained(
                Some(work),
                field_types,
                rows,
                generation,
                |_, write| write_projection_rows(sink, 0, write),
            )?;
            SealedStage::Resident(image)
        };
        let count = stage.row_count();
        self.published.push(stage);
        Ok(count)
    }

    /// Finalize small stages directly into columnar images, preserving
    /// the Free Join path for their consumers. Larger/spilled stages
    /// stream to one RAM-first scratch relation without reconstruction.
    fn stash_aggregate(
        &mut self,
        id: usize,
        field_types: &[ValueType],
        sink: &mut crate::exec::sink::AggregateSink,
        answer_scratch: &mut Vec<u64>,
        work: &crate::work::WorkContext,
        generation: &crate::work::GenerationHandle,
    ) -> Result<u64> {
        debug_assert_eq!(
            self.published.len(),
            id,
            "derived tables seal in declaration order"
        );
        if let Some(bound) = sink.resident_row_bound()
            && u32::try_from(bound).is_ok()
        {
            let image = self.working[id].refill_bounded(
                work,
                field_types,
                bound,
                generation,
                |_, write| {
                    sink.finalize_into(answer_scratch, |row| {
                        write(row);
                        Ok(())
                    })
                },
            )?;
            let count = image.row_count() as u64;
            self.published.push(SealedStage::Resident(image));
            return Ok(count);
        }
        let mut dest = ScratchRelation::new(work);
        let mut texts = crate::image::TextOwners::default();
        let count = sink.stream_finalize(&mut dest, answer_scratch, |row| {
            texts.pin_row(row, field_types, generation)
        })?;
        // dest.spilled() / dest.scratch_path() — never force_spill first.
        self.published.push(SealedStage::from_aggregate_dest(
            dest,
            field_types,
            count,
            generation.clone(),
            texts,
        ));
        Ok(count)
    }
}

/// Feed a projection drain into an image-slab write. The write itself is
/// infallible (pre-reserved columns); drain/put failures stay on the
/// [`crate::exec::sink::StageRowVisit`] result and surface immediately.
fn write_projection_rows(
    sink: &mut ProjectionSink,
    since: usize,
    write: &mut dyn FnMut(&[u64]),
) -> Result<()> {
    sink.drain_since(since, &mut |row| {
        write(row);
        Ok(true)
    })
}

/// One stage's row width in image words: the same slot arithmetic the
/// binding layout uses (interval/Pack and Uuid columns are two words).
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
/// Returns the sealed row count.
fn seal_interior(
    interior: &mut PreparedInterior,
    id: usize,
    derived: &mut DerivedImages,
    answer_scratch: &mut Vec<u64>,

    work: &crate::work::WorkContext,
    generation: &crate::work::GenerationHandle,
) -> Result<u64> {
    let field_types = &interior.field_types;
    let mut seal_aggregate = |sink: &mut crate::exec::sink::AggregateSink| -> Result<u64> {
        derived.stash_aggregate(id, field_types, sink, answer_scratch, work, generation)
    };
    match &mut interior.sink {
        EitherSink::Projection(sink) => {
            derived.stash_finished(id, field_types, sink, work, generation)
        }
        EitherSink::Computed(computed) => {
            if let Some(error) = &computed.error {
                return Err(error.clone());
            }
            match &mut computed.inner {
                EitherSink::Projection(sink) => {
                    derived.stash_finished(id, field_types, sink, work, generation)
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
    published: &'a mut [SealedStage],
    retained_texts: &'a mut crate::image::TextOwners,
}

pub(super) fn rule_uses_scratch_derived(
    plan: &crate::plan::fj::ValidatedPlan,
    published: &[SealedStage],
) -> bool {
    plan.occurrences().iter().any(|occurrence| {
        occurrence
            .bind
            .interior()
            .and_then(|id| published.get(id.index()))
            .is_some_and(|stage| !stage.is_resident())
    })
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
        let fast_eligible = self.params.is_empty();
        let mut ran = false;
        let interner = images.interner();

        let n_interiors = self.pipeline.interiors().len();
        if n_interiors > 0 {
            for i in 0..n_interiors {
                {
                    let interiors = self.pipeline.interiors_mut();
                    unbind_interior_views(&mut interiors[i], &mut self.derived.retired);
                    interiors[i].sink.reset();
                    // Main, interior and recursive sinks share cancellation.
                    interiors[i]
                        .sink
                        .begin_execution(Some(images.source().work().clone()));
                }
                let rule_count = self.pipeline.interiors()[i].rules.len();
                for rule_idx in 0..rule_count {
                    fill_finished_images(
                        &self.pipeline.interiors()[i].rules[rule_idx],
                        &mut self.derived,
                    );
                    let mut ctx = RunCtx {
                        schema: self.schema.as_ref(),
                        images,
                        interner: &interner,
                        resolved_params: &self.resolved_params,
                        missed_params: &self.missed_params,
                        fast_eligible,
                        fallback: self.forced_fallback,
                        published: &mut self.derived.published,
                        retained_texts: &mut self.execution_texts,
                    };
                    let occ_images = std::mem::take(&mut self.derived.occ_images);
                    let mut retired = std::mem::take(&mut self.derived.retired);
                    let interiors = self.pipeline.interiors_mut();
                    let units = interiors[i].units;
                    let interior = &mut interiors[i];
                    ran |= run_into_projection(
                        &mut ctx,
                        &mut interior.rules,
                        rule_idx,
                        units,
                        &occ_images,
                        &mut retired,
                        &mut interior.sink,
                        &mut self.bindings,
                        &mut self.key_scratch,
                        counters,
                    )?;
                    self.derived.occ_images = occ_images;
                    self.derived.retired = retired;
                }
                // Seal the stage: aggregate/computed stages finalize HERE,
                // so a required producer error (overflow, cardinality,
                // scalar failure) fails the query before any consumer
                // could discard it (the stage error boundary, Q-IR).
                {
                    let interiors = self.pipeline.interiors_mut();
                    seal_interior(
                        &mut interiors[i],
                        i,
                        &mut self.derived,
                        &mut self.answer_scratch,
                        images.source().work(),
                        images.generation(),
                    )?
                };
            }
        }

        let rec_ran = match &mut self.pipeline {
            PreparedPipeline::Reach { driver, rec_id, .. } => {
                let rec_id = usize::try_from(rec_id.0).expect("rec_id stored at validate");
                run_reach(
                    driver,
                    rec_id,
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
                    &mut self.execution_texts,
                    counters,
                )?
            }
            PreparedPipeline::Cq { .. } | PreparedPipeline::PointProbe { .. } => false,
        };
        ran |= rec_ran;

        Ok(ran)
    }

    pub(super) fn fill_main_images(&mut self, rule_idx: usize) {
        let Some(plan) = main_plan(self.pipeline.main_rules(), rule_idx) else {
            self.derived.occ_images.clear();
            return;
        };
        fill_plan_images(plan, &mut self.derived);
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "the prepared query's split borrows are clearer unpacked"
)]
fn run_reach<Cnt: Counters>(
    driver: &mut ReachDriver,
    rec_id: usize,

    derived: &mut DerivedImages,
    bindings: &mut Bindings,
    key_scratch: &mut crate::image::view::ResolvedWords,
    schema: &Schema,
    images: &SourceImages<'_>,
    interner: &InternerHandle<'_>,
    resolved_params: &[Const],
    missed_params: &[bool],
    fast_eligible: bool,
    forced_fallback: bool,
    retained_texts: &mut crate::image::TextOwners,
    counters: &mut Cnt,
) -> Result<bool> {
    let mut ran = false;

    driver.sink.reset();
    // Watermark drains preserve insertion order in either representation.
    driver.sink.begin(Some(images.source().work().clone()));
    for rule in &mut driver.base {
        if let PreparedRule::FreeJoin(fj) = rule {
            unbind_interior_rule(fj, &mut derived.retired);
        }
    }
    for rule in &mut driver.rec {
        unbind_interior_rule(rule, &mut derived.retired);
    }

    for rule_idx in 0..driver.base.len() {
        fill_finished_images(&driver.base[rule_idx], derived);
        let mut ctx = RunCtx {
            schema,
            images,
            interner,
            resolved_params,
            missed_params,
            fast_eligible,
            fallback: forced_fallback,
            published: &mut derived.published,
            retained_texts,
        };
        ran |= run_into_projection(
            &mut ctx,
            &mut driver.base,
            rule_idx,
            driver.units,
            &derived.occ_images,
            &mut derived.retired,
            &mut driver.sink,
            bindings,
            key_scratch,
            counters,
        )?;
    }
    let mut watermark = 0;
    loop {
        let len = driver.sink.len();
        images
            .source()
            .work()
            .checkpoint()
            .map_err(super::source::work_error)?;
        let any_delta = len > watermark;
        if !any_delta {
            let _rows = derived.stash_finished(
                rec_id,
                &driver.field_types,
                &mut driver.sink,
                images.source().work(),
                images.generation(),
            )?;
            break;
        }
        // Validation admits exactly one self-read per arm. Its source
        // slot holds this round's immutable frontier, not another copy
        // of the accumulated set. All consumers use the ordinary stage
        // environment, including scratch-backed execution.
        debug_assert_eq!(derived.published.len(), rec_id);
        derived.published.push(next_frontier(
            driver,
            images.source().work(),
            images.generation(),
            watermark,
            len,
            retained_texts,
        )?);
        watermark = len;

        for rule in &mut driver.rec {
            fill_plan_images(&rule.plan, derived);
            let mut ctx = RunCtx {
                schema,
                images,
                interner,
                resolved_params,
                missed_params,
                fast_eligible,
                fallback: forced_fallback,
                published: &mut derived.published,
                retained_texts,
            };
            let result = run_free_join_into_projection(
                &mut ctx,
                rule,
                driver.units,
                &derived.occ_images,
                &mut derived.retired,
                &mut driver.sink,
                bindings,
                counters,
            );
            // End every consumer lease, including a failed arm, before
            // the next refill. Derived views never enter parked memos.
            unbind_interior_rule(rule, &mut derived.retired);
            if result.is_err() {
                derived.occ_images.clear();
                derived.published.pop();
            }
            ran |= result?;
        }
        derived.occ_images.clear();
        derived.published.pop();
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
    fill_plan_images(plan, derived);
}

fn next_frontier(
    driver: &mut ReachDriver,
    work: &crate::work::WorkContext,
    generation: &crate::work::GenerationHandle,
    since: usize,
    len: usize,
    retained_texts: &mut crate::image::TextOwners,
) -> Result<SealedStage> {
    let ReachDriver {
        sink,
        frontier,
        field_types,
        ..
    } = driver;
    if sink.spilled() || u32::try_from(len - since).is_err() {
        // The scratch publication owns this round; the accumulator also
        // needs its tokens after that publication is dropped.
        let stage = seal_scratch_range(sink, work, field_types, generation, since)?;
        if let SealedStage::Scratch(stage) = &stage {
            retained_texts.extend_from(&stage.texts);
        }
        return Ok(stage);
    }
    // The round publication and all COLT views have been released. One
    // reusable SoA buffer suffices; the set remains the sole accumulator.
    debug_assert!(frontier.is_uniquely_owned());
    let image = frontier.refill_drained(
        Some(work),
        field_types,
        len - since,
        generation,
        |_, write| write_projection_rows(sink, since, write),
    )?;
    retained_texts.extend_from(image.texts());
    Ok(SealedStage::Resident(image))
}

fn seal_scratch_range(
    sink: &mut ProjectionSink,
    work: &crate::work::WorkContext,
    field_types: &[ValueType],
    generation: &crate::work::GenerationHandle,
    since: usize,
) -> Result<SealedStage> {
    let mut rows = ScratchRelation::new(work);
    rows.force_spill()?;
    let mut texts = crate::image::TextOwners::default();
    let count = sink.stream_into_scratch(&mut rows, since, 0, |row| {
        texts.pin_row(row, field_types, generation)
    })?;
    Ok(SealedStage::Scratch(Box::new(ScratchStage {
        rows,
        field_types: field_types.to_vec(),
        row_words: stage_row_words(field_types),
        count,
        generation: generation.clone(),
        texts,
    })))
}

fn fill_plan_images(plan: &crate::plan::fj::ValidatedPlan, derived: &mut DerivedImages) {
    derived.occ_images.clear();
    for (occ_idx, occurrence) in plan.occurrences().iter().enumerate() {
        if occurrence.role.discharged() {
            continue;
        }
        let Some(id) = occurrence.bind.interior() else {
            continue;
        };
        if let SealedStage::Resident(image) = &derived.published[id.index()] {
            derived.occ_images.insert(occ_idx, image.clone());
        }
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "the prepared query's split borrows are clearer unpacked"
)]
fn run_into_projection<S: StageSink, Cnt: Counters>(
    ctx: &mut RunCtx<'_>,
    rules: &mut [PreparedRule],
    rule_idx: usize,
    units: usize,
    occ_images: &OccImages,
    retired: &mut Vec<Vec<u32>>,
    sink: &mut S,
    bindings: &mut Bindings,
    key_scratch: &mut crate::image::view::ResolvedWords,
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
                &mut rule.row,
                key_scratch,
                bindings,
                sink,
                counters,
            )?;
            Ok(true)
        }
        PreparedRule::FreeJoin(rule) => run_free_join_into_projection(
            ctx, rule, units, occ_images, retired, sink, bindings, counters,
        ),
    }
}

#[expect(
    clippy::too_many_arguments,
    reason = "the prepared query's split borrows are clearer unpacked"
)]
fn run_free_join_into_projection<S: StageSink, Cnt: Counters>(
    ctx: &mut RunCtx<'_>,
    rule: &mut FreeJoinRule,
    units: usize,
    occ_images: &OccImages,
    retired: &mut Vec<Vec<u32>>,
    sink: &mut S,
    bindings: &mut Bindings,
    counters: &mut Cnt,
) -> Result<bool> {
    let multi_unit = units > 1;
    bindings.resize(rule.plan.slot_count());
    if ctx.fallback
        || rule_uses_scratch_derived(&rule.plan, ctx.published)
        || resident_edb_overflow(ctx.images.source(), &rule.plan)?
    {
        if multi_unit {
            sink.aim_stage(&rule.finds, rule.plan.slot_count(), &rule.dedup_spans);
        }
        let mut fallback_ctx = super::fallback::FallbackCtx {
            source: ctx.images.source(),
            schema: ctx.schema,
            interner: ctx.interner,
            params: ctx.resolved_params,
            missed: ctx.missed_params,
            retained_texts: ctx.retained_texts,
        };
        super::fallback::run_fallback(
            &mut rule.fallback,
            &mut fallback_ctx,
            ctx.published,
            bindings,
            sink,
        )?;
        return Ok(true);
    }
    let resolved = if ctx.fast_eligible && rule.resolution == super::ResolutionState::Complete {
        true
    } else {
        let complete = super::bind::LiteralResolution {
            interner: ctx.interner,
            work: ctx.images.source().work(),
            params: ctx.resolved_params,
            missed: ctx.missed_params,
        }
        .filters(
            &rule.plan,
            &mut rule.resolved_filters,
            &mut rule.resolved_selections,
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
    if multi_unit {
        sink.aim_stage(&rule.finds, rule.plan.slot_count(), &rule.dedup_spans);
    }
    run_join(
        &rule.plan,
        ctx.schema,
        ctx.images,
        ctx.images.source().work(),
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

fn resident_edb_overflow(
    source: &crate::api::prepared::source::QuerySource<'_>,
    plan: &crate::plan::fj::ValidatedPlan,
) -> Result<bool> {
    for occurrence in plan.occurrences() {
        if let crate::plan::fj::OccBind::Edb(relation) = occurrence.bind
            && source.exceeds_resident_positions(relation)?
        {
            return Ok(true);
        }
    }
    Ok(false)
}

#[cfg(test)]
mod spill_bounded;
