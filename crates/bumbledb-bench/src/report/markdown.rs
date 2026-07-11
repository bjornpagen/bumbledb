use std::fmt::Write as _;

use super::{GhzReport, RunReport, Verdict};

#[allow(clippy::cast_precision_loss)]
fn us(ns: u64) -> f64 {
    ns as f64 / 1000.0
}

fn markdown_header(out: &mut String, report: &RunReport) {
    let _ = writeln!(out, "# bumbledb bench report\n");
    let _ = writeln!(out, "## Provenance\n");
    let p = &report.provenance;
    let _ = writeln!(out, "- crate version: {}", p.crate_version);
    let _ = writeln!(out, "- engine rev: {}", p.git_rev);
    let _ = writeln!(out, "- timestamp: {}", p.timestamp);
    let _ = writeln!(out, "- host: {}", p.host);
    let _ = writeln!(
        out,
        "- config: scale {}, seed {}, {} samples",
        report.config.scale, report.config.seed, report.config.samples
    );
    let _ = writeln!(out, "- corpus digest: `{}`", report.corpus_digest);
    let _ = writeln!(out, "- verify stamp: `{}`\n", report.verify_stamp);

    let _ = writeln!(out, "## Gate verdict\n");
    if report.partial {
        let _ = writeln!(
            out,
            "PARTIAL — filtered run; the ALL-WIN claim needs every family."
        );
    } else if report.all_win() {
        let _ = writeln!(
            out,
            "ALL-WIN — every gated read family beats SQLite on p50."
        );
    } else {
        let losing: Vec<&str> = report
            .reads
            .iter()
            .filter(|family| family.verdict == Verdict::Loss)
            .map(|family| family.name.as_str())
            .collect();
        let _ = writeln!(out, "FAIL — losing families: {}.", losing.join(", "));
    }
    let budget = if report.budget_ok() { "PASS" } else { "FAIL" };
    let scope = if report.budget_gates {
        "gating at scale L"
    } else {
        "informational below scale L"
    };
    let _ = writeln!(out, "p99 budget (<= 10 ms warm): {budget} ({scope}).");
    let dirty = report.contaminated_families();
    if dirty.is_empty() {
        let _ = writeln!(out);
    } else {
        let _ = writeln!(
            out,
            "clock proxy: {} block(s) still contaminated after retry — treat their \
             percentiles as dirty: {}.\n",
            dirty.len(),
            dirty.join(", ")
        );
    }
}

fn markdown_family_tables(out: &mut String, report: &RunReport) {
    let _ = writeln!(out, "## Read families\n");
    let _ = writeln!(
        out,
        "| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|");
    for family in &report.reads {
        let _ = writeln!(
            out,
            "| {} | {:.1} / {:.1} / {:.1} | {:.1} / {:.1} / {:.1} | {:.2} | {} |",
            family.name,
            us(family.ours.p50),
            us(family.ours.p95),
            us(family.ours.p99),
            us(family.theirs.p50),
            us(family.theirs.p95),
            us(family.theirs.p99),
            family.ratio_p50,
            family.verdict.label(),
        );
    }
    let _ = writeln!(out);

    // The elision delta, named (docs/architecture/60-validation.md § the
    // calendar benchmark): the DU whole-read measured with the
    // rule-disjointness proof on and forced off — the delta is the
    // elision's number.
    let on = report.reads.iter().find(|f| f.name == "rsvp_union");
    let off = report.reads.iter().find(|f| f.name == "rsvp_union_off");
    if let (Some(on), Some(off)) = (on, off) {
        #[allow(clippy::cast_precision_loss)]
        let delta_pct =
            (off.ours.p50 as f64 - on.ours.p50 as f64) / on.ours.p50.max(1) as f64 * 100.0;
        let _ = writeln!(
            out,
            "elision delta (rsvp_union): proof on {:.1} us, forced off {:.1} us \
             ({delta_pct:+.1}% p50).\n",
            us(on.ours.p50),
            us(off.ours.p50),
        );
    }

    let _ = writeln!(out, "## Write families\n");
    let _ = writeln!(
        out,
        "| family | ours p50 (us) | sqlite p50 (us) | facts/sec |"
    );
    let _ = writeln!(out, "|---|---|---|---|");
    for family in &report.writes {
        let theirs = family
            .theirs
            .map_or_else(|| "-".to_owned(), |stats| format!("{:.1}", us(stats.p50)));
        let throughput = family
            .facts_per_sec
            .map_or_else(|| "-".to_owned(), |v| format!("{v:.0}"));
        let _ = writeln!(
            out,
            "| {} | {:.1} | {theirs} | {throughput} |",
            family.name,
            us(family.ours.p50),
        );
    }
    let _ = writeln!(out);
}

