use super::{VerifyConfig, stamp_value};

use std::path::Path;

/// Whether `path` holds the stamp for this config — the gate the harness
/// (PRD 13) and the CLI (PRD 19) consume before timing anything.
#[must_use]
pub fn stamp_matches(cfg: &VerifyConfig, path: &Path) -> bool {
    std::fs::read_to_string(path).is_ok_and(|stored| stored.trim() == stamp_value(cfg))
}
