fn render_markdown_results(results: &[BenchmarkRunResult]) -> String {
    let mut out = String::new();
    out.push_str("## Benchmark Results\n\n");
    out.push_str("| dataset | query | facts | compare mode | bumbledb materialized | sqlite materialized | cardinality | bumbledb avg us | sqlite avg us | sqlite ratio | chosen plan | image build us | image built during query | image cache images | image cache hits | image cache misses | image cache builds | image cache build us | planner stats cached | planner stats hits | planner stats misses | planner stats builds | planner stats build us | trie cache hits | trie cache misses | trie builds | lazy access slices | eager builds avoided | atom temp builds | iterator ops | hash build est | materialized | dict lookups | gate |\n");
    out.push_str("|---|---|---:|---|---|---|---|---:|---:|---:|---|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|\n");
    for result in results {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {:.2} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            markdown_escape(result.dataset),
            markdown_escape(result.query),
            result.facts,
            markdown_escape(&result.compare_mode),
            result.bumbledb_materialized_facts,
            result.sqlite_materialized_facts,
            result.cardinality_supported,
            duration_micros(result.bumbledb_avg),
            duration_micros(result.sqlite_avg),
            result.sqlite_ratio,
            markdown_escape(&result.chosen_plan),
            result.query_image_build_micros,
            result.query_image_built_during_query,
            result.query_image_cache_cached_images,
            result.query_image_cache_hits,
            result.query_image_cache_misses,
            result.query_image_cache_builds,
            result.query_image_cache_build_micros,
            result.planner_stats_cached_relations,
            result.planner_stats_hits,
            result.planner_stats_misses,
            result.planner_stats_builds,
            result.planner_stats_build_micros,
            result.sorted_trie_cache_hits,
            result.sorted_trie_cache_misses,
            result.sorted_trie_builds,
            result.lftj_lazy_access_slices,
            result.lftj_eager_builds_avoided,
            result.atom_temp_relation_builds,
            result.iterator_ops,
            result.hash_build_facts,
            result.materialized_values,
            result.dictionary_reverse_lookups,
            if result.gate.passed { "pass" } else { "fail" },
        );
    }
    out.push_str("\n## Mechanics Counters\n\n");
    out.push_str("| dataset | query | sink emits | project seen | project inserted | lftj next | lftj seek | lftj keys |\n");
    out.push_str("|---|---|---:|---:|---:|---:|---:|---:|\n");
    for result in results {
        let counters = &result.counters;
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} | {} |",
            markdown_escape(result.dataset),
            markdown_escape(result.query),
            counters.sink_emit_calls,
            counters.encoded_project_facts_seen,
            counters.encoded_project_facts_inserted,
            counters.lftj_next_calls,
            counters.lftj_seek_calls,
            counters.lftj_key_reads,
        );
    }
    out.push_str("\n## Cache Diagnostics\n\n");
    out.push_str("| dataset | query | cache mode | query image sample cache hits |\n");
    out.push_str("|---|---|---|---:|\n");
    for result in results {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} |",
            markdown_escape(result.dataset),
            markdown_escape(result.query),
            markdown_escape(&result.cache_mode),
            result.query_image_sample_cache_hits,
        );
    }
    out.push_str("\n## Measurement Contract\n\n");
    out.push_str("| dataset | query | allocation scope | query image scope | cold execution uses correctness output | count cold warmed by correctness |\n");
    out.push_str("|---|---|---|---|---|---|\n");
    for result in results {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} |",
            markdown_escape(result.dataset),
            markdown_escape(result.query),
            markdown_escape(&result.allocation_scope),
            markdown_escape(&result.query_image_scope),
            result.cold_execution_uses_correctness_output,
            result.count_cold_execution_warmed_by_correctness,
        );
    }
    out.push_str("\n## Phase Timing\n\n");
    out.push_str("| dataset | query | total us | validate us | normalize us | encode us | image us | plan us | lftj build us | execute us | lftj exec us | sink emit us | sink finish us | decode us | unaccounted us |\n");
    out.push_str("|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n");
    for result in results {
        let timings = result.timings;
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            markdown_escape(result.dataset),
            markdown_escape(result.query),
            timings.total_micros,
            timings.validate_inputs_micros,
            timings.normalize_micros,
            timings.encode_inputs_micros,
            timings.query_image_micros,
            timings.plan_micros,
            timings.lftj_build_micros,
            timings.execute_micros,
            timings.lftj_execute_micros,
            timings.sink_emit_micros,
            timings.sink_finish_micros,
            timings.decode_micros,
            timings.unaccounted_micros,
        );
    }
    out.push_str("\n## Allocation Summary\n\n");
    out.push_str("| dataset | query | enabled | alloc calls | dealloc calls | realloc calls | bytes allocated | bytes deallocated | net bytes | current live bytes | peak live bytes |\n");
    out.push_str("|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|\n");
    for result in results {
        let allocations = result.allocations;
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            markdown_escape(result.dataset),
            markdown_escape(result.query),
            allocations.enabled,
            allocations.alloc_calls,
            allocations.dealloc_calls,
            allocations.realloc_calls,
            allocations.bytes_allocated,
            allocations.bytes_deallocated,
            allocations.net_bytes,
            allocations.current_live_bytes,
            allocations.peak_live_bytes,
        );
    }
    out.push_str("\n## Allocation Phase Detail\n\n");
    out.push_str("| dataset | query | phase | enabled | alloc calls | bytes allocated | net bytes | current live bytes | peak live bytes |\n");
    out.push_str("|---|---|---|---|---:|---:|---:|---:|---:|\n");
    for result in results {
        write_allocation_phase_fact(&mut out, result, "total", result.allocations.total);
        write_allocation_phase_fact(
            &mut out,
            result,
            "validate_inputs",
            result.allocations.validate_inputs,
        );
        write_allocation_phase_fact(&mut out, result, "normalize", result.allocations.normalize);
        write_allocation_phase_fact(
            &mut out,
            result,
            "encode_inputs",
            result.allocations.encode_inputs,
        );
        write_allocation_phase_fact(
            &mut out,
            result,
            "query_image",
            result.allocations.query_image,
        );
        write_allocation_phase_fact(&mut out, result, "plan", result.allocations.plan);
        write_allocation_phase_fact(
            &mut out,
            result,
            "lftj_build",
            result.allocations.lftj_build,
        );
        write_allocation_phase_fact(&mut out, result, "execute", result.allocations.execute);
        write_allocation_phase_fact(
            &mut out,
            result,
            "sink_finish",
            result.allocations.sink_finish,
        );
    }
    out.push_str("\n## Distribution\n\n");
    out.push_str("| dataset | query | bumbledb correctness us | bumbledb cold execution us | bumbledb warmup samples | bumbledb warmup avg us | bumbledb samples | bumbledb min us | bumbledb p50 us | bumbledb p95 us | bumbledb max us | sqlite correctness us | sqlite cold execution us | sqlite warmup samples | sqlite warmup avg us | sqlite samples | sqlite min us | sqlite p50 us | sqlite p95 us | sqlite max us |\n");
    out.push_str("|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n");
    for result in results {
        let bumble = result.bumbledb_samples;
        let sqlite = result.sqlite_samples;
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            markdown_escape(result.dataset),
            markdown_escape(result.query),
            duration_micros(result.bumbledb_correctness_execution),
            duration_micros(result.bumbledb_cold_execution),
            result.bumbledb_warmup.samples,
            duration_micros(result.bumbledb_warmup.avg),
            bumble.samples,
            duration_micros(bumble.min),
            duration_micros(bumble.p50),
            duration_micros(bumble.p95),
            duration_micros(bumble.max),
            duration_micros(result.sqlite_correctness_execution),
            duration_micros(result.sqlite_cold_execution),
            result.sqlite_warmup.samples,
            duration_micros(result.sqlite_warmup.avg),
            sqlite.samples,
            duration_micros(sqlite.min),
            duration_micros(sqlite.p50),
            duration_micros(sqlite.p95),
            duration_micros(sqlite.max),
        );
    }
    out.push_str("\n## Interpretation Notes\n\n");
    out.push_str("| signal | interpretation |\n");
    out.push_str("|---|---|\n");
    out.push_str("| high image us | QueryImage acquisition or access-image build bottleneck |\n");
    out.push_str(
        "| high plan us | stats, variable ordering, or Free Join optimization bottleneck |\n",
    );
    out.push_str("| high lftj/hash build us | cached index lookup/build or atom relation preparation bottleneck |\n");
    out.push_str("| high execute us | runtime traversal/probe bottleneck |\n");
    out.push_str(
        "| high sink finish us | projection, aggregation, sorting, or decode bottleneck |\n",
    );
    out.push_str(
        "| high allocation counts | rerun with alloc-profile and then use a deep heap profiler for callsites |\n",
    );
    out.push_str("\n## Counter Gates\n\n");
    out.push_str("| dataset | query | cursor seeks | facts scanned | final values | materialized values | dictionary output | dictionary lookups | notes |\n");
    out.push_str("|---|---|---:|---:|---:|---:|---|---:|---|\n");
    for result in results {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            markdown_escape(result.dataset),
            markdown_escape(result.query),
            result.counters.cursor_seeks,
            result.counters.facts_scanned,
            result.final_output_values,
            result.materialized_values,
            result.output_contains_dictionary_values,
            result.dictionary_reverse_lookups,
            markdown_escape(&result.gate.notes.join("; ")),
        );
    }
    out
}

fn write_allocation_phase_fact(
    out: &mut String,
    result: &BenchmarkRunResult,
    phase: &str,
    stats: AllocationPhaseStats,
) {
    let _ = writeln!(
        out,
        "| {} | {} | {} | {} | {} | {} | {} | {} | {} |",
        markdown_escape(result.dataset),
        markdown_escape(result.query),
        markdown_escape(phase),
        stats.enabled,
        stats.alloc_calls,
        stats.bytes_allocated,
        stats.net_bytes,
        stats.current_live_bytes,
        stats.peak_live_bytes,
    );
}

