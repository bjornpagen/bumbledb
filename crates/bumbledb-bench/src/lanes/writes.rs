//! The writes lane: write/commit/delete throughput ladder across
//! durability lanes — REPORT-class ([`crate::lanes`] carries the
//! charter). This packet lands the durability axis, the report schema,
//! and the refusal shell; the lane body lands in MET-3-writes-lane,
//! which owns this file afterward and keeps the shape-pin test in sync
//! with any schema extension.

use std::fmt::Write as _;

use crate::harness::Stats;
use crate::json;
use crate::report::{GhzReport, Provenance};

/// The engine's durability axis has exactly two points — `Db::create`
/// (durable) and `Db::ephemeral` (`MDB_NOSYNC`). The mandate's
/// "durable / NOSYNC / ephemeral" collapses to these two because
/// ephemeral IS the NOSYNC constructor (docs/architecture/70-api.md);
/// making the third label unrepresentable is the honest representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DurabilityLane {
    Durable,
    NoSync,
}

impl DurabilityLane {
    /// The lane's name, as reports and `--lanes` tokens spell it.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Durable => "durable",
            Self::NoSync => "nosync",
        }
    }

    /// The `SQLite` twin's documented parity config for this lane.
    #[must_use]
    pub fn sqlite_sync_label(self) -> &'static str {
        match self {
            Self::Durable => "wal+synchronous=FULL+fullfsync=ON",
            Self::NoSync => "wal+synchronous=OFF",
        }
    }

    /// The bench store constructor this lane maps to.
    #[must_use]
    pub fn store_mode(self) -> crate::storemode::StoreMode {
        match self {
            Self::Durable => crate::storemode::StoreMode::Durable,
            Self::NoSync => crate::storemode::StoreMode::Ephemeral,
        }
    }
}

/// The whole writes report, plain data.
#[derive(Debug, Clone, PartialEq)]
pub struct WritesReport {
    pub provenance: Provenance,
    pub scale: &'static str,
    pub seed: u64,
    pub samples: u32,
    pub lanes: Vec<LaneReport>,
}

/// One durability lane's ladder.
#[derive(Debug, Clone, PartialEq)]
pub struct LaneReport {
    pub lane: &'static str,
    pub sqlite_sync: &'static str,
    pub rows: Vec<WriteRow>,
}

/// One (family, batch) cell, both engines.
#[derive(Debug, Clone, PartialEq)]
pub struct WriteRow {
    pub name: String,
    pub batch: u32,
    pub ours: Stats,
    pub theirs: Stats,
    pub commits_per_sec_ours: f64,
    pub commits_per_sec_theirs: f64,
    pub rows_per_sec_ours: f64,
    pub rows_per_sec_theirs: f64,
    pub ghz: Option<GhzReport>,
}

fn push_row(out: &mut String, row: &WriteRow) {
    out.push_str("{\"name\":");
    json::push_str_lit(out, &row.name);
    let _ = write!(out, ",\"batch\":{},\"ours\":", row.batch);
    super::push_stats(out, &row.ours);
    out.push_str(",\"theirs\":");
    super::push_stats(out, &row.theirs);
    let _ = write!(
        out,
        ",\"commits_per_sec_ours\":{:.2},\"commits_per_sec_theirs\":{:.2},\"rows_per_sec_ours\":{:.2},\"rows_per_sec_theirs\":{:.2}",
        row.commits_per_sec_ours,
        row.commits_per_sec_theirs,
        row.rows_per_sec_ours,
        row.rows_per_sec_theirs,
    );
    super::push_ghz(out, row.ghz);
    out.push('}');
}

/// The machine-consumable writes artifact — hand-rolled, like
/// `report/json_out.rs`.
#[must_use]
pub fn to_json(report: &WritesReport) -> String {
    let mut out = String::new();
    out.push_str("{\"provenance\":");
    super::push_provenance(&mut out, &report.provenance);
    let _ = write!(
        out,
        ",\"scale\":\"{}\",\"seed\":{},\"samples\":{},\"lanes\":[",
        report.scale, report.seed, report.samples
    );
    for (index, lane) in report.lanes.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            "{{\"lane\":\"{}\",\"sqlite_sync\":\"{}\",\"rows\":[",
            lane.lane, lane.sqlite_sync
        );
        for (row_index, row) in lane.rows.iter().enumerate() {
            if row_index > 0 {
                out.push(',');
            }
            push_row(&mut out, row);
        }
        out.push_str("]}");
    }
    out.push_str("]}");
    out
}

