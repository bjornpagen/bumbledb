use super::*;

fn stats(p50: u64) -> Stats {
    Stats {
        min: p50 / 2,
        p50,
        p90: p50 * 2,
        p95: p50 * 3,
        p99: p50 * 4,
        max: p50 * 5,
        mean_ns: p50,
    }
}

fn fixture() -> RunReport {
    RunReport {
        provenance: Provenance {
            crate_version: "0.1.0".to_owned(),
            git_rev: "unknown".to_owned(),
            timestamp: "2026-01-01T00:00:00Z".to_owned(),
            host: "test-host".to_owned(),
            shared: None,
        },
        config: RunConfig {
            scale: "S",
            seed: 1,
            samples: 256,
            store: "durable",
        },
        corpus_digest: "cafe".to_owned(),
        verify_stamp: "beef".to_owned(),
        budget_gates: false,
        partial: false,
        reads: vec![ReadFamilyReport {
            name: "point".to_owned(),
            batch: 1,
            ours: stats(10_000),
            theirs: stats(20_000),
            ratio_p50: 0.5,
            verdict: Verdict::Win,
            alloc: Some(AllocReport {
                allocs: 0,
                deallocs: 0,
                alloc_bytes: 0,
                dealloc_bytes: 0,
            }),
            p99_within_budget: true,
            ghz: None,
            ghz_ours: None,
            ghz_theirs: None,
            p50_norm: None,
        }],
        writes: vec![WriteFamilyReport {
            name: "commit_single".to_owned(),
            ours: stats(100_000),
            theirs: Some(stats(120_000)),
            facts_per_sec: None,
            ghz: None,
            ghz_ours: None,
            ghz_theirs: None,
        }],
        store: StoreNumbers {
            db_bytes: 1024,
            sqlite_bytes: 2048,
        },
    }
}

#[test]
fn the_markdown_is_golden() {
    let expected = "\
# bumbledb bench report

## Provenance

- crate version: 0.1.0
- engine rev: unknown
- timestamp: 2026-01-01T00:00:00Z
- host: test-host
- config: scale S, seed 1, 256 samples, durable stores
- corpus digest: `cafe`
- verify stamp: `beef`

## Gate verdict

ALL-WIN — every gated read family beats SQLite on p50.
p99 budget (<= 10 ms warm): PASS (informational below scale L).

## Read families

Batch is operations per timed sample, shared by both engines. For batch > 1, quantiles (including the p99 budget) describe per-operation batch averages, not individual-call tails. Displacement runs between batches.

| family | batch | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|---|
| point | 1 | 10.0 / 30.0 / 40.0 | 20.0 / 60.0 / 80.0 | 0.50 | WIN |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 100.0 | 120.0 | - |

## Allocations

| family | allocs | deallocs | alloc bytes | dealloc bytes |
|---|---|---|---|---|
| point | 0 | 0 | 0 | 0 |

## Store

- bumbledb file (compacted): 1024 bytes
- sqlite file: 2048 bytes

";
    assert_eq!(to_markdown(&fixture()), expected);

    let mut failing = fixture();
    failing.reads[0].verdict = Verdict::Loss;
    let md = to_markdown(&failing);
    assert!(md.contains("FAIL — losing families: point."), "{md}");
    assert!(!failing.all_win());

    let mut filtered = fixture();
    filtered.partial = true;
    let md = to_markdown(&filtered);
    assert!(md.contains("PARTIAL — filtered run"), "{md}");
    assert!(!md.contains("ALL-WIN — every gated"), "{md}");
}

