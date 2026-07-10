use super::CountingCounters;
use crate::ir::normalize::Role;
use crate::plan::fj::ValidatedPlan;
use crate::schema::Schema;

impl CountingCounters {
    /// Converts the counted execution into the stable stats surface —
    /// the one source of truth `Report` renders from and
    /// `Snapshot::profile` returns. The schema resolves relation names
    /// and renders each eliminated occurrence's licensing statement
    /// (`schema/render.rs`). `pinned` is the prepared query's rendered
    /// pin record (`PreparedQuery::pinned_rows` — the statistics the
    /// estimates derive from), carried through untouched.
    #[must_use]
    pub fn into_stats(
        self,
        plan: &ValidatedPlan,
        schema: &Schema,
        pinned: Vec<crate::api::stats::PinnedRows>,
    ) -> crate::api::stats::ExecutionStats {
        use crate::api::stats::{CoverStats, EliminatedOccurrence, ExecutionStats, NodeStats};
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
        ExecutionStats {
            nodes,
            eliminated,
            pinned,
            emits: self.emits,
            guard: None,
        }
    }
}
