use super::{Report, RulePlan};
use crate::exec::dispatch::GuardPlan;
use crate::plan::fj::ValidatedPlan;
use std::fmt;

impl fmt::Display for Report<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let multi = self.rules.len() > 1;
        for (rule_idx, rule) in self.rules.iter().enumerate() {
            let stats = &self.stats.rules[rule_idx];
            if multi {
                writeln!(f, "rule {rule_idx}:")?;
            }
            match rule {
                RulePlan::GuardProbe(plan) => fmt_guard_probe(f, plan)?,
                RulePlan::FreeJoin(plan) => fmt_free_join(f, plan, stats)?,
            }
            // The union accounting, per rule (docs/architecture/
            // 40-execution.md § the rule loop): what this rule handed the
            // shared sink and what the spanning seen-set absorbed.
            writeln!(
                f,
                "  emitted bindings: {}, absorbed by the union seen-set: {}",
                stats.emitted, stats.absorbed,
            )?;
        }
        if multi {
            let absorbed: u64 = self.stats.rules.iter().map(|r| r.absorbed).sum();
            writeln!(
                f,
                "head union: {} emitted across {} rules, {} absorbed",
                self.stats.emits,
                self.rules.len(),
                absorbed,
            )?;
            // The rule-disjointness proof (docs/architecture/
            // 40-execution.md § set semantics): an elision must name its
            // proof — the witness (R, f) whose differing pinned literals
            // forbid cross-rule head collisions.
            match &self.stats.disjoint_rules {
                Some(witness) => writeln!(
                    f,
                    "disjoint_rules: proven ({}.{})",
                    witness.relation, witness.field,
                )?,
                None => writeln!(f, "disjoint_rules: unproven")?,
            }
        }
        Ok(())
    }
}

fn fmt_guard_probe(f: &mut fmt::Formatter<'_>, plan: &GuardPlan) -> fmt::Result {
    writeln!(f, "access path: guard probe")?;
    writeln!(f, "  relation: {}", plan.relation.0)?;
    match plan.statement {
        Some(s) => writeln!(f, "  key statement: {}", s.0)?,
        None => writeln!(f, "  full-fact membership probe")?,
    }
    writeln!(
        f,
        "  key fields: {:?}",
        plan.key
            .iter()
            .map(|(field, _)| field.0)
            .collect::<Vec<_>>()
    )?;
    writeln!(f, "  remaining filters: {}", plan.remaining_filters.len())?;
    Ok(())
}

fn fmt_free_join(
    f: &mut fmt::Formatter<'_>,
    plan: &ValidatedPlan,
    stats: &crate::api::stats::RuleStats,
) -> fmt::Result {
    writeln!(f, "access path: free join ({} nodes)", plan.nodes().len())?;
    for (occ_idx, occurrence) in plan.occurrences().iter().enumerate() {
        writeln!(
            f,
            "  occurrence {occ_idx}: relation {} trie schema {:?} ({} filters)",
            occurrence.relation.0,
            occurrence
                .trie_schema
                .iter()
                .map(|level| level.iter().map(|v| v.0).collect::<Vec<_>>())
                .collect::<Vec<_>>(),
            occurrence.filters.len(),
        )?;
        // The pin record: what this occurrence's estimates derive from
        // (absent for occurrences that earned no statistics read —
        // negated, chase-eliminated).
        if let Some(pin) = stats
            .pinned
            .iter()
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
    // The chase's marks (`plan/chase.rs`): occurrences the
    // plan never joined, with the licensing statement.
    for eliminated in &stats.eliminated {
        writeln!(
            f,
            "  eliminated: {} via {}",
            eliminated.relation, eliminated.rendered,
        )?;
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
                match placed.mask {
                    crate::ir::MaskTerm::Literal(mask) => {
                        write!(f, " {:#06x}({}/13)", mask.bits(), mask.popcount())?;
                    }
                    crate::ir::MaskTerm::Param(param) => write!(f, " param{}(?/13)", param.0)?,
                }
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
