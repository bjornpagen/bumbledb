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
        },
        config: RunConfig {
            scale: "S",
            seed: 1,
            samples: 256,
        },
        corpus_digest: "cafe".to_owned(),
        verify_stamp: "beef".to_owned(),
        budget_gates: false,
        partial: false,
        reads: vec![ReadFamilyReport {
            name: "point".to_owned(),
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
            exec: Some(ExecDigest {
                worst_estimate_factor: 1.0,
                covers: "n0:t0x256".to_owned(),
                emits: 256,
            }),
            p99_within_budget: true,
            ghz: None,
            p50_norm: None,
        }],
        writes: vec![WriteFamilyReport {
            name: "commit_single".to_owned(),
            ours: stats(100_000),
            theirs: Some(stats(120_000)),
            facts_per_sec: None,
            ghz: None,
        }],
        store: StoreNumbers {
            db_bytes: 1024,
            sqlite_bytes: 2048,
            cache_images: 3,
            cache_bytes: 4096,
        },
        flames: vec![],
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
- config: scale S, seed 1, 256 samples
- corpus digest: `cafe`
- verify stamp: `beef`

## Gate verdict

ALL-WIN — every gated read family beats SQLite on p50.
p99 budget (<= 10 ms warm): PASS (informational below scale L).

## Read families

| family | ours p50/p95/p99 (us) | sqlite p50/p95/p99 (us) | ratio | verdict |
|---|---|---|---|---|
| point | 10.0 / 30.0 / 40.0 | 20.0 / 60.0 / 80.0 | 0.50 | WIN |

## Write families

| family | ours p50 (us) | sqlite p50 (us) | facts/sec |
|---|---|---|---|
| commit_single | 100.0 | 120.0 | - |

## Allocations

| family | allocs | deallocs | alloc bytes | dealloc bytes |
|---|---|---|---|---|
| point | 0 | 0 | 0 | 0 |

## Execution digests

| family | worst est/actual | covers | emits |
|---|---|---|---|
| point | 1.00 | n0:t0x256 | 256 |

## Store

- bumbledb file (compacted): 1024 bytes
- sqlite file: 2048 bytes
- image cache: 3 images, 4096 bytes

## Flame summaries

(none captured — run with --trace)
";
    assert_eq!(to_markdown(&fixture()), expected);

    // The losing render names the family and fails the gate.
    let mut failing = fixture();
    failing.reads[0].verdict = Verdict::Loss;
    let md = to_markdown(&failing);
    assert!(md.contains("FAIL — losing families: point."), "{md}");
    assert!(!failing.all_win());

    // A filtered run withholds the claim, whatever the families did.
    let mut filtered = fixture();
    filtered.partial = true;
    let md = to_markdown(&filtered);
    assert!(md.contains("PARTIAL — filtered run"), "{md}");
    assert!(!md.contains("ALL-WIN — every gated"), "{md}");
}

#[test]
fn the_json_is_structurally_sound() {
    let text = to_json(&fixture());
    assert_eq!(text.matches('{').count(), text.matches('}').count());
    assert_eq!(text.matches('[').count(), text.matches(']').count());
    for key in [
        "\"provenance\":",
        "\"config\":",
        "\"corpus_digest\":\"cafe\"",
        "\"verify_stamp\":\"beef\"",
        "\"all_win\":true",
        "\"budget_ok\":true",
        "\"partial\":false",
        "\"reads\":[",
        "\"writes\":[",
        "\"ratio_p50\":0.5000",
        "\"verdict\":\"WIN\"",
        "\"facts_per_sec\":null",
        "\"store\":{\"db_bytes\":1024",
        "\"flames\":[]",
    ] {
        assert!(text.contains(key), "missing {key} in {text}");
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
        text.contains("\"ghz\":{\"pre\":3.450,\"post\":3.410,\"retried\":false,\"contaminated\":false}"),
        "{text}"
    );

    // The merge: two runs; the second's point block is contaminated,
    // so the min must come from the first even though the second is
    // numerically lower.
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
        ("run1".to_owned(), json::parse(&to_json(&stamped)).expect("parses")),
        ("run2".to_owned(), json::parse(&to_json(&second)).expect("parses")),
    ];
    let merged = merge_markdown(&runs).expect("merges");
    assert!(
        merged.contains("| point | 10.0 | ~~5.0~~ | 10.0 | 30.0 |"),
        "{merged}"
    );
    assert!(
        merged.contains("excluded from the minima"),
        "{merged}"
    );
    // commit_single is contaminated in BOTH runs: no clean minima.
    assert!(
        merged.contains("| commit_single | ~~100.0~~ | ~~100.0~~ | - | - |"),
        "{merged}"
    );
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
