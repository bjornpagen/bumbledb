fn benchmark_result(
    dataset: &'static str,
    query: &BenchQuery,
    output: &QueryOutput,
    cache_hits: CacheHitStats,
    correctness_mode: CorrectnessMode,
    timing: QueryTimingSamples,
    query_image_stats: QueryImageBenchStats,
) -> BenchmarkRunResult {
    let final_output_values = (output.result.facts.len() * output.result.columns.len()) as u64;
    let output_contains_dictionary_values = output
        .result
        .facts
        .iter()
        .flatten()
        .any(|value| matches!(value, Value::String(_) | Value::Bytes(_)));
    let bumbledb_avg = timing.bumbledb_samples.avg;
    let sqlite_avg = timing.sqlite_samples.avg;
    let sqlite_ratio = duration_ratio(bumbledb_avg, sqlite_avg);
    let gate = evaluate_gate(
        dataset,
        query,
        output,
        bumbledb_avg,
        sqlite_ratio,
        final_output_values,
        output_contains_dictionary_values,
    );
    BenchmarkRunResult {
        dataset,
        query: query.name,
        facts: output.result.facts.len(),
        correctness_mode: correctness_mode.as_str().to_owned(),
        bumbledb_correctness_execution: timing.bumbledb_correctness_execution,
        sqlite_correctness_execution: timing.sqlite_correctness_execution,
        bumbledb_cold_execution: timing.bumbledb_cold_execution,
        sqlite_cold_execution: timing.sqlite_cold_execution,
        allocation_scope: "bumbledb.correctness_execution".to_owned(),
        query_image_scope: query_image_scope(output).to_owned(),
        bumbledb_warmup: timing.bumbledb_warmup,
        sqlite_warmup: timing.sqlite_warmup,
        bumbledb_samples: timing.bumbledb_samples,
        sqlite_samples: timing.sqlite_samples,
        bumbledb_avg,
        sqlite_avg,
        sqlite_ratio,
        chosen_plan: output.plan.optimizer.chosen.clone(),
        query_image_sample_cache_hits: cache_hits.query_image_cache_hits,
        sqlite_materialized_facts: true,
        timings: output.plan.timings,
        allocations: output.plan.allocations,
        iterator_ops: output.plan.free_join.estimates.iterator_ops,
        build_facts: output.plan.free_join.estimates.build_facts,
        materialized_values: output.plan.counters.materialized_output_values,
        dictionary_reverse_lookups: output.plan.counters.dictionary_reverse_lookups,
        counters: output.plan.counters.clone(),
        final_output_values,
        output_contains_dictionary_values,
        query_image_build_micros: query_image_stats.build_micros,
        query_image_built_during_query: output.plan.timings.query_image_micros > 0,
        query_image_cache_cached_images: output.plan.query_image_cache.cached_images,
        query_image_cache_hits: output.plan.query_image_cache.hits,
        query_image_cache_misses: output.plan.query_image_cache.misses,
        query_image_cache_builds: output.plan.query_image_cache.builds,
        query_image_cache_build_micros: output.plan.query_image_cache.build_micros,
        planner_stats_cached_relations: output.plan.planner_stats.cached_relations,
        planner_stats_hits: output.plan.planner_stats.hits,
        planner_stats_misses: output.plan.planner_stats.misses,
        planner_stats_builds: output.plan.planner_stats.builds,
        planner_stats_build_micros: output.plan.planner_stats.build_micros,
        sorted_trie_cache_hits: output.plan.counters.sorted_trie_cache_hits,
        sorted_trie_cache_misses: output.plan.counters.sorted_trie_cache_misses,
        sorted_trie_builds: output.plan.counters.sorted_trie_builds,
        lftj_lazy_access_slices: output.plan.counters.lftj_lazy_access_slices,
        lftj_eager_builds_avoided: output.plan.counters.lftj_eager_builds_avoided,
        atom_temp_relation_builds: output.plan.counters.atom_temp_relation_builds,
        query_image_relation_count: query_image_stats.relation_count,
        query_image_fact_count: query_image_stats.fact_count,
        query_image_encoded_column_bytes: query_image_stats.encoded_column_bytes,
        query_image_sorted_trie_bytes: query_image_stats.sorted_trie_bytes,
        gate,
    }
}

fn query_image_scope(output: &QueryOutput) -> &'static str {
    if output.plan.timings.query_image_micros == 0
        && output.plan.query_image_cache.cached_images == 0
        && output.plan.query_image_cache.builds == 0
    {
        "not_applicable"
    } else {
        "full_schema"
    }
}

fn emit_profile_summary(dataset: &str, query: &str, output: &QueryOutput) {
    let plan = &output.plan;
    let timings = plan.timings;
    tracing::debug!(
        dataset,
        query,
        facts = output.result.facts.len(),
        total_micros = timings.total_micros,
        plan_micros = timings.plan_micros,
        execute_micros = timings.execute_micros,
        unaccounted_micros = timings.unaccounted_micros,
        sink_finish_micros = timings.sink_finish_micros,
        allocations_enabled = plan.allocations.enabled,
        "benchmark query profile"
    );
}

