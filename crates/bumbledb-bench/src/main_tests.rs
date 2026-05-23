use super::*;

#[test]
fn exact_correctness_helpers_catch_count_value_mismatch() {
    let bumbledb = sorted_sql_facts(vec![vec![SqlValue::Integer(2)]]);
    let sqlite = sorted_sql_facts(vec![vec![SqlValue::Integer(3)]]);

    assert_ne!(bumbledb, sqlite);
}

#[test]
fn exact_correctness_helpers_catch_projection_duplicates() {
    let bumbledb = sorted_sql_facts(vec![vec![SqlValue::Integer(1)]]);
    let sqlite = sorted_sql_facts(vec![vec![SqlValue::Integer(1)], vec![SqlValue::Integer(1)]]);

    assert_ne!(bumbledb, sqlite);
}

#[test]
fn markdown_renderer_emits_gate_tables() {
    let sample_stats = TimingStats::from_samples(vec![
        Duration::from_micros(9),
        Duration::from_micros(10),
        Duration::from_micros(11),
    ]);
    let result = BenchmarkRunResult {
        dataset: "joinstress",
        query: "triangle_count",
        facts: 1,
        correctness_mode: "result-set".to_owned(),
        bumbledb_correctness_execution: Duration::from_micros(20),
        sqlite_correctness_execution: Duration::from_micros(12),
        bumbledb_cold_execution: Duration::from_micros(20),
        sqlite_cold_execution: Duration::from_micros(12),
        allocation_scope: "bumbledb.correctness_execution".to_owned(),
        query_image_scope: "full_schema".to_owned(),
        bumbledb_warmup: TimingStats::from_samples(vec![Duration::from_micros(13)]),
        sqlite_warmup: TimingStats::from_samples(vec![Duration::from_micros(8)]),
        bumbledb_samples: sample_stats,
        sqlite_samples: sample_stats,
        bumbledb_avg: Duration::from_micros(10),
        sqlite_avg: Duration::from_micros(5),
        sqlite_ratio: 2.0,
        query_image_sample_cache_hits: 1,
        sqlite_materialized_facts: false,
        timings: QueryTimings {
            total_micros: 10,
            execute_micros: 4,
            sink_finish_micros: 1,
            unaccounted_micros: 5,
            ..QueryTimings::default()
        },
        allocations: QueryAllocationStats::default(),
        materialized_values: 1,
        dictionary_reverse_lookups: 0,
        counters: PlanCounters {
            output_facts: 1,
            materialized_output_values: 1,
            ..PlanCounters::default()
        },
        final_output_values: 1,
        output_contains_dictionary_values: false,
        query_image_build_micros: 3,
        query_image_built_during_query: true,
        query_image_cache_cached_images: 1,
        query_image_cache_hits: 1,
        query_image_cache_misses: 1,
        query_image_cache_builds: 1,
        query_image_cache_build_micros: 3,
        planner_stats_cached_relations: 1,
        planner_stats_hits: 2,
        planner_stats_misses: 1,
        planner_stats_builds: 1,
        planner_stats_build_micros: 9,
        lftj_lazy_access_slices: 0,
        lftj_eager_builds_avoided: 0,
        query_image_relation_count: 1,
        query_image_fact_count: 3,
        query_image_encoded_column_bytes: 128,
        gate: GateOutcome {
            passed: true,
            notes: vec!["ok".to_owned()],
        },
    };

    let markdown = render_markdown_results(&[result]);
    assert!(markdown.contains("| joinstress | triangle_count |"));
    assert!(markdown.contains("## Phase Timing"));
    assert!(markdown.contains("unaccounted us"));
    assert!(markdown.contains("## Mechanics Counters"));
    assert!(markdown.contains("sink emits"));
    assert!(markdown.contains("## Cache Diagnostics"));
    assert!(markdown.contains("| joinstress | triangle_count | 1 |"));
    assert!(markdown.contains("## Measurement Contract"));
    assert!(markdown.contains("bumbledb.correctness_execution"));
    assert!(markdown.contains("## Allocation Summary"));
    assert!(markdown.contains("## Allocation Phase Detail"));
    assert!(markdown.contains("## Distribution"));
    assert!(markdown.contains("| dataset | query | cursor seeks |"));
    assert!(markdown.contains("| joinstress | triangle_count | 0 | 0 | 1 | 1 | false | 0 | ok |"));
}

