use super::{RunReport, Verdict};

impl RunReport {
    /// ALL-WIN ⇔ every gated read family wins.
    #[must_use]
    pub fn all_win(&self) -> bool {
        self.reads
            .iter()
            .all(|family| family.verdict != Verdict::Loss)
    }

    /// Every gated family's warm p99 within [`P99_BUDGET_NS`].
    #[must_use]
    pub fn budget_ok(&self) -> bool {
        self.reads
            .iter()
            .filter(|family| family.verdict != Verdict::ReportOnly)
            .all(|family| family.p99_within_budget)
    }

    /// The families whose measurement block still read contaminated
    /// after the bounded retry — dirty percentiles, named.
    #[must_use]
    pub fn contaminated_families(&self) -> Vec<&str> {
        self.reads
            .iter()
            .map(|f| (f.name.as_str(), f.ghz))
            .chain(self.writes.iter().map(|f| (f.name.as_str(), f.ghz)))
            .filter(|(_, ghz)| ghz.is_some_and(|g| g.contaminated))
            .map(|(name, _)| name)
            .collect()
    }
}
