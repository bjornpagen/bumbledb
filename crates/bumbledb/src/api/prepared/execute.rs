use super::{ExecPlan, PreparedQuery, ResultBuffer, ValueType};

use crate::error::Result;
use crate::exec::dispatch::execute_guard;
use crate::exec::run::NoopCounters;
use crate::image::cache::ImageCache;
use crate::ir::Value;
use crate::obs;
use crate::storage::env::ReadTxn;

use super::bind::resolve_predicates;
use super::finalize::finalize;
use super::run_join::run_join;

impl PreparedQuery<'_> {
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
        params: &[Value],
        out: &mut ResultBuffer,
    ) -> Result<()> {
        self.check_snapshot(txn)?;
        let mut execute_span = obs::span(obs::names::EXECUTE, obs::Category::Execute);
        out.clear();
        out.arity = self.finds.len();
        {
            let _s = obs::span(obs::names::BIND_PARAMS, obs::Category::Execute);
            self.bind_params(txn, params)?;
        }
        match &self.plan {
            ExecPlan::GuardProbe(guard) => {
                // The point fast lane (docs/perf/ PRD 11): one probe, one
                // fetch, cells decoded straight into the buffer — no
                // sink, no bindings, no finalize pass. Aggregate-find
                // guards (rare) keep the sink path below.
                if self.guard_finds.is_some() {
                    let result = self.execute_guard_direct(txn, out);
                    execute_span.set_args(out.len() as u64, 0);
                    return result;
                }
                self.sink.reset();
                execute_guard(
                    guard,
                    txn,
                    self.schema,
                    &self.resolved_params,
                    &mut self.guard_key,
                    &mut self.bindings,
                    &mut self.sink,
                )?;
            }
            ExecPlan::FreeJoin(plan) => {
                self.sink.reset();
                let resolved = {
                    let _s = obs::span(obs::names::RESOLVE_FILTERS, obs::Category::Execute);
                    resolve_predicates(
                        txn,
                        plan,
                        &self.resolved_params,
                        &self.missed_params,
                        &mut self.resolved_filters,
                        &mut self.resolved_selections,
                    )?
                };
                if !resolved {
                    return Ok(()); // Eq-anchored dictionary miss: empty result
                }
                // Phase attribution engages only under an active obs
                // capture (docs/architecture/50-validation.md): timing
                // runs — even obs builds — monomorphize NoopCounters and
                // pay nothing.
                macro_rules! run_join_with {
                    ($counters:expr) => {
                        run_join(
                            plan,
                            self.schema,
                            txn,
                            cache,
                            self.executor
                                .as_mut()
                                .expect("free join plans carry executor scratch"),
                            &mut self.bindings,
                            &self.resolved_filters,
                            &self.resolved_selections,
                            &mut self.memo,
                            &mut self.sink,
                            $counters,
                        )
                    };
                }
                #[cfg(feature = "trace")]
                if obs::capturing() {
                    let mut timers = crate::exec::run::PhaseTimers::new();
                    run_join_with!(&mut timers)?;
                    timers.flush();
                } else {
                    run_join_with!(&mut NoopCounters)?;
                }
                #[cfg(not(feature = "trace"))]
                run_join_with!(&mut NoopCounters)?;
            }
        }
        let result = {
            let _s = obs::span(obs::names::FINALIZE, obs::Category::Execute);
            finalize(
                &self.sink,
                &mut self.row_scratch,
                &mut self.resolve_memo,
                txn,
                &self.finds,
                self.all_words,
                out,
            )
        };
        execute_span.set_args(out.len() as u64, 0);
        result
    }

    /// The point fast lane's body (docs/perf/ PRD 11): probe + fetch +
    /// direct cell decode, no sink machinery.
    fn execute_guard_direct(&mut self, txn: &ReadTxn<'_>, out: &mut ResultBuffer) -> Result<()> {
        let ExecPlan::GuardProbe(guard) = &self.plan else {
            unreachable!("guard_finds implies a guard plan")
        };
        let guard_finds = self.guard_finds.as_ref().expect("checked by the caller");
        self.resolve_memo.clear();
        let Some(fact) = crate::exec::dispatch::guard_probe_fact(
            guard,
            txn,
            self.schema,
            &self.resolved_params,
            &mut self.guard_key,
        )?
        else {
            return Ok(());
        };
        out.cells.reserve(guard_finds.len());
        for (field, ty) in guard_finds {
            let word = crate::exec::dispatch::fact_word(self.schema, guard, fact, *field);
            match ty {
                ValueType::String | ValueType::Bytes => {
                    out.push_word(txn, ty, word, &mut self.resolve_memo)?;
                }
                _ => out.cells.push(ResultBuffer::word_cell(ty, word)),
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
        params: &[Value],
    ) -> Result<ResultBuffer> {
        let mut out = ResultBuffer::new();
        self.execute(txn, cache, params, &mut out)?;
        Ok(out)
    }
}
