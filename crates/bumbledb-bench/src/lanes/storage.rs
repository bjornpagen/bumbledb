//! The storage lane: on-disk bytes per corpus scale, both engines —
//! REPORT-class ([`crate::lanes`] carries the charter). This packet
//! lands the report schema and the refusal shell; the lane body lands
//! in MET-2-storage-lane, which owns this file afterward and keeps the
//! shape-pin test in sync with any schema extension.

use std::fmt::Write as _;

use crate::json;
use crate::report::Provenance;

/// The whole storage report, plain data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorageReport {
    pub provenance: Provenance,
    pub seed: u64,
    pub scales: Vec<ScaleStorage>,
    pub churn: Vec<ChurnRow>,
}

/// One corpus scale's worlds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScaleStorage {
    pub scale: &'static str,
    pub worlds: Vec<WorldStorage>,
}

/// One world's byte accounting at one scale, both engines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldStorage {
    pub world: &'static str,
    pub facts: u64,
    pub engine_raw_bytes: u64,
    pub engine_compacted_bytes: u64,
    pub sqlite_indexed_bytes: u64,
    pub sqlite_indexed_wal_bytes: u64,
    pub sqlite_tableonly_bytes: u64,
    pub sqlite_tableonly_wal_bytes: u64,
}

/// One churn-ladder step's post-state bytes (`None` = not measured).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChurnRow {
    pub name: String,
    pub engine_bytes: Option<u64>,
    pub sqlite_bytes: Option<u64>,
    pub sqlite_wal_bytes: Option<u64>,
}

fn push_world(out: &mut String, world: &WorldStorage) {
    let _ = write!(
        out,
        "{{\"world\":\"{}\",\"facts\":{},\"engine_raw_bytes\":{},\"engine_compacted_bytes\":{},\"engine_bytes_per_fact\":{:.4},\"sqlite_indexed_bytes\":{},\"sqlite_indexed_wal_bytes\":{},\"sqlite_indexed_bytes_per_fact\":{:.4},\"sqlite_tableonly_bytes\":{},\"sqlite_tableonly_wal_bytes\":{},\"sqlite_tableonly_bytes_per_fact\":{:.4}}}",
        world.world,
        world.facts,
        world.engine_raw_bytes,
        world.engine_compacted_bytes,
        super::per_unit(world.engine_compacted_bytes, world.facts),
        world.sqlite_indexed_bytes,
        world.sqlite_indexed_wal_bytes,
        super::per_unit(world.sqlite_indexed_bytes, world.facts),
        world.sqlite_tableonly_bytes,
        world.sqlite_tableonly_wal_bytes,
        super::per_unit(world.sqlite_tableonly_bytes, world.facts),
    );
}

fn push_opt_u64(out: &mut String, value: Option<u64>) {
    match value {
        Some(v) => {
            let _ = write!(out, "{v}");
        }
        None => out.push_str("null"),
    }
}

fn push_churn(out: &mut String, row: &ChurnRow) {
    out.push_str("{\"name\":");
    json::push_str_lit(out, &row.name);
    out.push_str(",\"engine_bytes\":");
    push_opt_u64(out, row.engine_bytes);
    out.push_str(",\"sqlite_bytes\":");
    push_opt_u64(out, row.sqlite_bytes);
    out.push_str(",\"sqlite_wal_bytes\":");
    push_opt_u64(out, row.sqlite_wal_bytes);
    out.push('}');
}

/// The machine-consumable storage artifact — hand-rolled, like
/// `report/json_out.rs`.
#[must_use]
pub fn to_json(report: &StorageReport) -> String {
    let mut out = String::new();
    out.push_str("{\"provenance\":");
    super::push_provenance(&mut out, &report.provenance);
    let _ = write!(out, ",\"seed\":{},\"scales\":[", report.seed);
    for (index, scale) in report.scales.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        let _ = write!(out, "{{\"scale\":\"{}\",\"worlds\":[", scale.scale);
        for (world_index, world) in scale.worlds.iter().enumerate() {
            if world_index > 0 {
                out.push(',');
            }
            push_world(&mut out, world);
        }
        out.push_str("]}");
    }
    out.push_str("],\"churn\":[");
    for (index, row) in report.churn.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        push_churn(&mut out, row);
    }
    out.push_str("]}");
    out
}

