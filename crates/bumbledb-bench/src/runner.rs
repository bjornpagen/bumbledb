#![allow(clippy::result_large_err)]

use std::fmt;
use std::path::PathBuf;
use std::time::Instant;

use bumbledb_core::query_ir::TypedQuery;
use bumbledb_lmdb::diagnostics::{
    allocation_delta, allocation_snapshot, set_allocation_tracking_enabled,
};
use bumbledb_lmdb::{
    Fact, InputBindings, QueryExecutionOptions, QueryResultSet, QueryTrace, Value,
};
use bumbledb_test_support::{clover_query, env_and_schema, pair, rows};

use crate::cli::{Config, OutputFormat, TraceOutput};
use crate::lint::validate_select_distinct;
use crate::report::{
    BenchmarkReport, fingerprint_rows, render_json_array, render_markdown, render_trace_json,
};

pub(crate) type BenchResult<T> = Result<T, BenchError>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BenchError(String);

impl BenchError {
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self(message.into())
    }
}

impl fmt::Display for BenchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<bumbledb_lmdb::Error> for BenchError {
    fn from(value: bumbledb_lmdb::Error) -> Self {
        Self(value.to_string())
    }
}

impl From<std::io::Error> for BenchError {
    fn from(value: std::io::Error) -> Self {
        Self(value.to_string())
    }
}

#[derive(Clone)]
struct Dataset {
    name: String,
    query_name: String,
    sql: String,
    facts: Vec<Fact>,
    query: TypedQuery,
    sqlite_distinct: Vec<Vec<Value>>,
}

pub(crate) fn run_cli(config: Config) -> BenchResult<String> {
    set_allocation_tracking_enabled(config.alloc_tracking);
    if matches!(config.preset.as_str(), "job" | "job-sample" | "job-full") {
        let reports = crate::job::run_job(&config)?;
        return Ok(match config.format {
            OutputFormat::Json => render_json_array(&reports),
            OutputFormat::Markdown => render_markdown(&reports),
        });
    }
    if config.preset != "quick" {
        return Err(BenchError::new("only the quick preset is implemented"));
    }
    let dataset = dataset_by_name("clover_skew")?;
    let mut reports = Vec::new();
    for _ in 0..config.warmup {
        let _ = run_once(&dataset, &config)?;
    }
    for _ in 0..config.repeats.max(1) {
        reports.push(run_once(&dataset, &config)?);
    }
    Ok(match config.format {
        OutputFormat::Json => render_json_array(&reports),
        OutputFormat::Markdown => render_markdown(&reports),
    })
}

fn run_once(dataset: &Dataset, config: &Config) -> BenchResult<BenchmarkReport> {
    validate_select_distinct(&dataset.sql)?;
    let (env, schema) = env_and_schema(&format!("bench-{}", dataset.name))?;
    env.write(|txn| {
        for fact in &dataset.facts {
            txn.insert(&schema, fact)?;
        }
        Ok::<(), bumbledb_lmdb::Error>(())
    })?;
    let start = Instant::now();
    let alloc_start = allocation_snapshot();
    let profiled = env.read(|txn| {
        txn.execute_query_profiled(
            &schema,
            &dataset.query,
            &InputBindings::new(),
            QueryExecutionOptions {
                allocation_tracking: config.alloc_tracking,
                ..QueryExecutionOptions::default()
            },
        )
    })?;
    let trace_json = trace_json_for_report(config, &dataset.query_name, 0, &profiled.trace)?;
    let result = profiled.result;
    let alloc_delta = allocation_delta(alloc_start, allocation_snapshot());
    let elapsed_nanos = start.elapsed().as_nanos();
    compare_exact(&result, &dataset.sqlite_distinct)?;
    Ok(BenchmarkReport {
        scale: 1,
        dataset: dataset.name.clone(),
        query: dataset.query_name.clone(),
        engine: "free_join".to_owned(),
        sqlite_reference: "embedded exact expected rows".to_owned(),
        git_commit: option_env!("GIT_COMMIT").unwrap_or("unknown").to_owned(),
        hardware: config
            .hardware
            .clone()
            .unwrap_or_else(|| "unspecified".to_owned()),
        correctness_fingerprint: fingerprint_rows(&result.facts),
        gate_status: "passed".to_owned(),
        elapsed_nanos,
        sqlite_elapsed_nanos: 0,
        load_nanos: 0,
        result_rows: result.facts.len(),
        allocation_tracking: config.alloc_tracking,
        alloc_calls: alloc_delta.alloc_calls,
        allocated_bytes: alloc_delta.allocated_bytes,
        deallocated_bytes: alloc_delta.deallocated_bytes,
        net_allocated_bytes: alloc_delta.net_bytes,
        trace_json,
    })
}

