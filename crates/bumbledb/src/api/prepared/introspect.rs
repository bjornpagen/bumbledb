use super::{Answers, ParamArg, PreparedPipeline, PreparedQuery, PreparedRule};

use crate::api::stats::{ExecutionStats, InteriorStats, KeyProbeStats, RuleStats};
use crate::error::Result;
use crate::exec::introspection::{IntrospectionHeader, IntrospectionReport, ReportBody, RulePlan};
use crate::image::cache::ImageCache;
use crate::image::view::{Const, FilterPredicate};
use crate::storage::env::ReadTxn;

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
        let body = match &self.pipeline {
            PreparedPipeline::Cq { rules, .. } if rules.is_empty() => ReportBody::Cq {
                plans: vec![RulePlan::Empty],
            },
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
                ReportBody::Cq { plans }
            }
            PreparedPipeline::Reach {
                driver,
                main,
                rec_id,
                ..
            } => {
                let mut units = Vec::new();
                for (rule_idx, rule) in driver.base.iter().enumerate() {
                    let plan = match rule {
                        PreparedRule::KeyProbe(rule) => RulePlan::KeyProbe(&rule.plan),
                        PreparedRule::FreeJoin(rule) => RulePlan::FreeJoin(&rule.plan),
                    };
                    units.push((format!("reach base {rule_idx}"), plan));
                }
                for (rule_idx, arm) in driver.rec.iter().enumerate() {
                    units.push((
                        format!("reach rec {rule_idx} (delta occ {})", arm.delta.0),
                        RulePlan::FreeJoin(&arm.rule.plan),
                    ));
                }
                for (rule_idx, rule) in main.iter().enumerate() {
                    let plan = match rule {
                        PreparedRule::KeyProbe(rule) => RulePlan::KeyProbe(&rule.plan),
                        PreparedRule::FreeJoin(rule) => RulePlan::FreeJoin(&rule.plan),
                    };
                    units.push((format!("main {rule_idx}"), plan));
                }
                ReportBody::Reach {
                    rec_id: *rec_id,
                    units,
                }
            }
        };
        let report = IntrospectionReport {
            header: Some(IntrospectionHeader {
                query: self.rendered.clone(),
                predicate: self.signature.to_string(),
                pending_literal: pending,
            }),
            body,
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
    /// One protocol with [`Self::execute`]: bind, then the same dispatch
    /// (`key_probe_direct` / empty Cq / `run_rules`), parameterized by
    /// counters. Profile does not fabricate a key-probe access path for
    /// a rule that ran through the sink.
    ///
    /// # Errors
    ///
    /// As [`Self::execute_args`].
    ///
    /// # Panics
    ///
    /// Only on programmer-invariant violations (plan/executor pairing).
    pub(crate) fn profile(
        &mut self,
        txn: &ReadTxn<'_>,
        cache: &ImageCache,
        params: &[ParamArg<'_>],
    ) -> Result<(Answers, ExecutionStats)> {
        self.check_snapshot(txn)?;
        let mut out = Answers::new();
        out.arity = self.signature.columns.len();
        {
            let _s = crate::obs::span(
                crate::obs::names::BIND_PARAMS,
                crate::obs::Category::Execute,
            );
            self.bind_param_args(txn, params)?;
        }
        if self.key_probe_direct {
            self.execute_key_probe_direct(txn, &mut out)?;
            let emitted = out.len() as u64;
            let distinct_bindings = self.pipeline.main_rules()[0].distinct_witness().is_some();
            let stats = ExecutionStats::cq(
                vec![RuleStats {
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
                Vec::new(),
                emitted,
                None,
                self.subsumed.clone(),
                self.dead.clone(),
            );
            return Ok((out, stats));
        }
        if self.pipeline.is_empty_cq() {
            return Ok((out, self.empty_stats()));
        }
        match &self.pipeline {
            PreparedPipeline::Reach { .. } => {
                let mut counters = crate::exec::introspection::ReachCounters::new();
                let ran = self.run_rules(txn, cache, &mut counters)?;
                self.finish_sink(txn, ran, &mut out)?;
                let emits = out.len() as u64;
                let stats = ExecutionStats::reach_body(
                    self.interior_stats(),
                    counters.into_reach(),
                    emits,
                    self.subsumed.clone(),
                    self.dead.clone(),
                );
                Ok((out, stats))
            }
            PreparedPipeline::Cq { .. } => {
                let mut rule_stats = Vec::new();
                let ran = self.run_rules_cq_profile(txn, cache, &mut rule_stats)?;
                self.finish_sink(txn, ran, &mut out)?;
                let emits = rule_stats.iter().map(|rule| rule.emitted).sum();
                Ok((
                    out,
                    ExecutionStats::cq(
                        rule_stats,
                        self.interior_stats(),
                        emits,
                        self.disjoint_rules_stat(),
                        self.subsumed.clone(),
                        self.dead.clone(),
                    ),
                ))
            }
        }
    }

    fn interior_stats(&self) -> Vec<InteriorStats> {
        self.pipeline
            .interiors()
            .iter()
            .enumerate()
            .map(|(i, interior)| InteriorStats {
                interior: u32::try_from(i).expect("InteriorIdOverflow screened at validate"),
                emits: interior.sink.len() as u64,
            })
            .collect()
    }

    /// The statically-empty program's counted execution: every count is
    /// honestly zero — nothing ran, nothing was read — and the death
    /// record (`stats.dead`) carries the per-rule killing conditions.
    fn empty_stats(&self) -> ExecutionStats {
        ExecutionStats::cq(
            Vec::new(),
            Vec::new(),
            0,
            None,
            self.subsumed.clone(),
            self.dead.clone(),
        )
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
    pub fn signature(&self) -> &crate::ir::validate::Signature {
        &self.signature
    }
}

fn pending_literal_label(bytes: &[u8]) -> String {
    format!(
        "{:?}",
        std::str::from_utf8(bytes).expect("validated String literal is UTF-8")
    )
}
