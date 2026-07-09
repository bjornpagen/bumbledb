use super::{stamp_value, VerifyConfig};

use std::path::Path;

/// Whether `path` holds the stamp for this config — the gate the harness
/// and the CLI consume before timing anything.
#[must_use]
pub fn stamp_matches(cfg: &VerifyConfig, path: &Path) -> bool {
    std::fs::read_to_string(path).is_ok_and(|stored| stored.trim() == stamp_value(cfg))
}
