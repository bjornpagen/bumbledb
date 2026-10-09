//! Linear recursion: evaluate interiors, then the recursive component, then
//! main. The base arms seed round zero. Each later round reads the watermark
//! frontier at its unique self-atom. An empty frontier ends the fixed point.
//! Queries containing only interiors never enter this loop.
use std::sync::Arc;

use super::run_join::{RuleCtx, RuleScratch, SinkUse, run_free_join, run_rule};
use super::{
    Bindings, EitherSink, FreeJoinRule, PreparedInterior, PreparedPipeline, PreparedQuery,
    PreparedRule, ProjectionSink,
};
use crate::error::Result;
use crate::exec::run::Counters;
use crate::image::SourceImages;
use crate::image::{RelationImage, TransientImage};
use bumbledb_theory::schema::ValueType;

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
    pub(super) published: Vec<Arc<RelationImage>>,
    pub(super) occ_images: OccImages,
    pub(super) recycled: Vec<Vec<u32>>,
}

impl DerivedImages {
    fn begin(&mut self, derived_count: usize) {
        self.working.resize_with(derived_count, Default::default);
        self.published.clear();
        self.occ_images.clear();
        self.recycled.clear();
    }

    /// Seal one finished projection stage or rec table into a resident image.
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
        let image = self.working[id].refill_drained(
            Some(work),
            field_types,
            sink.len(),
            generation,
            |_, write| write_projection_rows(sink, 0, write),
        )?;
        let count = image.row_count() as u64;
        self.published.push(image);
        Ok(count)
    }

    /// Finalize an aggregate stage into a columnar image.
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
        let bound = sink
            .resident_row_bound()
            .ok_or(crate::error::Error::Capacity(
                crate::error::Capacity::ResidentRows,
            ))?;
        let image =
            self.working[id].refill_bounded(work, field_types, bound, generation, |_, write| {
                sink.finalize_into(answer_scratch, |row| {
                    write(row);
                    Ok(())
                })
            })?;
        let count = image.row_count() as u64;
        self.published.push(image);
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

impl<S> PreparedQuery<S> {
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
        self.runtime.derived.begin(derived_count);

        // Arcs; drop them before refill so TransientImage can get_mut.
        {
            let recycled = &mut self.runtime.derived.recycled;
            for rule in self.pipeline.main_rules_mut() {
                if let PreparedRule::FreeJoin(fj) = rule {
                    unbind_interior_rule(fj, recycled);
                }
            }
        }
        let mut ran = false;
        let interner = images.interner();
        let ctx = RuleCtx {
            schema: self.program.schema.as_ref(),
            images,
            interner: &interner,
            params: &self.bound.resolved_params,
            missed: &self.bound.missed_params,
        };

        let n_interiors = self.pipeline.interiors().len();
        if n_interiors > 0 {
            for i in 0..n_interiors {
                {
                    let interiors = self.pipeline.interiors_mut();
                    unbind_interior_views(&mut interiors[i], &mut self.runtime.derived.recycled);
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
                        &mut self.runtime.derived,
                    );
                    let occ_images = std::mem::take(&mut self.runtime.derived.occ_images);
                    let mut recycled = std::mem::take(&mut self.runtime.derived.recycled);
                    let interior = &mut self.pipeline.interiors_mut()[i];
                    let sink_use = if interior.units > 1 {
                        SinkUse::Shared
                    } else {
                        SinkUse::Sole
                    };
                    let result = run_rule(
                        &ctx,
                        &mut RuleScratch {
                            bindings: &mut self.runtime.bindings,
                            occ_images: &occ_images,
                            recycled: &mut recycled,
                        },
                        &mut interior.rules[rule_idx],
                        sink_use,
                        &mut interior.sink,
                        counters,
                    );
                    self.runtime.derived.occ_images = occ_images;
                    self.runtime.derived.recycled = recycled;
                    ran |= result?;
                }
                // Seal the stage: aggregate/computed stages finalize HERE,
                // so a required producer error (overflow, cardinality,
                // scalar failure) fails the query before any consumer
                // could discard it.
                {
                    let interiors = self.pipeline.interiors_mut();
                    seal_interior(
                        &mut interiors[i],
                        i,
                        &mut self.runtime.derived,
                        &mut self.runtime.answer_scratch,
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
                    &ctx,
                    driver,
                    rec_id,
                    &mut self.runtime.derived,
                    &mut self.runtime.bindings,
                    &mut self.runtime.execution_texts,
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
            self.runtime.derived.occ_images.clear();
            return;
        };
        fill_plan_images(plan, &mut self.runtime.derived);
    }
}

fn run_reach<Cnt: Counters>(
    ctx: &RuleCtx<'_>,
    driver: &mut ReachDriver,
    rec_id: usize,
    derived: &mut DerivedImages,
    bindings: &mut Bindings,
    retained_texts: &mut crate::image::TextOwners,
    counters: &mut Cnt,
) -> Result<bool> {
    let images = ctx.images;
    let mut ran = false;

    driver.sink.reset();
    // Watermark drains preserve insertion order in either representation.
    driver.sink.begin(Some(images.source().work().clone()));
    for rule in &mut driver.base {
        if let PreparedRule::FreeJoin(fj) = rule {
            unbind_interior_rule(fj, &mut derived.recycled);
        }
    }
    for rule in &mut driver.rec {
        unbind_interior_rule(rule, &mut derived.recycled);
    }

    let sink_use = if driver.units > 1 {
        SinkUse::Shared
    } else {
        SinkUse::Sole
    };
    for rule in &mut driver.base {
        fill_finished_images(rule, derived);
        ran |= run_rule(
            ctx,
            &mut RuleScratch {
                bindings,
                occ_images: &derived.occ_images,
                recycled: &mut derived.recycled,
            },
            rule,
            sink_use,
            &mut driver.sink,
            counters,
        )?;
    }
    let mut watermark = 0;
    loop {
        let len = driver.sink.len();
        images.source().work().checkpoint()?;
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
        // of the accumulated set.
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
            let result = run_free_join(
                ctx,
                &mut RuleScratch {
                    bindings,
                    occ_images: &derived.occ_images,
                    recycled: &mut derived.recycled,
                },
                rule,
                sink_use,
                &mut driver.sink,
                counters,
            );
            // End every consumer lease, including a failed arm, before
            // the next refill. Derived views never enter parked memos.
            unbind_interior_rule(rule, &mut derived.recycled);
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

fn unbind_interior_views(interior: &mut PreparedInterior, recycled: &mut Vec<Vec<u32>>) {
    for rule in &mut interior.rules {
        if let PreparedRule::FreeJoin(fj) = rule {
            unbind_interior_rule(fj, recycled);
        }
    }
}

fn unbind_interior_rule(rule: &mut super::FreeJoinRule, pool: &mut Vec<Vec<u32>>) {
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
            pool.push(recycled);
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
) -> Result<Arc<RelationImage>> {
    let ReachDriver {
        sink,
        frontier,
        field_types,
        ..
    } = driver;
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
    Ok(image)
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
        derived
            .occ_images
            .insert(occ_idx, Arc::clone(&derived.published[id.index()]));
    }
}

#[cfg(test)]
mod tests;
