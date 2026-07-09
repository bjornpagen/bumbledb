use std::fmt::Write as _;

use crate::json;

/// One family's numbers pulled out of a parsed `report.json`.
struct MergeRow {
    p50: f64,
    p95: f64,
    contaminated: bool,
}

fn merge_rows(parsed: &json::Value, key: &str) -> Vec<(String, MergeRow)> {
    let Some(families) = parsed.get(key).and_then(json::Value::as_arr) else {
        return Vec::new();
    };
    families
        .iter()
        .filter_map(|family| {
            let name = family.get("name")?.as_str()?.to_owned();
            let ours = family.get("ours")?;
            Some((
                name,
                MergeRow {
                    p50: ours.get("p50")?.as_f64()?,
                    p95: ours.get("p95")?.as_f64()?,
                    contaminated: family
                        .get("ghz")
                        .and_then(|g| g.get("contaminated"))
                        .and_then(json::Value::as_bool)
                        .unwrap_or(false),
                },
            ))
        })
        .collect()
}

/// The cross-run merge: N
/// parsed `report.json` documents → one markdown table per family with
/// each run's p50 and the min-of-runs p50/p95. Blocks whose clock-proxy
/// bracket stayed contaminated are excluded from the minima, and the
/// exclusion count is printed.
///
/// # Errors
///
/// A run with no readable read families.
pub fn merge_markdown(runs: &[(String, json::Value)]) -> Result<String, String> {
    let mut out = String::new();
    let _ = writeln!(out, "# bumbledb bench merge ({} runs)\n", runs.len());
    let per_run: Vec<(String, Vec<(String, MergeRow)>)> = runs
        .iter()
        .map(|(label, parsed)| {
            let mut rows = merge_rows(parsed, "reads");
            rows.extend(merge_rows(parsed, "writes"));
            if rows.is_empty() {
                return Err(format!("{label}: no families in report.json"));
            }
            Ok((label.clone(), rows))
        })
        .collect::<Result<_, String>>()?;

    // Family order follows the first run; families missing from a run
    // render as `-`.
    let order: Vec<String> = per_run[0].1.iter().map(|(name, _)| name.clone()).collect();
    let mut excluded = 0usize;

    let _ = write!(out, "| family |");
    for (label, _) in &per_run {
        let _ = write!(out, " {label} p50 (us) |");
    }
    let _ = writeln!(out, " min p50 (us) | min p95 (us) |");
    let _ = write!(out, "|---|");
    for _ in &per_run {
        let _ = write!(out, "---|");
    }
    let _ = writeln!(out, "---|---|");

    for name in &order {
        let _ = write!(out, "| {name} |");
        let mut min_p50 = f64::INFINITY;
        let mut min_p95 = f64::INFINITY;
        for (_, rows) in &per_run {
            match rows.iter().find(|(n, _)| n == name) {
                Some((_, row)) if row.contaminated => {
                    excluded += 1;
                    let _ = write!(out, " ~~{:.1}~~ |", row.p50 / 1000.0);
                }
                Some((_, row)) => {
                    min_p50 = min_p50.min(row.p50);
                    min_p95 = min_p95.min(row.p95);
                    let _ = write!(out, " {:.1} |", row.p50 / 1000.0);
                }
                None => {
                    let _ = write!(out, " - |");
                }
            }
        }
        if min_p50.is_finite() {
            let _ = writeln!(out, " {:.1} | {:.1} |", min_p50 / 1000.0, min_p95 / 1000.0);
        } else {
            let _ = writeln!(out, " - | - |");
        }
    }
    let _ = writeln!(
        out,
        "\n{excluded} contaminated block(s) excluded from the minima."
    );
    Ok(out)
}
