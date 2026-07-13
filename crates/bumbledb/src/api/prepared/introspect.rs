use super::{BindValue, PreparedQuery, PreparedRule, Program, ResultBuffer};

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
            rules: match &self.program {
                Program::Empty => vec![RulePlan::Empty],
                Program::Rules(rules) => rules
                    .iter()
                    .map(|rule| match rule {
                        PreparedRule::Guard(rule) => RulePlan::GuardProbe(&rule.plan),
                        PreparedRule::FreeJoin(rule) => RulePlan::FreeJoin(&rule.plan),
                    })
                    .collect(),
            },
            stats,
        };
        // The report opens with the query in the rule notation
        // (`crate::ir::render` — the read-side syntax) and the predicate
        // it defines (`ir/validate` — the signature authority): EXPLAIN
        // prints what it explains.
        Ok((
            out,
            format!(
                "query:\n{}\npredicate: {}\n{report}",
                self.rendered, self.predicate
            ),
        ))
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
        out.arity = self.predicate.columns.len();
        // The statically-empty program mirrors `run_bound`'s
        // short-circuit: bind (errors surface), then nothing runs and
        // nothing is counted — the death record is the whole story.
        if matches!(self.program, Program::Empty) {
            self.bind_params(txn, params)?;
            return Ok((out, self.empty_stats()));
        }
        // The single-rule guard program keeps its fast lane: `execute`
        // dispatches it whole, and the stats are the probe's outcome.
        if matches!(self.program.rules(), [PreparedRule::Guard(_)]) {
            self.execute(txn, cache, params, &mut out)?;
            let emitted = out.len() as u64;
            let stats = ExecutionStats {
                rules: vec![RuleStats {
                    nodes: Vec::new(),
                    // A guard probe is a single-atom query: the grounding has
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
        let rule_count = self.program.rules().len();
        let mut rule_stats = Vec::with_capacity(rule_count);
        let mut ran = false;
        for rule_idx in 0..rule_count {
            let seen_before = self.sink.distinct_seen().unwrap_or(0);
            let mut counters = match &self.program.rules()[rule_idx] {
                PreparedRule::FreeJoin(rule) => CountingCounters::new(&rule.plan),
                PreparedRule::Guard(_) => CountingCounters::for_guard(),
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
            rule_stats.push(match &self.program.rules()[rule_idx] {
                PreparedRule::FreeJoin(rule) => counters.into_rule_stats(
                    &rule.plan,
                    self.schema,
                    self.rule_pinned_rows(rule_idx),
                    absorbed,
                ),
                PreparedRule::Guard(_) => RuleStats {
                    nodes: Vec::new(),
                    eliminated: Vec::new(),
                    folded: Vec::new(),
                    pinned: Vec::new(),
                    emitted,
                    absorbed,
                    guard: Some(GuardStats { hit: emitted > 0 }),
                },
            });
        }
        if ran {
            finalize(
                &mut self.sink,
                &mut self.row_scratch,
                &mut self.resolve_memo,
                txn,
                &self.predicate.columns,
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
    /// record (`stats.dead`) carries the per-rule killing conditions.
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
            // An empty program has no pair to prove.
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
        match self.program.rules() {
            [rule] => rule.distinct_bindings(),
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

    /// The predicate this query defines — the buffer-typing authority
    /// (docs/architecture/70-api.md): one column per head position, the
    /// metadata a generic host needs to type an (even empty) result.
    /// The buffer itself stays typeless: stamping owned types per
    /// execution would allocate on the warm path.
    #[must_use]
    pub fn predicate(&self) -> &crate::ir::validate::Predicate {
        &self.predicate
    }
}
