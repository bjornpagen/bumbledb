use std::sync::Arc;

use super::finalize::finalize;
use super::run_join::run_join;
use super::source::QuerySource;
use super::{Answers, PreparedPipeline, PreparedQuery, PreparedRule, ValueType};

use crate::api::db::{OwnedInstance, ReadInstance};
use crate::error::Result;
use crate::exec::dispatch::execute_key_probe;
use crate::exec::run::{Counters, NoopCounters};
use crate::image::SourceImages;
use crate::image::canon::RowWords;
use crate::image::intern::InternerHandle;
use crate::obs;

use super::bind::resolve_filters;

impl<S> PreparedQuery<S> {
    /// Execute against one committed snapshot lease.
    /// # Errors
    /// Foreign source, bind refusal, storage/work failure, or a semantic
    /// execution error — `out` never holds a partial answer set on error.
    /// # Panics
    /// Only on programmer-invariant violations (plan/executor pairing).
    pub(crate) fn execute<'p, P: super::BindArgs<'p>>(
        &mut self,
        instance: &ReadInstance<'_, S>,
        params: P,
        out: &mut Answers,
    ) -> Result<()> {
        let source = QuerySource::store(instance.snapshot(), instance.work());
        self.execute_source(&source, params, out)
    }

    /// # Errors
    /// As [`Self::execute`].
    pub(crate) fn execute_collect<'p, P: super::BindArgs<'p>>(
        &mut self,
        instance: &ReadInstance<'_, S>,
        params: P,
    ) -> Result<Answers> {
        let mut out = Answers::new();
        self.execute(instance, params, &mut out)?;
        Ok(out)
    }

    /// Execute against one admitted heap instance. Heap instances carry no
    /// durable identity, so every execution rebuilds its images from the
    /// instance it was handed (a fresh `ViewEpoch::Heap` tick).
    /// # Errors
    /// As [`Self::execute`].
    pub(crate) fn execute_owned<'p, P: super::BindArgs<'p>>(
        &mut self,
        instance: &OwnedInstance<S>,
        params: P,
        out: &mut Answers,
    ) -> Result<()> {
        self.heap_tick += 1;
        let source = QuerySource::heap(instance, self.heap_tick)?;
        self.execute_source(&source, params, out)
    }

    pub(crate) fn execute_source<'p, P: super::BindArgs<'p>>(
        &mut self,
        source: &QuerySource<'_>,
        params: P,
        out: &mut Answers,
    ) -> Result<()> {
        self.check_identity(source.pinned())?;
        let mut execute_span = obs::span(obs::names::EXECUTE);
        out.begin(self.signature.columns.len());
        {
            let _s = obs::span(obs::names::BIND_PARAMS);
            params.bind(self, source.work())?;
        }
        // The main sink's distinct state is measured against this
        // execution's ledger and continues in the scratch map beyond its
        // RAM allowance (chapter 12 §4).
        self.sink
            .begin_execution(Some(crate::exec::sink::SinkBudget {
                work: source.work().clone(),
                ram_bytes: self.sink_ram,
            }));
        let cache = Arc::clone(&self.cache);
        let images = SourceImages::bind(source, &cache);
        let result = self.run_bound(&images, out);
        execute_span.set_count(out.len() as u64);
        result
    }

    /// The main sink's RAM allowance before its distinct state continues
    /// in temporary LMDB. A tuning default; zero forces the spill from the
    /// first row (the Q-FALLBACK/F-RESOURCE forcing affordance).
    #[doc(hidden)]
    pub fn set_sink_ram(&mut self, bytes: usize) {
        self.sink_ram = bytes;
    }

