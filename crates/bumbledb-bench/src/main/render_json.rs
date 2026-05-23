fn render_json_results(results: &[BenchmarkRunResult]) -> String {
    let mut out = String::new();
    out.push_str("{\"results\":[");
    for (index, result) in results.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            "{{\"dataset\":\"{}\",\"query\":\"{}\",\"facts\":{},\"correctness_mode\":\"{}\",\"result\":{{\"logical_facts\":{},\"materialized_facts\":{},\"materialized_values\":{},\"output_mode\":\"materialized\"}},\"chosen_plan\":\"{}\",\"cache_mode\":\"{}\",\"query_image_cache_hit\":{},\"query_image_sample_cache_hits\":{},\"sqlite_materialized_facts\":{},\"query_image_built_during_query\":{},\"allocation_scope\":\"{}\",\"query_image_scope\":\"{}\",",
            json_escape(result.dataset),
            json_escape(result.query),
            result.facts,
            json_escape(&result.correctness_mode),
            result.facts,
            result.facts,
            result.final_output_values,
            json_escape(&result.chosen_plan),
            json_escape(&result.cache_mode),
            result.query_image_sample_cache_hits > 0,
            result.query_image_sample_cache_hits,
            result.sqlite_materialized_facts,
            result.query_image_built_during_query,
            json_escape(&result.allocation_scope),
            json_escape(&result.query_image_scope),
        );
        out.push_str("\"bumbledb\":{");
        let _ = write!(
            out,
            "\"correctness_execution\":{{\"elapsed_us\":{},\"output_mode\":\"materialized\"}},\"cold_execution\":{{\"elapsed_us\":{},\"query_image_built\":{},\"query_image_scope\":\"{}\",\"materialized_facts\":{},\"logical_facts\":{},\"output_values\":{}}},\"warmup\":{{\"samples\":{},\"avg_us\":{}}},\"samples\":",
            duration_micros(result.bumbledb_correctness_execution),
            duration_micros(result.bumbledb_cold_execution),
            result.query_image_built_during_query,
            json_escape(&result.query_image_scope),
            result.facts,
            result.facts,
            result.final_output_values,
            result.bumbledb_warmup.samples,
            duration_micros(result.bumbledb_warmup.avg),
        );
        write_timing_stats_value(&mut out, result.bumbledb_samples);
        out.push_str("},\"sqlite\":{");
        let _ = write!(
            out,
            "\"correctness_execution\":{{\"elapsed_us\":{},\"output_mode\":\"facts\"}},\"cold_execution\":{{\"elapsed_us\":{}}},\"warmup\":{{\"samples\":{},\"avg_us\":{}}},\"samples\":",
            duration_micros(result.sqlite_correctness_execution),
            duration_micros(result.sqlite_cold_execution),
            result.sqlite_warmup.samples,
            duration_micros(result.sqlite_warmup.avg),
        );
        write_timing_stats_value(&mut out, result.sqlite_samples);
        out.push('}');
        let timings = result.timings;
        let allocations = result.allocations;
        let _ = write!(
            out,
            ",\"phase_timing\":{{\"scope\":\"{}\",\"total_us\":{},\"validate_us\":{},\"normalize_us\":{},\"encode_us\":{},\"image_us\":{},\"plan_us\":{},\"lftj_build_us\":{},\"execute_us\":{},\"lftj_execute_us\":{},\"sink_emit_us\":{},\"sink_finish_us\":{},\"decode_us\":{},\"unaccounted_us\":{}}},\"allocations\":{{\"scope\":\"{}\",\"enabled\":{},\"alloc_calls\":{},\"dealloc_calls\":{},\"realloc_calls\":{},\"bytes_allocated\":{},\"bytes_deallocated\":{},\"net_bytes\":{},\"current_live_bytes\":{},\"peak_live_bytes\":{}",
            json_escape(&result.allocation_scope),
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
            json_escape(&result.allocation_scope),
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
        out.push_str(",\"phases\":{");
        write_allocation_phase_json(&mut out, "total", allocations.total, true);
        write_allocation_phase_json(
            &mut out,
            "validate_inputs",
            allocations.validate_inputs,
            false,
        );
        write_allocation_phase_json(&mut out, "normalize", allocations.normalize, false);
        write_allocation_phase_json(&mut out, "encode_inputs", allocations.encode_inputs, false);
        write_allocation_phase_json(&mut out, "query_image", allocations.query_image, false);
        write_allocation_phase_json(&mut out, "plan", allocations.plan, false);
        write_allocation_phase_json(&mut out, "lftj_build", allocations.lftj_build, false);
        write_allocation_phase_json(&mut out, "execute", allocations.execute, false);
        write_allocation_phase_json(&mut out, "lftj_execute", allocations.lftj_execute, false);
        write_allocation_phase_json(&mut out, "sink_finish", allocations.sink_finish, false);
        out.push_str("},\"size_class_allocs\":[");
        for (index, count) in allocations.size_class_allocs.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            let _ = write!(out, "{}", count);
        }
        let _ = write!(
            out,
            "]}},\"counters\":{{\"cursor_seeks\":{},\"facts_scanned\":{},\"dictionary_reverse_lookups\":{},\"materialized_output_values\":{},\"bindings_completed\":{},\"sink_emit_calls\":{},\"encoded_project_facts_seen\":{},\"encoded_project_facts_inserted\":{},\"encoded_project_fact_bytes\":{},\"project_decode_values\":{},\"lftj_open_calls\":{},\"lftj_up_calls\":{},\"lftj_next_calls\":{},\"lftj_seek_calls\":{},\"lftj_key_reads\":{},\"lftj_candidate_values\":{},\"lftj_bind_successes\":{},\"lftj_bind_rejects\":{},\"lftj_completed_bindings\":{},\"lftj_lazy_access_slices\":{},\"lftj_eager_builds_avoided\":{},\"query_image_relations_loaded\":{},\"query_image_facts_loaded\":{},\"query_image_encoded_bytes\":{},\"sorted_trie_bytes\":{}}},\"gate\":{{\"passed\":{},\"notes\":[",
            result.counters.cursor_seeks,
            result.counters.facts_scanned,
            result.dictionary_reverse_lookups,
            result.materialized_values,
            result.counters.bindings_completed,
            result.counters.sink_emit_calls,
            result.counters.encoded_project_facts_seen,
            result.counters.encoded_project_facts_inserted,
            result.counters.encoded_project_fact_bytes,
            result.counters.project_decode_values,
            result.counters.lftj_open_calls,
            result.counters.lftj_up_calls,
            result.counters.lftj_next_calls,
            result.counters.lftj_seek_calls,
            result.counters.lftj_key_reads,
            result.counters.lftj_candidate_values,
            result.counters.lftj_bind_successes,
            result.counters.lftj_bind_rejects,
            result.counters.lftj_completed_bindings,
            result.counters.lftj_lazy_access_slices,
            result.counters.lftj_eager_builds_avoided,
            result.query_image_relation_count,
            result.query_image_fact_count,
            result.query_image_encoded_column_bytes,
            result.query_image_sorted_trie_bytes,
            result.gate.passed,
        );
        for (note_index, note) in result.gate.notes.iter().enumerate() {
            if note_index > 0 {
                out.push(',');
            }
            let _ = write!(out, "\"{}\"", json_escape(note));
        }
        out.push_str("]}}");
    }
    out.push_str("]}");
    out
}