#[test]
fn json_renderer_emits_structured_results() {
    let result = BenchmarkRunResult {
        dataset: "ledger",
        query: "tag_lookup_join",
        facts: 2,
        correctness_mode: "result-set".to_owned(),
        bumbledb_correctness_execution: Duration::from_micros(21),
        sqlite_correctness_execution: Duration::from_micros(10),
        bumbledb_cold_execution: Duration::from_micros(20),
        sqlite_cold_execution: Duration::from_micros(10),
        allocation_scope: "bumbledb.correctness_execution".to_owned(),
        query_image_scope: "full_schema".to_owned(),
        bumbledb_warmup: TimingStats::from_samples(vec![Duration::from_micros(11)]),
        sqlite_warmup: TimingStats::from_samples(vec![Duration::from_micros(7)]),
        bumbledb_samples: TimingStats::from_samples(vec![Duration::from_micros(9)]),
        sqlite_samples: TimingStats::from_samples(vec![Duration::from_micros(3)]),
        bumbledb_avg: Duration::from_micros(9),
        sqlite_avg: Duration::from_micros(3),
        sqlite_ratio: 3.0,
        query_image_sample_cache_hits: 1,
        sqlite_materialized_facts: false,
        timings: QueryTimings {
            total_micros: 20,
            unaccounted_micros: 7,
            ..QueryTimings::default()
        },
        allocations: QueryAllocationStats::default(),
        materialized_values: 2,
        dictionary_reverse_lookups: 0,
        counters: PlanCounters::default(),
        final_output_values: 2,
        output_contains_dictionary_values: false,
        query_image_build_micros: 1,
        query_image_built_during_query: true,
        query_image_cache_cached_images: 1,
        query_image_cache_hits: 1,
        query_image_cache_misses: 0,
        query_image_cache_builds: 1,
        query_image_cache_build_micros: 1,
        planner_stats_cached_relations: 1,
        planner_stats_hits: 1,
        planner_stats_misses: 0,
        planner_stats_builds: 1,
        planner_stats_build_micros: 1,
        lftj_lazy_access_slices: 0,
        lftj_eager_builds_avoided: 0,
        query_image_relation_count: 1,
        query_image_fact_count: 2,
        query_image_encoded_column_bytes: 1,
        gate: GateOutcome {
            passed: true,
            notes: vec!["ok".to_owned()],
        },
    };

    let json = render_json_results(&[result]);
    assert!(json.contains("\"dataset\":\"ledger\""));
    assert!(json.contains("\"correctness_mode\":\"result-set\""));
    assert!(json.contains("\"query_image_cache_hit\":true"));
    assert!(json.contains("\"allocation_scope\":\"bumbledb.correctness_execution\""));
    assert!(json.contains("\"query_image_scope\":\"full_schema\""));
    assert!(json.contains("\"correctness_execution\""));
    assert!(json.contains("\"cold_execution\""));
    assert!(!json.contains("\"prepare\""));
    assert!(json.contains("\"query_image_built_during_query\":true"));
    assert!(json.contains("\"phase_timing\""));
    assert!(json.contains("\"unaccounted_us\":7"));
    assert!(json.contains("\"sink_emit_calls\""));
    assert!(json.contains("\"encoded_project_facts_seen\""));
    assert!(json.contains("\"lftj_next_calls\""));
    assert!(json.contains("\"lftj_eager_builds_avoided\""));
    assert!(json.contains("\"allocations\""));
    assert!(json.contains("\"phases\""));
    assert!(json.contains("\"size_class_allocs\""));
}

#[test]
fn cli_parser_accepts_repeated_query_filters() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_args(
        [
            "--dataset",
            "ledger",
            "--query",
            "tag_lookup_join",
            "--query",
            "balances_by_instrument",
            "--warmup",
            "2",
            "--open-limit",
            "123",
            "--job-dir",
            "/tmp/job",
            "--format",
            "json",
        ]
        .into_iter()
        .map(str::to_owned),
    )?
    .ok_or_else(|| bench_error("expected config"))?;

    assert_eq!(config.datasets, vec!["ledger"]);
    assert_eq!(
        config.queries,
        vec!["tag_lookup_join", "balances_by_instrument"]
    );
    assert_eq!(config.open_limit, Some(123));
    assert_eq!(config.warmup, 2);
    assert_eq!(config.job_dir.as_deref(), Some("/tmp/job"));
    assert!(config.has_open_datasets());
    assert_eq!(config.format, OutputFormat::Json);
    Ok(())
}

