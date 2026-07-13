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
                    ExecPlan::Empty => RulePlan::Empty,
                })
                .collect(),
            stats,
        };
        // The report opens with the query in the rule notation
        // (`crate::ir::render` — the read-side syntax): EXPLAIN prints
        // what it explains.
        Ok((out, format!("query:\n{}\n{report}", self.rendered)))
    }

    /// The query in the rule notation, rendered at prepare
    /// ([`crate::ir::render`] — one clause per rule, `;`-terminated):
    /// the diagnostic twin of the EXPLAIN report's header, for hosts
    /// that log or display the query a prepared handle answers.
    #[must_use]
    pub fn rendered_query(&self) -> &str {
        &self.rendered
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
        // The statically-empty program mirrors `run_bound`'s
        // short-circuit: bind (errors surface), then nothing runs and
        // nothing is counted — the death record is the whole story.
        if matches!(self.rules[0].plan, ExecPlan::Empty) {
            self.bind_params(txn, params)?;
            return Ok((out, self.empty_stats()));
        }
        // The single-rule guard program keeps its fast lane: `execute`
        // dispatches it whole, and the stats are the probe's outcome.
        if self.rules.len() == 1 && matches!(self.rules[0].plan, ExecPlan::GuardProbe(_)) {
            self.execute(txn, cache, params, &mut out)?;
            let emitted = out.len() as u64;
            let stats = ExecutionStats {
                rules: vec![RuleStats {
                    nodes: Vec::new(),
                    // A guard probe is a single-atom query: the chase has
                    // nothing to pair and nothing to fold, so no marks
                    // can exist.
                    eliminated: Vec::new(),
                    folded: Vec::new(),
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
                // ... but may still be a deletion pass's residue: a
                // program deleted down to one guard rule keeps both
                // records.
                subsumed: self.subsumed.clone(),
                dead: self.dead.clone(),
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
                ExecPlan::Empty => {
                    unreachable!("the empty plan short-circuited above")
                }
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
                    folded: Vec::new(),
                    pinned: Vec::new(),
                    emitted,
                    absorbed,
                    guard: Some(GuardStats { hit: emitted > 0 }),
                },
                ExecPlan::Empty => {
                    unreachable!("the empty plan short-circuited above")
                }
            });
        }
        if ran {
            finalize(
                &mut self.sink,
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
                subsumed: self.subsumed.clone(),
                dead: self.dead.clone(),
            },
        ))
    }

    /// The statically-empty program's counted execution: every count is
    /// honestly zero — nothing ran, nothing was read — and the death
    /// record (`stats.dead`) carries the per-rule killing predicates.
    fn empty_stats(&self) -> ExecutionStats {
        ExecutionStats {
            rules: vec![RuleStats {
                nodes: Vec::new(),
                eliminated: Vec::new(),
                folded: Vec::new(),
                pinned: Vec::new(),
                emitted: 0,
                absorbed: 0,
                guard: None,
            }],
            emits: 0,
            // One empty plan, no pair to prove.
            disjoint_rules: None,
            subsumed: self.subsumed.clone(),
            dead: self.dead.clone(),
        }
    }

    /// Whether the aggregate sink's binding seen-set is elided
    /// (40-execution) — the regime observable for the batch-fold fast
    /// path. A single-rule program may elide under its plan's
    /// distinct-bindings proof. A multi-rule program always returns false:
    /// its spanning head-projection seen-set is the union representation.
    #[must_use]
    pub fn distinct_bindings(&self) -> bool {
        match &*self.rules {
            [rule] => rule.plan.distinct_bindings(),
            _ => false,
        }
    }

    /// Whether the program's rules are provably pairwise disjoint
    /// (docs/architecture/40-execution.md § set semantics). This is
    /// diagnostic knowledge, not an executor switch: the measured
    /// cross-rule optimization was reverted. Always `false` for
    /// single-rule programs (no pair exists). The witness is reported by
    /// EXPLAIN and
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

    /// The result column types, one per head position — the metadata a
    /// generic host needs to type an (even empty) result. The buffer
    /// itself stays typeless: stamping owned types per execution would
    /// allocate on the warm path.
    pub fn column_types(&self) -> impl Iterator<Item = &ValueType> {
        self.column_types.iter()
    }
}
