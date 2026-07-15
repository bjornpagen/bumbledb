//! The per-stratum fixpoint driver (`docs/architecture/40-execution.md`
//! § the fixpoint driver) — semi-naive evaluation over the existing
//! run-rule machinery, computing exactly
//! `lean/Bumbledb/Exec/Fixpoint.lean: evalProgram`'s answers
//! (`program_eval_sound`).
//!
//! Per stratum, in condensation order (`ir/validate/strata.rs` — the
//! SCC index IS the stratification witness): round 0 runs the
//! stratum's non-recursive rules through the rule loop verbatim —
//! rules sequentially into one sink, the sink's dedup spanning them —
//! and round r ≥ 1 runs each recursive rule's delta variants with the
//! delta occurrence bound to round r−1's frontier and same-stratum
//! occurrences bound to the accumulated images. An empty Δ ends the
//! stratum (`lean/Bumbledb/Exec/Fixpoint.lean: fueledLoop` stops on no
//! change; `semi_naive_agrees` is why the frontier form walks the same
//! chain). Strata above the output's are never evaluated —
//! `evalProgram` reads the output's table after ITS stratum closes
//! (`evalProgramAt`), and the driver matches the model.
//!
//! **The frontier IS the sink's seen-set with a per-round watermark**:
//! `WordMap` preserves insertion order with dense O(len) iteration, so
//! round r's frontier is exactly the dense suffix `[watermark, len)` —
//! one `usize` read per round and a cold suffix walk
//! (`exec/wordmap/clear.rs: iter_since`,
//! `exec/sink/projection/new.rs: answers_since`); no flag, no branch,
//! no state on the emit path. Interior predicates own
//! projection-shaped seen-sets of bound variables — projection-shaped
//! by construction: the validation roster refuses folds in interior
//! heads (`AggregateInteriorPredicate`), measures in interior heads
//! recursive or not (`MeasureInteriorPredicate`), and measures in
//! recursive heads (`MeasureInRecursiveHead`); folds and measures are
//! legal only at the output predicate's head, where the ordinary
//! head-owned sink and finalize live. **Union stays the sink and only the
//! sink**: no merge node, no frontier queue, no worklist structure
//! exists.
//!
//! **The budget is the one new trust boundary.** Termination is a
//! theorem of the validation roster (`lean/Bumbledb/Exec/Fixpoint.lean:
//! program_den_finite` — the fuel bound is `missingCount_le`, a lemma,
//! not a hope), but the fixpoint's *size* is data-shaped: a foreign
//! query may legally demand a quadratic closure. The driver carries an
//! iteration/tuple budget with documented defaults and the typed
//! execution error [`crate::Error::FixpointBudgetExceeded`] — on
//! `MeasureOfRay`'s model: aborts the query, the snapshot stays
//! usable, payload ids and counts, never strings. Policy stays
//! host-owned ([`crate::PreparedQuery::set_fixpoint_budget`]); the
//! defaults exist so the boundary is never unguarded.

use std::sync::Arc;

use super::run_join::run_join;
use super::{Bindings, EitherSink, PreparedQuery, PreparedRule, Program, ProjectionSink};
use crate::encoding::TypeDesc;
use crate::error::{Error, Result};
use crate::exec::run::Counters;
use crate::image::cache::ImageCache;
use crate::image::view::Const;
use crate::image::{RelationImage, TransientImage};
use crate::ir::PredId;
use crate::ir::normalize::OccId;
use crate::schema::Schema;
use crate::storage::env::ReadTxn;

/// The default per-stratum round budget. Generous by the safety
/// theorem's own measure: the fueled loop reaches the least fixpoint
/// within candidate-count growing rounds
/// (`lean/Bumbledb/Exec/Fixpoint.lean: fueledLoop_fixpoint`), and a
/// real closure's round count is its graph's diameter — three orders of
/// magnitude of headroom over any workload the scale axiom admits.
pub const DEFAULT_FIXPOINT_ROUNDS: u32 = 1 << 16;

/// The default per-stratum derived-tuple budget: the 10⁷-row scale
/// axiom's order of magnitude (`docs/architecture/00-product.md`),
/// applied to derived tuples — a quadratic closure over a large
/// component crosses it long before the OS backstop is in play.
pub const DEFAULT_FIXPOINT_TUPLES: u64 = 10_000_000;