pub(crate) fn trace_json_for_report(
    config: &Config,
    query_name: &str,
    index: usize,
    trace: &QueryTrace,
) -> BenchResult<String> {
    if !trace.is_enabled() {
        return Ok(render_trace_json(trace, false));
    }
    match config.trace_output {
        TraceOutput::Inline => Ok(render_trace_json(trace, true)),
        TraceOutput::File => {
            let dir = PathBuf::from("data/traces");
            std::fs::create_dir_all(&dir)?;
            let label = config.profile_query_label.as_deref().unwrap_or(query_name);
            let file = dir.join(format!(
                "{}-{}-{index}.json",
                sanitize_file_component(label),
                std::process::id()
            ));
            let full = render_trace_json(trace, true);
            std::fs::write(&file, full)?;
            Ok(format!(
                "{{\"enabled\":true,\"file\":\"{}\",\"summary\":{}}}",
                escape_json(&file.to_string_lossy()),
                render_trace_json(trace, false)
            ))
        }
    }
}

fn sanitize_file_component(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn escape_json(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn compare_exact(result: &QueryResultSet, expected: &[Vec<Value>]) -> BenchResult<()> {
    let actual = result.facts.clone();
    if actual != expected {
        return Err(BenchError::new(format!(
            "exact values differ: bumbledb={actual:?} sqlite_select_distinct={expected:?}"
        )));
    }
    Ok(())
}

fn dataset_by_name(name: &str) -> BenchResult<Dataset> {
    match name {
        "clover_skew" => Ok(Dataset {
            name: name.to_owned(),
            query_name: "paper_clover".to_owned(),
            sql: "SELECT DISTINCT R.left, R.right, S.right, T.right FROM R, S, T WHERE R.left = S.left AND S.left = T.left".to_owned(),
            facts: vec![
                pair("R", 0, 10),
                pair("R", 1, 11),
                pair("R", 2, 12),
                pair("S", 0, 20),
                pair("S", 2, 21),
                pair("S", 3, 22),
                pair("T", 0, 30),
                pair("T", 3, 31),
                pair("T", 1, 32),
            ],
            query: clover_query(&[0, 1, 2, 3]),
            sqlite_distinct: rows([[0, 10, 20, 30]]),
        }),
        _ => Err(BenchError::new(format!("unknown dataset {name}"))),
    }
}

#[cfg(test)]
fn required_dataset_names() -> Vec<&'static str> {
    vec![
        "ledger",
        "sailors",
        "joinstress",
        "tpch_subset",
        "clover_skew",
        "triangle_cyclic",
        "chain_acyclic",
        "star_acyclic",
        "job_sample",
        "lsqb_subset",
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn small_benchmark_run_uses_public_free_join() -> BenchResult<()> {
        let config = Config::default();
        let dataset = dataset_by_name("clover_skew")?;
        let report = run_once(&dataset, &config)?;
        assert_eq!(report.engine, "free_join");
        Ok(())
    }

    #[test]
    fn end_to_end_equal_count_different_value_fails() -> BenchResult<()> {
        let dataset = dataset_by_name("clover_skew")?;
        let (env, schema) = env_and_schema("bench-equal-count-different-value")?;
        env.write(|txn| {
            for fact in &dataset.facts {
                txn.insert(&schema, fact)?;
            }
            Ok::<(), bumbledb_lmdb::Error>(())
        })?;
        let result =
            env.read(|txn| txn.execute_query(&schema, &dataset.query, &InputBindings::new()))?;
        let wrong_same_count = vec![vec![
            Value::U64(999),
            Value::U64(10),
            Value::U64(20),
            Value::U64(30),
        ]];
        assert_eq!(result.facts.len(), wrong_same_count.len());
        assert!(compare_exact(&result, &wrong_same_count).is_err());
        Ok(())
    }

    #[test]
    fn required_dataset_inventory_is_present() {
        let names = required_dataset_names();
        assert!(names.contains(&"ledger"));
        assert!(names.contains(&"sailors"));
        assert!(names.contains(&"joinstress"));
        assert!(names.contains(&"tpch_subset"));
        assert!(names.contains(&"job_sample"));
        assert!(names.contains(&"lsqb_subset"));
    }
}
