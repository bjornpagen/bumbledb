use super::Verdict;

use crate::families::Kind;

impl Verdict {
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Win => "WIN",
            Self::Loss => "LOSS",
            Self::ReportOnly => "report",
        }
    }
}

/// The gate rule, pinned here.
#[must_use]
pub fn verdict(kind: Kind, ours_p50: u64, theirs_p50: u64) -> Verdict {
    match kind {
        Kind::Report => Verdict::ReportOnly,
        Kind::Gate => {
            if ours_p50 < theirs_p50 {
                Verdict::Win
            } else {
                Verdict::Loss
            }
        }
    }
}
