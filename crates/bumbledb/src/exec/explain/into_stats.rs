use super::CountingCounters;
use crate::plan::fj::ValidatedPlan;

impl CountingCounters {
    /// Converts the counted execution into the stable stats surface —
    /// the one source of truth `Report` renders from and
    /// `Snapshot::profile` returns.
    #[must_use]
    pub fn into_stats(self, plan: &ValidatedPlan) -> crate::api::stats::ExecutionStats {
        use crate::api::stats::{CoverStats, ExecutionStats, NodeStats};
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
        ExecutionStats {
            nodes,
            emits: self.emits,
            guard: None,
        }
    }
}