fn evaluate_gate(
    dataset: &'static str,
    query: &BenchQuery,
    output: &QueryOutput,
    bumbledb_avg: Duration,
    sqlite_ratio: f64,
    final_output_values: u64,
    output_contains_dictionary_values: bool,
) -> GateOutcome {
    let mut passed = true;
    let mut notes = Vec::new();
    if let Some(gate) = benchmark_gate(dataset, query.name) {
        notes.push(format!("performance gate {}.{}", gate.dataset, gate.query));
        let avg_micros = duration_micros(bumbledb_avg);
        let max_bumbledb_avg_micros = gate.max_bumbledb_avg_micros;
        if let Some(max) = max_bumbledb_avg_micros
            && avg_micros > u128::from(max)
        {
            passed = false;
            notes.push(format!("avg {avg_micros}us exceeds {max}us"));
        }
        if let Some(max) = gate.max_sqlite_ratio
            && sqlite_ratio > max
        {
            passed = false;
            notes.push(format!("sqlite ratio {sqlite_ratio:.2} exceeds {max:.2}"));
        }
        if let Some(max) = gate.max_iterator_ops
            && output.plan.free_join.estimates.iterator_ops > max
        {
            passed = false;
            notes.push(format!(
                "iterator_ops {} exceeds {max}",
                output.plan.free_join.estimates.iterator_ops
            ));
        }
        if let Some(max) = gate.max_materialized_values
            && output.plan.counters.materialized_output_values > max
        {
            passed = false;
            notes.push(format!(
                "materialized_output_values {} exceeds {max}",
                output.plan.counters.materialized_output_values
            ));
        }
    } else {
        notes.push("no performance gate configured for query".to_owned());
    }

    let counters = &output.plan.counters;
    if counters.cursor_seeks != 0 || counters.facts_scanned != 0 {
        passed = false;
        notes.push(format!(
            "LMDB scan counters nonzero: cursor_seeks={} facts_scanned={}",
            counters.cursor_seeks, counters.facts_scanned
        ));
    }
    if !output_contains_dictionary_values && counters.dictionary_reverse_lookups != 0 {
        passed = false;
        notes.push(format!(
            "dictionary_reverse_lookups {} without string/bytes output",
            counters.dictionary_reverse_lookups
        ));
    }
    if counters.materialized_output_values != final_output_values {
        passed = false;
        notes.push(format!(
            "materialized_output_values {} != final output values {}",
            counters.materialized_output_values, final_output_values
        ));
    }
    if passed && notes.is_empty() {
        notes.push("all configured gates passed".to_owned());
    }
    GateOutcome { passed, notes }
}

fn benchmark_gate(dataset: &'static str, query: &'static str) -> Option<BenchmarkGate> {
    let gate = match (dataset, query) {
        ("joinstress", "triangle_count") => BenchmarkGate {
            dataset,
            query,
            max_bumbledb_avg_micros: Some(250_000),
            max_sqlite_ratio: None,
            max_iterator_ops: Some(1_000_000),
            max_materialized_values: Some(1),
        },
        ("ledger", "tag_lookup_join") => BenchmarkGate {
            dataset,
            query,
            max_bumbledb_avg_micros: Some(250_000),
            max_sqlite_ratio: None,
            max_iterator_ops: Some(2_000_000),
            max_materialized_values: None,
        },
        ("sailors", "red_boat_sailors") => BenchmarkGate {
            dataset,
            query,
            max_bumbledb_avg_micros: Some(250_000),
            max_sqlite_ratio: None,
            max_iterator_ops: Some(2_000_000),
            max_materialized_values: None,
        },
        ("tpch", "supplier_nation_orders") => BenchmarkGate {
            dataset,
            query,
            max_bumbledb_avg_micros: Some(250_000),
            max_sqlite_ratio: None,
            max_iterator_ops: Some(2_000_000),
            max_materialized_values: None,
        },
        ("sailors", "sailor_range_reserves") => BenchmarkGate {
            dataset,
            query,
            max_bumbledb_avg_micros: Some(75),
            max_sqlite_ratio: None,
            max_iterator_ops: None,
            max_materialized_values: None,
        },
        ("joinstress", "chain4_from_a") => BenchmarkGate {
            dataset,
            query,
            max_bumbledb_avg_micros: Some(75),
            max_sqlite_ratio: None,
            max_iterator_ops: None,
            max_materialized_values: None,
        },
        ("job", "job_q09_voice_us_actor") => BenchmarkGate {
            dataset,
            query,
            max_bumbledb_avg_micros: Some(3_000),
            max_sqlite_ratio: Some(1.0),
            max_iterator_ops: None,
            max_materialized_values: Some(1),
        },
        ("job", "job_q16_character_title_us") => BenchmarkGate {
            dataset,
            query,
            max_bumbledb_avg_micros: Some(1_000),
            max_sqlite_ratio: Some(1.0),
            max_iterator_ops: None,
            max_materialized_values: Some(1),
        },
        ("job", "job_q24_voice_keyword_actor") => BenchmarkGate {
            dataset,
            query,
            max_bumbledb_avg_micros: Some(1_000),
            max_sqlite_ratio: Some(1.0),
            max_iterator_ops: None,
            max_materialized_values: Some(0),
        },
        _ => return None,
    };
    Some(gate)
}
