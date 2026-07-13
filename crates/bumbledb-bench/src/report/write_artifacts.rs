use std::path::Path;

use super::{RunReport, to_json, to_markdown};

use crate::families;

/// Writes exactly three artifacts into `out_dir`: `report.md`,
/// `report.json`, and `QUERIES.md` (the versioned query list from the
/// family registry). The tool never writes into `docs/` — publishing a
/// run is a human copy.
///
/// # Errors
///
/// I/O errors verbatim.
pub fn write_artifacts(report: &RunReport, out_dir: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(out_dir)?;
    std::fs::write(out_dir.join("report.md"), to_markdown(report))?;
    std::fs::write(out_dir.join("report.json"), to_json(report))?;
    std::fs::write(out_dir.join("QUERIES.md"), families::render_queries_md())
}
