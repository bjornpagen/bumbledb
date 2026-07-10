use super::{ExecPlan, PreparedQuery, ResultBuffer, ValueType};

use crate::api::stats::ExecutionStats;
use crate::error::Result;
use crate::exec::explain::{CountingCounters, Report};
use crate::image::cache::ImageCache;
use crate::ir::Value;
use crate::storage::env::ReadTxn;

use super::bind::resolve_predicates;
use super::finalize::finalize;
use super::run_join::run_join;

impl PreparedQuery<'_> {
    /// EXPLAIN (docs/architecture/40-execution.md): executes the query with counting instrumentation
    /// (ANALYZE semantics) and returns the rows alongside the rendered
    /// report.
    ///
    /// # Errors
    ///
    /// As [`Self::execute`].
    ///
    /// # Panics
    ///
    /// Only on programmer-invariant violations (plan/executor pairing).
    pub(crate) fn explain(
        &mut self,
        txn: &ReadTxn<'_>,
        cache: &ImageCache,
        params: &[Value],
    ) -> Result<(ResultBuffer, String)> {
        let (out, stats) = self.profile(txn, cache, params)?;
        let report = match &self.plan {
            ExecPlan::GuardProbe(guard) => format!("{}", Report::GuardProbe { plan: guard }),
            ExecPlan::FreeJoin(plan) => format!("{}", Report::FreeJoin { plan, stats }),
        };
        Ok((out, report))
    }

    /// ANALYZE with structured output: executes with counting
    /// instrumentation and returns the rows alongside [`ExecutionStats`]
    /// — the data `explain` renders. Allocation-sanctioned exactly like
    /// `explain`.
    ///
    /// # Errors
    ///
    /// As [`Self::execute`].
    ///
    /// # Panics
    ///
    /// Only on programmer-invariant violations (plan/executor pairing).
    pub(crate) fn profile(
        &mut self,
        txn: &ReadTxn<'_>,
        cache: &ImageCache,
        params: &[Value],
    ) -> Result<(ResultBuffer, ExecutionStats)> {
        self.check_snapshot(txn)?;
        let mut out = ResultBuffer::new();
        out.arity = self.finds.len();
        if matches!(&self.plan, ExecPlan::GuardProbe(_)) {
            self.execute(txn, cache, params, &mut out)?;
            let stats = ExecutionStats {
                nodes: Vec::new(),
                // A guard probe is a single-atom query: the chase has
                // nothing to pair, so no marks can exist.
                eliminated: Vec::new(),
                emits: out.len() as u64,
                guard: Some(crate::api::stats::GuardStats {
                    hit: !out.is_empty(),
                }),
            };
            return Ok((out, stats));
        }
        // Bind before borrowing the plan (bind_params takes &mut self).
        self.bind_params(txn, params)?;
        match &self.plan {
            ExecPlan::GuardProbe(_) => unreachable!("handled above"),
            ExecPlan::FreeJoin(plan) => {
                let mut counters = CountingCounters::new(plan);
                let short_circuit = !resolve_predicates(
                    txn,
                    plan,
                    &self.resolved_params,
                    &self.missed_params,
                    &mut self.resolved_filters,
                    &mut self.resolved_selections,
                )?;
                if short_circuit {
                    return Ok((out, counters.into_stats(plan, self.schema)));
                }
                self.sink.reset();
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
                    &mut counters,
                )?;
                finalize(
                    &self.sink,
                    &mut self.row_scratch,
                    &mut self.resolve_memo,
                    txn,
                    &self.finds,
                    self.all_words,
                    &mut out,
                )?;
                Ok((out, counters.into_stats(plan, self.schema)))
            }
        }
    }

    /// The result column types, one per find term in `finds` order — the
    /// metadata a generic host needs to type an (even empty) result. The
    /// buffer itself stays typeless: stamping owned types per execution
    /// would allocate on the warm path.
    /// Whether every plan node binds a sink-relevant variable — the
    /// pipelined executor's eligibility; `None` for
    /// guard plans (no join runs at all).
    #[must_use]
    pub fn skip_free(&self) -> Option<bool> {
        match &self.plan {
            ExecPlan::FreeJoin(plan) => Some(plan.skip_free()),
            ExecPlan::GuardProbe(_) => None,
        }
    }

    /// Whether the plan proved distinct bindings (the aggregate sink's
    /// seen-set elision, 30-execution) — the regime observable for the
    /// batch-fold fast path.
    #[must_use]
    pub fn distinct_bindings(&self) -> bool {
        self.plan.distinct_bindings()
    }

    pub fn column_types(&self) -> impl Iterator<Item = &ValueType> {
        self.finds.iter().map(|(_, ty)| ty)
    }
}
