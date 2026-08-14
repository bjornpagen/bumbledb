use super::{IntrospectionReport, ReportBody, RulePlan};
use crate::exec::dispatch::KeyProbePlan;
use crate::plan::fj::ValidatedPlan;
use std::fmt;

impl fmt::Display for IntrospectionReport<'_> {
    #[expect(
        clippy::too_many_lines,
        reason = "one report shape: header then body arms in declaration order"
    )]
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(
            f,
            "introspection v{}",
            crate::api::stats::INTROSPECTION_VERSION
        )?;
        if let Some(header) = &self.header {
            writeln!(f, "query:\n{}", header.query)?;
            writeln!(f, "predicate: {}", header.predicate)?;
            if let Some(pending) = &header.pending_literal {
                write!(f, "{pending}")?;
            }
        }
        let rec_id = match &self.body {
            ReportBody::Reach { rec_id, .. } => Some(*rec_id),
            ReportBody::Cq { .. } => None,
        };
        match &self.body {
            ReportBody::Cq { plans } => {
                let multi = self.stats.rules().len() > 1;
                for (rule_idx, rule) in plans.iter().enumerate() {
                    let stats = self.stats.rules().get(rule_idx);
                    if multi {
                        writeln!(f, "rule {rule_idx}:")?;
                    }
                    match rule {
                        RulePlan::KeyProbe(plan) => fmt_key_probe(f, plan)?,
                        RulePlan::FreeJoin(plan) => fmt_free_join(f, plan, stats, rec_id)?,
                        RulePlan::Empty => writeln!(f, "access path: statically empty")?,
                    }
                    let Some(stats) = stats else { continue };
                    writeln!(
                        f,
                        "  distinct_bindings: {}",
                        if stats.distinct_bindings {
                            "proven"
                        } else {
                            "unproven"
                        }
                    )?;
                    writeln!(
                        f,
                        "  emitted bindings: {}, absorbed by the union seen-set: {}",
                        stats.emitted, stats.absorbed,
                    )?;
                }
                if multi {
                    let absorbed: u64 = self.stats.rules().iter().map(|r| r.absorbed).sum();
                    writeln!(
                        f,
                        "head union: {} emitted across {} rules, {} absorbed",
                        self.stats.emits,
                        plans.len(),
                        absorbed,
                    )?;
                    match &self.stats.disjoint_rules {
                        Some(witness) => writeln!(
                            f,
                            "disjoint_rules: proven ({}.{})",
                            witness.relation, witness.field,
                        )?,
                        None => writeln!(f, "disjoint_rules: unproven")?,
                    }
                }
            }
            ReportBody::Reach { units, .. } => {
                for (label, rule) in units {
                    writeln!(f, "{label}:")?;
                    match rule {
                        RulePlan::KeyProbe(plan) => fmt_key_probe(f, plan)?,
                        RulePlan::FreeJoin(plan) => fmt_free_join(f, plan, None, rec_id)?,
                        RulePlan::Empty => writeln!(f, "access path: statically empty")?,
                    }
                }
            }
        }
        for interior in self.stats.interiors() {
            writeln!(
                f,
                "interior {}: {} emits",
                interior.interior, interior.emits
            )?;
        }
        if let Some(reach) = self.stats.reach() {
            writeln!(f, "reach: {} rounds", reach.rounds.len())?;
            for (round_idx, round) in reach.rounds.iter().enumerate() {
                write!(f, "  round {round_idx}:")?;
                if round.delta > 0 {
                    write!(f, " delta {};", round.delta)?;
                }
                writeln!(f, " emitted {}, absorbed {}", round.emitted, round.absorbed)?;
            }
        }
        // The subsumption record (`plan/ground.rs`): rules deleted at
        // prepare with the subsuming rule's index — lowered-rule
        // indices; the per-rule sections above are the survivors.
        for subsumed in &self.stats.subsumed {
            writeln!(
                f,
                "subsumed: rule {} by rule {}",
                subsumed.rule, subsumed.by
            )?;
        }
        // The death record (`ir/normalize/fold.rs`): each statically-
        // empty rule with its killing condition — lowered-rule indices,
        // exactly as the subsumption lines.
        for dead in &self.stats.dead {
            writeln!(f, "statically empty: rule {}: {}", dead.rule, dead.rendered)?;
        }
        Ok(())
    }
}

fn fmt_key_probe(f: &mut fmt::Formatter<'_>, plan: &KeyProbePlan) -> fmt::Result {
    writeln!(f, "access path: key probe")?;
    writeln!(f, "  relation: {}", plan.relation.0)?;
    match &plan.kind {
        crate::exec::dispatch::KeyProbeKind::Uniqueness { statement, .. } => {
            writeln!(f, "  key statement: {}", statement.0)?;
        }
        crate::exec::dispatch::KeyProbeKind::Membership { .. } => {
            writeln!(f, "  full-fact membership probe")?;
        }
    }
    writeln!(
        f,
        "  key fields: {:?}",
        plan.kind
            .key()
            .iter()
            .map(|(field, _)| field.0)
            .collect::<Vec<_>>()
    )?;
    writeln!(f, "  remaining filters: {}", plan.remaining_filters.len())?;
    Ok(())
}

