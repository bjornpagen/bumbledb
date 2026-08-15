//! The linear-reach driver (`docs/architecture/40-execution.md` § the
//! linear reach driver): one rec SCC, interiors then
//! rec then main. Round 0 runs the base arms through the ordinary rule
//! loop; rounds ≥ 1 run each rec arm against the watermark frontier
//! (the unique self-atom is the marked delta occurrence). An empty Δ ends the rec
//! (`lean/Bumbledb/Exec/Reach.lean: evalLinearReach_eq_lfp`).
//! Interiors-only never enters this module's loop.

use std::sync::Arc;

use super::run_join::run_join;
use super::{
    Bindings, FreeJoinRule, PreparedInterior, PreparedPipeline, PreparedQuery, PreparedRule,
    ProjectionSink, RecArm,
};
use crate::error::{Error, Result};
use crate::exec::run::Counters;
use crate::image::cache::ImageCache;
use crate::image::view::Const;
use crate::image::{RelationImage, TransientImage};
use crate::schema::Schema;
use crate::storage::env::ReadTxn;
use bumbledb_theory::TypeDesc;

/// The default rec-round budget. Generous by the safety theorem's own
/// measure: a real closure's round count is its graph's diameter.
pub const DEFAULT_REACH_ROUNDS: u32 = 1 << 16;

/// The default derived-tuple budget: the 10⁷-row scale axiom, applied
/// to every derived sink (each interior, then the rec table).
pub const DEFAULT_DERIVED_TUPLES: u64 = 10_000_000;

/// The rec's prepared artifact: base arms (round 0), rec arms
/// (rounds ≥ 1, each one [`super::RecArm`]), and the rec's
/// projection sink. Main lives on [`super::PreparedPipeline::Reach`],
/// not here. Tuples budget lives on [`PreparedQuery`]; rounds budget
/// lives on the Reach pipeline arm.
pub(crate) struct ReachDriver {
    pub(super) base: Vec<PreparedRule>,
    pub(super) rec: Vec<RecArm>,
    pub(super) field_types: Vec<TypeDesc>,
    pub(super) sink: crate::exec::sink::ProjectionSink,
    pub(super) units: usize,
    pub(super) scratch: RecPingPong,
}

/// Per-occurrence derived images for one plan unit. Only live derived
/// occurrences have a slot — EDB and discharged occurrences are absent,
/// not `None`.
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

/// How a derived occurrence binds this round. Rec carries both images
/// so delta-without-acc is unrepresentable.
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
    published: Vec<Arc<RelationImage>>,
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

    fn stash_finished(
        &mut self,
        id: usize,
        field_types: &[TypeDesc],
        sink: &ProjectionSink,
    ) -> Arc<RelationImage> {
        debug_assert_eq!(
            self.published.len(),
            id,
            "derived tables seal in declaration order"
        );
        let image = self.working[id].refill(field_types, sink.len(), sink.answers_since(0));
        self.published.push(image.clone());
        image
    }
}

