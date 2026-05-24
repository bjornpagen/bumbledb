use bumbledb_lmdb::{QueryTrace, TraceCounters, TraceSpan, Value};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BenchmarkReport {
    pub(crate) scale: u64,
    pub(crate) dataset: String,
    pub(crate) query: String,
    pub(crate) engine: String,
    pub(crate) sqlite_reference: String,
    pub(crate) git_commit: String,
    pub(crate) hardware: String,
    pub(crate) correctness_fingerprint: String,
    pub(crate) gate_status: String,
    pub(crate) elapsed_nanos: u128,
    pub(crate) sqlite_elapsed_nanos: u128,
    pub(crate) load_nanos: u128,
    pub(crate) result_rows: usize,
    pub(crate) allocation_tracking: bool,
    pub(crate) alloc_calls: u64,
    pub(crate) allocated_bytes: u64,
    pub(crate) deallocated_bytes: u64,
    pub(crate) net_allocated_bytes: i128,
    pub(crate) trace_json: String,
}

impl BenchmarkReport {
    pub(crate) fn render_json(&self) -> String {
        format!(
            "{{\"scale\":{},\"dataset\":\"{}\",\"query\":\"{}\",\"engine\":\"{}\",\"sqlite_reference\":\"{}\",\"git_commit\":\"{}\",\"hardware\":\"{}\",\"correctness_fingerprint\":\"{}\",\"gate_status\":\"{}\",\"elapsed_nanos\":{},\"sqlite_elapsed_nanos\":{},\"load_nanos\":{},\"result_rows\":{},\"allocation_tracking\":{},\"alloc_calls\":{},\"allocated_bytes\":{},\"deallocated_bytes\":{},\"net_allocated_bytes\":{},\"trace\":{}}}",
            self.scale,
            escape(&self.dataset),
            escape(&self.query),
            escape(&self.engine),
            escape(&self.sqlite_reference),
            escape(&self.git_commit),
            escape(&self.hardware),
            escape(&self.correctness_fingerprint),
            escape(&self.gate_status),
            self.elapsed_nanos,
            self.sqlite_elapsed_nanos,
            self.load_nanos,
            self.result_rows,
            self.allocation_tracking,
            self.alloc_calls,
            self.allocated_bytes,
            self.deallocated_bytes,
            self.net_allocated_bytes,
            self.trace_json,
        )
    }

    pub(crate) fn render_markdown(&self) -> String {
        format!(
            "| field | value |\n| --- | --- |\n| dataset | {} |\n| query | {} |\n| engine | {} |\n| sqlite_reference | {} |\n| bumbledb_ms | {:.3} |\n| sqlite_ms | {:.3} |\n| load_ms | {:.3} |\n| result_rows | {} |\n| allocation_tracking | {} |\n| alloc_calls | {} |\n| allocated_bytes | {} |\n| deallocated_bytes | {} |\n| net_allocated_bytes | {} |\n| trace | {} |\n| correctness_fingerprint | {} |\n| gate_status | {} |\n",
            self.dataset,
            self.query,
            self.engine,
            self.sqlite_reference,
            self.elapsed_nanos as f64 / 1_000_000.0,
            self.sqlite_elapsed_nanos as f64 / 1_000_000.0,
            self.load_nanos as f64 / 1_000_000.0,
            self.result_rows,
            self.allocation_tracking,
            self.alloc_calls,
            self.allocated_bytes,
            self.deallocated_bytes,
            self.net_allocated_bytes,
            self.trace_json,
            self.correctness_fingerprint,
            self.gate_status,
        )
    }
}

pub(crate) fn render_json_array(reports: &[BenchmarkReport]) -> String {
    let body = reports
        .iter()
        .map(BenchmarkReport::render_json)
        .collect::<Vec<_>>()
        .join(",");
    format!("{{\"reports\":[{body}]}}")
}

pub(crate) fn render_markdown(reports: &[BenchmarkReport]) -> String {
    reports
        .iter()
        .map(BenchmarkReport::render_markdown)
        .collect::<Vec<_>>()
        .join("\n")
}

pub(crate) fn fingerprint_rows(rows: &[Vec<Value>]) -> String {
    let mut rows = rows.to_vec();
    rows.sort();
    let mut hasher = blake3::Hasher::new();
    for row in rows {
        hasher.update(format!("{row:?}\n").as_bytes());
    }
    hasher.finalize().to_hex().to_string()
}

pub(crate) fn render_trace_json(trace: &QueryTrace, include_spans: bool) -> String {
    if !trace.is_enabled() {
        return "{\"enabled\":false}".to_owned();
    }
    assert!(
        !trace.spans.is_empty() || trace.counters != TraceCounters::default(),
        "enabled trace rendering requires source measurements"
    );
    format!(
        "{{\"enabled\":true,\"metadata\":{},\"counters\":{},\"top_elapsed\":{},\"top_allocated\":{},\"spans\":{}}}",
        metadata_json(trace),
        counters_json(trace.counters),
        top_spans_json(trace, TopSpanKey::Elapsed),
        top_spans_json(trace, TopSpanKey::Allocated),
        if include_spans {
            spans_json(trace)
        } else {
            "[]".to_owned()
        },
    )
}