/// The storage lane entry point.
///
/// # Errors
///
/// Always, for now: the lane body lands in MET-2-storage-lane; until
/// then this is the typed refusal naming that packet.
pub fn run(_args: &crate::cli::StorageArgs) -> Result<i32, String> {
    Err("the storage lane lands in MET-2-storage-lane; this subcommand refuses until then"
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

    #[test]
    fn report_json_shape_is_pinned() {
        let report = StorageReport {
            provenance: provenance(),
            seed: 7,
            scales: vec![ScaleStorage {
                scale: "S",
                worlds: vec![WorldStorage {
                    world: "ledger",
                    facts: 1000,
                    engine_raw_bytes: 4000,
                    engine_compacted_bytes: 2000,
                    sqlite_indexed_bytes: 8000,
                    sqlite_indexed_wal_bytes: 128,
                    sqlite_tableonly_bytes: 3000,
                    sqlite_tableonly_wal_bytes: 64,
                }],
            }],
            churn: vec![ChurnRow {
                name: "delete-half".to_owned(),
                engine_bytes: Some(1500),
                sqlite_bytes: None,
                sqlite_wal_bytes: None,
            }],
        };
        let parsed = crate::json::parse(&to_json(&report)).expect("valid JSON");
        assert_eq!(
            parsed
                .get("provenance")
                .and_then(|p| p.get("git_rev"))
                .and_then(Value::as_str),
            Some("deadbeef")
        );
        assert_eq!(parsed.get("seed").and_then(Value::as_f64), Some(7.0));
        let scales = parsed.get("scales").and_then(Value::as_arr).expect("scales");
        assert_eq!(scales.len(), 1);
        assert_eq!(scales[0].get("scale").and_then(Value::as_str), Some("S"));
        let worlds = scales[0].get("worlds").and_then(Value::as_arr).expect("worlds");
        let world = &worlds[0];
        assert_eq!(world.get("world").and_then(Value::as_str), Some("ledger"));
        assert_eq!(world.get("facts").and_then(Value::as_f64), Some(1000.0));
        assert_eq!(
            world.get("engine_raw_bytes").and_then(Value::as_f64),
            Some(4000.0)
        );
        assert_eq!(
            world.get("engine_compacted_bytes").and_then(Value::as_f64),
            Some(2000.0)
        );
        // The derived per-fact columns: compacted/facts and friends.
        assert_eq!(
            world.get("engine_bytes_per_fact").and_then(Value::as_f64),
            Some(2.0)
        );
        assert_eq!(
            world
                .get("sqlite_indexed_bytes_per_fact")
                .and_then(Value::as_f64),
            Some(8.0)
        );
        assert_eq!(
            world
                .get("sqlite_tableonly_bytes_per_fact")
                .and_then(Value::as_f64),
            Some(3.0)
        );
        assert_eq!(
            world
                .get("sqlite_indexed_wal_bytes")
                .and_then(Value::as_f64),
            Some(128.0)
        );
        assert_eq!(
            world
                .get("sqlite_tableonly_wal_bytes")
                .and_then(Value::as_f64),
            Some(64.0)
        );
        let churn = parsed.get("churn").and_then(Value::as_arr).expect("churn");
        assert_eq!(churn[0].get("name").and_then(Value::as_str), Some("delete-half"));
        assert_eq!(
            churn[0].get("engine_bytes").and_then(Value::as_f64),
            Some(1500.0)
        );
        assert_eq!(churn[0].get("sqlite_bytes"), Some(&Value::Null));
        assert_eq!(churn[0].get("sqlite_wal_bytes"), Some(&Value::Null));
    }

    #[test]
    fn run_refuses_naming_the_landing_packet() {
        let err = run(&crate::cli::StorageArgs::default()).unwrap_err();
        assert!(err.contains("MET-2"), "{err}");
    }
}
