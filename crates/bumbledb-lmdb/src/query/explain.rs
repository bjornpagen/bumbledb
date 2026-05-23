use super::*;

impl QueryPlan {
    /// Renders this physical plan and its current execution counters.
    pub fn explain(&self) -> String {
        let mut out = String::new();
        out.push_str("QueryPlan\n");
        out.push_str(&format!("variable_order: {:?}\n", self.variable_order));
        out.push_str("timings:\n");
        out.push_str(&format!(
            "  query_timing total_micros={} validate_inputs_micros={} normalize_micros={} encode_inputs_micros={} query_image_micros={} plan_micros={} lftj_build_micros={} execute_micros={} lftj_execute_micros={} sink_finish_micros={} unaccounted_micros={}\n",
            self.timings.total_micros,
            self.timings.validate_inputs_micros,
            self.timings.normalize_micros,
            self.timings.encode_inputs_micros,
            self.timings.query_image_micros,
            self.timings.plan_micros,
            self.timings.lftj_build_micros,
            self.timings.execute_micros,
            self.timings.lftj_execute_micros,
            self.timings.sink_finish_micros,
            self.timings.unaccounted_micros
        ));
        out.push_str("allocations:\n");
        out.push_str(&format!(
            "  allocation_summary enabled={} alloc_calls={} dealloc_calls={} realloc_calls={} bytes_allocated={} bytes_deallocated={} net_bytes={} current_live_bytes={} peak_live_bytes={}\n",
            self.allocations.enabled,
            self.allocations.alloc_calls,
            self.allocations.dealloc_calls,
            self.allocations.realloc_calls,
            self.allocations.bytes_allocated,
            self.allocations.bytes_deallocated,
            self.allocations.net_bytes,
            self.allocations.current_live_bytes,
            self.allocations.peak_live_bytes
        ));
        self.allocations
            .validate_inputs
            .write_explain(&mut out, "validate_inputs");
        self.allocations
            .normalize
            .write_explain(&mut out, "normalize");
        self.allocations
            .encode_inputs
            .write_explain(&mut out, "encode_inputs");
        self.allocations
            .query_image
            .write_explain(&mut out, "query_image");
        self.allocations.plan.write_explain(&mut out, "plan");
        self.allocations
            .lftj_build
            .write_explain(&mut out, "lftj_build");
        self.allocations.execute.write_explain(&mut out, "execute");
        self.allocations
            .sink_finish
            .write_explain(&mut out, "sink_finish");
        out.push_str("planner:\n");
        out.push_str(&format!(
            "  query_image_cache cached_images={} hits={} misses={} builds={} build_micros={}\n",
            self.query_image_cache.cached_images,
            self.query_image_cache.hits,
            self.query_image_cache.misses,
            self.query_image_cache.builds,
            self.query_image_cache.build_micros
        ));
        out.push_str(&format!(
            "  planner_stats cached_relations={} hits={} misses={} builds={} build_micros={} field_stats_built={} index_stats_built={} stats_from_access_images={}\n",
            self.planner_stats.cached_relations,
            self.planner_stats.hits,
            self.planner_stats.misses,
            self.planner_stats.builds,
            self.planner_stats.build_micros,
            self.planner_stats.field_stats_built,
            self.planner_stats.index_stats_built,
            self.planner_stats.stats_from_access_images
        ));
        out.push_str("free_join_plan:\n");
        for node in &self.free_join.nodes {
            out.push_str(&format!(
                "  free_join_node id={} bind_vars={:?}\n",
                node.id.0,
                node.bind_vars.iter().map(|var| var.0).collect::<Vec<_>>()
            ));
        }
        out.push_str("counters:\n");
        out.push_str(&format!(
            "  bindings_yielded: {}\n",
            self.counters.bindings_yielded
        ));
        out.push_str(&format!(
            "  comparisons_evaluated: {}\n",
            self.counters.comparisons_evaluated
        ));
        out.push_str(&format!(
            "  comparisons_failed: {}\n",
            self.counters.comparisons_failed
        ));
        out.push_str(&format!(
            "  trie_intersections: {}\n",
            self.counters.trie_intersections
        ));
        out.push_str(&format!(
            "  variable_candidates: {}\n",
            self.counters.variable_candidates
        ));
        out.push_str(&format!(
            "  decoded_values: {}\n",
            self.counters.decoded_values
        ));
        out.push_str(&format!(
            "  dictionary_reverse_lookups: {}\n",
            self.counters.dictionary_reverse_lookups
        ));
        out.push_str(&format!(
            "  encoded_comparisons_evaluated: {}\n",
            self.counters.encoded_comparisons_evaluated
        ));
        out.push_str(&format!(
            "  decoded_comparisons_evaluated: {}\n",
            self.counters.decoded_comparisons_evaluated
        ));
        out.push_str(&format!(
            "  materialized_output_values: {}\n",
            self.counters.materialized_output_values
        ));
        out.push_str(&format!("  trie_open: {}\n", self.counters.trie_open));
        out.push_str(&format!("  trie_up: {}\n", self.counters.trie_up));
        out.push_str(&format!("  trie_next: {}\n", self.counters.trie_next));
        out.push_str(&format!("  trie_seek: {}\n", self.counters.trie_seek));
        out.push_str(&format!(
            "  trie_key_reads: {}\n",
            self.counters.trie_key_reads
        ));
        out.push_str(&format!(
            "  lftj_lazy_access_slices: {}\n",
            self.counters.lftj_lazy_access_slices
        ));
        out.push_str(&format!("  output_facts: {}\n", self.counters.output_facts));
        out
    }
}
