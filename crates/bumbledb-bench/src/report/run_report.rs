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
}
