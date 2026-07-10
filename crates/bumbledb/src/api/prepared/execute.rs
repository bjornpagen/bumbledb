use super::{BindValue, ExecPlan, PreparedQuery, ResultBuffer, ValueType};

use crate::error::Result;
use crate::exec::dispatch::execute_guard;
use crate::exec::run::{Counters, NoopCounters};
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
        out: &mut ResultBuffer,
    ) -> Result<()> {
        self.check_snapshot(txn)?;
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

    /// The post-bind execution body shared by every bind shape: the rule
    /// loop into the one sink, then finalize.
    fn run_bound(
        &mut self,
        txn: &ReadTxn<'_>,
        cache: &ImageCache,
        out: &mut ResultBuffer,
    ) -> Result<()> {
        // The point fast lane, single-rule programs only: one probe, one
        // fetch, cells decoded straight into the buffer — no sink, no
        // bindings, no finalize pass. Aggregate-find guards (rare) and
        // guard rules inside multi-rule programs keep the sink path (the
        // union must hear them).
        if self.rules.len() == 1
            && matches!(self.rules[0].plan, ExecPlan::GuardProbe(_))
            && self.rules[0].guard_finds.is_some()
        {
            return self.execute_guard_direct(txn, out);
        }
        // Phase attribution engages only under an active obs capture
        // (docs/architecture/60-validation.md): timing runs — even obs
        // builds — monomorphize NoopCounters and pay nothing.
        #[cfg(feature = "trace")]
        let ran = if obs::capturing() {
            let mut timers = crate::exec::run::PhaseTimers::new();
            let ran = self.run_rules(txn, cache, &mut timers)?;
            timers.flush();
            ran
        } else {
            self.run_rules(txn, cache, &mut NoopCounters)?
        };
        #[cfg(not(feature = "trace"))]
        let ran = self.run_rules(txn, cache, &mut NoopCounters)?;
        if !ran {
            return Ok(()); // every rule short-circuited: empty result
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
        self.sink.reset();
        let mut ran = false;
        for rule_idx in 0..self.rules.len() {
            ran |= self.run_rule(rule_idx, txn, cache, counters)?;
        }
        Ok(ran)
    }

    /// One rule of the loop: re-aim the sink's slot tables at the rule's
    /// binding layout, resolve this execution's predicate constants, and
    /// run the rule's plan — guard probe or Free Join — into the shared
    /// sink. `Ok(false)` = the positive-occurrence `Eq` short-circuit (a
    /// dictionary miss or empty set emptied this conjunctive rule; the
    /// other rules still run — a rule is one disjunct).
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
        if self.rules.len() > 1 {
            let rule = &self.rules[rule_idx];
            self.sink.aim(&rule.finds, rule.plan.slot_count());
        }
        // The rule-shared binding-slot scratch, sized to this rule's
        // layout (capacity is the high-water across all rules).
        self.bindings.resize(self.rules[rule_idx].plan.slot_count());
        let rule = &mut self.rules[rule_idx];
        let ran = match &rule.plan {
            ExecPlan::GuardProbe(guard) => {
                execute_guard(
                    guard,
                    txn,
                    self.schema,
                    &self.resolved_params,
                    &mut self.guard_key,
                    &mut self.bindings,
                    &mut self.sink,
                    counters,
                )?;
                true
            }
            ExecPlan::FreeJoin(plan) => {
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
                if resolved {
                    // This execution's Allen-residual masks (literal or
                    // bound param) resolve into the executor before the
                    // join runs — the hot path never touches the param
                    // slice.
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
                        &mut self.bindings,
                        &rule.resolved_filters,
                        &rule.resolved_selections,
                        &mut rule.memo,
                        &mut self.sink,
                        counters,
                    )?;
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
        Ok(ran)
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