/// Rec ping-pong: two working transients each for Δ and accumulated,
/// flipped each round so buffers recycle. `round_delta` / `round_acc`
/// are locals in [`run_reach`], not fields.
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
    /// The rounds axis is stored on the Reach arm and ignored on Cq
    /// (rounds never advance). The tuples axis judges every query —
    /// interiors-only included. Hosts copy-paste.
    pub fn set_derived_budget(&mut self, rounds: u32, tuples: u64) {
        self.tuples_budget = tuples;
        if let PreparedPipeline::Reach { rounds_budget, .. } = &mut self.pipeline {
            *rounds_budget = rounds;
        }
    }

    /// Interiors in declaration order, then rec. One derived tuples
    /// ledger. Interiors-only never enters [`run_reach`].
    #[expect(
        clippy::too_many_lines,
        reason = "the derived phase reads as one protocol: interiors, then rec"
    )]
    pub(super) fn run_derived<C: Counters>(
        &mut self,
        txn: &ReadTxn<'_>,
        cache: &ImageCache,
        counters: &mut C,
    ) -> Result<bool> {
        let derived_count = match &self.pipeline {
            PreparedPipeline::Cq { interiors, .. } => interiors.len(),
            PreparedPipeline::Reach { derived_count, .. } => {
                usize::try_from(*derived_count).expect("derived_count stored at validate")
            }
        };
        self.derived.begin(derived_count);
        // Main's Interior views still hold last execution's published
        // Arcs; drop them before refill so TransientImage can get_mut.
        {
            let retired = &mut self.derived.retired;
            for rule in self.pipeline.main_rules_mut() {
                if let PreparedRule::FreeJoin(fj) = rule {
                    unbind_interior_rule(fj, retired);
                }
            }
        }
        let fast_eligible = self.unresolved_literals == 0 && self.params.is_empty();
        let mut latched = 0u32;
        let mut ran = false;
        let mut derived_tuples: u64 = 0;

        let n_interiors = self.pipeline.interiors().len();
        if n_interiors > 0 {
            let mut interiors_span =
                crate::obs::span(crate::obs::names::INTERIORS, crate::obs::Category::Execute);
            let mut interior_emits: u64 = 0;
            for i in 0..n_interiors {
                {
                    let interiors = self.pipeline.interiors_mut();
                    unbind_interior_views(&mut interiors[i], &mut self.derived.retired);
                    interiors[i].sink.reset();
                }
                let ctx = RunCtx {
                    schema: self.schema,
                    txn,
                    cache,
                    resolved_params: &self.resolved_params,
                    missed_params: &self.missed_params,
                    fast_eligible,
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
                    ran |= run_into_projection(
                        ctx,
                        &mut interior.rules,
                        rule_idx,
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
                derived_tuples += self.pipeline.interiors()[i].sink.len() as u64;
                interior_emits += self.pipeline.interiors()[i].sink.len() as u64;
                if derived_tuples > self.tuples_budget {
                    return Err(Error::DerivedBudgetExceeded {
                        rounds: 0,
                        tuples: derived_tuples,
                    });
                }
                self.derived.stash_finished(
                    i,
                    &self.pipeline.interiors()[i].field_types,
                    &self.pipeline.interiors()[i].sink,
                );
                if ran {
                    let latched = {
                        let sets = &mut self.pipeline.interiors_mut()[i].ray_probes;
                        run_ray_probe_sets(
                            sets,
                            Some(&mut self.derived),
                            self.schema,
                            txn,
                            cache,
                            &self.resolved_params,
                            &self.missed_params,
                            self.unresolved_literals == 0 && self.params.is_empty(),
                            &mut self.bindings,
                            counters,
                        )?
                    };
                    self.unresolved_literals = self.unresolved_literals.saturating_sub(latched);
                }
            }
            interiors_span.set_args(n_interiors as u64, interior_emits);
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
                    &mut self.derived,
                    &mut self.bindings,
                    &mut self.determinant_key,
                    self.schema,
                    txn,
                    cache,
                    &self.resolved_params,
                    &self.missed_params,
                    fast_eligible,
                    counters,
                    &mut latched,
                    derived_tuples,
                )?
            }
            PreparedPipeline::Cq { .. } => false,
        };
        ran |= rec_ran;

        self.unresolved_literals = self.unresolved_literals.saturating_sub(latched);
        Ok(ran)
    }

    /// Fills `derived.occ_images` for a main-rule plan from finished
    /// interiors (and finished rec).
    pub(super) fn fill_main_images(&mut self, rule_idx: usize) {
        let Some(plan) = main_plan(self.pipeline.main_rules(), rule_idx) else {
            self.derived.occ_images.clear();
            return;
        };
        fill_plan_images(plan, &mut self.derived, DerivedBind::Finished);
    }
}

