use super::GhzReport;

impl GhzReport {

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
