use super::{Answers, ParamArg, PreparedPipeline, PreparedQuery, PreparedRule};

use crate::api::stats::{ExecutionStats, InteriorStats, KeyProbeStats, RuleStats};
use crate::error::Result;
use crate::exec::introspection::{
    CountingCounters, IntrospectionHeader, IntrospectionReport, RulePlan,
};
use crate::exec::run::Counters;
use crate::image::cache::ImageCache;
use crate::image::view::{Const, FilterPredicate};
use crate::storage::env::ReadTxn;

use super::finalize::finalize;

enum ProfileShape {
    Empty,
    KeyProbe,
    Reach,
    Cq,
}

impl<S> PreparedQuery<'_, S> {
    /// Plan introspection (docs/architecture/40-execution.md): executes the query with counting instrumentation
    /// (ANALYZE semantics) and returns the answers alongside the rendered
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
    pub(crate) fn introspect(
        &mut self,
        txn: &ReadTxn<'_>,
        cache: &ImageCache,
        params: &[ParamArg<'_>],
    ) -> Result<(Answers, String)> {
        let (out, stats) = self.profile(txn, cache, params)?;
        let pending = self.pending_literal_note();
        // A fixpoint program reports every predicate's plan units in
        // predicate order — a recursive rule as its delta variants —
        // each under a label naming its (predicate, rule, variant);
        // the counted surface is the per-stratum round section
        // (`stats.strata`), never per-unit node stats.
        let (rules, unit_labels) = match &self.pipeline {
            PreparedPipeline::Cq { rules, .. } if rules.is_empty() => {
                (vec![RulePlan::Empty], Vec::new())
            }
            PreparedPipeline::Cq { rules, .. } => {
                let mut plans = Vec::new();
                for rule in rules {
                    match rule {
                        PreparedRule::KeyProbe(rule) => {
                            plans.push(RulePlan::KeyProbe(&rule.plan));
                        }
                        PreparedRule::FreeJoin(rule) => {
                            plans.push(RulePlan::FreeJoin(&rule.plan));
                        }
                    }
                }
                (plans, Vec::new())
            }
            PreparedPipeline::Reach { driver, main, .. } => {
                let mut plans = Vec::new();
                let mut labels = Vec::new();
                for (rule_idx, rule) in driver.base.iter().enumerate() {
                    match rule {
                        PreparedRule::KeyProbe(rule) => {
                            plans.push(RulePlan::KeyProbe(&rule.plan));
                            labels.push(format!("reach base {rule_idx}"));
                        }
                        PreparedRule::FreeJoin(rule) => {
                            plans.push(RulePlan::FreeJoin(&rule.plan));
                            labels.push(format!("reach base {rule_idx}"));
                        }
                    }
                }
                for (rule_idx, arm) in driver.rec.iter().enumerate() {
                    plans.push(RulePlan::FreeJoin(&arm.rule.plan));
                    labels.push(format!("reach rec {rule_idx} (delta occ {})", arm.delta.0));
                }
                for (rule_idx, rule) in main.iter().enumerate() {
                    match rule {
                        PreparedRule::KeyProbe(rule) => {
                            plans.push(RulePlan::KeyProbe(&rule.plan));
                            labels.push(format!("main {rule_idx}"));
                        }
                        PreparedRule::FreeJoin(rule) => {
                            plans.push(RulePlan::FreeJoin(&rule.plan));
                            labels.push(format!("main {rule_idx}"));
                        }
                    }
                }
                (plans, labels)
            }
        };
        let report = IntrospectionReport {
            header: Some(IntrospectionHeader {
                query: self.rendered.clone(),
                predicate: self.predicate.to_string(),
                pending_literal: pending,
            }),
            rules,
            unit_labels,
            stats,
        };
        // After the version marker, the report opens with the query in the rule notation
        // (`crate::ir::render` — the read-side syntax) and the predicate
        // it defines (`ir/validate` — the signature authority): introspection
        // prints what it explains.
        Ok((out, report.to_string()))
    }

    /// The pending-literal explanation is derived from the mutable plan
    /// templates after execution: a hit has already latched to `Word` and
    /// disappears; a dictionary miss remains owned raw bytes here.
    fn pending_literal_note(&self) -> Option<String> {
        if self.unresolved_literals == 0 {
            return None;
        }
        let mut literals = Vec::new();
        self.visit_free_join(|rule| {
            let plan = &rule.plan;
            for occurrence in plan
                .occurrences()
                .iter()
                .filter(|occurrence| !occurrence.role.discharged())
            {
                for selection in &occurrence.selections {
                    if let Const::PendingIntern { bytes } = &selection.value {
                        let label = pending_literal_label(bytes);
                        if !literals.contains(&label) {
                            literals.push(label);
                        }
                    }
                }
                for filter in &occurrence.filters {
                    if let FilterPredicate::Compare {
                        value: Const::PendingIntern { bytes },
                        ..
                    } = filter
                    {
                        let label = pending_literal_label(bytes);
                        if !literals.contains(&label) {
                            literals.push(label);
                        }
                    }
                }
            }
        });
        Some(format!(
            "pending literals: {} — an unresolved Eq literal empties its rule at execution until latched\n",
            literals.join(", ")
        ))
    }

    /// The query in the rule notation, rendered at prepare
    /// ([`crate::ir::render`] — one rendered block per rule, `;`-terminated):
    /// the diagnostic twin of the introspection report's header.
    /// Harness-only (not embedding API).
    #[doc(hidden)]
    #[must_use]
    pub fn rendered_query(&self) -> &str {
        &self.rendered
    }

    /// ANALYZE with structured output: executes with counting
    /// instrumentation and returns the answers alongside [`ExecutionStats`]
    /// — the data `introspect` renders. Allocation-sanctioned exactly like
    /// `introspect`. Takes the mixed [`ParamArg`] entry — execute-symmetry
    /// (R13): whatever `execute_args` binds, profiling binds.
    ///
    /// # Errors
    ///
    /// As [`Self::execute_args`].
    ///
    /// # Panics
    ///
    /// Only on programmer-invariant violations (plan/executor pairing).
    #[expect(
        clippy::too_many_lines,
        reason = "the counted rule loop mirrors run_rules with per-rule accounting inline"
    )]
    pub(crate) fn profile(
        &mut self,
        txn: &ReadTxn<'_>,
        cache: &ImageCache,
        params: &[ParamArg<'_>],
    ) -> Result<(Answers, ExecutionStats)> {
        self.check_snapshot(txn)?;
        let mut out = Answers::new();
        out.arity = self.predicate.columns.len();
        // The statically-empty Cq (no interiors, no surviving main rules)
        // mirrors `run_bound`'s short-circuit: bind (errors surface), then
        // nothing runs and nothing is counted — the death record is the
        // whole story. Dead-main with live interiors does not take this
        // path: the preamble still runs and interior emits are counted.
        let shape = match &self.pipeline {
            PreparedPipeline::Cq { interiors, rules } => {
                match (interiors.as_slice(), rules.as_slice()) {
                    ([], []) => ProfileShape::Empty,
                    ([], [PreparedRule::KeyProbe(_)]) => ProfileShape::KeyProbe,
                    _ => ProfileShape::Cq,
                }
            }
            PreparedPipeline::Reach { .. } => ProfileShape::Reach,
        };
        match shape {
            ProfileShape::Empty => {
                self.bind_param_args(txn, params)?;
                return Ok((out, self.empty_stats()));
            }
            ProfileShape::KeyProbe => {
                self.execute_args(txn, cache, params, &mut out)?;
                let emitted = out.len() as u64;
                let distinct_bindings = self.pipeline.main_rules()[0].distinct_witness().is_some();
                let stats = ExecutionStats {
                    introspection_version: crate::api::stats::INTROSPECTION_VERSION,
                    rules: vec![RuleStats {
                        distinct_bindings,
                        nodes: Vec::new(),
                        eliminated: Vec::new(),
                        folded: Vec::new(),
                        pinned: Vec::new(),
                        emitted,
                        absorbed: 0,
                        key_probe: Some(KeyProbeStats {
                            hit: !out.is_empty(),
                        }),
                    }],
                    emits: emitted,
                    disjoint_rules: None,
                    subsumed: self.subsumed.clone(),
                    dead: self.dead.clone(),
                    interiors: Vec::new(),
                    reach: None,
                };
                return Ok((out, stats));
            }
            ProfileShape::Reach => {
                self.bind_param_args(txn, params)?;
                let mut counters = crate::exec::introspection::ReachCounters::new();
                let ran = self.run_rules(txn, cache, &mut counters)?;
                if let Some([start, end]) = self.sink.measure_of_ray() {
                    return Err(crate::error::Error::MeasureOfRay { start, end });
                }
                if ran {
                    finalize(
                        &mut self.sink,
                        &mut self.answer_scratch,
                        &mut self.resolve_memo,
                        txn,
                        &self.predicate.columns,
                        &mut out,
                    )?;
                }
                let emits = out.len() as u64;
                let stats = ExecutionStats {
                    introspection_version: crate::api::stats::INTROSPECTION_VERSION,
                    rules: Vec::new(),
                    emits,
                    disjoint_rules: None,
                    subsumed: self.subsumed.clone(),
                    dead: self.dead.clone(),
                    interiors: self.interior_stats(),
                    reach: Some(counters.into_reach(Vec::new())),
                };
                return Ok((out, stats));
            }
            ProfileShape::Cq => {}
        }
        // Bind once (params reach every rule), reset the sink once (the
        // spanning is the union), then the rule loop with per-rule
        // counting instrumentation; finalize only if some rule ran (a
        // fully short-circuited query counted nothing and has nothing
        // to drain). Interiors-only runs the preamble first so main
        // Interior atoms see finished images — never via `run_reach`.
        self.bind_param_args(txn, params)?;
        if !self.pipeline.interiors().is_empty() {
            self.run_derived(txn, cache, &mut crate::exec::run::NoopCounters)?;
        }
        self.sink.reset();
        let rule_count = self.pipeline.main_rules().len();
        let mut rule_stats = Vec::with_capacity(rule_count);
        let mut ran = false;
        for rule_idx in 0..rule_count {
            let seen_before = self.sink.distinct_seen().unwrap_or(0);
            let mut counters = match &self.pipeline.main_rules()[rule_idx] {
                PreparedRule::FreeJoin(rule) => CountingCounters::new(&rule.plan),
                PreparedRule::KeyProbe(_) => CountingCounters::for_key_probe(),
            };
            ran |= self.run_rule(rule_idx, txn, cache, &mut counters)?;
            let emitted = Counters::emits(&counters);
            let newly_seen = self
                .sink
                .distinct_seen()
                .map_or(emitted, |seen| (seen - seen_before) as u64);
            let absorbed = emitted - newly_seen;
            rule_stats.push(match &self.pipeline.main_rules()[rule_idx] {
                PreparedRule::FreeJoin(rule) => counters.into_rule_stats(
                    &rule.plan,
                    self.schema,
                    self.rule_pinned_rows(rule_idx),
                    absorbed,
                ),
                PreparedRule::KeyProbe(rule) => RuleStats {
                    distinct_bindings: rule.distinct_witness.is_some(),
                    nodes: Vec::new(),
                    eliminated: Vec::new(),
                    folded: Vec::new(),
                    pinned: Vec::new(),
                    emitted,
                    absorbed,
                    key_probe: Some(KeyProbeStats { hit: emitted > 0 }),
                },
            });
        }
        if ran {
            // The ray-probe pass (R6), execute-parity: profiling a
            // query whose Kleene verdict is Ray raises exactly like
            // executing it. Uncounted — the arbiter consumes no
            // answers and belongs to no rule's node table.
            self.run_ray_probes(txn, cache, &mut crate::exec::run::NoopCounters)?;
            finalize(
                &mut self.sink,
                &mut self.answer_scratch,
                &mut self.resolve_memo,
                txn,
                &self.predicate.columns,
                &mut out,
            )?;
        }
        let emits = rule_stats.iter().map(|rule| rule.emitted).sum();
        Ok((
            out,
            ExecutionStats {
                introspection_version: crate::api::stats::INTROSPECTION_VERSION,
                rules: rule_stats,
                emits,
                disjoint_rules: self.disjoint_rules_stat(),
                subsumed: self.subsumed.clone(),
                dead: self.dead.clone(),
                interiors: self.interior_stats(),
                reach: None,
            },
        ))
    }

    fn interior_stats(&self) -> Vec<InteriorStats> {
        self.pipeline
            .interiors()
            .iter()
            .enumerate()
            .map(|(i, interior)| InteriorStats {
                interior: u32::try_from(i).expect("InteriorIdOverflow screened at validate"),
                rules: Vec::new(),
                emits: interior.sink.len() as u64,
            })
            .collect()
    }

    /// The statically-empty program's counted execution: every count is
    /// honestly zero — nothing ran, nothing was read — and the death
    /// record (`stats.dead`) carries the per-rule killing conditions.
    fn empty_stats(&self) -> ExecutionStats {
        ExecutionStats {
            introspection_version: crate::api::stats::INTROSPECTION_VERSION,
            rules: vec![RuleStats {
                distinct_bindings: false,
                nodes: Vec::new(),
                eliminated: Vec::new(),
                folded: Vec::new(),
                pinned: Vec::new(),
                emitted: 0,
                absorbed: 0,
                key_probe: None,
            }],
            emits: 0,
            // An empty program has no pair to prove.
            disjoint_rules: None,
            subsumed: self.subsumed.clone(),
            dead: self.dead.clone(),
            interiors: Vec::new(),
            reach: None,
        }
    }

    /// Whether the aggregate sink's binding seen-set is elided
    /// (40-execution) — the regime observable for the batch-fold fast
    /// path. A single-rule program may elide under its plan's
    /// distinct-bindings proof. A multi-rule program always returns false:
    /// its spanning head-projection seen-set is the union representation.
    #[must_use]
    pub fn distinct_bindings(&self) -> bool {
        match self.pipeline.main_rules() {
            [rule] => rule.distinct_witness().is_some(),
            _ => false,
        }
    }

    /// Whether the program's rules are provably pairwise disjoint
    /// (docs/architecture/40-execution.md § set semantics). This is
    /// diagnostic knowledge, not an executor switch: the measured
    /// cross-rule optimization was reverted. Always `false` for
    /// single-rule programs (no pair exists). The witness is reported by
    /// introspection and
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

fn pending_literal_label(bytes: &[u8]) -> String {
    format!(
        "{:?}",
        std::str::from_utf8(bytes).expect("validated String literal is UTF-8")
    )
}
