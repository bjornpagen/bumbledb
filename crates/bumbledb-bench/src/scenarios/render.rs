use super::{geomean, QueryReport};
use crate::harness::Protocol;

#[expect(
    clippy::cast_precision_loss,
    reason = "reporting accepts lossy integer-to-float conversion"
)]
fn us(ns: u64) -> f64 {
    ns as f64 / 1000.0
}

/// Renders the scenario report as markdown.
#[must_use]
pub fn render(reports: &[QueryReport], proto: Protocol) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "# Scenario benchmarks\n");
    let _ = writeln!(
        out,
        "Report-class measurements over non-ledger worlds; every query \
         oracle-gated (value-identical results on both engines) before \
         timing. Protocol: {} warmups, {} samples, medians; `SQLite` \
         file-backed WAL `synchronous=FULL`, fully indexed, prepared \
         statements reused, ANALYZE run. ratio = ours/theirs (lower is \
         better; <1 = bumbledb faster).\n",
        proto.warmups, proto.samples,
    );
    let mut scenario = "";
    for r in reports {
        if r.scenario != scenario {
            scenario = r.scenario;
            let in_scenario: Vec<&QueryReport> =
                reports.iter().filter(|q| q.scenario == scenario).collect();
            let _ = writeln!(
                out,
                "\n## {scenario} (geomean ratio {:.2})\n",
                geomean(&in_scenario)
            );
            let _ = writeln!(
                out,
                "| query | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |"
            );
            let _ = writeln!(out, "|---|---:|---:|---:|---:|---|");
        }
        let _ = writeln!(
            out,
            "| {} | {} | {:.1} | {:.1} | {:.2} | {} |",
            r.name,
            r.rows,
            us(r.ours.p50),
            us(r.theirs.p50),
            r.ratio_p50,
            r.about,
        );
    }
    let every: Vec<&QueryReport> = reports.iter().collect();
    let _ = writeln!(
        out,
        "\nOverall geomean ratio across {} queries: **{:.2}**.",
        every.len(),
        geomean(&every)
    );
    out
}