/// The prepared fixpoint program: per-predicate prepared rules under
/// the stratification witness, plus the driver's retained-capacity
/// image scratch (the allocation contract's iteration-shape axis).
pub(crate) struct FixpointProgram {
    /// Predicates in `PredId` order. Predicates above the output's
    /// stratum carry no rules and no sink — `evalProgram` never
    /// evaluates them.
    pub(super) predicates: Vec<FixpointPredicate>,
    /// The program's answer predicate — its sink is the prepared
    /// query's main head-owned sink.
    pub(super) output: PredId,
    /// The output predicate's stratum: the last stratum the driver runs.
    pub(super) top_stratum: u16,
    /// Per stratum `0..=top_stratum`: its predicates' indices — computed
    /// once at prepare so the driver's stratum walk allocates nothing.
    pub(super) strata_members: Vec<Vec<usize>>,
    /// The per-stratum round budget (host-settable;
    /// [`DEFAULT_FIXPOINT_ROUNDS`]).
    pub(super) rounds_budget: u32,
    /// The per-stratum derived-tuple budget (host-settable;
    /// [`DEFAULT_FIXPOINT_TUPLES`]).
    pub(super) tuples_budget: u64,
    /// The driver's transient-image pools and frontier bookkeeping.
    pub(super) scratch: FixpointScratch,
}

/// One predicate's prepared artifact inside a fixpoint program.
pub(super) struct FixpointPredicate {
    /// The predicate's stratum (the SCC condensation index).
    pub(super) stratum: u16,
    /// Whether the predicate belongs to a recursive SCC (it reads its
    /// own stratum) — such predicates enter the round loop.
    pub(super) recursive: bool,
    /// The signature columns' encoding shapes — the transient image's
    /// field types (`image::TransientImage::refill`).
    pub(super) field_types: Vec<TypeDesc>,
    /// The predicate's prepared rules: `FreeJoin`/`KeyProbe` run in
    /// round 0; `Recursive` variants run in rounds ≥ 1.
    pub(super) rules: Vec<PreparedRule>,
    /// The interior projection-shaped seen-set sink; `None` for the
    /// output predicate (whose sink is the main head-owned one).
    pub(super) sink: Option<ProjectionSink>,
    /// How many plan units the predicate carries (rules, with a
    /// recursive rule counting its variants): a single-unit predicate's
    /// sink is built aimed and never re-aims.
    pub(super) units: usize,
}

/// The driver's retained-capacity scratch: pooled transient-image
/// slots (delta and accumulated ping-pong pairs plus one finished slot
/// per predicate) and the per-round bookkeeping. Pools reach the
/// monotone high-water fixpoint per (data generation, parameter
/// envelope, iteration shape); a warm re-execution whose per-round row
/// counts fit every prior high-water touches the allocator zero times.
#[derive(Default)]
pub(super) struct FixpointScratch {
    /// Per predicate: the delta image ping-pong — round r's delta
    /// builds into the half round r−1's views no longer hold. The
    /// driver unbinds every `Idb` COLT at execution entry and resets
    /// the flips, so the (round → half) assignment is
    /// execution-invariant: one run at a new parameter envelope grows
    /// every half it will ever need, and repeats are allocation-silent
    /// (the high-water window's contract).
    delta: Vec<[TransientImage; 2]>,
    /// Per predicate: the accumulated image ping-pong.
    acc: Vec<[TransientImage; 2]>,
    /// Per predicate: which delta/acc half the next refill targets —
    /// reset per execution (the deterministic-assignment discipline).
    flip: Vec<bool>,
    /// Per predicate: the finished-image slot (lower-stratum readers) —
    /// one slot suffices: it refills once per execution, after the
    /// entry unbind returned its `Arc` to refcount 1.
    finished_slot: Vec<TransientImage>,
    /// Per predicate: rows already inside a previous frontier.
    watermark: Vec<usize>,
    /// Per predicate: this round's delta image.
    round_delta: Vec<Option<Arc<RelationImage>>>,
    /// Per predicate: this round's accumulated image.
    round_acc: Vec<Option<Arc<RelationImage>>>,
    /// Per predicate: the finished image, built once per execution at
    /// the first lower-stratum read.
    finished: Vec<Option<Arc<RelationImage>>>,
    /// Per occurrence of the currently running plan: the `Idb` images
    /// `run_join` binds. Reused across variants and rounds.
    idb_images: Vec<Option<Arc<RelationImage>>>,
    /// Retired survivor buffers: the entry unbind recycles each `Idb`
    /// COLT's view buffer, and a filtered `Idb` occurrence circulates
    /// TWO buffers (the live view's and the spare) against one spare
    /// slot — the second parks here and the first spare-starved rebind
    /// pops it back (`run_join`'s Idb arm). Balance is one push and one
    /// pop per filtered occurrence per execution; the stack's capacity
    /// is a high-water like every pool.
    retired: Vec<Vec<u32>>,
}