/// Rec least fixpoint. Interiors-only never calls this. The driver is
/// borrowed from the Reach arm so the match stays matched — derived
/// scratch is a sibling field of the pipeline.
#[expect(
    clippy::too_many_arguments,
    reason = "the prepared query's split borrows are clearer unpacked"
)]
#[expect(
    clippy::too_many_lines,
    reason = "the driver reads as one protocol: reset, round 0, Δ loop, budget"
)]
fn run_reach<C: Counters>(
    driver: &mut ReachDriver,
    rec_id: usize,
    rounds_budget: u32,
    tuples_budget: u64,
    derived: &mut DerivedImages,
    bindings: &mut Bindings,
    determinant_key: &mut Vec<u8>,
    schema: &Schema,
    txn: &ReadTxn<'_>,
    cache: &ImageCache,
    resolved_params: &[Const],
    missed_params: &[bool],
    fast_eligible: bool,
    counters: &mut C,
    latched: &mut u32,
    mut derived_tuples: u64,
) -> Result<bool> {
    let mut ran = false;
    let mut reach_span = crate::obs::span(crate::obs::names::REACH, crate::obs::Category::Execute);

    driver.sink.reset();
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
        txn,
        cache,
        resolved_params,
        missed_params,
        fast_eligible,
    };

    let mut round_span = Some(crate::obs::span(
        crate::obs::names::FIXPOINT_ROUND,
        crate::obs::Category::Execute,
    ));
    let mut round_emits_before = counters.emits();

    // Round 0: base arms into the rec sink.
    for rule_idx in 0..driver.base.len() {
        fill_finished_images(&driver.base[rule_idx], derived);
        ran |= run_into_projection(
            ctx,
            &mut driver.base,
            rule_idx,
            driver.units,
            &derived.occ_images,
            &mut derived.retired,
            &mut driver.sink,
            bindings,
            determinant_key,
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

    let mut rounds: u32 = 0;
    loop {
        let len = driver.sink.len();
        let tuples = derived_tuples + len as u64;
        let any_delta = len > driver.scratch.watermark;
        if !any_delta {
            derived.stash_finished(rec_id, &driver.field_types, &driver.sink);
            reach_span.set_args(u64::from(rounds), tuples);
            break;
        }
        if rounds >= rounds_budget || tuples > tuples_budget {
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
        let round_delta = driver.scratch.delta_mut().refill(
            &driver.field_types,
            len - since,
            driver.sink.answers_since(since),
        );
        let filled = driver.scratch.acc_filled[flip];
        let round_acc = driver
            .scratch
            .acc_mut()
            .append(&driver.field_types, filled, len, |from| {
                driver.sink.answers_since(from)
            });
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
            ran |= run_free_join_into_projection(
                ctx,
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
            span.set_args(emitted, emitted.saturating_sub(newly));
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
        let spare = &mut rule.memo.spare_buffers[occ_idx];
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

/// One ray-probe protocol: resolve interns, latch filters, run the
/// probe Free Join into the arbiter, raise on the first Ray. Main
/// passes no derived images; each interior stage fills finished
/// images before the join.
#[expect(
    clippy::too_many_arguments,
    reason = "the prepared query's split borrows are clearer unpacked"
)]
pub(super) fn run_ray_probe_sets<C: Counters>(
    sets: &mut [super::RayProbeSet],
    mut derived: Option<&mut DerivedImages>,
    schema: &Schema,
    txn: &ReadTxn<'_>,
    cache: &ImageCache,
    resolved_params: &[crate::image::view::Const],
    missed_params: &[bool],
    fast_eligible: bool,
    bindings: &mut Bindings,
    counters: &mut C,
) -> Result<u32> {
    let mut latched = 0u32;
    let empty = OccImages::default();
    let mut no_retired = Vec::new();
    for set in sets {
        set.verdict.resolve_interns(txn)?;
        let super::RayProbeSet { verdict, probes } = set;
        for probe in probes {
            if let Some(derived) = derived.as_mut() {
                fill_plan_images(&probe.rule.plan, derived, DerivedBind::Finished);
            }
            let resolved =
                if fast_eligible && probe.rule.resolution == super::ResolutionState::Complete {
                    true
                } else {
                    let complete = super::bind::resolve_filters(
                        txn,
                        &mut probe.rule.plan,
                        resolved_params,
                        missed_params,
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
            bindings.resize(probe.rule.plan.slot_count());
            let mut arbiter = crate::exec::verdict::RayArbiter::new(
                verdict,
                resolved_params,
                probe.measured_slot,
            );
            let (images, retired) = match derived.as_mut() {
                Some(derived) => (&derived.occ_images, &mut derived.retired),
                None => (&empty, &mut no_retired),
            };
            run_join(
                &probe.rule.plan,
                schema,
                txn,
                cache,
                &mut probe.rule.executor,
                bindings,
                &probe.rule.resolved_filters,
                &probe.rule.resolved_selections,
                &mut probe.rule.memo,
                images,
                retired,
                &mut arbiter,
                counters,
            )?;
            if let Some([start, end]) = arbiter.measure_of_ray() {
                return Err(crate::error::Error::MeasureOfRay { start, end });
            }
        }
    }
    Ok(latched)
}

/// Runs one plan unit into a projection sink (interior or rec base).
#[expect(
    clippy::too_many_arguments,
    reason = "the prepared query's split borrows are clearer unpacked"
)]
fn run_into_projection<C: Counters>(
    ctx: RunCtx<'_>,
    rules: &mut [PreparedRule],
    rule_idx: usize,
    units: usize,
    occ_images: &OccImages,
    retired: &mut Vec<Vec<u32>>,
    sink: &mut ProjectionSink,
    bindings: &mut Bindings,
    determinant_key: &mut Vec<u8>,
    latched: &mut u32,
    counters: &mut C,
) -> Result<bool> {
    let multi_unit = units > 1;
    match &mut rules[rule_idx] {
        PreparedRule::KeyProbe(rule) => {
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
            Ok(true)
        }
        PreparedRule::FreeJoin(rule) => run_free_join_into_projection(
            ctx, rule, units, occ_images, retired, sink, bindings, latched, counters,
        ),
    }
}

/// Runs one rec arm's Free Join into a projection sink.
#[expect(
    clippy::too_many_arguments,
    reason = "the prepared query's split borrows are clearer unpacked"
)]
fn run_free_join_into_projection<C: Counters>(
    ctx: RunCtx<'_>,
    rule: &mut FreeJoinRule,
    units: usize,
    occ_images: &OccImages,
    retired: &mut Vec<Vec<u32>>,
    sink: &mut ProjectionSink,
    bindings: &mut Bindings,
    latched: &mut u32,
    counters: &mut C,
) -> Result<bool> {
    let multi_unit = units > 1;
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
