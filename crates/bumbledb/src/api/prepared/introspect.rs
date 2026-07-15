use super::{Answers, BindValue, PreparedQuery, PreparedRule, Program};

use crate::api::stats::{ExecutionStats, KeyProbeStats, RuleStats};
use crate::error::Result;
use crate::exec::introspection::{
    CountingCounters, IntrospectionHeader, IntrospectionReport, RulePlan,
};
use crate::exec::run::Counters;
use crate::image::cache::ImageCache;
use crate::image::view::{Const, FilterPredicate};
use crate::storage::env::ReadTxn;

use super::finalize::finalize;

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
        params: &[BindValue<'_>],
    ) -> Result<(Answers, String)> {
        let (out, stats) = self.profile(txn, cache, params)?;
        let pending = self.pending_literal_note();
        // A fixpoint program reports every predicate's plan units in
        // predicate order — a recursive rule as its delta variants —
        // each under a label naming its (predicate, rule, variant);
        // the counted surface is the per-stratum round section
        // (`stats.strata`), never per-unit node stats.
        let (rules, unit_labels) = match &self.program {
            Program::Empty => (vec![RulePlan::Empty], Vec::new()),
            Program::Rules(rules) => {
                let mut plans = Vec::new();
                for rule in rules {
                    match rule {
                        PreparedRule::KeyProbe(rule) => {
                            plans.push(RulePlan::KeyProbe(&rule.plan));
                        }
                        PreparedRule::FreeJoin(rule) => {
                            plans.push(RulePlan::FreeJoin(&rule.plan));
                        }
                        PreparedRule::Recursive(_) => {
                            unreachable!("recursive rules live under Program::Fixpoint")
                        }
                    }
                }
                (plans, Vec::new())
            }
            Program::Fixpoint(program) => {
                let mut plans = Vec::new();
                let mut labels = Vec::new();
                for (pred_idx, pred) in program.predicates.iter().enumerate() {
                    for (rule_idx, rule) in pred.rules.iter().enumerate() {
                        match rule {
                            PreparedRule::KeyProbe(rule) => {
                                plans.push(RulePlan::KeyProbe(&rule.plan));
                                labels.push(format!("predicate p{pred_idx} rule {rule_idx}"));
                            }
                            PreparedRule::FreeJoin(rule) => {
                                plans.push(RulePlan::FreeJoin(&rule.plan));
                                labels.push(format!("predicate p{pred_idx} rule {rule_idx}"));
                            }
                            PreparedRule::Recursive(rule) => {
                                for (variant_idx, variant) in rule.variants.iter().enumerate() {
                                    plans.push(RulePlan::FreeJoin(&variant.rule.plan));
                                    labels.push(format!(
                                        "predicate p{pred_idx} rule {rule_idx} delta variant \
                                         {variant_idx} (delta occ {})",
                                        variant.delta.0
                                    ));
                                }
                            }
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
        let free_join_plans = self.program.all_rules().flat_map(|rule| match rule {
            PreparedRule::FreeJoin(rule) => std::slice::from_ref(rule)
                .iter()
                .map(|rule| &rule.plan)
                .collect::<Vec<_>>(),
            PreparedRule::Recursive(rule) => rule
                .variants
                .iter()
                .map(|variant| &variant.rule.plan)
                .collect(),
            PreparedRule::KeyProbe(_) => Vec::new(),
        });
        for plan in free_join_plans {
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
        }
        Some(format!(
            "pending literals: {} — an unresolved Eq literal empties its rule at execution until latched\n",
            literals.join(", ")
        ))
    }

    /// The query in the rule notation, rendered at prepare
    /// ([`crate::ir::render`] — one rendered block per rule, `;`-terminated):
    /// the diagnostic twin of the introspection report's header, for hosts
    /// that log or display the query a prepared handle answers.
    #[must_use]
    pub fn rendered_query(&self) -> &str {
        &self.rendered
    }

    /// ANALYZE with structured output: executes with counting
    /// instrumentation and returns the answers alongside [`ExecutionStats`]
    /// — the data `introspect` renders. Allocation-sanctioned exactly like
    /// `introspect`.
    ///
    /// # Errors
    ///
    /// As [`Self::execute`].
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
        params: &[BindValue<'_>],
    ) -> Result<(Answers, ExecutionStats)> {
        self.check_snapshot(txn)?;
        let mut out = Answers::new();
        out.arity = self.predicate.columns.len();
        // The statically-empty program mirrors `run_bound`'s
        // short-circuit: bind (errors surface), then nothing runs and
        // nothing is counted — the death record is the whole story.
        if matches!(self.program, Program::Empty) {
            self.bind_params(txn, params)?;
            return Ok((out, self.empty_stats()));
        }
        // The single-rule key-probe program keeps its fast lane: `execute`
        // dispatches it whole, and the stats are the probe's outcome.
        if matches!(self.program.rules(), [PreparedRule::KeyProbe(_)]) {
            self.execute(txn, cache, params, &mut out)?;
            let emitted = out.len() as u64;
            let stats = ExecutionStats {
                introspection_version: crate::api::stats::INTROSPECTION_VERSION,
                rules: vec![RuleStats {
                    distinct_bindings: self.program.rules()[0].distinct_witness().is_some(),
                    nodes: Vec::new(),
                    // A key probe is a single-atom query: the grounding has
                    // nothing to pair and nothing to fold, so no marks
                    // can exist.
                    eliminated: Vec::new(),
                    folded: Vec::new(),
                    // Classification precedes statistics: a key probe
                    // reads none, so nothing is pinned.
                    pinned: Vec::new(),
                    emitted,
                    absorbed: 0,
                    key_probe: Some(KeyProbeStats {
                        hit: !out.is_empty(),
                    }),
                }],
                emits: emitted,
                // A single-rule program has no pair to prove.
                disjoint_rules: None,
                // ... but may still be a deletion pass's residue: a
                // program deleted down to one key-probe rule keeps both
                // records.
                subsumed: self.subsumed.clone(),
                dead: self.dead.clone(),
                strata: Vec::new(),
            };
            return Ok((out, stats));
        }
        // A fixpoint program executes whole under the driver with the
        // driver-level counter: the counted surface is per-stratum,
        // per-round — delta sizes and the union accounting — through
        // the `Counters` seam's fixpoint hooks
        // (docs/architecture/40-execution.md § the fixpoint driver).
        // Per-unit node stats deliberately do not exist: one counter
        // spans many differently shaped plan units.
        if matches!(self.program, Program::Fixpoint(_)) {
            self.bind_params(txn, params)?;
            let mut counters = crate::exec::introspection::FixpointCounters::new();
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
                    self.answer_heap,
                    &mut out,
                )?;
            }
            let emits = counters.total_emits();
            let stats = ExecutionStats {
                introspection_version: crate::api::stats::INTROSPECTION_VERSION,
                rules: Vec::new(),
                emits,
                disjoint_rules: None,
                subsumed: self.subsumed.clone(),
                dead: self.dead.clone(),
                strata: counters.into_strata(),
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
                PreparedRule::KeyProbe(_) => CountingCounters::for_key_probe(),
                PreparedRule::Recursive(_) => {
                    unreachable!("recursive rules live under Program::Fixpoint, handled above")
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
            rule_stats.push(match &self.program.rules()[rule_idx] {
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
                PreparedRule::Recursive(_) => {
                    unreachable!("recursive rules live under Program::Fixpoint, handled above")
                }
            });
        }
        if ran {
            finalize(
                &mut self.sink,
                &mut self.answer_scratch,
                &mut self.resolve_memo,
                txn,
                &self.predicate.columns,
                self.answer_heap,
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
                strata: Vec::new(),
            },
        ))
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
            strata: Vec::new(),
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