impl FixpointScratch {
    /// Begins one execution: sizes the pools (a no-op past the first
    /// execution), resets the frontier bookkeeping and the ping-pong
    /// flips, and drops the per-round `Arc` clones.
    fn begin(&mut self, count: usize) {
        self.delta.resize_with(count, Default::default);
        self.acc.resize_with(count, Default::default);
        self.flip.clear();
        self.flip.resize(count, false);
        self.finished_slot.resize_with(count, Default::default);
        self.watermark.clear();
        self.watermark.resize(count, 0);
        self.round_delta.clear();
        self.round_delta.resize(count, None);
        self.round_acc.clear();
        self.round_acc.resize(count, None);
        self.finished.clear();
        self.finished.resize(count, None);
        self.idb_images.clear();
    }
}

/// The interior view of one predicate's seen-set sink: its own
/// projection sink, or the main sink when the predicate is the output.
///
/// # Panics
///
/// Only on a programmer-invariant violation: a fold-headed output is
/// never recursive (`AggregationThroughCycle`) and never read by an
/// evaluated predicate (a reader at or below the output's stratum would
/// share its SCC), so no seen-set read can land on an aggregate sink.
fn seen_sink<'a>(pred: &'a FixpointPredicate, main: &'a EitherSink) -> &'a ProjectionSink {
    match &pred.sink {
        Some(sink) => sink,
        None => match main {
            EitherSink::Projection(sink) => sink,
            EitherSink::Aggregate(_) => {
                unreachable!("a fold-headed output is never read as a predicate table")
            }
        },
    }
}

