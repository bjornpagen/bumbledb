use super::{LaneOutcome, QueryReport, dnf_count, geomean};
use crate::harness::Protocol;

#[expect(
    clippy::cast_precision_loss,
    reason = "reporting accepts lossy integer-to-float conversion"
)]
fn us(ns: u64) -> f64 {
    ns as f64 / 1000.0
}

#[must_use]
#[expect(
    clippy::too_many_lines,
    reason = "one linear report: the lane table then the optional alloc/flame sections"
)]
pub fn render(reports: &[QueryReport], proto: Protocol) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "# Scenario benchmarks\n");
    let _ = writeln!(
        out,
        "Report-class measurements over non-ledger worlds; every query \
         oracle-gated (value-identical results on both engines, every \
         `SQLite` lane, never under a cap) before timing. Adversarial \
         lanes run under a per-sample wall-clock cap (`SQLite`'s progress \
         handler): a lane that trips it reports `DNF>cap` with NO \
         percentiles — excluded from geomeans and counted. Protocol: {} \
         warmups, {} samples, medians; `SQLite` file-backed WAL \
         `synchronous=FULL`, fully indexed, prepared statements reused, \
         ANALYZE run. ratio = ours/theirs (lower is better; <1 = bumbledb \
         faster).\n",
        proto.warmups, proto.samples,
    );
    let mut scenario = "";
    for r in reports {
        if r.scenario != scenario {
            scenario = r.scenario;
            let in_scenario: Vec<&QueryReport> =
                reports.iter().filter(|q| q.scenario == scenario).collect();
            let timed = in_scenario
                .iter()
                .filter(|q| q.primary_ratio().is_some())
                .count();
            let dnf = dnf_count(&in_scenario);
            let dnf_clause = if dnf > 0 {
                format!(", {dnf} DNF > cap — excluded and counted")
            } else {
                String::new()
            };
            let _ = writeln!(
                out,
                "\n## {scenario} (geomean ratio {:.2} over {timed} timed{dnf_clause})\n",
                geomean(&in_scenario)
            );
            let _ = writeln!(
                out,
                "| query | lane | rows | ours p50 (us) | sqlite p50 (us) | ratio | regime |"
            );
            let _ = writeln!(out, "|---|---|---:|---:|---:|---:|---|");
        }
        for lane in &r.lanes {
            match &lane.outcome {
                LaneOutcome::Timed { stats, ratio_p50 } => {
                    let _ = writeln!(
                        out,
                        "| {} | {} | {} | {:.1} | {:.1} | {:.2} | {} |",
                        r.name,
                        lane.lane,
                        r.answers,
                        us(r.ours.p50),
                        us(stats.p50),
                        ratio_p50,
                        r.about,
                    );
                }
                LaneOutcome::ExceededCap { cap } => {
                    let _ = writeln!(
                        out,
                        "| {} | {} | {} | {:.1} | DNF>{}ms | — | {} |",
                        r.name,
                        lane.lane,
                        r.answers,
                        us(r.ours.p50),
                        cap.0,
                        r.about,
                    );
                }
            }
        }
    }
    let every: Vec<&QueryReport> = reports.iter().collect();
    let dnf_lanes = reports
        .iter()
        .flat_map(|r| r.lanes.iter())
        .filter(|lane| matches!(lane.outcome, LaneOutcome::ExceededCap { .. }))
        .count();
    if dnf_lanes > 0 {
        let _ = writeln!(
            out,
            "\nOverall geomean ratio across {} queries: **{:.2}**; {dnf_lanes} lane(s) \
             DNF > cap (excluded, counted).",
            every.len(),
            geomean(&every)
        );
    } else {
        let _ = writeln!(
            out,
            "\nOverall geomean ratio across {} queries: **{:.2}**.",
            every.len(),
            geomean(&every)
        );
    }

    if reports.iter().any(|r| r.alloc.is_some()) {
        let _ = writeln!(out, "\n## Allocations (per query, --alloc)\n");
        let _ = writeln!(
            out,
            "| query | allocs | alloc bytes | deallocs | dealloc bytes |"
        );
        let _ = writeln!(out, "|---|---:|---:|---:|---:|");
        for r in reports {
            if let Some(a) = &r.alloc {
                let _ = writeln!(
                    out,
                    "| {} | {} | {} | {} | {} |",
                    r.name, a.allocs, a.alloc_bytes, a.deallocs, a.dealloc_bytes,
                );
            }
        }
    }

    if reports.iter().any(|r| r.flame.is_some()) {
        let _ = writeln!(out, "\n## Flame summaries (per query, --trace)\n");
        for r in reports {
            if let Some(flame) = &r.flame {
                let _ = writeln!(out, "### {} / {}\n", r.scenario, r.name);
                let _ = writeln!(out, "```text\n{flame}```\n");
            }
        }
    }
    out
}
