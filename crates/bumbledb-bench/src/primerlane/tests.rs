use std::path::PathBuf;

use bumbledb::obs::{TraceArgs, TraceEvent, names};

use super::{PrimerConfig, components, corpus, run};
use crate::cli::PrimerlaneArgs;
use crate::json::Value;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bumbledb-primerlane-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// The tiny end-to-end pass: both write lanes and the scan read lane on
/// 500 facts, the artifact parsed back and its phase roster pinned.
#[test]
fn tiny_run_emits_the_phase_table() {
    let dir = scratch("tiny");
    let out = dir.join("out");
    let code = run(&PrimerlaneArgs {
        facts: 500,
        relations: 4,
        seed: 1,
        dir: dir.clone(),
        trace: false,
        alloc: false,
        out: Some(out.clone()),
    })
    .expect("tiny primerlane runs");
    assert_eq!(code, 0);
    let raw = std::fs::read_to_string(out.join("primerlane-report.json")).expect("artifact");
    let parsed = crate::json::parse(&raw).expect("valid JSON");
    let names: Vec<&str> = parsed
        .get("phases")
        .and_then(Value::as_arr)
        .expect("phases")
        .iter()
        .filter_map(|row| row.get("name").and_then(Value::as_str))
        .collect();
    assert_eq!(
        names,
        [
            "builder_load",
            "builder_admit",
            "builder_publish",
            "delta_create",
            "delta_seed",
            "delta_write",
            "scan_decode",
        ]
    );
    for row in parsed
        .get("phases")
        .and_then(Value::as_arr)
        .expect("phases")
    {
        assert!(
            row.get("wall_ns").is_some(),
            "every phase carries wall time"
        );
        assert!(row.get("rows").is_some(), "every phase carries a row count");
    }
    let telemetry = parsed.get("telemetry").expect("telemetry");
    for key in ["a", "i", "r", "f", "j"] {
        assert!(telemetry.get(key).is_some(), "{key}");
    }
    // An untraced run has no component fold.
    assert!(parsed.get("components").is_none());
    assert!(out.join("primerlane-report.md").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

/// The generator is a pure function of the config: identical config ⇒
/// identical rows, and the row counts respect the skew floor.
#[test]
fn corpus_is_deterministic_and_floored() {
    let cfg = PrimerConfig {
        relations: 12,
        facts: 500,
        seed: 7,
    };
    let counts = corpus::relation_rows(&cfg);
    assert_eq!(counts.len(), 12);
    assert!(counts.iter().all(|&n| n >= 2), "{counts:?}");
    let rel = bumbledb::RelationId(3);
    assert_eq!(
        corpus::row(&cfg, &counts, rel, 17),
        corpus::row(&cfg, &counts, rel, 17)
    );
}

/// The component fold registers every accumulation-side LEAF point of
/// 10-measurement.md — the names the 20/30 lanes wire land in a table
/// that already has their rows — and NO container: `BUILDER_LOAD`
/// encloses the `DYN_PARSE`/`DYN_ENCODE`/`DELTA_APPLY` leaves on the
/// builder lane, so mapping it into any row would sum a leaf with its
/// parent (the containment law, `components.rs`).
#[test]
fn the_component_table_names_the_new_points() {
    for point in [
        names::MARSHAL_FACTS,
        names::DYN_PARSE,
        names::DYN_ENCODE,
        names::DELTA_APPLY,
        names::INTERN_PROBE,
        names::PUBLISH_COPY,
        names::PUBLISH_SYNC,
    ] {
        assert!(
            components::COMPONENTS
                .iter()
                .any(|(_, points)| points.contains(&point)),
            "{point} is unmapped"
        );
    }
    assert!(
        !components::COMPONENTS
            .iter()
            .any(|(_, points)| points.contains(&names::BUILDER_LOAD)),
        "BUILDER_LOAD is a container — folding it double-counts its leaves"
    );
    let rows = components::totals(&[]);
    assert_eq!(rows.len(), components::COMPONENTS.len());
    assert!(rows.iter().all(|row| row.calls == 0 && row.total_ns == 0));
}

/// The double-counting pin: a builder-lane fixture whose `BUILDER_LOAD`
/// span ENCLOSES the `DYN_PARSE`/`DYN_ENCODE`/`DELTA_APPLY` leaves —
/// exactly the nesting `observed_load` produces. Each component row must
/// fold its leaf's duration ALONE (the pre-fix fold summed `DELTA_APPLY`
/// with `BUILDER_LOAD` in the 'delta apply' row: 3 µs of apply reported
/// as 13 µs, and the parse/encode microseconds charged twice across the
/// table), and the rows must stay non-overlapping — their sum bounded by
/// the one enclosing span.
#[test]
fn the_component_fold_never_sums_a_leaf_with_its_container() {
    let events = [
        TraceEvent::Span {
            point: names::BUILDER_LOAD,
            start_ns: 1_000,
            dur_ns: 10_000,
            args: TraceArgs::Count(3),
        },
        TraceEvent::Span {
            point: names::DYN_PARSE,
            start_ns: 2_000,
            dur_ns: 2_000,
            args: TraceArgs::Count(3),
        },
        TraceEvent::Span {
            point: names::DYN_ENCODE,
            start_ns: 4_500,
            dur_ns: 2_000,
            args: TraceArgs::Count(3),
        },
        TraceEvent::Span {
            point: names::DELTA_APPLY,
            start_ns: 7_000,
            dur_ns: 3_000,
            args: TraceArgs::Count(3),
        },
    ];
    let rows = components::totals(&events);
    let row = |name: &str| {
        rows.iter()
            .find(|row| row.name == name)
            .expect("component row")
    };
    assert_eq!(row("native batch parsing").total_ns, 2_000);
    assert_eq!(row("string ownership and interning").total_ns, 2_000);
    assert_eq!(row("delta apply").total_ns, 3_000, "the leaf alone");
    assert_eq!(row("delta apply").calls, 1);
    let sum: u64 = rows.iter().map(|row| row.total_ns).sum();
    assert!(
        sum <= 10_000,
        "non-overlapping leaves never exceed their container: {sum}"
    );
}
