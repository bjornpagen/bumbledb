use std::fmt::Write as _;

use super::{RunReport, Verdict};

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
    if let Some(shared) = &p.shared {
        let _ = writeln!(out, "- shared machine: {}", shared.describe());
    }
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
    let _ = writeln!(out, "p99 budget (<= 10 ms warm): {budget} ({scope}).\n");
}

fn markdown_family_tables(out: &mut String, report: &RunReport) {
    let _ = writeln!(out, "## Read families\n");
    let _ = writeln!(
        out,
        "Batch is operations per timed sample, shared by both engines. For batch > 1, \
         quantiles (including the p99 budget) describe per-operation batch averages, \
         not individual-call tails. Displacement runs between batches.\n"
    );
    let _ = writeln!(
        out,
        "| family | batch | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |"
    );
    let _ = writeln!(out, "|---|---|---|---|---|---|");
    for family in &report.reads {
        let _ = writeln!(
            out,
            "| {} | {} | {:.1} / {:.1} / {:.1} | {:.1} / {:.1} / {:.1} | {:.2} | {} |",
            family.name,
            family.batch,
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

    let _ = writeln!(out, "## Store\n");
    let _ = writeln!(
        out,
        "- bumbledb file (compacted): {} bytes",
        report.store.db_bytes
    );
    let _ = writeln!(out, "- sqlite file: {} bytes\n", report.store.sqlite_bytes);
}

#[must_use]
pub fn to_markdown(report: &RunReport) -> String {
    let mut out = String::new();
    markdown_header(&mut out, report);
    markdown_family_tables(&mut out, report);
    markdown_diagnostics(&mut out, report);
    out
}