impl<S> PreparedQuery<'_, S> {
    /// Amends this prepared query's fixpoint budget — the host-owned
    /// policy knob behind the typed condition
    /// ([`crate::Error::FixpointBudgetExceeded`]): the engine ships the
    /// condition and a documented default
    /// ([`DEFAULT_FIXPOINT_ROUNDS`] / [`DEFAULT_FIXPOINT_TUPLES`]),
    /// never a threshold loop — the staleness doctrine verbatim. A
    /// no-op on non-recursive programs (no fixpoint exists to bound).
    pub fn set_fixpoint_budget(&mut self, rounds: u32, tuples: u64) {
        if let Program::Fixpoint(program) = &mut self.program {
            program.rounds_budget = rounds;
            program.tuples_budget = tuples;
        }
    }

    /// The driver: strata in condensation order, round 0 the rule loop
    /// verbatim, rounds ≥ 1 the delta variants against the watermark
    /// frontier. Returns `Ok(true)` when any rule ran (the rule loop's
    /// contract — `Ok(false)` means every rule short-circuited and the
    /// caller skips finalize).
    ///
    /// # Errors
    ///
    /// [`Error::FixpointBudgetExceeded`] past the budget;
    /// `Lmdb`/`Corruption` from storage reads. (`MeasureOfRay` cannot
    /// arise inside the driver: interior measure heads are
    /// validation-refused, and the output's measure poison is checked
    /// at finalize on the main sink.)
    ///
    /// # Panics
    ///
    /// Only on programmer-invariant violations (plan/executor pairing,
    /// the seen-sink shape argument above).
    #[expect(
        clippy::too_many_lines,
        reason = "the driver reads as one protocol: reset, resolve, strata, rounds"
    )]
    pub(super) fn run_fixpoint<C: Counters>(
        &mut self,
        txn: &ReadTxn<'_>,
        cache: &ImageCache,
        counters: &mut C,
    ) -> Result<bool> {
        self.sink.reset();
        let fast_eligible = self.unresolved_literals == 0 && self.params.is_empty();
        let mut latched = 0u32;
        let mut ran = false;
        let Program::Fixpoint(program) = &mut self.program else {
            unreachable!("run_fixpoint is dispatched on the Fixpoint variant")
        };
        let FixpointProgram {
            predicates,
            output,
            top_stratum,
            strata_members,
            rounds_budget,
            tuples_budget,
            scratch,
        } = program.as_mut();
        let output_idx = usize::from(output.0);
        scratch.begin(predicates.len());
        // Reset the interior seen-set sinks (capacity retained — the
        // rule loop's discipline, per predicate) and unbind every `Idb`
        // COLT view: dropping last execution's transient views returns
        // every pooled image `Arc` to refcount 1 — the refill-in-place
        // precondition — and makes the (round → ping-pong half)
        // assignment execution-invariant, which is what makes a repeat
        // execution allocation-silent. Survivor buffers are preserved
        // whole: the spare slot keeps one, the retired stack the other.
        for pred in predicates.iter_mut() {
            if let Some(sink) = &mut pred.sink {
                sink.reset();
            }
            for rule in &mut pred.rules {
                match rule {
                    PreparedRule::FreeJoin(fj) => unbind_idb_views(fj, &mut scratch.retired),
                    PreparedRule::Recursive(rule) => {
                        for variant in &mut rule.variants {
                            unbind_idb_views(&mut variant.rule, &mut scratch.retired);
                        }
                    }
                    PreparedRule::KeyProbe(_) => {}
                }
            }
        }

        // The strata, in condensation order, through the output's own
        // stratum (`evalProgramAt`: higher strata are never evaluated).
        for stratum in 0..=*top_stratum {
            let members: &[usize] = &strata_members[usize::from(stratum)];
            // Only strata with recursive predicates iterate (a
            // non-recursive SCC's fixpoint is one application:
            // `lean/Bumbledb/Exec/Fixpoint.lean: lfpP_const` is the
            // degenerate embedding's engine) — and only they carry the
            // counted round surface (`Counters::fixpoint_round`; the
            // rule_N span convention, stratum index in the name).
            let recursive = members.iter().any(|&p| predicates[p].recursive);
            let mut stratum_span = recursive.then(|| {
                crate::obs::span(
                    crate::obs::names::STRATUM[usize::from(stratum)],
                    crate::obs::Category::Execute,
                )
            });
            let mut round_span = recursive.then(|| {
                crate::obs::span(
                    crate::obs::names::FIXPOINT_ROUND,
                    crate::obs::Category::Execute,
                )
            });
            let mut round_emits_before = counters.emits();
            // Round 0: the stratum's non-recursive rules, the rule loop
            // verbatim — each predicate's rules sequentially into its
            // one sink, dedup spanning them.
            for &p in members {
                for rule_idx in 0..predicates[p].rules.len() {
                    if matches!(predicates[p].rules[rule_idx], PreparedRule::Recursive(_)) {
                        continue;
                    }
                    build_lower_images(predicates, scratch, &self.sink, p, rule_idx, None, stratum);
                    ran |= run_unit(
                        RunCtx {
                            schema: self.schema,
                            txn,
                            cache,
                            resolved_params: &self.resolved_params,
                            missed_params: &self.missed_params,
                            fast_eligible,
                        },
                        &mut predicates[p],
                        rule_idx,
                        None,
                        &scratch.idb_images,
                        &mut scratch.retired,
                        &mut self.sink,
                        &mut self.bindings,
                        &mut self.determinant_key,
                        output_idx == p,
                        &mut latched,
                        counters,
                    )?;
                }
            }
            // Round 0 closes for a recursive stratum: the counted
            // union accounting (emitted vs newly seen — the rule
            // loop's O(1)-reads convention) reported through the
            // fixpoint hooks; `NoopCounters` monomorphizes the report
            // away and the emit path stays untouched.
            if recursive {
                let emitted = counters.emits() - round_emits_before;
                let newly: u64 = members
                    .iter()
                    .map(|&p| seen_sink(&predicates[p], &self.sink).len() as u64)
                    .sum();
                counters.fixpoint_round(stratum, emitted, emitted.saturating_sub(newly));
                if let Some(mut span) = round_span.take() {
                    span.set_args(emitted, emitted.saturating_sub(newly));
                }
            }
            // The round loop — recursive strata only (`recursive`
            // above).
            if !recursive {
                continue;
            }
            let mut rounds: u32 = 0;
            loop {
                // The frontier check: one `usize` read per predicate.
                let mut tuples: u64 = 0;
                let mut any_delta = false;
                for &p in members {
                    let len = seen_sink(&predicates[p], &self.sink).len();
                    tuples += len as u64;
                    any_delta |= len > scratch.watermark[p];
                }
                if !any_delta {
                    // Convergence: the stratum is finished. Later
                    // readers build the finished image from the closed
                    // seen-set into the DEDICATED finished slot — never
                    // an alias of the accumulated pool, whose halves
                    // the next execution's rounds refill in place (the
                    // continuous-flip discipline demands each pool's
                    // `Arc`s return to refcount 1 on their own cadence).
                    if let Some(mut span) = stratum_span.take() {
                        span.set_args(u64::from(rounds), tuples);
                    }
                    break;
                }
                // The budget — the one new trust boundary: rounds and
                // derived tuples, checked before another round runs.
                if rounds >= *rounds_budget || tuples > *tuples_budget {
                    return Err(Error::FixpointBudgetExceeded {
                        stratum,
                        rounds,
                        tuples,
                    });
                }
                rounds += 1;
                round_span = Some(crate::obs::span(
                    crate::obs::names::FIXPOINT_ROUND,
                    crate::obs::Category::Execute,
                ));
                round_emits_before = counters.emits();
                // Build the round's delta and accumulated images: the
                // seen-set's dense suffix transposed into pooled slots
                // (never the cache, never the memo — the transient-image
                // invariant: outside every generation-keyed mechanism).
                for &p in members {
                    let flip = usize::from(scratch.flip[p]);
                    let sink = seen_sink(&predicates[p], &self.sink);
                    let len = sink.len();
                    let since = scratch.watermark[p];
                    counters.fixpoint_delta(
                        u16::try_from(p).expect("MAX_PREDICATES bounds predicate indices"),
                        (len - since) as u64,
                    );
                    scratch.round_delta[p] = Some(scratch.delta[p][flip].refill(
                        &predicates[p].field_types,
                        len - since,
                        sink.answers_since(since),
                    ));
                    scratch.round_acc[p] = Some(scratch.acc[p][flip].refill(
                        &predicates[p].field_types,
                        len,
                        sink.answers_since(0),
                    ));
                    scratch.flip[p] = !scratch.flip[p];
                    scratch.watermark[p] = len;
                }
                // Every recursive rule's variants, delta bound to the
                // frontier, accumulated to the rest. D2's suffix skip
                // stays per-rule and within-round — each variant run is
                // one `run_join`, exactly the rule loop's unit.
                for &p in members {
                    for rule_idx in 0..predicates[p].rules.len() {
                        let PreparedRule::Recursive(rule) = &predicates[p].rules[rule_idx] else {
                            continue;
                        };
                        let variant_count = rule.variants.len();
                        for variant_idx in 0..variant_count {
                            let PreparedRule::Recursive(rule) = &predicates[p].rules[rule_idx]
                            else {
                                unreachable!("matched above")
                            };
                            let delta = rule.variants[variant_idx].delta;
                            build_lower_images(
                                predicates,
                                scratch,
                                &self.sink,
                                p,
                                rule_idx,
                                Some(variant_idx),
                                stratum,
                            );
                            fill_round_images(
                                predicates,
                                scratch,
                                p,
                                rule_idx,
                                variant_idx,
                                delta,
                                stratum,
                            );
                            ran |= run_unit(
                                RunCtx {
                                    schema: self.schema,
                                    txn,
                                    cache,
                                    resolved_params: &self.resolved_params,
                                    missed_params: &self.missed_params,
                                    fast_eligible,
                                },
                                &mut predicates[p],
                                rule_idx,
                                Some(variant_idx),
                                &scratch.idb_images,
                                &mut scratch.retired,
                                &mut self.sink,
                                &mut self.bindings,
                                &mut self.determinant_key,
                                output_idx == p,
                                &mut latched,
                                counters,
                            )?;
                        }
                    }
                }
                // The round closes: the same union accounting as round
                // 0 — emitted across the round's variant runs, newly
                // seen against the pre-round watermarks.
                let emitted = counters.emits() - round_emits_before;
                let newly: u64 = members
                    .iter()
                    .map(|&p| {
                        (seen_sink(&predicates[p], &self.sink).len() - scratch.watermark[p]) as u64
                    })
                    .sum();
                counters.fixpoint_round(stratum, emitted, emitted.saturating_sub(newly));
                if let Some(mut span) = round_span.take() {
                    span.set_args(emitted, emitted.saturating_sub(newly));
                }
            }
        }
        // No interior poison check exists: interior heads carry bound
        // variables only — folds and measures are validation-refused
        // (`AggregateInteriorPredicate` / `MeasureInteriorPredicate` /
        // `MeasureInRecursiveHead`), so no interior seen-set can record
        // a measure ray. The output's measure poison lives on the main
        // sink, checked at finalize (`execute.rs`).
        self.unresolved_literals = self.unresolved_literals.saturating_sub(latched);
        Ok(ran)
    }
}

