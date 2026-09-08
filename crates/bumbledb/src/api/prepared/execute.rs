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

use super::bind::LiteralResolution;

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
        self.execute_collect_with_work(instance, instance.work(), params)
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
        let source =
            QuerySource::heap(instance, self.heap_tick, super::source::heap_default_work());
        self.execute_source(&source, params, out)
    }

    /// As [`Self::execute_collect`], using this caller's cancellation context
    /// instead of the snapshot frame's context. Retaining a snapshot does not
    /// extend an earlier operation's cancellation lifetime.
    /// # Errors
    /// As [`Self::execute`].
    ///
    /// `doc(hidden)` bridge seam (P06/W3-SESSION); embedders use
    /// `execute`/`execute_collect`.
    #[doc(hidden)]
    pub fn execute_collect_with_work<'p, P: super::BindArgs<'p>>(
        &mut self,
        instance: &ReadInstance<'_, S>,
        work: &crate::work::WorkContext,
        params: P,
    ) -> Result<Answers> {
        let source = QuerySource::store(instance.snapshot(), work);
        let mut out = Answers::new();
        self.execute_source(&source, params, &mut out)?;
        work.checkpoint().map_err(super::source::work_error)?;
        Ok(out)
    }

    pub(crate) fn execute_source<'p, P: super::BindArgs<'p>>(
        &mut self,
        source: &QuerySource<'_>,
        params: P,
        out: &mut Answers,
    ) -> Result<()> {
        self.check_identity(source.pinned())?;
        // Previous raw sink contents are invalid for this new execution;
        // completed Answers own their payload independently.
        self.execution_texts.clear();
        let generation = if self.no_text_probe {
            debug_assert!(self.text_generation.is_none());
            None
        } else {
            let generation = self.cache.acquire();
            self.bind_text_generation(&generation);
            Some(generation)
        };
        #[cfg(test)]
        {
            self.last_visits = 0;
        }
        out.begin(self.signature.columns.len());
        params.bind(self, source.work())?;
        let result = if matches!(self.pipeline, PreparedPipeline::PointProbe { .. }) {
            // Direct probes consume rows and this execution's resolver,
            // never images. Keep the same generation pinned across binding
            // and finalization without retaining an unused cache owner.
            self.execute_key_probe_direct(source, generation.as_ref(), out)
        } else {
            let cache = Arc::clone(&self.cache);
            let images = SourceImages::with_generation(
                source,
                &cache,
                generation.expect("non-probe pipelines always pin a generation"),
            );
            self.run_bound(&images, out)
        };
        #[cfg(test)]
        {
            self.last_visits = source.visit_count();
        }
        let result = result.and_then(|()| {
            source
                .work()
                .checkpoint()
                .map_err(super::source::work_error)
        });
        if result.is_err() {
            out.clear();
            self.resolve_memo.clear();
        }
        result
    }

    fn run_bound(&mut self, images: &SourceImages<'_>, out: &mut Answers) -> Result<()> {
        if self.pipeline.is_empty_cq() {
            return Ok(());
        }
        // Only pipelines that consume the main sink retain its cancellation context.
        // Point probes copy directly into Answers; empty CQs emit nothing.
        self.sink
            .begin_execution(Some(images.source().work().clone()));
        // ONE numerical guard per whole engine operation:
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

        let ran = self.run_rules(images, &mut NoopCounters)?;
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
        let interner = images.interner();
        finalize(
            &mut self.sink,
            &mut self.answer_scratch,
            &mut self.resolve_memo,
            &interner,
            &self.signature.columns,
            out,
            images.source().work(),
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
        let fast_eligible = self.params.is_empty();
        let fallback = match &self.pipeline.main_rules()[rule_idx] {
            PreparedRule::FreeJoin(rule) => {
                self.forced_fallback
                    || super::reach::rule_uses_scratch_derived(&rule.plan, &self.derived.published)
                    || resident_positions_overflow(images.source(), &rule.plan)?
            }
            PreparedRule::KeyProbe(_) => false,
        };
        let interner = images.interner();
        let rules = self.pipeline.main_rules_mut();
        let ran = match &mut rules[rule_idx] {
            PreparedRule::KeyProbe(rule) => {
                execute_key_probe(
                    &rule.plan,
                    images.source(),
                    self.schema.as_ref(),
                    &interner,
                    &self.resolved_params,
                    &mut rule.row,
                    &mut self.key_scratch,
                    &mut self.bindings,
                    &mut self.sink,
                    counters,
                )?;
                true
            }
            PreparedRule::FreeJoin(rule) => {
                let mut ran_resident = false;
                if !fallback {
                    let plan = &rule.plan;
                    let resolved =
                        if fast_eligible && rule.resolution == super::ResolutionState::Complete {
                            true
                        } else {
                            let complete = LiteralResolution {
                                interner: &interner,
                                work: images.source().work(),
                                params: &self.resolved_params,
                                missed: &self.missed_params,
                            }
                            .filters(
                                plan,
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
                    ran_resident = resolved;
                    if resolved {
                        let work = images.source().work();
                        match &mut self.sink {
                            super::EitherSink::Computed(s) => run_join(
                                plan,
                                self.schema.as_ref(),
                                images,
                                work,
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
                                work,
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
                            super::EitherSink::Aggregate(s) => {
                                // This capability belongs to one resident
                                // invocation, not the prepared sink: unions
                                // and every fallback still require dedup.
                                let witness = (rule_count == 1)
                                    .then(|| plan.scalar_set_traversal())
                                    .flatten();
                                let witness = s.set_physical_distinct(witness);
                                rule.executor.set_physical_distinct(witness);
                                let joined = run_join(
                                    plan,
                                    self.schema.as_ref(),
                                    images,
                                    work,
                                    &mut rule.executor,
                                    &mut self.bindings,
                                    &rule.resolved_filters,
                                    &rule.resolved_selections,
                                    &mut rule.memo,
                                    &occ_images,
                                    &mut retired,
                                    s.as_mut(),
                                    counters,
                                );
                                // Restore the invocation-local proof before propagating errors.
                                rule.executor.set_physical_distinct(None);
                                let _ = s.set_physical_distinct(None);
                                joined?;
                            }
                        }
                    }
                }
                if fallback {
                    let mut ctx = super::fallback::FallbackCtx {
                        source: images.source(),
                        schema: self.schema.as_ref(),
                        interner: &interner,
                        params: &self.resolved_params,
                        missed: &self.missed_params,
                        retained_texts: &mut self.execution_texts,
                    };
                    match &mut self.sink {
                        super::EitherSink::Computed(s) => super::fallback::run_fallback(
                            &mut rule.fallback,
                            &mut ctx,
                            &mut self.derived.published,
                            &mut self.bindings,
                            s.as_mut(),
                        )?,
                        super::EitherSink::Projection(s) => super::fallback::run_fallback(
                            &mut rule.fallback,
                            &mut ctx,
                            &mut self.derived.published,
                            &mut self.bindings,
                            s,
                        )?,
                        super::EitherSink::Aggregate(s) => super::fallback::run_fallback(
                            &mut rule.fallback,
                            &mut ctx,
                            &mut self.derived.published,
                            &mut self.bindings,
                            s.as_mut(),
                        )?,
                    }
                    true
                } else {
                    ran_resident
                }
            }
        };

        self.derived.occ_images = occ_images;
        self.derived.retired = retired;
        Ok(ran)
    }

    pub(super) fn execute_key_probe_direct(
        &mut self,
        source: &QuerySource<'_>,
        generation: Option<&crate::work::GenerationHandle>,
        out: &mut Answers,
    ) -> Result<()> {
        let PreparedPipeline::PointProbe {
            finds: key_probe_finds,
            rule,
        } = &mut self.pipeline
        else {
            unreachable!("PointProbe arm sealed at build");
        };
        let key_probe = &rule.plan;
        self.resolve_memo.clear();
        let interner = match generation {
            Some(generation) => {
                crate::image::intern::InternerHandle::new(generation, source.work())
            }
            None => crate::image::intern::InternerHandle::without_text(source.work()),
        };
        let row = &mut rule.row;
        let hit = crate::exec::dispatch::key_probe_row(
            key_probe,
            source,
            self.schema.as_ref(),
            &interner,
            &self.resolved_params,
            row,
            &mut self.key_scratch,
        )?;
        if !hit {
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
            if matches!(ty, ValueType::Uuid) {
                out.cells.push(Answers::uuid_cell(words[0], words[1]));
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

/// Select fallback before a resident image/COLT would overflow the `u32`
/// position regime.
fn resident_positions_overflow(
    source: &QuerySource<'_>,
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