fn markdown_diagnostics(out: &mut String, report: &RunReport) {
    let _ = writeln!(out, "## Allocations\n");
    let mut any_window = false;
    for family in &report.reads {
        let Some(alloc) = family.alloc else { continue };
        if !any_window {
            let _ = writeln!(
                out,
                "| family | allocs | deallocs | alloc bytes | dealloc bytes |"
            );
            let _ = writeln!(out, "|---|---|---|---|---|");
            any_window = true;
        }
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} |",
            family.name, alloc.allocs, alloc.deallocs, alloc.alloc_bytes, alloc.dealloc_bytes,
        );
    }
    if any_window {
        let _ = writeln!(out);
    } else {
        let _ = writeln!(out, "(not captured — run with the alloc window)\n");
    }

    let _ = writeln!(out, "## Execution digests\n");
    let _ = writeln!(out, "| family | worst est/actual | covers | emits |");
    let _ = writeln!(out, "|---|---|---|---|");
    for family in &report.reads {
        if let Some(exec) = &family.exec {
            let _ = writeln!(
                out,
                "| {} | {:.2} | {} | {} |",
                family.name, exec.worst_estimate_factor, exec.covers, exec.emits,
            );
        }
    }
    let _ = writeln!(out);

    let _ = writeln!(out, "## Store\n");
    let _ = writeln!(
        out,
        "- bumbledb file (compacted): {} bytes",
        report.store.db_bytes
    );
    let _ = writeln!(out, "- sqlite file: {} bytes", report.store.sqlite_bytes);
    let _ = writeln!(
        out,
        "- image cache: {} images, {} bytes\n",
        report.store.cache_images, report.store.cache_bytes
    );

    let stamped: Vec<(&str, GhzReport, Option<u64>)> = report
        .reads
        .iter()
        .map(|f| (f.name.as_str(), f.ghz, f.p50_norm))
        .chain(report.writes.iter().map(|f| (f.name.as_str(), f.ghz, None)))
        .filter_map(|(name, ghz, norm)| ghz.map(|g| (name, g, norm)))
        .collect();
    if !stamped.is_empty() {
        let _ = writeln!(out, "## Clock proxy\n");
        let _ = writeln!(
            out,
            "| family | GHz pre | GHz post | status | norm p50 (us) |"
        );
        let _ = writeln!(out, "|---|---|---|---|---|");
        for (name, ghz, norm) in &stamped {
            let norm = norm.map_or_else(|| "-".to_owned(), |n| format!("{:.1}", us(n)));
            let _ = writeln!(
                out,
                "| {name} | {:.2} | {:.2} | {} | {norm} |",
                ghz.pre,
                ghz.post,
                ghz.status(),
            );
        }
        let _ = writeln!(out);
    }

    let _ = writeln!(out, "## Flame summaries\n");
    if report.flames.is_empty() {
        let _ = writeln!(out, "(none captured — run with --trace)");
    } else {
        for flame in &report.flames {
            let _ = writeln!(out, "### {}\n", flame.name);
            let _ = writeln!(out, "```text\n{}```\n", flame.table);
        }
    }
}

/// The markdown artifact.
#[must_use]
pub fn to_markdown(report: &RunReport) -> String {
    let mut out = String::new();
    markdown_header(&mut out, report);
    markdown_family_tables(&mut out, report);
    markdown_diagnostics(&mut out, report);
    out
}
