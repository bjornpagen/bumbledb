use super::GhzReport;

impl GhzReport {
    /// The rendered status word.
    #[must_use]
    pub fn status(&self) -> &'static str {
        if self.contaminated {
            "CONTAMINATED"
        } else if self.retried {
            "retried"
        } else {
            "clean"
        }
    }
}