    fn run_bound(&mut self, images: &SourceImages<'_>, out: &mut Answers) -> Result<()> {
        if matches!(self.pipeline, PreparedPipeline::PointProbe { .. }) {
            return self.execute_key_probe_direct(images, out);
        }
        if self.pipeline.is_empty_cq() {
            return Ok(());
        }
        // ONE numerical guard per whole engine operation (chapter 11 §3):
        // queries with computed scalar outputs establish the canonical FPU
        // environment here, hold it across every rule/derived stage and
        // finalization, and restore the host state when the operation
        // ends — never per tuple, never per arithmetic node. No host
        // callback runs while the guard is live (the engine calls none).
        let _numeric_guard = match self.numeric_outputs {
            Some(find) => Some(
                crate::exec::kernel::numeric::NumericalGuard::enter().map_err(|_| {
                    crate::error::Error::Scalar {
                        find,
                        source: crate::ScalarError::UnsupportedPlatform,
                    }
                })?,
            ),
            None => None,
        };

        let attempt = if obs::capturing() {
            let mut timers = crate::exec::run::PhaseTimers::new();
            let ran = self.run_rules(images, &mut timers);
            timers.flush();
            ran
        } else {
            self.run_rules(images, &mut NoopCounters)
        };
        let ran = match attempt {
            Ok(ran) => ran,
            // One bounded restart on the SAME pinned snapshot (chapter 12
            // §6): a resident reservation refusal reroutes every Free Join
            // rule through the complete cursor fallback — never an endless
            // replan loop, and the discarded work is recorded.
            Err(error) if !self.forced_fallback && super::source::is_working_exhaustion(&error) => {
                obs::event(obs::names::FALLBACK_RESTART, obs::TraceArgs::Count(1));
                self.forced_fallback = true;
                let retried = if obs::capturing() {
                    let mut timers = crate::exec::run::PhaseTimers::new();
                    let ran = self.run_rules(images, &mut timers);
                    timers.flush();
                    ran
                } else {
                    self.run_rules(images, &mut NoopCounters)
                };
                self.forced_fallback = false;
                retried?
            }
            Err(error) => return Err(error),
        };
        self.finish_sink(images, ran, out)
    }

    /// Route every Free Join rule through the complete cursor fallback —
    /// the Q-FALLBACK forcing affordance. Answers and errors must agree
    /// with the resident path.
    #[doc(hidden)]
    pub fn force_cursor_fallback(&mut self, forced: bool) {
        self.forced_fallback = forced;
    }

    /// Drain the sink into `out` after the shared rule loop. Empty
    pub(super) fn finish_sink(
        &mut self,
        images: &SourceImages<'_>,
        ran: bool,
        out: &mut Answers,
    ) -> Result<()> {
        // raised before finalize — never a partial result. Executor-side

        if !ran {
            return Ok(());
        }
        let _s = obs::span(obs::names::FINALIZE);
        let interner = InternerHandle::new(images.cache().interner(), images.source().work());
        finalize(
            &mut self.sink,
            &mut self.answer_scratch,
            &mut self.resolve_memo,
            &interner,
            &self.signature.columns,
            out,
        )
    }

    pub(super) fn run_rules<Cnt: Counters>(
        &mut self,
        images: &SourceImages<'_>,
        counters: &mut Cnt,
    ) -> Result<bool> {
        if self.pipeline.has_derived() {
            let derived_ran = self.run_derived(images, counters)?;
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
            ran |= self.run_rule(rule_idx, images, counters)?;
        }
        Ok(ran)
    }

