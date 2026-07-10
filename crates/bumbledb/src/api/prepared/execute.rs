use super::{BindValue, ExecPlan, PreparedQuery, ResultBuffer, ValueType};

use crate::error::{Error, Result};
use crate::exec::dispatch::execute_guard;
use crate::exec::run::NoopCounters;
use crate::image::cache::ImageCache;
use crate::obs;
use crate::storage::env::ReadTxn;

use super::bind::resolve_predicates;
use super::finalize::finalize;
use super::run_join::run_join;

impl<S> PreparedQuery<'_, S> {
    /// Executes with the given parameters into the caller's buffer.
    ///
    /// # Errors
    ///
    /// `ParamCountMismatch`/`ParamTypeMismatch` at bind time;
    /// `MultiRuleExecution` for a 2+-rule program (the union loop is PRD
    /// ALG-07's); `Overflow` from aggregate finalization;
    /// `Lmdb`/`Corruption` from storage.
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
        out: &mut ResultBuffer,
    ) -> Result<()> {
        self.check_snapshot(txn)?;
        self.gate_single_rule()?;
        let mut execute_span = obs::span(obs::names::EXECUTE, obs::Category::Execute);
        out.clear();
        out.arity = self.column_types.len();
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
        out: &mut ResultBuffer,
    ) -> Result<()> {
        self.check_snapshot(txn)?;
        self.gate_single_rule()?;
        let mut execute_span = obs::span(obs::names::EXECUTE, obs::Category::Execute);
        out.clear();
        out.arity = self.column_types.len();
        {
            let _s = obs::span(obs::names::BIND_PARAMS, obs::Category::Execute);
            self.bind_param_args(txn, args)?;
        }
        let result = self.run_bound(txn, cache, out);
        execute_span.set_args(out.len() as u64, 0);
        result
    }

    /// The single-rule execution gate: every rule's plan is prepared, but
    /// the union-driving executor loop (one head, one sink) is PRD
    /// ALG-07's deliverable — a 2+-rule program is a typed refusal at
    /// every execution entry, never a wrong answer.
    pub(super) fn gate_single_rule(&self) -> Result<()> {
        match self.rules.len() {
            1 => Ok(()),
            rules => Err(Error::MultiRuleExecution { rules }),
        }
    }

    /// The post-bind execution body shared by every bind shape. Runs the
    /// gated single rule (`self.rules[0]`).
    fn run_bound(
        &mut self,
        txn: &ReadTxn<'_>,
        cache: &ImageCache,
        out: &mut ResultBuffer,
    ) -> Result<()> {
        let rule = &mut self.rules[0];
        match &rule.plan {
            ExecPlan::GuardProbe(guard) => {
                // The point fast lane: one probe, one
                // fetch, cells decoded straight into the buffer — no
                // sink, no bindings, no finalize pass. Aggregate-find
                // guards (rare) keep the sink path below.
                if rule.guard_finds.is_some() {
                    return self.execute_guard_direct(txn, out);
                }
                self.sink.reset();
                execute_guard(
                    guard,
                    txn,
                    self.schema,
                    &self.resolved_params,
                    &mut self.guard_key,
                    &mut rule.bindings,
                    &mut self.sink,
                )?;
            }
            ExecPlan::FreeJoin(_) => {
                // Phase attribution engages only under an active obs
                // capture (docs/architecture/60-validation.md): timing
                // runs — even obs builds — monomorphize NoopCounters and
                // pay nothing.
                #[cfg(feature = "trace")]
                let ran = if obs::capturing() {
                    let mut timers = crate::exec::run::PhaseTimers::new();
                    let ran = self.run_free_join(txn, cache, &mut timers)?;
                    timers.flush();
                    ran
                } else {
                    self.run_free_join(txn, cache, &mut NoopCounters)?
                };
                #[cfg(not(feature = "trace"))]
                let ran = self.run_free_join(txn, cache, &mut NoopCounters)?;
                if !ran {
                    return Ok(()); // Eq-anchored dictionary miss: empty result
                }
            }
        }
        let _s = obs::span(obs::names::FINALIZE, obs::Category::Execute);
        finalize(
            &self.sink,
            &mut self.row_scratch,
            &mut self.resolve_memo,
            txn,
            &self.column_types,
            self.all_words,
            out,
        )
    }

    /// The Free Join body shared by every entry (`execute`,
    /// `execute_args`, `profile`), over the gated single rule: resets the
    /// sink, resolves this execution's predicate constants, and runs the
    /// join. `Ok(false)` = the positive-occurrence `Eq` short-circuit (a
    /// dictionary miss or empty set emptied the whole conjunctive rule —
    /// nothing ran, the sink stays reset, and the caller skips finalize).
    pub(super) fn run_free_join<C: crate::exec::run::Counters>(
        &mut self,
        txn: &ReadTxn<'_>,
        cache: &ImageCache,
        counters: &mut C,
    ) -> Result<bool> {
        let rule = &mut self.rules[0];
        let ExecPlan::FreeJoin(plan) = &rule.plan else {
            unreachable!("free join entries dispatch on the plan")
        };
        self.sink.reset();
        let resolved = {
            let _s = obs::span(obs::names::RESOLVE_FILTERS, obs::Category::Execute);
            resolve_predicates(
                txn,
                plan,
                &self.resolved_params,
                &self.missed_params,
                &mut rule.resolved_filters,
                &mut rule.resolved_selections,
            )?
        };
        if !resolved {
            return Ok(false);
        }
        // This execution's Allen-residual masks (literal or bound param)
        // resolve into the executor before the join runs — the hot path
        // never touches the param slice.
        rule.executor
            .as_mut()
            .expect("free join plans carry executor scratch")
            .bind_allen_masks(&self.resolved_params);
        run_join(
            plan,
            self.schema,
            txn,
            cache,
            rule.executor
                .as_mut()
                .expect("free join plans carry executor scratch"),
            &mut rule.bindings,
            &rule.resolved_filters,
            &rule.resolved_selections,
            &mut rule.memo,
            &mut self.sink,
            counters,
        )?;
        Ok(true)
    }

    /// The point fast lane's body: probe + fetch +
    /// direct cell decode, no sink machinery.
    fn execute_guard_direct(&mut self, txn: &ReadTxn<'_>, out: &mut ResultBuffer) -> Result<()> {
        let rule = &self.rules[0];
        let ExecPlan::GuardProbe(guard) = &rule.plan else {
            unreachable!("guard_finds implies a guard plan")
        };
        let guard_finds = rule.guard_finds.as_ref().expect("checked by the caller");
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
            if let ValueType::Interval { element } = ty {
                let crate::exec::dispatch::FactOperand::Pair(start, end) =
                    crate::exec::dispatch::fact_operand(self.schema, guard.relation, fact, *field)
                else {
                    unreachable!("validated: interval finds read interval fields")
                };
                out.cells
                    .push(ResultBuffer::interval_cell(*element, start, end));
                continue;
            }
            let word = crate::exec::dispatch::fact_word(self.schema, guard.relation, fact, *field);
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
        params: &[BindValue<'_>],
    ) -> Result<ResultBuffer> {
        let mut out = ResultBuffer::new();
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
    ) -> Result<ResultBuffer> {
        let mut out = ResultBuffer::new();
        self.execute_args(txn, cache, args, &mut out)?;
        Ok(out)
    }
}