#[test]
fn the_json_is_structurally_sound() {
    let text = to_json(&fixture());
    crate::json::parse(&text).expect("report must parse, not merely balance delimiters");
    for key in [
        "\"provenance\":",
        "\"config\":",
        "\"corpus_digest\":\"cafe\"",
        "\"verify_stamp\":\"beef\"",
        "\"all_win\":true",
        "\"budget_ok\":true",
        "\"partial\":false",
        "\"reads\":[",
        "\"batch\":1",
        "\"writes\":[",
        "\"ratio_p50\":0.5000",
        "\"verdict\":\"WIN\"",
        "\"facts_per_sec\":null",
        "\"store\":{\"db_bytes\":1024",
    ] {
        assert!(text.contains(key), "missing {key} in {text}");
    }

    assert!(!text.contains("shared_machine"), "{text}");
}

#[test]
fn a_boosted_run_stamps_shared_machine_provenance() {
    let mut boosted = fixture();
    boosted.provenance.shared = Some(SharedMachine {
        boost: "qos-user-interactive",
        load_start: [1.25, 2.5, 3.75],
        load_end: [4.0, 5.0, 6.0],
    });
    let text = to_json(&boosted);
    for key in [
        "\"shared_machine\":true",
        "\"boost\":\"qos-user-interactive\"",
        "\"load_start\":[1.25,2.50,3.75]",
        "\"load_end\":[4.00,5.00,6.00]",
    ] {
        assert!(text.contains(key), "missing {key} in {text}");
    }
    let md = to_markdown(&boosted);
    assert!(
        md.contains("- shared machine: boost qos-user-interactive"),
        "{md}"
    );
    assert!(md.contains("1.25 2.50 3.75 (start)"), "{md}");
}

#[test]
fn the_shared_stamp_bridges_the_engaged_boost() {
    assert_eq!(provenance::shared_stamp(None), None);
    let engaged = crate::boost::Engaged {
        boost: "qos-user-interactive",
        load_start: [1.0, 2.0, 3.0],
    };
    let shared = provenance::shared_stamp(Some(engaged)).expect("an engaged boost stamps");
    assert_eq!(shared.boost, "qos-user-interactive");
    for (slot, expected) in shared.load_start.iter().zip([1.0, 2.0, 3.0]) {
        assert!((slot - expected).abs() < f64::EPSILON, "start {slot}");
    }

    for slot in shared.load_end {
        assert!(
            slot >= 0.0 || (slot + 1.0).abs() < f64::EPSILON,
            "end slot {slot} is neither a sample nor the marker"
        );
    }
}

#[test]
fn ghz_stamps_render_in_markdown_json_and_the_merge_excludes_dirt() {
    let mut stamped = fixture();
    stamped.reads[0].ghz = Some(GhzReport {
        pre: 3.45,
        post: 3.41,
        retried: false,
        contaminated: false,
    });
    stamped.writes[0].ghz = Some(GhzReport {
        pre: 2.41,
        post: 3.44,
        retried: true,
        contaminated: true,
    });
    let md = to_markdown(&stamped);
    assert!(md.contains("## Clock proxy"), "{md}");
    assert!(md.contains("| point | 3.45 | 3.41 | clean |"), "{md}");
    assert!(
        md.contains("| commit_single | 2.41 | 3.44 | CONTAMINATED |"),
        "{md}"
    );
    assert!(
        md.contains("clock proxy: 1 block(s) still contaminated after retry"),
        "{md}"
    );
    let text = to_json(&stamped);
    assert!(
        text.contains(
            "\"ghz\":{\"pre\":3.450,\"post\":3.410,\"retried\":false,\"contaminated\":false}"
        ),
        "{text}"
    );

    let mut second = stamped.clone();
    second.reads[0].ours.p50 = 5_000;
    second.reads[0].ours.p95 = 6_000;
    second.reads[0].ghz = Some(GhzReport {
        pre: 2.0,
        post: 2.0,
        retried: true,
        contaminated: true,
    });
    let runs = vec![
        (
            "run1".to_owned(),
            json::parse(&to_json(&stamped)).expect("parses"),
        ),
        (
            "run2".to_owned(),
            json::parse(&to_json(&second)).expect("parses"),
        ),
    ];
    let merged = merge_markdown(&runs).expect("merges");
    assert!(
        merged.contains("| point | 10.0 | ~~5.0~~ | 10.0 | 30.0 |"),
        "{merged}"
    );
    assert!(merged.contains("excluded from the minima"), "{merged}");

    assert!(
        merged.contains("| commit_single | ~~100.0~~ | ~~100.0~~ | - | - |"),
        "{merged}"
    );
}

