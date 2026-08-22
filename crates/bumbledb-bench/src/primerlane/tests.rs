use std::path::PathBuf;

use super::{PrimerConfig, corpus, run};
use crate::cli::PrimerlaneArgs;
use crate::json::Value;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("bumbledb-primerlane-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

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
    assert!(out.join("primerlane-report.md").exists());
    let _ = std::fs::remove_dir_all(&dir);
}

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
