use std::sync::Arc;

use super::finalize::finalize;
use super::run_join::{RuleCtx, RuleScratch, SinkUse, run_rule};
use super::source::QuerySource;
use super::{Answers, PreparedPipeline, PreparedQuery, ValueType};

use crate::api::db::{OwnedInstance, ReadInstance};
use crate::error::Result;
use crate::exec::run::{Counters, NoopCounters};
use crate::image::SourceImages;

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
        self.runtime.heap_tick += 1;
        let source = QuerySource::heap(
            instance,
            self.runtime.heap_tick,
            super::source::heap_default_work(),
        );
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
        work.checkpoint()?;
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
        self.runtime.execution_texts.clear();
        let generation = if self.program.no_text_probe {
            debug_assert!(self.bound.text_generation.is_none());
            None
        } else {
            let generation = self.program.cache.acquire();
            self.bind_text_generation(&generation);
            Some(generation)
        };
        #[cfg(test)]
        {
            self.runtime.last_visits = 0;
        }
        out.begin(self.program.signature.columns.len());
        params.bind(self, source.work())?;
        let result = if matches!(self.pipeline, PreparedPipeline::PointProbe { .. }) {
            // Direct probes consume rows and this execution's resolver,
            // never images. Keep the same generation pinned across binding
            // and finalization without retaining an unused cache owner.
            self.execute_key_probe_direct(source, generation.as_ref(), out)
        } else {
            let cache = Arc::clone(&self.program.cache);
            let images = SourceImages::with_generation(
                source,
                &cache,
                generation.expect("non-probe pipelines always pin a generation"),
            );
            self.run_bound(&images, out)
        };
        #[cfg(test)]
        {
            self.runtime.last_visits = source.visit_count();
        }
        let result = result.and_then(|()| {
            source
                .work()
                .checkpoint()
                .map_err(crate::error::Error::from)
        });
        if result.is_err() {
            out.clear();
            self.runtime.resolve_memo.clear();
        }
        result
    }

    fn run_bound(&mut self, images: &SourceImages<'_>, out: &mut Answers) -> Result<()> {
        if self.pipeline.is_empty_cq() {
            return Ok(());
        }
        // Only pipelines that consume the main sink retain its cancellation context.
        // Point probes copy directly into Answers; empty CQs emit nothing.
        self.runtime
            .sink
            .begin_execution(Some(images.source().work().clone()));
        let ran = self.run_rules(images, &mut NoopCounters)?;
        self.finish_sink(images, ran, out)
    }

    /// Drain the sink into `out` after the shared rule loop.
    pub(super) fn finish_sink(
        &mut self,
        images: &SourceImages<'_>,
        ran: bool,
        out: &mut Answers,
    ) -> Result<()> {
        if !ran {
            return Ok(());
        }
        let interner = images.interner();
        finalize(
            &mut self.runtime.sink,
            &mut self.runtime.answer_scratch,
            &mut self.runtime.resolve_memo,
            &interner,
            &self.program.signature.columns,
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
        self.runtime.sink.reset();
        let mut ran = false;
        let rule_count = self.pipeline.main_rules().len();
        for rule_idx in 0..rule_count {
            ran |= self.run_rule(rule_idx, images, counters)?;
        }
        Ok(ran)
    }

    fn run_rule<Cnt: Counters>(
        &mut self,
        rule_idx: usize,
        images: &SourceImages<'_>,
        counters: &mut Cnt,
    ) -> Result<bool> {
        self.fill_main_images(rule_idx);
        let occ_images = std::mem::take(&mut self.runtime.derived.occ_images);
        let mut retired = std::mem::take(&mut self.runtime.derived.retired);
        let interner = images.interner();
        let ctx = RuleCtx {
            schema: self.program.schema.as_ref(),
            images,
            interner: &interner,
            params: &self.bound.resolved_params,
            missed: &self.bound.missed_params,
        };
        let rules = self.pipeline.main_rules_mut();
        let sink_use = if rules.len() > 1 {
            SinkUse::Shared
        } else {
            SinkUse::SoleMain
        };
        let ran = run_rule(
            &ctx,
            &mut RuleScratch {
                bindings: &mut self.runtime.bindings,
                key_scratch: &mut self.runtime.key_scratch,
                occ_images: &occ_images,
                retired: &mut retired,
            },
            &mut rules[rule_idx],
            sink_use,
            &mut self.runtime.sink,
            counters,
        );
        self.runtime.derived.occ_images = occ_images;
        self.runtime.derived.retired = retired;
        ran
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
        self.runtime.resolve_memo.clear();
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
            self.program.schema.as_ref(),
            &interner,
            &self.bound.resolved_params,
            row,
            &mut self.runtime.key_scratch,
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
                    out.push_word(&interner, ty, words[0], &mut self.runtime.resolve_memo)?;
                }
                _ => out.cells.push(Answers::word_cell(ty, words[0])?),
            }
        }
        Ok(())
    }
}
