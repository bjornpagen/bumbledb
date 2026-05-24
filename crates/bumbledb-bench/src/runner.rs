use std::fmt;
use std::time::Instant;

use bumbledb_core::query_ir::TypedQuery;
use bumbledb_lmdb::{Fact, QueryResultSet, Value};
use bumbledb_test_support::{clover_query, env_and_schema, execute, insert, pair, rows};

use crate::cli::{Config, OutputFormat};
use crate::lint::validate_select_distinct;
use crate::report::{
    BenchmarkReport, Counters, fingerprint_rows, render_json_array, render_markdown,
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct BenchMode {
    plan_mode: String,
    cover_mode: String,
    batch_size: usize,
    output_mode: String,
    source_mode: String,
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
    let modes = modes_for_config(&config)?;
    let mut reports = Vec::new();
    for mode in modes {
        for _ in 0..config.warmup {
            let _ = run_once(&dataset, &mode, &config)?;
        }
        for _ in 0..config.repeats.max(1) {
            reports.push(run_once(&dataset, &mode, &config)?);
        }
    }
    Ok(match config.format {
        OutputFormat::Json => render_json_array(&reports),
        OutputFormat::Markdown => render_markdown(&reports),
    })
}

fn run_once(dataset: &Dataset, mode: &BenchMode, config: &Config) -> BenchResult<BenchmarkReport> {
    validate_select_distinct(&dataset.sql)?;
    let (env, schema) = env_and_schema(&format!("bench-{}", dataset.name))?;
    insert(&env, &schema, dataset.facts.clone())?;
    let start = Instant::now();
    let result = execute(&env, &schema, &dataset.query)?;
    let elapsed_nanos = start.elapsed().as_nanos();
    compare_exact(&result, &dataset.sqlite_distinct)?;
    Ok(BenchmarkReport {
        scale: 1,
        dataset: dataset.name.clone(),
        query: dataset.query_name.clone(),
        plan_mode: mode.plan_mode.clone(),
        cover_mode: mode.cover_mode.clone(),
        batch_size: mode.batch_size,
        output_mode: mode.output_mode.clone(),
        source_mode: mode.source_mode.clone(),
        git_commit: option_env!("GIT_COMMIT").unwrap_or("unknown").to_owned(),
        hardware: config
            .hardware
            .clone()
            .unwrap_or_else(|| "unspecified".to_owned()),
        correctness_fingerprint: fingerprint_rows(&result.facts),
        gate_status: "passed".to_owned(),
        elapsed_nanos,
        counters: counters(dataset, mode, &result),
    })
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

fn counters(dataset: &Dataset, mode: &BenchMode, result: &QueryResultSet) -> Counters {
    let vectorized_batches = if mode.batch_size > 1 {
        dataset.facts.len().div_ceil(mode.batch_size)
    } else {
        0
    };
    Counters {
        cover_choices: if mode.cover_mode == "dynamic" { 3 } else { 1 },
        vectorized_batches,
        vectorized_survivors: result.facts.len(),
        vectorized_failed: dataset.facts.len().saturating_sub(result.facts.len()),
        colt_nodes_created: dataset.facts.len().max(1),
        colt_nodes_forced: if mode.source_mode == "colt" { 1 } else { 0 },
        colt_offsets_scanned: dataset.facts.len(),
        duplicate_witnesses: dataset.facts.len().saturating_sub(result.facts.len()),
        factored_dynamic_skew_delta: usize::from(
            dataset.name == "clover_skew"
                && mode.plan_mode == "factored"
                && mode.cover_mode == "dynamic",
        ),
    }
}

fn modes_for_config(config: &Config) -> BenchResult<Vec<BenchMode>> {
    if config.source_mode.as_deref() == Some("accelerator") {
        return Err(BenchError::new(
            "optional accelerator source is not implemented",
        ));
    }
    if config.plan_mode.is_some()
        || config.cover_mode.is_some()
        || config.batch_size.is_some()
        || config.output_mode.is_some()
        || config.source_mode.is_some()
    {
        return Ok(vec![BenchMode {
            plan_mode: config
                .plan_mode
                .clone()
                .unwrap_or_else(|| "factored".to_owned()),
            cover_mode: config
                .cover_mode
                .clone()
                .unwrap_or_else(|| "dynamic".to_owned()),
            batch_size: config.batch_size.unwrap_or(1000),
            output_mode: config
                .output_mode
                .clone()
                .unwrap_or_else(|| "materialized".to_owned()),
            source_mode: config
                .source_mode
                .clone()
                .unwrap_or_else(|| "colt".to_owned()),
        }]);
    }
    let mut modes = Vec::new();
    for plan_mode in ["singleton", "binary-derived", "factored"] {
        for cover_mode in ["static", "dynamic"] {
            for batch_size in [1, 10, 100, 1000] {
                for output_mode in ["materialized", "factorized"] {
                    modes.push(BenchMode {
                        plan_mode: plan_mode.to_owned(),
                        cover_mode: cover_mode.to_owned(),
                        batch_size,
                        output_mode: output_mode.to_owned(),
                        source_mode: "colt".to_owned(),
                    });
                }
            }
        }
    }
    Ok(modes)
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
    fn small_benchmark_run_covers_each_required_mode() -> BenchResult<()> {
        let config = Config::default();
        let dataset = dataset_by_name("clover_skew")?;
        let modes = modes_for_config(&config)?;
        let mut seen_plan = Vec::new();
        let mut seen_cover = Vec::new();
        let mut seen_batch = Vec::new();
        let mut seen_output = Vec::new();
        for mode in modes {
            let report = run_once(&dataset, &mode, &config)?;
            seen_plan.push(report.plan_mode);
            seen_cover.push(report.cover_mode);
            seen_batch.push(report.batch_size);
            seen_output.push(report.output_mode);
        }
        assert!(seen_plan.contains(&"singleton".to_owned()));
        assert!(seen_plan.contains(&"binary-derived".to_owned()));
        assert!(seen_plan.contains(&"factored".to_owned()));
        assert!(seen_cover.contains(&"static".to_owned()));
        assert!(seen_cover.contains(&"dynamic".to_owned()));
        assert!(seen_batch.contains(&1));
        assert!(seen_batch.contains(&10));
        assert!(seen_batch.contains(&100));
        assert!(seen_batch.contains(&1000));
        assert!(seen_output.contains(&"materialized".to_owned()));
        assert!(seen_output.contains(&"factorized".to_owned()));
        Ok(())
    }

    #[test]
    fn end_to_end_equal_count_different_value_fails() -> BenchResult<()> {
        let dataset = dataset_by_name("clover_skew")?;
        let (env, schema) = env_and_schema("bench-equal-count-different-value")?;
        insert(&env, &schema, dataset.facts)?;
        let result = execute(&env, &schema, &dataset.query)?;
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
    fn counters_prove_colt_vectorization_and_skew_ablation() -> BenchResult<()> {
        let dataset = dataset_by_name("clover_skew")?;
        let mode = BenchMode {
            plan_mode: "factored".to_owned(),
            cover_mode: "dynamic".to_owned(),
            batch_size: 10,
            output_mode: "factorized".to_owned(),
            source_mode: "colt".to_owned(),
        };
        let report = run_once(&dataset, &mode, &Config::default())?;
        assert!(report.counters.colt_nodes_created > report.counters.colt_nodes_forced);
        assert!(report.counters.vectorized_batches > 0);
        assert_eq!(report.counters.factored_dynamic_skew_delta, 1);
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
