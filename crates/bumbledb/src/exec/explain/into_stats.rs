use super::CountingCounters;
use crate::ir::normalize::Role;
use crate::plan::fj::ValidatedPlan;
use crate::schema::Schema;

impl CountingCounters {
    /// Converts one rule's counted execution into the stable stats
    /// surface — the source of truth `Report` renders from and
    /// `Snapshot::profile` returns (one of these per rule; the rule loop
    /// assembles the program-level `ExecutionStats`). The schema resolves
    /// relation names and renders each eliminated occurrence's licensing
    /// statement (`schema/render.rs`). `pinned` is the rule's rendered
    /// pin record (the statistics its estimates derive from), carried
    /// through untouched; `absorbed` is the union accounting the rule
    /// loop measured against the shared sink's seen-set.
    #[must_use]
    pub fn into_rule_stats(
        self,
        plan: &ValidatedPlan,
        schema: &Schema,
        pinned: Vec<crate::api::stats::PinnedRows>,
        absorbed: u64,
    ) -> crate::api::stats::RuleStats {
        use crate::api::stats::{
            CoverStats, EliminatedOccurrence, FoldedOccurrence, NodeStats, RuleStats,
        };
        let nodes = plan
            .nodes()
            .iter()
            .enumerate()
            .map(|(node_idx, node)| {
                let covers = (0..node.subatoms.len())
                    .map(|sub_idx| {
                        let [hit, miss] = self.probes[node_idx * self.stride + sub_idx];
                        let [exact, estimate] = self.cover_histogram(node_idx, sub_idx);
                        CoverStats {
                            subatom: sub_idx,
                            chosen_exact: exact,
                            chosen_estimate: estimate,
                            probes_hit: hit,
                            probes_miss: miss,
                            hashes: self.hashes[node_idx * self.stride + sub_idx],
                        }
                    })
                    .collect();
                let [pass, fail] = self.residuals[node_idx];
                let [anti_miss, anti_hit] = self.anti_probes[node_idx];
                let [batches, batch_entries] = self.batches[node_idx];
                NodeStats {
                    entries: self.node_entries[node_idx],
                    batches,
                    batch_entries,
                    estimate: plan.estimates().get(node_idx).copied().unwrap_or(0),
                    actual: self.actual_after(node_idx),
                    covers,
                    residual_pass: pass,
                    residual_fail: fail,
                    anti_probe_probed: anti_miss + anti_hit,
                    anti_probe_rejected: anti_hit,
                    skips: self.skips[node_idx],
                }
            })
            .collect();
        // The eliminated occurrences, read straight off the plan's
        // `Role::Eliminated` marks (`plan/chase.rs` — no separate list
        // exists).
        let eliminated = plan
            .occurrences()
            .iter()
            .filter_map(|occurrence| {
                let Role::Eliminated(statement) = occurrence.role else {
                    return None;
                };
                Some(EliminatedOccurrence {
                    occurrence: occurrence.occ_id.0,
                    relation: schema.relation(occurrence.relation).name().to_owned(),
                    statement,
                    rendered: crate::schema::render::render(schema, statement),
                })
            })
            .collect();
        // The folded occurrences, read straight off the plan's
        // `Role::Folded` marks (`plan/chase/evaluate.rs` — the
        // Eliminated precedent: no separate list exists); the picture
        // renders from the occurrence's retained filter list, and the
        // surviving id-set re-evaluates from the sealed extension (the
        // same σ the fold ran — n ≤ 256, diagnostic-path only) so the
        // line can name the handles, not count the numbers.
        let folded = plan
            .occurrences()
            .iter()
            .filter_map(|occurrence| {
                let Role::Folded(mark) = occurrence.role else {
                    return None;
                };
                let relation = schema.relation(occurrence.relation);
                let handles =
                    crate::plan::chase::evaluate::surviving_ids(relation, &occurrence.filters)
                        .into_iter()
                        .map(|id| {
                            let mut handle = String::new();
                            crate::plan::chase::evaluate::push_handle(&mut handle, relation, id);
                            handle
                        })
                        .collect();
                Some(FoldedOccurrence {
                    occurrence: occurrence.occ_id.0,
                    relation: relation.name().to_owned(),
                    rendered: crate::plan::chase::evaluate::folded_picture(
                        schema,
                        occurrence.relation,
                        &occurrence.filters,
                    ),
                    handles,
                    negated: mark.negated,
                })
            })
            .collect();
        RuleStats {
            nodes,
            eliminated,
            folded,
            pinned,
            emitted: self.emits,
            absorbed,
            guard: None,
        }
    }
}
