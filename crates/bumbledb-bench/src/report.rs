use bumbledb_lmdb::Value;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Counters {
    pub(crate) cover_choices: usize,
    pub(crate) vectorized_batches: usize,
    pub(crate) vectorized_survivors: usize,
    pub(crate) vectorized_failed: usize,
    pub(crate) colt_nodes_created: usize,
    pub(crate) colt_nodes_forced: usize,
    pub(crate) colt_offsets_scanned: usize,
    pub(crate) duplicate_witnesses: usize,
    pub(crate) factored_dynamic_skew_delta: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BenchmarkReport {
    pub(crate) scale: u64,
    pub(crate) dataset: String,
    pub(crate) query: String,
    pub(crate) plan_mode: String,
    pub(crate) cover_mode: String,
    pub(crate) batch_size: usize,
    pub(crate) output_mode: String,
    pub(crate) source_mode: String,
    pub(crate) git_commit: String,
    pub(crate) hardware: String,
    pub(crate) correctness_fingerprint: String,
    pub(crate) gate_status: String,
    pub(crate) elapsed_nanos: u128,
    pub(crate) counters: Counters,
}

impl BenchmarkReport {
    pub(crate) fn render_json(&self) -> String {
        format!(
            "{{\"scale\":{},\"dataset\":\"{}\",\"query\":\"{}\",\"plan_mode\":\"{}\",\"cover_mode\":\"{}\",\"batch_size\":{},\"output_mode\":\"{}\",\"source_mode\":\"{}\",\"git_commit\":\"{}\",\"hardware\":\"{}\",\"correctness_fingerprint\":\"{}\",\"gate_status\":\"{}\",\"elapsed_nanos\":{},\"cover_choices\":{},\"vectorized_batches\":{},\"vectorized_survivors\":{},\"vectorized_failed\":{},\"colt_nodes_created\":{},\"colt_nodes_forced\":{},\"colt_offsets_scanned\":{},\"duplicate_witnesses\":{},\"factored_dynamic_skew_delta\":{}}}",
            self.scale,
            escape(&self.dataset),
            escape(&self.query),
            escape(&self.plan_mode),
            escape(&self.cover_mode),
            self.batch_size,
            escape(&self.output_mode),
            escape(&self.source_mode),
            escape(&self.git_commit),
            escape(&self.hardware),
            escape(&self.correctness_fingerprint),
            escape(&self.gate_status),
            self.elapsed_nanos,
            self.counters.cover_choices,
            self.counters.vectorized_batches,
            self.counters.vectorized_survivors,
            self.counters.vectorized_failed,
            self.counters.colt_nodes_created,
            self.counters.colt_nodes_forced,
            self.counters.colt_offsets_scanned,
            self.counters.duplicate_witnesses,
            self.counters.factored_dynamic_skew_delta,
        )
    }

    pub(crate) fn render_markdown(&self) -> String {
        format!(
            "| field | value |\n| --- | --- |\n| dataset | {} |\n| query | {} |\n| plan_mode | {} |\n| cover_mode | {} |\n| batch_size | {} |\n| output_mode | {} |\n| source_mode | {} |\n| correctness_fingerprint | {} |\n| gate_status | {} |\n| Free Join counters | covers={} duplicate_witnesses={} skew_delta={} |\n| COLT counters | nodes_created={} nodes_forced={} offsets_scanned={} |\n| vectorized counters | batches={} survivors={} failed={} |\n",
            self.dataset,
            self.query,
            self.plan_mode,
            self.cover_mode,
            self.batch_size,
            self.output_mode,
            self.source_mode,
            self.correctness_fingerprint,
            self.gate_status,
            self.counters.cover_choices,
            self.counters.duplicate_witnesses,
            self.counters.factored_dynamic_skew_delta,
            self.counters.colt_nodes_created,
            self.counters.colt_nodes_forced,
            self.counters.colt_offsets_scanned,
            self.counters.vectorized_batches,
            self.counters.vectorized_survivors,
            self.counters.vectorized_failed,
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
            plan_mode: "factored".to_owned(),
            cover_mode: "dynamic".to_owned(),
            batch_size: 1000,
            output_mode: "factorized".to_owned(),
            source_mode: "colt".to_owned(),
            git_commit: "unknown".to_owned(),
            hardware: "unspecified".to_owned(),
            correctness_fingerprint: "abc".to_owned(),
            gate_status: "passed".to_owned(),
            elapsed_nanos: 1,
            counters: Counters {
                cover_choices: 1,
                vectorized_batches: 2,
                vectorized_survivors: 3,
                vectorized_failed: 4,
                colt_nodes_created: 5,
                colt_nodes_forced: 1,
                colt_offsets_scanned: 6,
                duplicate_witnesses: 7,
                factored_dynamic_skew_delta: 1,
            },
        };

        let json = report.render_json();
        let markdown = report.render_markdown();

        assert!(json.contains("\"plan_mode\":\"factored\""));
        assert!(json.contains("\"cover_mode\":\"dynamic\""));
        assert!(json.contains("\"batch_size\":1000"));
        assert!(json.contains("\"source_mode\":\"colt\""));
        assert!(markdown.contains("Free Join counters"));
        assert!(markdown.contains("COLT counters"));
        assert!(markdown.contains("vectorized counters"));
    }
}