/// The writes lane entry point.
///
/// # Errors
///
/// Always, for now: the lane body lands in MET-3-writes-lane; until
/// then this is the typed refusal naming that packet.
pub fn run(_args: &crate::cli::WritesArgs) -> Result<i32, String> {
    Err("the writes lane lands in MET-3-writes-lane; this subcommand refuses until then"
        .to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::json::Value;

    fn provenance() -> Provenance {
        Provenance {
            crate_version: "0.0.0-test".to_owned(),
            git_rev: "deadbeef".to_owned(),
            timestamp: "2026-07-19T00:00:00Z".to_owned(),
            host: "test-host".to_owned(),
        }
    }

    fn stats(base: u64) -> Stats {
        Stats {
            min: base,
            p50: base + 1,
            p90: base + 2,
            p95: base + 3,
            p99: base + 4,
            max: base + 5,
            mean_ns: base + 2,
        }
    }

    #[test]
    fn the_durability_axis_has_exactly_two_points() {
        assert_eq!(DurabilityLane::Durable.label(), "durable");
        assert_eq!(DurabilityLane::NoSync.label(), "nosync");
        assert_eq!(
            DurabilityLane::Durable.sqlite_sync_label(),
            "wal+synchronous=FULL+fullfsync=ON"
        );
        assert_eq!(
            DurabilityLane::NoSync.sqlite_sync_label(),
            "wal+synchronous=OFF"
        );
        assert_eq!(
            DurabilityLane::Durable.store_mode(),
            crate::storemode::StoreMode::Durable
        );
        assert_eq!(
            DurabilityLane::NoSync.store_mode(),
            crate::storemode::StoreMode::Ephemeral
        );
    }

    #[test]
    fn report_json_shape_is_pinned() {
        let report = WritesReport {
            provenance: provenance(),
            scale: "S",
            seed: 9,
            samples: 8,
            lanes: vec![LaneReport {
                lane: DurabilityLane::NoSync.label(),
                sqlite_sync: DurabilityLane::NoSync.sqlite_sync_label(),
                rows: vec![
                    WriteRow {
                        name: "append".to_owned(),
                        batch: 10,
                        ours: stats(100),
                        theirs: stats(200),
                        commits_per_sec_ours: 1234.25,
                        commits_per_sec_theirs: 617.5,
                        rows_per_sec_ours: 12342.5,
                        rows_per_sec_theirs: 6175.0,
                        ghz: Some(GhzReport {
                            pre: 3.5,
                            post: 3.25,
                            retried: false,
                            contaminated: false,
                        }),
                    },
                    WriteRow {
                        name: "delete".to_owned(),
                        batch: 1,
                        ours: stats(300),
                        theirs: stats(400),
                        commits_per_sec_ours: 100.5,
                        commits_per_sec_theirs: 50.25,
                        rows_per_sec_ours: 100.5,
                        rows_per_sec_theirs: 50.25,
                        ghz: None,
                    },
                ],
            }],
        };
        let parsed = crate::json::parse(&to_json(&report)).expect("valid JSON");
        assert_eq!(
            parsed
                .get("provenance")
                .and_then(|p| p.get("host"))
                .and_then(Value::as_str),
            Some("test-host")
        );
        assert_eq!(parsed.get("scale").and_then(Value::as_str), Some("S"));
        assert_eq!(parsed.get("seed").and_then(Value::as_f64), Some(9.0));
        assert_eq!(parsed.get("samples").and_then(Value::as_f64), Some(8.0));
        let lanes = parsed.get("lanes").and_then(Value::as_arr).expect("lanes");
        assert_eq!(lanes.len(), 1);
        assert_eq!(lanes[0].get("lane").and_then(Value::as_str), Some("nosync"));
        assert_eq!(
            lanes[0].get("sqlite_sync").and_then(Value::as_str),
            Some("wal+synchronous=OFF")
        );
        let rows = lanes[0].get("rows").and_then(Value::as_arr).expect("rows");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].get("name").and_then(Value::as_str), Some("append"));
        assert_eq!(rows[0].get("batch").and_then(Value::as_f64), Some(10.0));
        // Stats objects carry the exact report/json_out.rs shape.
        let ours = rows[0].get("ours").expect("ours");
        assert_eq!(ours.get("min").and_then(Value::as_f64), Some(100.0));
        assert_eq!(ours.get("p99").and_then(Value::as_f64), Some(104.0));
        assert_eq!(ours.get("mean_ns").and_then(Value::as_f64), Some(102.0));
        let theirs = rows[0].get("theirs").expect("theirs");
        assert_eq!(theirs.get("p50").and_then(Value::as_f64), Some(201.0));
        assert_eq!(
            rows[0].get("commits_per_sec_ours").and_then(Value::as_f64),
            Some(1234.25)
        );
        assert_eq!(
            rows[0]
                .get("commits_per_sec_theirs")
                .and_then(Value::as_f64),
            Some(617.5)
        );
        assert_eq!(
            rows[0].get("rows_per_sec_ours").and_then(Value::as_f64),
            Some(12342.5)
        );
        assert_eq!(
            rows[0].get("rows_per_sec_theirs").and_then(Value::as_f64),
            Some(6175.0)
        );
        // Ghz renders like push_ghz: present on row 0, null on row 1.
        let ghz = rows[0].get("ghz").expect("ghz");
        assert_eq!(ghz.get("pre").and_then(Value::as_f64), Some(3.5));
        assert_eq!(ghz.get("post").and_then(Value::as_f64), Some(3.25));
        assert_eq!(ghz.get("retried").and_then(Value::as_bool), Some(false));
        assert_eq!(rows[1].get("ghz"), Some(&Value::Null));
    }

    #[test]
    fn run_refuses_naming_the_landing_packet() {
        let err = run(&crate::cli::WritesArgs::default()).unwrap_err();
        assert!(err.contains("MET-3"), "{err}");
    }
}
