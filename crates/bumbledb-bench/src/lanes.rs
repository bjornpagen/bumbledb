//! The metric lanes: three REPORT-class subcommands — `storage`,
//! `writes`, `curves`.
//!
//! The charter: each lane produces a report artifact and exits 0 on
//! success; non-zero only on refusal, setup failure, oracle
//! disagreement, or post-state mismatch. Gate-class membership is
//! structurally impossible: the lane reports are their own plain-data
//! types that never construct the budget-gated run-report type in
//! [`crate::report`], never join ALL-WIN, and never touch the verdict
//! or the p99 budget gates. Numbers are claimed only from the owner's
//! measurement sessions — this tool run never times for publication.

pub mod curves;
pub mod storage;
pub mod writes;

use std::fmt::Write as _;

use crate::harness::Stats;
use crate::json;
use crate::report::{GhzReport, Provenance};

/// Appends the provenance object (the same fields `report.json` pins).
pub(crate) fn push_provenance(out: &mut String, provenance: &Provenance) {
    out.push_str("{\"crate_version\":");
    json::push_str_lit(out, &provenance.crate_version);
    out.push_str(",\"git_rev\":");
    json::push_str_lit(out, &provenance.git_rev);
    out.push_str(",\"timestamp\":");
    json::push_str_lit(out, &provenance.timestamp);
    out.push_str(",\"host\":");
    json::push_str_lit(out, &provenance.host);
    out.push('}');
}

/// Appends one stats object — the exact `report/json_out.rs` shape.
pub(crate) fn push_stats(out: &mut String, stats: &Stats) {
    let _ = write!(
        out,
        "{{\"min\":{},\"p50\":{},\"p90\":{},\"p95\":{},\"p99\":{},\"max\":{},\"mean_ns\":{}}}",
        stats.min, stats.p50, stats.p90, stats.p95, stats.p99, stats.max, stats.mean_ns
    );
}

/// Appends a stats object or `null`.
pub(crate) fn push_opt_stats(out: &mut String, stats: Option<&Stats>) {
    match stats {
        Some(stats) => push_stats(out, stats),
        None => out.push_str("null"),
    }
}

/// Appends `,"ghz":{…}` or `,"ghz":null` — the exact
/// `report/json_out.rs` shape.
pub(crate) fn push_ghz(out: &mut String, ghz: Option<GhzReport>) {
    out.push_str(",\"ghz\":");
    match ghz {
        Some(g) => {
            let _ = write!(
                out,
                "{{\"pre\":{:.3},\"post\":{:.3},\"retried\":{},\"contaminated\":{}}}",
                g.pre, g.post, g.retried, g.contaminated
            );
        }
        None => out.push_str("null"),
    }
}

/// Bytes per unit as a report float; a zero denominator reads 0.0
/// (an empty world has no per-fact claim to make).
#[expect(
    clippy::cast_precision_loss,
    reason = "reporting accepts lossy integer-to-float conversion"
)]
pub(crate) fn per_unit(bytes: u64, count: u64) -> f64 {
    if count == 0 {
        0.0
    } else {
        bytes as f64 / count as f64
    }
}