#[test]
fn read_batch_is_reported_and_merges_never_invent_legacy_protocols() {
    let mut batched = fixture();
    batched.reads[0].batch = 16;
    let text = to_json(&batched);
    assert!(text.contains("\"batch\":16"), "{text}");
    assert!(to_markdown(&batched).contains("| point | 16 |"));
    let runs = [
        ("a".to_owned(), json::parse(&text).expect("parses")),
        ("b".to_owned(), json::parse(&text).expect("parses")),
    ];
    let merged = merge_markdown(&runs).expect("same batch merges");
    assert!(merged.contains("| point | 16 | 16 |"), "{merged}");
    assert!(
        merged.contains("| point | 10.0 | 10.0 | 10.0 | 30.0 |"),
        "{merged}"
    );

    for replacement in [
        "",
        "\"batch\":null,",
        "\"batch\":0,",
        "\"batch\":1.5,",
        "\"batch\":-1,",
        "\"batch\":1,",
    ] {
        let legacy = text.replace("\"batch\":16,", replacement);
        let runs = [
            ("a".to_owned(), json::parse(&text).expect("parses")),
            ("b".to_owned(), json::parse(&legacy).expect("parses")),
        ];
        let merged = merge_markdown(&runs).expect("keeps incomparable rows visible");
        assert!(
            merged.contains("| point | 10.0 | 10.0 | - | - |"),
            "{merged}"
        );
        let expected = if replacement == "\"batch\":1," {
            "1"
        } else {
            "unknown"
        };
        assert!(
            merged.contains(&format!("| point | 16 | {expected} |")),
            "{merged}"
        );
        assert!(
            merged.contains("| commit_single | 100.0 | 100.0 | 100.0 | 300.0 |"),
            "{merged}"
        );
    }
    let legacy = text.replace("\"batch\":16,", "");
    let runs = [
        ("a".to_owned(), json::parse(&legacy).expect("parses")),
        ("b".to_owned(), json::parse(&legacy).expect("parses")),
    ];
    let merged = merge_markdown(&runs).expect("old reports remain readable");
    assert!(merged.contains("| point | unknown | unknown |"), "{merged}");
    assert!(
        merged.contains("| point | 10.0 | 10.0 | - | - |"),
        "{merged}"
    );
}