/// The shared per-run context (split borrows of the prepared query).
#[derive(Clone, Copy)]
struct RunCtx<'a> {
    schema: &'a Schema,
    txn: &'a ReadTxn<'a>,
    cache: &'a ImageCache,
    resolved_params: &'a [Const],
    missed_params: &'a [bool],
    fast_eligible: bool,
}

/// Unbinds a rule's `Idb` COLT views (`View::Unbound`) at execution
/// entry, preserving both circulating survivor buffers: the spare slot
/// keeps one, the retired stack the other (a filtered `Idb`
/// occurrence's rebind pops it back — the allocation contract's
/// balance).
fn unbind_idb_views(rule: &mut super::FreeJoinRule, retired: &mut Vec<Vec<u32>>) {
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

/// Ensures every finished (lower-stratum) predicate this plan unit
/// reads has its image built, and clears + sizes the per-occurrence
/// image scratch. Runs before the unit's mutable borrow: it reads every
/// sink immutably.
fn build_lower_images(
    predicates: &[FixpointPredicate],
    scratch: &mut FixpointScratch,
    main: &EitherSink,
    pred_idx: usize,
    rule_idx: usize,
    variant_idx: Option<usize>,
    stratum: u16,
) {
    let Some(plan) = unit_plan(&predicates[pred_idx].rules[rule_idx], variant_idx) else {
        scratch.idb_images.clear();
        return;
    };
    scratch.idb_images.clear();
    scratch.idb_images.resize(plan.occurrences().len(), None);
    for occurrence in plan.occurrences() {
        if occurrence.role.discharged() {
            continue;
        }
        let Some(target) = occurrence.source.idb() else {
            continue;
        };
        let q = usize::from(target.0);
        if predicates[q].stratum == stratum {
            continue; // in-stratum images are the round's (fill_round_images)
        }
        if scratch.finished[q].is_none() {
            let sink = seen_sink(&predicates[q], main);
            scratch.finished[q] = Some(scratch.finished_slot[q].refill(
                &predicates[q].field_types,
                sink.len(),
                sink.answers_since(0),
            ));
        }
    }
    fill_images(
        predicates,
        scratch,
        pred_idx,
        rule_idx,
        variant_idx,
        None,
        stratum,
    );
}

/// Fills the per-occurrence image scratch for one recursive variant:
/// the delta occurrence takes the round's frontier image, other
/// same-stratum occurrences the accumulated image, lower strata their
/// finished images (already built by [`build_lower_images`]).
fn fill_round_images(
    predicates: &[FixpointPredicate],
    scratch: &mut FixpointScratch,
    pred_idx: usize,
    rule_idx: usize,
    variant_idx: usize,
    delta: OccId,
    stratum: u16,
) {
    fill_images(
        predicates,
        scratch,
        pred_idx,
        rule_idx,
        Some(variant_idx),
        Some(delta),
        stratum,
    );
}

/// The shared image-slot fill (both callers above).
fn fill_images(
    predicates: &[FixpointPredicate],
    scratch: &mut FixpointScratch,
    pred_idx: usize,
    rule_idx: usize,
    variant_idx: Option<usize>,
    delta: Option<OccId>,
    stratum: u16,
) {
    let Some(plan) = unit_plan(&predicates[pred_idx].rules[rule_idx], variant_idx) else {
        return;
    };
    for (occ_idx, occurrence) in plan.occurrences().iter().enumerate() {
        if occurrence.role.discharged() {
            continue;
        }
        let Some(target) = occurrence.source.idb() else {
            continue;
        };
        let q = usize::from(target.0);
        let image = if predicates[q].stratum == stratum {
            if delta == Some(occurrence.occ_id) {
                scratch.round_delta[q].clone()
            } else {
                scratch.round_acc[q].clone()
            }
        } else {
            scratch.finished[q].clone()
        };
        debug_assert!(
            image.is_some(),
            "every evaluated Idb target has an image before its reader runs"
        );
        scratch.idb_images[occ_idx] = image;
    }
}

/// The plan of one unit (a rule, or one variant of a recursive rule) —
/// `None` for key probes (no Free Join plan, no `Idb` occurrence).
fn unit_plan(
    rule: &PreparedRule,
    variant_idx: Option<usize>,
) -> Option<&crate::plan::fj::ValidatedPlan> {
    match (rule, variant_idx) {
        (PreparedRule::FreeJoin(rule), None) => Some(&rule.plan),
        (PreparedRule::Recursive(rule), Some(idx)) => Some(&rule.variants[idx].rule.plan),
        (PreparedRule::KeyProbe(_), None) => None,
        _ => unreachable!("variant indices address recursive rules only"),
    }
}

/// Runs one plan unit into its predicate's sink: re-aim (multi-unit
/// predicates only), resolve this execution's filter constants, and run
/// the plan — key probe or Free Join — through `run_join` with the
/// driver's `Idb` images. `Ok(false)` = the positive-occurrence `Eq`
/// short-circuit (this unit contributes nothing on this snapshot).
#[expect(
    clippy::too_many_arguments,
    reason = "the prepared query's split borrows are clearer unpacked (run_join's own convention)"
)]
#[expect(
    clippy::too_many_lines,
    reason = "one unit's protocol reads whole: aim, resolve, run — per sink shape"
)]
fn run_unit<C: Counters>(
    ctx: RunCtx<'_>,
    pred: &mut FixpointPredicate,
    rule_idx: usize,
    variant_idx: Option<usize>,
    idb_images: &[Option<Arc<RelationImage>>],
    idb_retired: &mut Vec<Vec<u32>>,
    main: &mut EitherSink,
    bindings: &mut Bindings,
    determinant_key: &mut Vec<u8>,
    is_output: bool,
    latched: &mut u32,
    counters: &mut C,
) -> Result<bool> {
    let FixpointPredicate {
        rules, sink, units, ..
    } = pred;
    let multi_unit = *units > 1;
    // Key probes carry no Free Join scratch; run them first.
    if let (PreparedRule::KeyProbe(rule), None) = (&rules[rule_idx], variant_idx) {
        bindings.resize(rule.plan.slot_count());
        return match (sink.as_mut(), is_output) {
            (Some(sink), false) => {
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
            (None, true) => {
                if multi_unit {
                    main.aim(&rule.finds, rule.plan.slot_count());
                }
                crate::exec::dispatch::execute_key_probe(
                    &rule.plan,
                    ctx.txn,
                    ctx.schema,
                    ctx.resolved_params,
                    determinant_key,
                    bindings,
                    main,
                    counters,
                )?;
                Ok(true)
            }
            _ => unreachable!("exactly the output predicate lacks an interior sink"),
        };
    }
    let rule = match (&mut rules[rule_idx], variant_idx) {
        (PreparedRule::FreeJoin(rule), None) => rule,
        (PreparedRule::Recursive(rule), Some(idx)) => &mut rule.variants[idx].rule,
        _ => unreachable!("variant indices address recursive rules only"),
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
    match (sink.as_mut(), is_output) {
        (Some(sink), false) => {
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
                idb_images,
                idb_retired,
                sink,
                counters,
            )?;
        }
        (None, true) => {
            if multi_unit {
                main.aim(&rule.finds, rule.plan.slot_count());
            }
            match main {
                EitherSink::Projection(sink) => run_join(
                    &rule.plan,
                    ctx.schema,
                    ctx.txn,
                    ctx.cache,
                    &mut rule.executor,
                    bindings,
                    &rule.resolved_filters,
                    &rule.resolved_selections,
                    &mut rule.memo,
                    idb_images,
                    idb_retired,
                    sink,
                    counters,
                )?,
                EitherSink::Aggregate(sink) => run_join(
                    &rule.plan,
                    ctx.schema,
                    ctx.txn,
                    ctx.cache,
                    &mut rule.executor,
                    bindings,
                    &rule.resolved_filters,
                    &rule.resolved_selections,
                    &mut rule.memo,
                    idb_images,
                    idb_retired,
                    sink.as_mut(),
                    counters,
                )?,
            }
        }
        _ => unreachable!("exactly the output predicate lacks an interior sink"),
    }
    Ok(true)
}
