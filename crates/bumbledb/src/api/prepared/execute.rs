use super::{Answers, ParamArg, PreparedPipeline, PreparedQuery, PreparedRule, ValueType};

use crate::error::Result;
use crate::exec::dispatch::execute_key_probe;
use crate::exec::run::{Counters, NoopCounters};
use crate::image::ImageBind;
use crate::image::LmdbSource;
use crate::image::cache::ImageCache;
use crate::obs;
use crate::storage::catalog::CatalogRead;
use crate::storage::env::{CatalogIdentity, ReadTxn};

use super::bind::resolve_filters;
use super::finalize::finalize;
use super::run_join::run_join;

impl<S> PreparedQuery<S> {
    /// # Errors
    /// # Panics
    /// Only on programmer-invariant violations (plan/executor pairing,
    pub(crate) fn execute<'p, P: super::BindArgs<'p>>(
        &mut self,
        txn: &ReadTxn<'_>,
        cache: &ImageCache,
        params: P,
        out: &mut Answers,
    ) -> Result<()> {
        self.check_identity(txn.identity())?;
        let mut execute_span = obs::span(obs::names::EXECUTE);
        out.begin(self.signature.columns.len());
        {
            let _s = obs::span(obs::names::BIND_PARAMS);
            params.bind(self, txn)?;
        }
        let catalog = txn.catalog();
        let images = LmdbSource::bind(txn, cache);
        let result = self.run_bound(&catalog, &images, out);
        execute_span.set_count(out.len() as u64);
        result
    }

    /// checked before bind.
    pub(crate) fn execute_on<C: CatalogRead, I: ImageBind>(
        &mut self,
        identity: &CatalogIdentity,
        catalog: &C,
        images: &I,
        params: &[ParamArg<'_>],
        out: &mut Answers,
    ) -> Result<()> {
        self.check_identity(identity)?;
        let mut execute_span = obs::span(obs::names::EXECUTE);
        out.begin(self.signature.columns.len());
        {
            let _s = obs::span(obs::names::BIND_PARAMS);
            self.bind_param_args(catalog, params)?;
        }
        let result = self.run_bound(catalog, images, out);
        execute_span.set_count(out.len() as u64);
        result
    }

    fn run_bound<C: CatalogRead, I: ImageBind>(
        &mut self,
        catalog: &C,
        images: &I,
        out: &mut Answers,
    ) -> Result<()> {
        if matches!(self.pipeline, PreparedPipeline::PointProbe { .. }) {
            return self.execute_key_probe_direct(catalog, out);
        }
        if self.pipeline.is_empty_cq() {
            return Ok(());
        }

        let ran = if obs::capturing() {
            let mut timers = crate::exec::run::PhaseTimers::new();
            let ran = self.run_rules(catalog, images, &mut timers)?;
            timers.flush();
            ran
        } else {
            self.run_rules(catalog, images, &mut NoopCounters)?
        };
        self.finish_sink(catalog, ran, out)
    }

    /// Drain the sink into `out` after the shared rule loop. Empty
    pub(super) fn finish_sink<C: CatalogRead>(
        &mut self,
        catalog: &C,
        ran: bool,
        out: &mut Answers,
    ) -> Result<()> {
        // raised before finalize — never a partial result. Executor-side

        if !ran {
            return Ok(());
        }
        let _s = obs::span(obs::names::FINALIZE);
        finalize(
            &mut self.sink,
            &mut self.answer_scratch,
            &mut self.resolve_memo,
            catalog,
            &self.signature.columns,
            out,
        )
    }

    pub(super) fn run_rules<Cnt: Counters, C: CatalogRead, I: ImageBind>(
        &mut self,
        catalog: &C,
        images: &I,
        counters: &mut Cnt,
    ) -> Result<bool> {
        if self.pipeline.has_derived() {
            let derived_ran = self.run_derived(catalog, images, counters)?;
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
            ran |= self.run_rule(rule_idx, catalog, images, counters)?;
        }
        Ok(ran)
    }

    pub(super) fn run_rule<Cnt: Counters, C: CatalogRead, I: ImageBind>(
        &mut self,
        rule_idx: usize,
        catalog: &C,
        images: &I,
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
        let rules = self.pipeline.main_rules_mut();
        let ran = match &mut rules[rule_idx] {
            PreparedRule::KeyProbe(rule) => {
                execute_key_probe(
                    &rule.plan,
                    catalog,
                    self.schema.as_ref(),
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
                        let _s = obs::span(obs::names::RESOLVE_FILTERS);
                        let complete = resolve_filters(
                            catalog,
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

    pub(super) fn execute_key_probe_direct<C: CatalogRead>(
        &mut self,
        catalog: &C,
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
        let Some(stored) = crate::exec::dispatch::key_probe_fact(
            key_probe,
            catalog,
            self.schema.as_ref(),
            &self.resolved_params,
            &mut self.determinant_key,
        )?
        else {
            return Ok(());
        };
        let fact = self
            .schema
            .relation(key_probe.relation)
            .layout()
            .encoded(stored.as_ref());
        out.cells.reserve(key_probe_finds.len());
        for (field, ty) in key_probe_finds {
            if let Some(element) = ty.interval_element() {
                let crate::exec::dispatch::FactOperand::Pair(start, end) =
                    crate::exec::dispatch::fact_operand(fact, *field)?
                else {
                    unreachable!("validated: interval finds read interval fields")
                };
                out.cells.push(Answers::interval_cell(element, start, end));
                continue;
            }
            if let ValueType::FixedBytes { len } = ty {
                match crate::exec::dispatch::fact_operand(fact, *field)? {
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
            let word = crate::exec::dispatch::fact_word(fact, *field)?;
            match ty {
                ValueType::String => {
                    out.push_word(catalog, ty, word, &mut self.resolve_memo)?;
                }
                _ => out.cells.push(Answers::word_cell(ty, word)),
            }
        }
        Ok(())
    }

    /// # Errors
    pub(crate) fn execute_collect<'p, P: super::BindArgs<'p>>(
        &mut self,
        txn: &ReadTxn<'_>,
        cache: &ImageCache,
        params: P,
    ) -> Result<Answers> {
        let mut out = Answers::new();
        self.execute(txn, cache, params, &mut out)?;
        Ok(out)
    }
}
