use super::{
    Answers, BindValue, KeyProbeRule, PreparedPipeline, PreparedQuery, PreparedRule, ValueType,
};

use crate::error::Result;
use crate::exec::dispatch::execute_key_probe;
use crate::exec::run::{Counters, NoopCounters};
use crate::image::cache::ImageCache;
use crate::obs;
use crate::storage::env::ReadTxn;

use super::bind::resolve_filters;
use super::finalize::finalize;
use super::run_join::run_join;

impl<S> PreparedQuery<'_, S> {
    /// Executes with the given parameters into the caller's buffer.
    ///
    /// # Errors
    ///
    /// `ParamCountMismatch`/`ParamTypeMismatch` at bind time; `Overflow`
    /// from aggregate finalization; `Lmdb`/`Corruption` from storage.
    ///
    /// # Panics
    ///
    /// Only on programmer-invariant violations (plan/executor pairing,
    /// validated id widths).
    pub(crate) fn execute(
        &mut self,
        txn: &ReadTxn<'_>,
        cache: &ImageCache,
        params: &[BindValue<'_>],
        out: &mut Answers,
    ) -> Result<()> {
        self.check_snapshot(txn)?;
        let mut execute_span = obs::span(obs::names::EXECUTE, obs::Category::Execute);
        out.clear();
        out.arity = self.signature.columns.len();
        {
            let _s = obs::span(obs::names::BIND_PARAMS, obs::Category::Execute);
            self.bind_params(txn, params)?;
        }
        let result = self.run_bound(txn, cache, out);
        execute_span.set_args(out.len() as u64, 0);
        result
    }

    /// Executes with mixed scalar/set parameter arguments — the
    /// [`super::ParamArg`] entry behind [`crate::Snapshot::execute_args`].
    ///
    /// # Errors
    ///
    /// As [`Self::execute`], plus the precise per-position bind errors
    /// (`ParamSetExpected`/`ParamScalarExpected`/
    /// `ParamElementTypeMismatch`).
    pub(crate) fn execute_args(
        &mut self,
        txn: &ReadTxn<'_>,
        cache: &ImageCache,
        args: &[super::ParamArg<'_>],
        out: &mut Answers,
    ) -> Result<()> {
        self.check_snapshot(txn)?;
        let mut execute_span = obs::span(obs::names::EXECUTE, obs::Category::Execute);
        out.clear();
        out.arity = self.signature.columns.len();
        {
            let _s = obs::span(obs::names::BIND_PARAMS, obs::Category::Execute);
            self.bind_param_args(txn, args)?;
        }
        let result = self.run_bound(txn, cache, out);
        execute_span.set_args(out.len() as u64, 0);
        result
    }

    /// The post-bind execution body shared by every bind shape: the rule
    /// loop into the one sink, then finalize.
    fn run_bound(
        &mut self,
        txn: &ReadTxn<'_>,
        cache: &ImageCache,
        out: &mut Answers,
    ) -> Result<()> {
        // Direct lane and empty Cq are parsed at build / are the
        // zero-iteration loop — not re-detected per call.
        if self.pipeline.is_key_probe_direct() {
            return self.execute_key_probe_direct(txn, out);
        }
        if self.pipeline.is_empty_cq() {
            return Ok(());
        }
        // Phase attribution engages only under an active obs capture
        // (docs/architecture/60-validation.md): timing runs — even obs
        // builds — monomorphize NoopCounters and pay nothing. With
        // `trace` off, `obs::capturing()` is a compile-time `false` and
        // `PhaseTimers` its ZST twin — the branch folds away, written
        // once, `#[cfg]`-free (the obs.rs law).
        let ran = if obs::capturing() {
            let mut timers = crate::exec::run::PhaseTimers::new();
            let ran = self.run_rules(txn, cache, &mut timers)?;
            timers.flush();
            ran
        } else {
            self.run_rules(txn, cache, &mut NoopCounters)?
        };
        self.finish_sink(txn, ran, out)
    }

    /// Drain the sink into `out` after the shared rule loop. Empty
    /// short-circuit (nothing ran) skips finalize.
    pub(super) fn finish_sink(
        &mut self,
        txn: &ReadTxn<'_>,
        ran: bool,
        out: &mut Answers,
    ) -> Result<()> {
        // The sink-side measure poison (a ray reached a projected or
        // folded `Duration`): the engine's one runtime type error,
        // raised before finalize — never a partial result. Executor-side
        // rays (measure residuals) already surfaced through `run_join`.
        if let Some([start, end]) = self.sink.measure_of_ray() {
            return Err(crate::error::Error::MeasureOfRay { start, end });
        }
        if !ran {
            return Ok(()); // every rule short-circuited: empty result
        }
        let _s = obs::span(obs::names::FINALIZE, obs::Category::Execute);
        finalize(
            &mut self.sink,
            &mut self.answer_scratch,
            &mut self.resolve_memo,
            txn,
            &self.signature.columns,
            out,
        )
    }

    /// The rule loop (docs/architecture/40-execution.md § the rule loop):
    /// the sink resets ONCE, then every rule runs sequentially into it —
    /// its seen-set spanning rules is the entire implementation of set
    /// union; no merge node or concat-then-dedup pass exists. Params were
    /// bound once and reach every rule through the shared resolved slots.
    /// `Ok(false)` = every rule short-circuited on an `Eq`-anchored
    /// dictionary miss (nothing ran, the sink stays reset, and the caller
    /// skips finalize).
    pub(super) fn run_rules<C: Counters>(
        &mut self,
        txn: &ReadTxn<'_>,
        cache: &ImageCache,
        counters: &mut C,
    ) -> Result<bool> {
        // Interiors then rec (if any) then main. Interiors-only never
        // enters the reach driver (`run_derived` is the Cq derived phase
        // and the Reach interiors+rec phase).
        let has_derived = match &self.pipeline {
            PreparedPipeline::Cq { interiors, .. } => !interiors.is_empty(),
            PreparedPipeline::Reach { .. } => true,
        };
        if has_derived {
            let derived_ran = self.run_derived(txn, cache, counters)?;
            if self.pipeline.main_rules().is_empty() {
                return Ok(derived_ran);
            }
        }
        if self.pipeline.main_rules().is_empty() {
            return Ok(false);
        }
        self.sink.reset();
        let mut ran = false;
        let rule_count = self.pipeline.main_rules().len();
        for rule_idx in 0..rule_count {
            ran |= self.run_rule(rule_idx, txn, cache, counters)?;
        }
        if ran {
            self.run_ray_probes(txn, cache, counters)?;
        }
        Ok(ran)
    }

    /// CQ profile's counted rule loop: the same derived → main → ray
    /// protocol as [`Self::run_rules`], with per-main-rule
    /// [`crate::exec::introspection::CountingCounters`] so node stats
    /// land on the counted surface.
    pub(super) fn run_rules_cq_profile(
        &mut self,
        txn: &ReadTxn<'_>,
        cache: &ImageCache,
        rule_stats: &mut Vec<crate::api::stats::RuleStats>,
    ) -> Result<bool> {
        use crate::exec::introspection::CountingCounters;
        let has_derived = match &self.pipeline {
            PreparedPipeline::Cq { interiors, .. } => !interiors.is_empty(),
            PreparedPipeline::Reach { .. } => true,
        };
        if has_derived {
            let derived_ran = self.run_derived(txn, cache, &mut NoopCounters)?;
            if self.pipeline.main_rules().is_empty() {
                return Ok(derived_ran);
            }
        }
        if self.pipeline.main_rules().is_empty() {
            return Ok(false);
        }
        self.sink.reset();
        let mut ran = false;
        let rule_count = self.pipeline.main_rules().len();
        rule_stats.reserve(rule_count);
        for rule_idx in 0..rule_count {
            let seen_before = self.sink.distinct_seen().unwrap_or(0);
            let mut counters = match &self.pipeline.main_rules()[rule_idx] {
                PreparedRule::FreeJoin(rule) => CountingCounters::new(&rule.plan),
                PreparedRule::KeyProbe(_) => CountingCounters::for_key_probe(),
            };
            ran |= self.run_rule(rule_idx, txn, cache, &mut counters)?;
            let emitted = Counters::emits(&counters);
            let newly_seen = self
                .sink
                .distinct_seen()
                .map_or(emitted, |seen| (seen - seen_before) as u64);
            let absorbed = emitted - newly_seen;
            rule_stats.push(match &self.pipeline.main_rules()[rule_idx] {
                PreparedRule::FreeJoin(rule) => counters.into_rule_stats(
                    &rule.plan,
                    self.schema,
                    self.rule_pinned_rows(rule_idx),
                    absorbed,
                ),
                PreparedRule::KeyProbe(rule) => crate::api::stats::RuleStats {
                    distinct_bindings: rule.distinct_witness.is_some(),
                    nodes: Vec::new(),
                    eliminated: Vec::new(),
                    folded: Vec::new(),
                    pinned: Vec::new(),
                    emitted,
                    absorbed,
                    key_probe: Some(crate::api::stats::KeyProbeStats { hit: emitted > 0 }),
                },
            });
        }
        if ran {
            self.run_ray_probes(txn, cache, &mut NoopCounters)?;
        }
        Ok(ran)
    }

    /// One written rule's probes: resolve (the rule loop's latch
    /// discipline verbatim), run the probe's Free Join into the
    /// [`crate::exec::verdict::RayArbiter`], and raise on the first
    /// Ray verdict.
    pub(super) fn run_ray_probes<C: Counters>(
        &mut self,
        txn: &ReadTxn<'_>,
        cache: &ImageCache,
        counters: &mut C,
    ) -> Result<()> {
        let latched = super::reach::run_ray_probe_sets(
            &mut self.ray_probes,
            None,
            self.schema,
            txn,
            cache,
            &self.resolved_params,
            &self.missed_params,
            self.unresolved_literals == 0 && self.params.is_empty(),
            &mut self.bindings,
            counters,
        )?;
        self.unresolved_literals = self.unresolved_literals.saturating_sub(latched);
        Ok(())
    }

    /// One rule of the loop: re-aim the sink's slot tables at the rule's
    /// binding layout, resolve this execution's filter constants, and
    /// run the rule's plan — key probe or Free Join — into the shared
    /// sink. `Ok(false)` = the positive-occurrence `Eq` short-circuit (a
    /// dictionary miss or empty set emptied this conjunctive rule; the
    /// other rules still run — a rule is one disjunct).
    #[expect(
        clippy::too_many_lines,
        reason = "the rule loop's one step reads as one protocol: aim, resolve, run, account"
    )]
    pub(super) fn run_rule<C: Counters>(
        &mut self,
        rule_idx: usize,
        txn: &ReadTxn<'_>,
        cache: &ImageCache,
        counters: &mut C,
    ) -> Result<bool> {
        let mut rule_span = obs::span(obs::names::RULE[rule_idx], obs::Category::Execute);
        let emits_before = counters.emits();
        let seen_before = self.sink.distinct_seen().unwrap_or(0);
        // Re-aim per rule only where a switch exists: a single-rule sink
        // is built aimed, and the hot single-rule path stays untouched.
        let rule_count = self.pipeline.main_rules().len();
        if rule_count > 1 {
            let rule = &self.pipeline.main_rules()[rule_idx];
            self.sink
                .aim(rule.finds(), rule.slot_count(), rule.dedup_spans());
        }
        let slot_count = self.pipeline.main_rules()[rule_idx].slot_count();
        self.bindings.resize(slot_count);
        // The fully-latched fast path: zero pending literals and zero
        // params of any shape means the resolved tables were written
        // once and are final — `resolve_filters` is skipped entirely
        // (one cold branch; the latch only removes work).
        self.fill_main_images(rule_idx);
        let occ_images = std::mem::take(&mut self.derived.occ_images);
        let mut retired = std::mem::take(&mut self.derived.retired);
        let fast_eligible = self.unresolved_literals == 0 && self.params.is_empty();
        let mut latched = 0u32;
        let rules = self.pipeline.main_rules_mut();
        let ran = match &mut rules[rule_idx] {
            PreparedRule::KeyProbe(rule) => {
                execute_key_probe(
                    &rule.plan,
                    txn,
                    self.schema,
                    &self.resolved_params,
                    &mut self.determinant_key,
                    &mut self.bindings,
                    &mut self.sink,
                    counters,
                )?;
                true
            }
            PreparedRule::FreeJoin(rule) => {
                let plan = &mut rule.plan;
                let resolved =
                    if fast_eligible && rule.resolution == super::ResolutionState::Complete {
                        true
                    } else {
                        let _s = obs::span(obs::names::RESOLVE_FILTERS, obs::Category::Execute);
                        let complete = resolve_filters(
                            txn,
                            plan,
                            &self.resolved_params,
                            &self.missed_params,
                            &mut rule.resolved_filters,
                            &mut rule.resolved_selections,
                            &mut latched,
                        )?;
                        // A short-circuited pass leaves later slots
                        // unwritten; only a completed one arms the skip.
                        rule.resolution = if complete {
                            super::ResolutionState::Complete
                        } else {
                            super::ResolutionState::Pending
                        };
                        complete
                    };
                if resolved {
                    // This execution's Allen-residual masks (literal or
                    // bound param) resolve into the executor before the
                    // join runs — the hot path never touches the param
                    // slice.
                    rule.executor.bind_allen_masks(&self.resolved_params);
                    // One match per execution: the executor monomorphizes
                    // per concrete sink type — no per-emit enum branch on
                    // the hot path. No Interior image slice (and so no
                    // retired-buffer pool): a query-path plan carries
                    // only stored occurrences.
                    match &mut self.sink {
                        super::EitherSink::Projection(s) => run_join(
                            plan,
                            self.schema,
                            txn,
                            cache,
                            &mut rule.executor,
                            &mut self.bindings,
                            &rule.resolved_filters,
                            &rule.resolved_selections,
                            &mut rule.memo,
                            &occ_images,
                            &mut retired,
                            s,
                            counters,
                        )?,
                        super::EitherSink::Aggregate(s) => run_join(
                            plan,
                            self.schema,
                            txn,
                            cache,
                            &mut rule.executor,
                            &mut self.bindings,
                            &rule.resolved_filters,
                            &rule.resolved_selections,
                            &mut rule.memo,
                            &occ_images,
                            &mut retired,
                            s.as_mut(),
                            counters,
                        )?,
                    }
                }
                resolved
            }
        };
        // The union accounting (docs/architecture/40-execution.md
        // § observability): emitted = bindings this rule handed the
        // sink; absorbed = the spanning seen-set's duplicates among
        // them — an elided seen-set absorbs nothing by proof. Deltas of
        // O(1) reads; the executor itself counts nothing extra.
        let emitted = counters.emits() - emits_before;
        let newly_seen = self
            .sink
            .distinct_seen()
            .map_or(emitted, |seen| (seen - seen_before) as u64);
        // Saturating: an uncounted path reports emitted = 0 against a
        // real seen-set delta — the honest args there are (0, 0).
        rule_span.set_args(emitted, emitted.saturating_sub(newly_seen));
        self.unresolved_literals = self.unresolved_literals.saturating_sub(latched);
        self.derived.occ_images = occ_images;
        self.derived.retired = retired;
        Ok(ran)
    }

    /// The point fast lane's body: probe + fetch +
    /// direct cell decode, no sink machinery. The lane is the
    /// no-interior single key-probe Cq already on `self.pipeline`.
    pub(super) fn execute_key_probe_direct(
        &mut self,
        txn: &ReadTxn<'_>,
        out: &mut Answers,
    ) -> Result<()> {
        debug_assert!(self.pipeline.is_key_probe_direct());
        let PreparedRule::KeyProbe(KeyProbeRule {
            plan: key_probe,
            key_probe_finds: Some(key_probe_finds),
            ..
        }) = &self.pipeline.main_rules()[0]
        else {
            unreachable!("key_probe_direct parsed at build");
        };
        self.resolve_memo.clear();
        let Some(fact) = crate::exec::dispatch::key_probe_fact(
            key_probe,
            txn,
            self.schema,
            &self.resolved_params,
            &mut self.determinant_key,
        )?
        else {
            return Ok(());
        };
        out.cells.reserve(key_probe_finds.len());
        for (field, ty) in key_probe_finds {
            if let Some(element) = ty.interval_element() {
                let crate::exec::dispatch::FactOperand::Pair(start, end) =
                    crate::exec::dispatch::fact_operand(
                        self.schema,
                        key_probe.relation,
                        fact,
                        *field,
                    )?
                else {
                    unreachable!("validated: interval finds read interval fields")
                };
                out.cells.push(Answers::interval_cell(element, start, end));
                continue;
            }
            if let ValueType::FixedBytes { len } = ty {
                // Inline value: the padded words come straight off the
                // fact — no dictionary, and no temporary heap (the
                // operand's fixed block slices straight into the
                // caller's buffer; this is the point fast lane).
                match crate::exec::dispatch::fact_operand(
                    self.schema,
                    key_probe.relation,
                    fact,
                    *field,
                )? {
                    crate::exec::dispatch::FactOperand::Word(word) => {
                        out.push_fixed_bytes(*len, &[word]);
                    }
                    crate::exec::dispatch::FactOperand::Block { words, count } => {
                        out.push_fixed_bytes(*len, &words[..usize::from(count)]);
                    }
                    crate::exec::dispatch::FactOperand::Pair(..) => {
                        unreachable!("validated: bytes<N> finds read bytes<N> fields")
                    }
                }
                continue;
            }
            let word =
                crate::exec::dispatch::fact_word(self.schema, key_probe.relation, fact, *field)?;
            match ty {
                ValueType::String => {
                    out.push_word(txn, ty, word, &mut self.resolve_memo)?;
                }
                _ => out.cells.push(Answers::word_cell(ty, word)),
            }
        }
        Ok(())
    }

    /// Convenience path: a fresh buffer per call.
    ///
    /// # Errors
    ///
    /// As [`Self::execute`].
    pub(crate) fn execute_collect(
        &mut self,
        txn: &ReadTxn<'_>,
        cache: &ImageCache,
        params: &[BindValue<'_>],
    ) -> Result<Answers> {
        let mut out = Answers::new();
        self.execute(txn, cache, params, &mut out)?;
        Ok(out)
    }

    /// [`Self::execute_args`]'s fresh-buffer convenience.
    ///
    /// # Errors
    ///
    /// As [`Self::execute_args`].
    pub(crate) fn execute_collect_args(
        &mut self,
        txn: &ReadTxn<'_>,
        cache: &ImageCache,
        args: &[super::ParamArg<'_>],
    ) -> Result<Answers> {
        let mut out = Answers::new();
        self.execute_args(txn, cache, args, &mut out)?;
        Ok(out)
    }
}
