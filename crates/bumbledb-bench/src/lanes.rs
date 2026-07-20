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
use crate::report::GhzReport;

// The provenance object emitter is `report.json`'s own
// (`crate::report::push_provenance`) — one spelling, so the lane
// reports and the ledger report can never drift, shared-machine stamp
// included.
pub(crate) use crate::report::push_provenance;

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

#[cfg(test)]
mod tests {
    use crate::json::{self, Value};
    use crate::report::{Provenance, SharedMachine};

    /// The one provenance emitter every lane report shares: boost-on
    /// stamps `shared_machine`/`boost`/`load_start`/`load_end` (the
    /// owner's 2026-07-20 shared-machine ruling), boost-off emits the
    /// pre-boost block byte-identically.
    #[test]
    fn the_shared_machine_stamp_shape_is_pinned() {
        let base = Provenance {
            crate_version: "0.0.0-test".to_owned(),
            git_rev: "deadbeef".to_owned(),
            timestamp: "2026-07-19T00:00:00Z".to_owned(),
            host: "test-host".to_owned(),
            shared: None,
        };
        let mut off = String::new();
        super::push_provenance(&mut off, &base);
        assert_eq!(
            off,
            "{\"crate_version\":\"0.0.0-test\",\"git_rev\":\"deadbeef\",\
             \"timestamp\":\"2026-07-19T00:00:00Z\",\"host\":\"test-host\"}",
            "boost-off is the pre-boost block, byte for byte"
        );

        let boosted = Provenance {
            shared: Some(SharedMachine {
                boost: "qos-user-interactive",
                load_start: [1.25, 2.5, 3.75],
                load_end: [4.0, 5.0, 6.0],
            }),
            ..base
        };
        let mut on = String::new();
        super::push_provenance(&mut on, &boosted);
        let parsed = json::parse(&on).expect("valid JSON");
        assert_eq!(
            parsed.get("shared_machine").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            parsed.get("boost").and_then(Value::as_str),
            Some("qos-user-interactive")
        );
        let start = parsed
            .get("load_start")
            .and_then(Value::as_arr)
            .expect("load_start");
        assert_eq!(start.len(), 3);
        assert_eq!(start[0].as_f64(), Some(1.25));
        assert_eq!(start[2].as_f64(), Some(3.75));
        let end = parsed
            .get("load_end")
            .and_then(Value::as_arr)
            .expect("load_end");
        assert_eq!(end.len(), 3);
        assert_eq!(end[1].as_f64(), Some(5.0));
    }
}