#[test]
fn cli_preset_job_sample_is_obvious() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_args(
        ["--preset", "job-sample", "--job-dir", "/tmp/job"]
            .into_iter()
            .map(str::to_owned),
    )?
    .ok_or_else(|| bench_error("expected config"))?;

    assert_eq!(config.datasets, vec!["job"]);
    assert_eq!(config.open_limit, Some(DEFAULT_OPEN_LIMIT));
    assert_eq!(config.job_dir.as_deref(), Some("/tmp/job"));
    assert_eq!(config.repeats, 30);
    assert_eq!(config.warmup, 2);
    Ok(())
}

#[test]
fn cli_preset_job_full_is_explicit() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_args(
        ["--preset", "job-full", "--job-dir", "/tmp/job"]
            .into_iter()
            .map(str::to_owned),
    )?
    .ok_or_else(|| bench_error("expected config"))?;

    assert_eq!(config.datasets, vec!["job"]);
    assert_eq!(config.open_limit, None);
    assert_eq!(config.job_dir.as_deref(), Some("/tmp/job"));
    Ok(())
}

#[test]
fn cli_open_full_overrides_default_limit() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_args(["--open-full"].into_iter().map(str::to_owned))?
        .ok_or_else(|| bench_error("expected config"))?;

    assert_eq!(config.open_limit, None);
    Ok(())
}

#[test]
fn cli_parser_rejects_invalid_numbers() {
    let result = Config::from_args(["--repeats", "nope"].into_iter().map(str::to_owned));
    assert!(result.is_err());
}

#[test]
fn output_format_both_includes_json() {
    assert!(OutputFormat::Both.includes_markdown());
    assert!(OutputFormat::Both.includes_json());
}

#[test]
fn trace_scripts_exist() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(std::path::Path::parent)
        .ok_or_else(|| bench_error("workspace root missing"))?;
    assert!(root.join("scripts/bench-trace-nonjob.sh").is_file());
    assert!(root.join("scripts/summarize-trace-jsonl.sh").is_file());
    Ok(())
}

#[test]
fn cli_parser_accepts_trace_output_without_default_subscriber()
-> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_args(
        ["--trace-output", "trace.log", "--trace-format", "json"]
            .into_iter()
            .map(str::to_owned),
    )?
    .ok_or_else(|| bench_error("expected config"))?;

    assert!(config.trace);
    assert_eq!(config.trace_output.as_deref(), Some("trace.log"));
    assert_eq!(config.trace_format, TraceFormat::Json);
    Ok(())
}

#[cfg(feature = "alloc-profile")]
#[test]
fn allocation_profile_records_known_vector() {
    let before = bumbledb_lmdb::allocation::snapshot();
    let values = vec![42u8; 4096];
    black_box(&values);
    let after = bumbledb_lmdb::allocation::snapshot();
    let delta = bumbledb_lmdb::allocation::delta(before, after);

    assert!(delta.enabled);
    assert!(delta.alloc_calls > 0);
    assert!(delta.bytes_allocated >= 4096);
}

#[test]
fn focused_gate_definitions_are_present() {
    assert!(benchmark_gate("joinstress", "triangle_count").is_some());
    assert!(benchmark_gate("ledger", "tag_lookup_join").is_some());
    assert!(benchmark_gate("sailors", "red_boat_sailors").is_some());
    assert!(benchmark_gate("tpch", "supplier_nation_orders").is_some());
    assert!(benchmark_gate("job", "job_q09_voice_us_actor").is_some());
    assert!(benchmark_gate("job", "job_q24_voice_keyword_actor").is_some());
    assert!(benchmark_gate("ledger", "unknown").is_none());
    assert_eq!(
        benchmark_gate("job", "job_q09_voice_us_actor")
            .map(|gate| (gate.max_bumbledb_avg_micros, gate.max_sqlite_ratio,)),
        Some((Some(3_000), Some(1.0)))
    );
    assert_eq!(
        benchmark_gate("job", "job_q24_voice_keyword_actor")
            .map(|gate| (gate.max_bumbledb_avg_micros, gate.max_sqlite_ratio,)),
        Some((Some(1_000), Some(1.0)))
    );
}

#[test]
fn duration_ratio_handles_zero_sqlite_time() {
    assert!(duration_ratio(Duration::from_micros(1), Duration::ZERO).is_infinite());
}