    #[expect(
        clippy::too_many_lines,
        reason = "one rule's regime dispatch reads as a single protocol"
    )]
    pub(super) fn run_rule<Cnt: Counters>(
        &mut self,
        rule_idx: usize,
        images: &SourceImages<'_>,
        counters: &mut Cnt,
    ) -> Result<bool> {
        let mut rule_span = obs::span(obs::names::RULE[rule_idx]);
        let emits_before = counters.emits();
        let seen_before = self.sink.distinct_seen().unwrap_or(0);

        let rule_count = self.pipeline.main_rules().len();
        if rule_count > 1 {
            let rule = &self.pipeline.main_rules()[rule_idx];
            self.sink
                .aim(rule.finds(), rule.slot_count(), rule.dedup_spans());
        }
        let slot_count = self.pipeline.main_rules()[rule_idx].slot_count();
        self.bindings.resize(slot_count);

        self.fill_main_images(rule_idx);
        let occ_images = std::mem::take(&mut self.derived.occ_images);
        let mut retired = std::mem::take(&mut self.derived.retired);
        let fast_eligible = self.latch.is_latched() && self.params.is_empty();
        let mut latched = 0u32;
        let interner = InternerHandle::new(self.cache.interner(), images.source().work());
        let rules = self.pipeline.main_rules_mut();
        let ran = match &mut rules[rule_idx] {
            PreparedRule::KeyProbe(rule) => {
                execute_key_probe(
                    &rule.plan,
                    images.source(),
                    self.schema.as_ref(),
                    &interner,
                    &self.resolved_params,
                    &mut self.key_scratch,
                    &mut self.bindings,
                    &mut self.sink,
                    counters,
                )?;
                true
            }
            PreparedRule::FreeJoin(rule) if self.forced_fallback => {
                let derived = super::reach::finished_resolver(&self.derived.published);
                let ctx = super::fallback::FallbackCtx {
                    source: images.source(),
                    schema: self.schema.as_ref(),
                    interner: &interner,
                    params: &self.resolved_params,
                    missed: &self.missed_params,
                    derived: &derived,
                };
                match &mut self.sink {
                    super::EitherSink::Computed(s) => super::fallback::run_fallback(
                        &mut rule.fallback,
                        &ctx,
                        &mut self.bindings,
                        s.as_mut(),
                        &mut latched,
                    )?,
                    super::EitherSink::Projection(s) => super::fallback::run_fallback(
                        &mut rule.fallback,
                        &ctx,
                        &mut self.bindings,
                        s,
                        &mut latched,
                    )?,
                    super::EitherSink::Aggregate(s) => super::fallback::run_fallback(
                        &mut rule.fallback,
                        &ctx,
                        &mut self.bindings,
                        s.as_mut(),
                        &mut latched,
                    )?,
                }
                true
            }
            PreparedRule::FreeJoin(rule) => {
                let plan = &mut rule.plan;
                let resolved =
                    if fast_eligible && rule.resolution == super::ResolutionState::Complete {
                        true
                    } else {
                        let _s = obs::span(obs::names::RESOLVE_FILTERS);
                        let complete = resolve_filters(
                            &interner,
                            plan,
                            &self.resolved_params,
                            &self.missed_params,
                            &mut rule.resolved_filters,
                            &mut rule.resolved_selections,
                            &mut latched,
                        )?;

                        rule.resolution = if complete {
                            super::ResolutionState::Complete
                        } else {
                            super::ResolutionState::Pending
                        };
                        complete
                    };
                if resolved {
                    // bound param) resolve into the executor before the

                    rule.executor.bind_allen_masks(&self.resolved_params);

                    match &mut self.sink {
                        super::EitherSink::Computed(s) => run_join(
                            plan,
                            self.schema.as_ref(),
                            images,
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
                        super::EitherSink::Projection(s) => run_join(
                            plan,
                            self.schema.as_ref(),
                            images,
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
                            self.schema.as_ref(),
                            images,
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

        let emitted = counters.emits() - emits_before;
        let newly_seen = self
            .sink
            .distinct_seen()
            .map_or(emitted, |seen| (seen - seen_before) as u64);

        rule_span.set_pair(emitted, emitted.saturating_sub(newly_seen));
        self.latch = self.latch.credit(latched);
        self.derived.occ_images = occ_images;
        self.derived.retired = retired;
        Ok(ran)
    }

    pub(super) fn execute_key_probe_direct(
        &mut self,
        images: &SourceImages<'_>,
        out: &mut Answers,
    ) -> Result<()> {
        let PreparedPipeline::PointProbe {
            finds: key_probe_finds,
            rule,
        } = &self.pipeline
        else {
            unreachable!("PointProbe arm sealed at build");
        };
        let key_probe = &rule.plan;
        self.resolve_memo.clear();
        let interner = InternerHandle::new(self.cache.interner(), images.source().work());
        let field_types: Vec<ValueType> = self
            .schema
            .relation(key_probe.relation)
            .fields()
            .iter()
            .map(|f| f.value_type)
            .collect();
        let mut row = RowWords::new(&field_types);
        if !crate::exec::dispatch::key_probe_row(
            key_probe,
            images.source(),
            self.schema.as_ref(),
            &interner,
            &self.resolved_params,
            &mut row,
            &mut self.key_scratch,
        )? {
            return Ok(());
        }
        out.cells.reserve(key_probe_finds.len());
        for (field, ty) in key_probe_finds {
            let words = row.span_words(*field);
            if let Some(element) = ty.interval_element() {
                out.cells
                    .push(Answers::interval_cell(element, words[0], words[1]));
                continue;
            }
            if matches!(ty, ValueType::Id128) {
                out.cells.push(Answers::id128_cell(words[0], words[1]));
                continue;
            }
            if let ValueType::FixedBytes { len } = ty {
                out.push_fixed_bytes(*len, words);
                continue;
            }
            match ty {
                ValueType::String => {
                    out.push_word(&interner, ty, words[0], &mut self.resolve_memo)?;
                }
                _ => out.cells.push(Answers::word_cell(ty, words[0])?),
            }
        }
        Ok(())
    }
}