#[expect(
    clippy::too_many_lines,
    reason = "one plan's rendering reads as one fixed-order artifact"
)]
fn fmt_free_join(
    f: &mut fmt::Formatter<'_>,
    plan: &ValidatedPlan,
    stats: Option<&crate::api::stats::RuleStats>,
    rec_id: Option<crate::ir::InteriorId>,
) -> fmt::Result {
    writeln!(f, "access path: free join ({} nodes)", plan.nodes().len())?;
    for (occ_idx, occurrence) in plan.occurrences().iter().enumerate() {
        let source = match occurrence.source {
            crate::ir::AtomSource::Edb(relation) => format!("relation {}", relation.0),
            crate::ir::AtomSource::Interior(pred) if rec_id == Some(pred) => "rec".to_string(),
            crate::ir::AtomSource::Interior(pred) => format!("interior {}", pred.0),
        };
        writeln!(
            f,
            "  occurrence {occ_idx}: {source} trie schema {:?} ({} filters)",
            occurrence
                .trie_schema
                .iter()
                .map(|level| level.iter().map(|v| v.0).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            occurrence.filters.len(),
        )?;
        // The pin record: what this occurrence's estimates derive from
        // (absent for occurrences that earned no statistics read —
        // negated, grounding-eliminated).
        if let Some(pin) = stats
            .into_iter()
            .flat_map(|stats| stats.pinned.iter())
            .find(|p| usize::from(p.occurrence) == occ_idx)
        {
            write!(
                f,
                "    estimated from (pinned rows at prepare): {}",
                pin.rows
            )?;
            match pin.survivors {
                Some(survivors) => writeln!(f, ", filtered-view survivors {survivors}")?,
                None => writeln!(f)?,
            }
        }
    }
    let Some(stats) = stats else {
        // A Reach plan unit: the plan is the whole per-unit story;
        // the counted stats live in the reach rounds.
        return Ok(());
    };
    // The grounding's marks (`plan/ground.rs`): occurrences the
    // plan never joined, with the licensing statement.
    for eliminated in &stats.eliminated {
        writeln!(
            f,
            "  eliminated: {} via {}",
            eliminated.relation, eliminated.rendered,
        )?;
    }
    // The evaluator's marks (`plan/ground/evaluate.rs`): closed atoms
    // evaluated at prepare — the filters and the surviving handle set
    // (the vocabulary's names, the set IS the payload); a negated
    // fold's attached set is the complement, and the named handles are
    // what the deleted anti-probe would have rejected.
    for folded in &stats.folded {
        let set = folded.handles.join(", ");
        if folded.negated {
            writeln!(f, "  folded: !{} → {{{set}}} rejected", folded.rendered)?;
        } else {
            writeln!(f, "  folded: {} → {{{set}}}", folded.rendered)?;
        }
    }
    for (node_idx, node) in plan.nodes().iter().enumerate() {
        let node_stats = &stats.nodes[node_idx];
        writeln!(f, "  node {node_idx}:")?;
        for (sub_idx, subatom) in node.subatoms.iter().enumerate() {
            let cover = &node_stats.covers[sub_idx];
            writeln!(
                f,
                "    subatom {sub_idx}: occ {} vars {:?} cover({}) chosen \
                 exact={} estimate={} probes hit={} miss={}",
                subatom.occ.0,
                subatom.vars.iter().map(|v| v.0).collect::<Vec<_>>(),
                node.covers
                    .contains(&u8::try_from(sub_idx).expect("subatoms per node fit u8")),
                cover.chosen_exact,
                cover.chosen_estimate,
                cover.probes_hit,
                cover.probes_miss,
            )?;
        }
        writeln!(
            f,
            "    residuals: {} placed, pass={} fail={}",
            node.residuals.len(),
            node_stats.residual_pass,
            node_stats.residual_fail,
        )?;
        // Per-node mask selectivity, est vs actual: est is the mask's
        // measure in the coordinate system (popcount/13, the
        // selectivity model's fraction — `plan/selectivity.rs`); actual
        // rides the residual pass/fail counts above (the configuration
        // kernel fires the same residual counter per element — no new
        // instrumentation category).
        if !node.allen_residuals.is_empty() {
            write!(f, "    allen masks (est keep, actual above):")?;
            for placed in &node.allen_residuals {
                write!(
                    f,
                    " {:#06x}({}/13)",
                    placed.mask.bits(),
                    placed.mask.popcount()
                )?;
            }
            writeln!(f)?;
        }
        writeln!(
            f,
            "    anti-probes: {} placed, probed={} rejected={}",
            node.anti_probes.len(),
            node_stats.anti_probe_probed,
            node_stats.anti_probe_rejected,
        )?;
        writeln!(
            f,
            "    estimated={} actual={} entries={} skips={}",
            node_stats.estimate, node_stats.actual, node_stats.entries, node_stats.skips,
        )?;
    }
    Ok(())
}
