use bumbledb_lmdb::Value;

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
}

impl BenchmarkReport {
    pub(crate) fn render_json(&self) -> String {
        format!(
            "{{\"scale\":{},\"dataset\":\"{}\",\"query\":\"{}\",\"engine\":\"{}\",\"sqlite_reference\":\"{}\",\"git_commit\":\"{}\",\"hardware\":\"{}\",\"correctness_fingerprint\":\"{}\",\"gate_status\":\"{}\",\"elapsed_nanos\":{},\"sqlite_elapsed_nanos\":{},\"load_nanos\":{},\"result_rows\":{},\"allocation_tracking\":{},\"alloc_calls\":{},\"allocated_bytes\":{},\"deallocated_bytes\":{},\"net_allocated_bytes\":{}}}",
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
        )
    }

    pub(crate) fn render_markdown(&self) -> String {
        format!(
            "| field | value |\n| --- | --- |\n| dataset | {} |\n| query | {} |\n| engine | {} |\n| sqlite_reference | {} |\n| bumbledb_ms | {:.3} |\n| sqlite_ms | {:.3} |\n| load_ms | {:.3} |\n| result_rows | {} |\n| allocation_tracking | {} |\n| alloc_calls | {} |\n| allocated_bytes | {} |\n| deallocated_bytes | {} |\n| net_allocated_bytes | {} |\n| correctness_fingerprint | {} |\n| gate_status | {} |\n",
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
        };

        let json = report.render_json();
        let markdown = report.render_markdown();

        assert!(json.contains("\"engine\":\"free_join\""));
        assert!(json.contains("\"sqlite_elapsed_nanos\":2"));
        assert!(json.contains("\"allocation_tracking\":false"));
        assert!(markdown.contains("sqlite_reference"));
    }
}