fn metadata_json(trace: &QueryTrace) -> String {
    format!(
        "{{\"selected_plan_family\":\"{}\",\"node_count\":{},\"cover_policy\":\"{}\",\"execution_mode\":\"{}\",\"output_mode\":\"{}\"}}",
        escape(&trace.metadata.selected_plan_family),
        trace.metadata.node_count,
        escape(&trace.metadata.cover_policy),
        escape(&trace.metadata.execution_mode),
        escape(&trace.metadata.output_mode),
    )
}

fn counters_json(counters: TraceCounters) -> String {
    format!(
        "{{\"base_image_cache_hits\":{},\"base_image_cache_misses\":{},\"live_rows_scanned\":{},\"column_values_loaded\":{},\"loaded_bytes\":{},\"source_filters_encoded\":{},\"source_filter_false_decisions\":{},\"source_filter_rows_tested\":{},\"source_filter_survivors\":{},\"colt_nodes_created\":{},\"colt_nodes_forced\":{},\"colt_offsets_scanned\":{},\"colt_map_entries_built\":{},\"tuples_yielded\":{},\"batches_yielded\":{},\"cover_choices\":{},\"probe_calls\":{},\"probe_misses\":{},\"recursive_node_entries\":{},\"max_recursion_depth\":{},\"binding_copies\":{},\"source_frame_changes\":{},\"sink_consumes\":{},\"projection_duplicates_suppressed\":{},\"decoded_values\":{}}}",
        counters.base_image_cache_hits,
        counters.base_image_cache_misses,
        counters.live_rows_scanned,
        counters.column_values_loaded,
        counters.loaded_bytes,
        counters.source_filters_encoded,
        counters.source_filter_false_decisions,
        counters.source_filter_rows_tested,
        counters.source_filter_survivors,
        counters.colt_nodes_created,
        counters.colt_nodes_forced,
        counters.colt_offsets_scanned,
        counters.colt_map_entries_built,
        counters.tuples_yielded,
        counters.batches_yielded,
        counters.cover_choices,
        counters.probe_calls,
        counters.probe_misses,
        counters.recursive_node_entries,
        counters.max_recursion_depth,
        counters.binding_copies,
        counters.source_frame_changes,
        counters.sink_consumes,
        counters.projection_duplicates_suppressed,
        counters.decoded_values,
    )
}

enum TopSpanKey {
    Elapsed,
    Allocated,
}

fn top_spans_json(trace: &QueryTrace, key: TopSpanKey) -> String {
    let mut spans = trace.spans.iter().collect::<Vec<_>>();
    match key {
        TopSpanKey::Elapsed => spans.sort_by_key(|span| std::cmp::Reverse(span.elapsed_nanos)),
        TopSpanKey::Allocated => {
            spans.sort_by_key(|span| std::cmp::Reverse(span.allocs.allocated_bytes));
        }
    }
    let body = spans
        .into_iter()
        .take(10)
        .map(span_json)
        .collect::<Vec<_>>()
        .join(",");
    format!("[{body}]")
}

fn spans_json(trace: &QueryTrace) -> String {
    let body = trace
        .spans
        .iter()
        .map(span_json)
        .collect::<Vec<_>>()
        .join(",");
    format!("[{body}]")
}

fn span_json(span: &TraceSpan) -> String {
    format!(
        "{{\"id\":{},\"parent_id\":{},\"phase\":\"{:?}\",\"label\":\"{}\",\"elapsed_nanos\":{},\"alloc_calls\":{},\"allocated_bytes\":{}}}",
        span.id,
        span.parent_id
            .map_or_else(|| "null".to_owned(), |id| id.to_string()),
        span.phase,
        escape(&span.label),
        span.elapsed_nanos,
        span.allocs.alloc_calls,
        span.allocs.allocated_bytes,
    )
}

fn escape(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renderer_outputs_required_fields() {
        let report = BenchmarkReport {
            scale: 1,
            dataset: "clover_skew".to_owned(),
            query: "clover".to_owned(),
            engine: "free_join".to_owned(),
            sqlite_reference: "exact SELECT DISTINCT".to_owned(),
            git_commit: "unknown".to_owned(),
            hardware: "unspecified".to_owned(),
            correctness_fingerprint: "abc".to_owned(),
            gate_status: "passed".to_owned(),
            elapsed_nanos: 1,
            sqlite_elapsed_nanos: 2,
            load_nanos: 3,
            result_rows: 1,
            allocation_tracking: false,
            alloc_calls: 0,
            allocated_bytes: 0,
            deallocated_bytes: 0,
            net_allocated_bytes: 0,
            trace_json: "{\"enabled\":false}".to_owned(),
        };

        let json = report.render_json();
        let markdown = report.render_markdown();

        assert!(json.contains("\"engine\":\"free_join\""));
        assert!(json.contains("\"sqlite_elapsed_nanos\":2"));
        assert!(json.contains("\"allocation_tracking\":false"));
        assert!(json.contains("\"trace\":{\"enabled\":false}"));
        assert!(markdown.contains("sqlite_reference"));
    }

    #[test]
    fn trace_renderer_rejects_enabled_empty_measurements() {
        let trace = QueryTrace::new();

        let result = std::panic::catch_unwind(|| render_trace_json(&trace, false));

        assert!(result.is_err());
    }
}
