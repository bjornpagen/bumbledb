use super::{VerifyConfig, stamp_value};

use std::path::Path;

/// True when `path` holds the verify stamp of this binary and corpus, the
/// check the bench and the CLI make before timing anything.
#[must_use]
pub fn stamp_matches(cfg: &VerifyConfig, path: &Path) -> bool {
    std::fs::read_to_string(path).is_ok_and(|stored| stored.trim() == stamp_value(cfg))
}
