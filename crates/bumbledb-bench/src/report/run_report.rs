use super::{RunReport, Verdict};

impl RunReport {

    #[must_use]
    pub fn all_win(&self) -> bool {
        self.reads
            .iter()
            .all(|family| family.verdict != Verdict::Loss)
    }

    #[must_use]
    pub fn budget_ok(&self) -> bool {
        self.reads
            .iter()
            .filter(|family| family.verdict != Verdict::ReportOnly)
            .all(|family| family.p99_within_budget)
    }

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