fn write_timing_stats_value(out: &mut String, stats: TimingStats) {
    let _ = write!(
        out,
        "{{\"samples\":{},\"total_us\":{},\"avg_us\":{},\"min_us\":{},\"p50_us\":{},\"p95_us\":{},\"max_us\":{}}}",
        stats.samples,
        duration_micros(stats.total),
        duration_micros(stats.avg),
        duration_micros(stats.min),
        duration_micros(stats.p50),
        duration_micros(stats.p95),
        duration_micros(stats.max),
    );
}

fn write_allocation_phase_json(
    out: &mut String,
    name: &str,
    stats: AllocationPhaseStats,
    first: bool,
) {
    if !first {
        out.push(',');
    }
    let _ = write!(
        out,
        "\"{}\":{{\"enabled\":{},\"alloc_calls\":{},\"dealloc_calls\":{},\"realloc_calls\":{},\"bytes_allocated\":{},\"bytes_deallocated\":{},\"net_bytes\":{},\"current_live_bytes\":{},\"peak_live_bytes\":{}}}",
        json_escape(name),
        stats.enabled,
        stats.alloc_calls,
        stats.dealloc_calls,
        stats.realloc_calls,
        stats.bytes_allocated,
        stats.bytes_deallocated,
        stats.net_bytes,
        stats.current_live_bytes,
        stats.peak_live_bytes,
    );
}

fn duration_ratio(left: Duration, right: Duration) -> f64 {
    let right = right.as_nanos();
    if right == 0 {
        return f64::INFINITY;
    }
    left.as_nanos() as f64 / right as f64
}

fn duration_micros(duration: Duration) -> u128 {
    duration.as_micros()
}

fn markdown_escape(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn json_escape(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => {
                let _ = write!(out, "\\u{:04x}", ch as u32);
            }
            ch => out.push(ch),
        }
    }
    out
}

fn print_explain(explain: &str) {
    for line in explain.lines() {
        if line.contains("relation=")
            || line.contains("query_timing")
            || line.contains("allocation_summary")
            || line.contains("variable_estimate")
            || line.contains("missing_index")
            || line.contains("query_image_cache")
            || line.contains("planner_stats")
            || line.contains("chosen_plan")
            || line.contains("candidate_plan")
            || line.contains("free_join_estimates")
            || line.contains("free_join_node")
            || line.contains("node_facts")
            || line.contains("node_timing")
            || line.contains("free_join_subatom")
            || line.contains("facts_scanned")
            || line.contains("cursor_seeks")
            || line.contains("trie_intersections")
            || line.contains("variable_candidates")
            || line.contains("decoded_values")
            || line.contains("dictionary_reverse_lookups")
            || line.contains("encoded_comparisons_evaluated")
            || line.contains("decoded_comparisons_evaluated")
            || line.contains("materialized_output_values")
            || line.contains("trie_open")
            || line.contains("trie_up")
            || line.contains("trie_next")
            || line.contains("trie_seek")
            || line.contains("trie_key_reads")
            || line.contains("sorted_trie_cache")
            || line.contains("sorted_trie_build")
            || line.contains("atom_temp_relation")
            || line.contains("output_facts")
        {
            println!("  {line}");
        }
    }
}
