use super::ValidatedQuery;
use crate::ir::{FindTerm, VarId};
use std::collections::BTreeSet;

impl ValidatedQuery {
    /// The plan's sink-relevance set (the D2 gating bits' source). For a
    /// pure projection it is the group key — the suffix skip may cross
    /// nodes binding nothing projected. For an aggregate-bearing find
    /// list it is **every** variable: the fold is defined over the
    /// distinct full binding set, so no node's bindings are skippable,
    /// and the `sink_relevant` bits themselves encode the illegality —
    /// any `SkipSuffix` a future sink ever signaled under an aggregate
    /// plan is absorbed at the node that produced it.
    #[must_use]
    pub fn sink_vars(&self) -> BTreeSet<VarId> {
        let has_aggregate = self
            .query
            .finds
            .iter()
            .any(|term| matches!(term, FindTerm::Aggregate { .. }));
        if has_aggregate {
            self.var_types.keys().copied().collect()
        } else {
            self.group_key.clone()
        }
    }
}
