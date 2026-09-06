use std::fmt::Write as _;

use crate::json;

struct MergeRow {
    p50: f64,
    p95: f64,
    contaminated: bool,
    is_read: bool,
    batch: Option<u32>,
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
                    is_read: key == "reads",
                    // Legacy reports never recorded batch. Preserve unknown;
                    // absence, null, zero, and malformed values do not mean one.
                    batch: family
                        .get("batch")
                        .and_then(json::Value::as_f64)
                        .and_then(|batch| batch.to_string().parse().ok())
                        .filter(|batch| *batch > 0),
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

type RunRows = (String, Vec<(String, MergeRow)>);

fn render_read_protocol(out: &mut String, per_run: &[RunRows], order: &[String]) {
    let _ = writeln!(out, "## Read sampling protocol\n");
    let _ = write!(out, "| family |");
    for (label, _) in per_run {
        let _ = write!(out, " {label} batch |");
    }
    let _ = writeln!(out);
    let _ = write!(out, "|---|");
    for _ in per_run {
        let _ = write!(out, "---|");
    }
    let _ = writeln!(out);
    for name in order {
        if !per_run
            .iter()
            .any(|(_, rows)| rows.iter().any(|(n, row)| n == name && row.is_read))
        {
            continue;
        }
        let _ = write!(out, "| {name} |");
        for (_, rows) in per_run {
            let row = rows.iter().find(|(n, row)| n == name && row.is_read);
            match row {
                Some((
                    _,
                    MergeRow {
                        batch: Some(batch), ..
                    },
                )) => {
                    let _ = write!(out, " {batch} |");
                }
                Some(_) => {
                    let _ = write!(out, " unknown |");
                }
                None => {
                    let _ = write!(out, " - |");
                }
            }
        }
        let _ = writeln!(out);
    }
    let _ = writeln!(
        out,
        "\nRead minima require the same known batch in every present run. \
         Legacy reports without batch remain unknown. Batch > 1 quantiles \
         summarize per-operation batch averages, not individual-call tails.\n"
    );
}

/// # Errors
pub fn merge_markdown(runs: &[(String, json::Value)]) -> Result<String, String> {
    let stores: Vec<(&str, &str)> = runs
        .iter()
        .map(|(label, parsed)| {
            let store = parsed
                .get("config")
                .and_then(|c| c.get("store"))
                .and_then(json::Value::as_str)
                .ok_or_else(|| format!("{label}: report.json carries no config.store label"))?;
            Ok((label.as_str(), store))
        })
        .collect::<Result<_, String>>()?;
    if let Some(&(label, store)) = stores.iter().find(|(_, store)| *store != stores[0].1) {
        return Err(format!(
            "merge refuses mixed durability: {} is `{}` but {label} is `{store}` — \
             a min column across sync levels labels nothing",
            stores[0].0, stores[0].1
        ));
    }
    let mut out = String::new();
    let _ = writeln!(out, "# bumbledb bench merge ({} runs)\n", runs.len());
    let per_run: Vec<RunRows> = runs
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

    let order: Vec<String> = per_run[0].1.iter().map(|(name, _)| name.clone()).collect();
    let mut excluded = 0usize;

    render_read_protocol(&mut out, &per_run, &order);

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
        let batches: Vec<_> = per_run
            .iter()
            .flat_map(|(_, rows)| rows)
            .filter(|(n, row)| n == name && row.is_read)
            .map(|(_, row)| row.batch)
            .collect();
        let comparable = batches
            .iter()
            .all(|batch| batch.is_some() && *batch == batches[0]);
        for (_, rows) in &per_run {
            match rows.iter().find(|(n, _)| n == name) {
                Some((_, row)) if row.contaminated => {
                    excluded += 1;
                    let _ = write!(out, " ~~{:.1}~~ |", row.p50 / 1000.0);
                }
                Some((_, row)) => {
                    if comparable {
                        min_p50 = min_p50.min(row.p50);
                        min_p95 = min_p95.min(row.p95);
                    }
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
