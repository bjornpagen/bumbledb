use super::{BindValue, ExecPlan, PreparedQuery, ResultBuffer, ValueType};

use crate::api::stats::{ExecutionStats, GuardStats, RuleStats};
use crate::error::Result;
use crate::exec::explain::{CountingCounters, Report, RulePlan};
use crate::exec::run::Counters;
use crate::image::cache::ImageCache;
use crate::storage::env::ReadTxn;

use super::finalize::finalize;

impl<S> PreparedQuery<'_, S> {
    /// EXPLAIN (docs/architecture/40-execution.md): executes the query with counting instrumentation
    /// (ANALYZE semantics) and returns the rows alongside the rendered
    /// report — per-rule plans and node stats under the head-level union
    /// accounting.
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
        params: &[BindValue<'_>],
    ) -> Result<(ResultBuffer, String)> {
        let (out, stats) = self.profile(txn, cache, params)?;
        let report = Report {
            rules: self
                .rules
                .iter()
                .map(|rule| match &rule.plan {
                    ExecPlan::GuardProbe(guard) => RulePlan::GuardProbe(guard),
                    ExecPlan::FreeJoin(plan) => RulePlan::FreeJoin(plan),
                })
                .collect(),
            stats,
        };
        Ok((out, format!("{report}")))
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
        params: &[BindValue<'_>],
    ) -> Result<(ResultBuffer, ExecutionStats)> {
        self.check_snapshot(txn)?;
        let mut out = ResultBuffer::new();
        out.arity = self.column_types.len();
        // The single-rule guard program keeps its fast lane: `execute`
        // dispatches it whole, and the stats are the probe's outcome.
        if self.rules.len() == 1 && matches!(self.rules[0].plan, ExecPlan::GuardProbe(_)) {
            self.execute(txn, cache, params, &mut out)?;
            let emitted = out.len() as u64;
            let stats = ExecutionStats {
                rules: vec![RuleStats {
                    nodes: Vec::new(),
                    // A guard probe is a single-atom query: the chase has
                    // nothing to pair, so no marks can exist.
                    eliminated: Vec::new(),
                    // Classification precedes statistics: a guard probe
                    // reads none, so nothing is pinned.
                    pinned: Vec::new(),
                    emitted,
                    absorbed: 0,
                    guard: Some(GuardStats {
                        hit: !out.is_empty(),
                    }),
                }],
                emits: emitted,
                // A single-rule program has no pair to prove.
                disjoint_rules: None,
            };
            return Ok((out, stats));
        }
        // Bind once (params reach every rule), reset the sink once (the
        // spanning is the union), then the rule loop with per-rule
        // counting instrumentation; finalize only if some rule ran (a
        // fully short-circuited program counted nothing and has nothing
        // to drain).
        self.bind_params(txn, params)?;
        self.sink.reset();
        let mut rule_stats = Vec::with_capacity(self.rules.len());
        let mut ran = false;
        for rule_idx in 0..self.rules.len() {
            let seen_before = self.sink.distinct_seen().unwrap_or(0);
            let mut counters = match &self.rules[rule_idx].plan {
                ExecPlan::FreeJoin(plan) => CountingCounters::new(plan),
                ExecPlan::GuardProbe(_) => CountingCounters::for_guard(),
            };
            ran |= self.run_rule(rule_idx, txn, cache, &mut counters)?;
            // The union accounting (docs/architecture/40-execution.md
            // § observability): absorbed = emitted − newly-seen; an
            // elided seen-set absorbs nothing by proof.
            let emitted = Counters::emits(&counters);
            let newly_seen = self
                .sink
                .distinct_seen()
                .map_or(emitted, |seen| (seen - seen_before) as u64);
            let absorbed = emitted - newly_seen;
            rule_stats.push(match &self.rules[rule_idx].plan {
                ExecPlan::FreeJoin(plan) => counters.into_rule_stats(
                    plan,
                    self.schema,
                    self.rule_pinned_rows(rule_idx),
                    absorbed,
                ),
                ExecPlan::GuardProbe(_) => RuleStats {
                    nodes: Vec::new(),
                    eliminated: Vec::new(),
                    pinned: Vec::new(),
                    emitted,
                    absorbed,
                    guard: Some(GuardStats { hit: emitted > 0 }),
                },
            });
        }
        if ran {
            finalize(
                &self.sink,
                &mut self.row_scratch,
                &mut self.resolve_memo,
                txn,
                &self.column_types,
                self.all_words,
                &mut out,
            )?;
        }
        let emits = rule_stats.iter().map(|rule| rule.emitted).sum();
        Ok((
            out,
            ExecutionStats {
                rules: rule_stats,
                emits,
                disjoint_rules: self.disjoint_rules_stat(),
            },
        ))
    }

    /// Whether every join rule's plan nodes all bind sink-relevant
    /// variables — the pipelined executor's eligibility, per rule (D2 is
    /// per-rule; a skip never crosses rules). `None` when no rule joins
    /// at all (guard probes run no join).
    #[must_use]
    pub fn skip_free(&self) -> Option<bool> {
        let mut joins = self.rules.iter().filter_map(|rule| match &rule.plan {
            ExecPlan::FreeJoin(plan) => Some(plan.skip_free()),
            ExecPlan::GuardProbe(_) => None,
        });
        let first = joins.next()?;
        Some(joins.fold(first, |all, one| all && one))
    }

    /// Whether the aggregate sink's binding seen-set is elided
    /// (30-execution) — the regime observable for the batch-fold fast
    /// path. Per-query-shape: a single-rule program elides under its
    /// plan's distinct-bindings proof; a multi-rule program elides under
    /// the rule-disjointness composition (docs/architecture/
    /// 40-execution.md § set semantics): disjoint rules ∧ per-rule
    /// distinct bindings ∧ heads reading every slot.
    #[must_use]
    pub fn distinct_bindings(&self) -> bool {
        match &*self.rules {
            [rule] => rule.plan.distinct_bindings(),
            _ => self.union_elided,
        }
    }

    /// Whether the program's rules are provably pairwise disjoint
    /// (docs/architecture/40-execution.md § set semantics) — the flag
    /// that drops the projection sink's cross-rule guard and, composed
    /// with per-rule distinct bindings, elides the aggregate seen-set.
    /// Always `false` for single-rule programs (no pair exists). The
    /// witness itself is reported by EXPLAIN and
    /// [`crate::api::stats::ExecutionStats::disjoint_rules`].
    #[must_use]
    pub fn disjoint_rules(&self) -> bool {
        self.disjoint_rules.is_some()
    }

    /// The stats-facing witness rendering: `(relation, field)` by name,
    /// through the schema the query was prepared against.
    fn disjoint_rules_stat(&self) -> Option<crate::api::stats::DisjointRules> {
        self.disjoint_rules.map(|witness| {
            let relation = self.schema.relation(witness.relation);
            crate::api::stats::DisjointRules {
                relation: relation.name().to_owned(),
                field: relation.field(witness.field).name.to_string(),
            }
        })
    }

    /// The differential guard's override: forces the disjointness
    /// elision off before an execution, so every covered query can run
    /// both regimes against identical inputs — the elision is *never*
    /// semantic, and the results must be byte-identical.
    #[cfg(test)]
    pub(super) fn force_disjoint_off(&mut self) {
        self.sink.force_disjoint_off();
    }

    /// The result column types, one per head position — the metadata a
    /// generic host needs to type an (even empty) result. The buffer
    /// itself stays typeless: stamping owned types per execution would
    /// allocate on the warm path.
    pub fn column_types(&self) -> impl Iterator<Item = &ValueType> {
        self.column_types.iter()
    }
}