#[test]
fn the_merge_refuses_mixed_durability_labels() {
    let mut ephemeral = fixture();
    ephemeral.config.store = "ephemeral";
    let runs = vec![
        (
            "run1".to_owned(),
            json::parse(&to_json(&fixture())).expect("parses"),
        ),
        (
            "run2".to_owned(),
            json::parse(&to_json(&ephemeral)).expect("parses"),
        ),
    ];
    let err = merge_markdown(&runs).expect_err("mixed durability refuses");
    assert!(err.contains("merge refuses mixed durability"), "{err}");
    assert!(
        err.contains("run1 is `durable` but run2 is `ephemeral`"),
        "{err}"
    );

    let unlabeled = json::parse(r#"{"config":{"scale":"S"}}"#).expect("parses");
    let err = merge_markdown(&[("bare".to_owned(), unlabeled)]).expect_err("no label refuses");
    assert_eq!(err, "bare: report.json carries no config.store label");
}

#[test]
fn verdict_and_budget_logic_is_table_tested() {
    assert_eq!(verdict(Kind::Gate, 10, 11), Verdict::Win);
    assert_eq!(verdict(Kind::Gate, 10, 10), Verdict::Loss, "a tie loses");
    assert_eq!(verdict(Kind::Gate, 11, 10), Verdict::Loss);
    assert_eq!(verdict(Kind::Report, 1, 100), Verdict::ReportOnly);
    assert!(within_budget(P99_BUDGET_NS), "the boundary passes on <=");
    assert!(!within_budget(P99_BUDGET_NS + 1));
}

#[test]
fn write_artifacts_creates_exactly_the_three_files() {
    let dir = std::env::temp_dir().join("bumbledb-bench-report");
    let _ = std::fs::remove_dir_all(&dir);
    write_artifacts(&fixture(), &dir).expect("writes");
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .expect("read dir")
        .map(|entry| {
            entry
                .expect("entry")
                .file_name()
                .into_string()
                .expect("utf-8")
        })
        .collect();
    names.sort();
    assert_eq!(names, ["QUERIES.md", "report.json", "report.md"]);
    let queries = std::fs::read_to_string(dir.join("QUERIES.md")).expect("read");
    assert_eq!(queries, families::render_queries_md());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_timestamp_formatter_matches_known_epochs() {
    assert_eq!(civil(0), "1970-01-01T00:00:00Z");
    assert_eq!(civil(86_399), "1970-01-01T23:59:59Z");
    // 2026-07-01T12:30:05Z.
    assert_eq!(civil(1_782_909_005), "2026-07-01T12:30:05Z");
}

#[test]
fn engine_clock_attribution_serializes_without_reclassifying_legacy_reports() {
    use crate::clockproxy::GhzStamp;

    let legacy = fixture();
    let legacy_json = to_json(&legacy);
    assert!(!legacy_json.contains("ghz_ours"));
    assert!(!legacy_json.contains("ghz_theirs"));
    assert!(!to_markdown(&legacy).contains("## Per-engine clock proxy"));

    let outer = GhzStamp {
        pre: 3.4,
        post: 3.0,
        retried: false,
        threshold: 3.2,
    };
    let (ours, theirs) = outer.split_at(3.4);
    let mut combined_only = legacy.clone();
    combined_only.reads[0].ghz = Some(ours.merge(theirs).into());
    combined_only.writes[0].ghz = Some(outer.into());
    let mut attributed = combined_only.clone();
    attributed.reads[0].ghz_ours = Some(ours.into());
    attributed.reads[0].ghz_theirs = Some(theirs.into());
    attributed.writes[0].ghz_ours = Some(ours.into());
    attributed.writes[0].ghz_theirs = Some(theirs.into());
    let text = to_json(&attributed);
    assert!(text.contains(
        "\"ghz_ours\":{\"pre\":3.400,\"post\":3.400,\"retried\":false,\"contaminated\":false}"
    ));
    assert!(text.contains(
        "\"ghz_theirs\":{\"pre\":3.400,\"post\":3.000,\"retried\":false,\"contaminated\":true}"
    ));
    let md = to_markdown(&attributed);
    assert!(md.contains("| commit_single | bumbledb | 3.40 | 3.40 | clean |"));
    assert!(md.contains("| commit_single | SQLite | 3.40 | 3.00 | CONTAMINATED |"));
    // The existing merge keeps its historical combined-flag policy.
    // New attribution is additional evidence, not a rewrite of old runs.
    let merge = |report: &RunReport| {
        merge_markdown(&[("run".into(), json::parse(&to_json(report)).unwrap())]).unwrap()
    };
    assert_eq!(merge(&combined_only), merge(&attributed));

    // Engine-only writes have a measured ours bracket, no invented oracle.
    attributed.writes[0].theirs = None;
    attributed.writes[0].ghz_theirs = None;
    let parsed = json::parse(&to_json(&attributed)).unwrap();
    let write = &parsed.get("writes").unwrap().as_arr().unwrap()[0];
    assert!(write.get("ghz_ours").is_some());
    assert!(write.get("ghz_theirs").is_none());
}
