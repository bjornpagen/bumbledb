use std::path::PathBuf;

use crate::{json, report};

/// `merge`: N run directories' `report.json` → the min-of-runs table on
/// stdout.
///
/// # Errors
///
/// Unreadable or unparseable report files, named.
pub fn cmd_merge(dirs: &[PathBuf]) -> Result<i32, String> {
    let runs: Vec<(String, json::Value)> = dirs
        .iter()
        .map(|dir| {
            let path = dir.join("report.json");
            let text = std::fs::read_to_string(&path)
                .map_err(|e| format!("read {}: {e}", path.display()))?;
            let parsed = json::parse(&text).map_err(|e| format!("{}: {e}", path.display()))?;
            let label = dir.file_name().map_or_else(
                || dir.display().to_string(),
                |n| n.to_string_lossy().into_owned(),
            );
            Ok((label, parsed))
        })
        .collect::<Result<_, String>>()?;
    print!("{}", report::merge_markdown(&runs)?);
    Ok(0)
}
