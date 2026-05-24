#![allow(clippy::result_large_err)]

mod load;
mod queries;
mod schema;
mod sqlite;

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Instant;

use bumbledb_lmdb::diagnostics::{allocation_delta, allocation_snapshot};
use bumbledb_lmdb::{Environment, InputBindings, QueryExecutionOptions, StorageSchema};

use crate::cli::Config;
use crate::report::{BenchmarkReport, fingerprint_rows};
use crate::runner::{BenchError, BenchResult, trace_json_for_report};

pub(crate) fn run_job(config: &Config) -> BenchResult<Vec<BenchmarkReport>> {
    let job_dir = config
        .job_dir
        .clone()
        .or_else(|| std::env::var("BUMBLED_JOB_DIR").ok())
        .ok_or_else(|| BenchError::new("JOB preset requires --job-dir or BUMBLED_JOB_DIR"))?;
    let job_dir = PathBuf::from(job_dir);
    let schema = schema::job_schema();
    let queries =
        queries::job_queries(&schema).map_err(|error| BenchError::new(error.to_string()))?;
    let selected = if config.queries.is_empty() {
        queries
    } else {
        queries
            .into_iter()
            .filter(|query| config.queries.iter().any(|name| name == query.name))
            .collect()
    };
    if selected.is_empty() {
        return Err(BenchError::new("no JOB queries selected"));
    }

    let bench_schema = StorageSchema::new(schema, 511)?;
    let budgets = match &config.allocation_budget {
        Some(path) => Some(load_budgets(path)?),
        None => None,
    };
    if budgets.is_some() && !config.alloc_tracking {
        return Err(BenchError::new(
            "allocation budgets require --alloc on so allocation counts are measured",
        ));
    }
    let path = std::env::temp_dir().join(format!("bumbledb-job-bench-{}", std::process::id()));
    if path.exists() {
        std::fs::remove_dir_all(&path)?;
    }
    let env = Environment::open_with_schema(&path, &bench_schema)?;
    let facts = load::load_job_facts(&job_dir, config.open_limit).map_err(BenchError::new)?;
    let load_start = Instant::now();
    env.write(|txn| {
        for fact in facts.iter().cloned() {
            txn.insert(&bench_schema, fact)?;
        }
        Ok::<(), bumbledb_lmdb::Error>(())
    })?;
    let load_nanos = load_start.elapsed().as_nanos();

    let sqlite_db =
        std::env::temp_dir().join(format!("bumbledb-job-sqlite-{}.db", std::process::id()));
    sqlite::load_sqlite(&sqlite_db, &facts).map_err(BenchError::new)?;

    let mut reports = Vec::new();
    for query in &selected {
        for _ in 0..config.warmup {
            let _ = env.read(|txn| {
                txn.execute_query(&bench_schema, &query.query, &InputBindings::new())
            })?;
            let _ = sqlite::query_sqlite(&sqlite_db, query.sqlite).map_err(BenchError::new)?;
        }
        for _ in 0..config.repeats.max(1) {
            let sqlite_start = Instant::now();
            let expected =
                sqlite::query_sqlite(&sqlite_db, query.sqlite).map_err(BenchError::new)?;
            let sqlite_elapsed_nanos = sqlite_start.elapsed().as_nanos();
            let start = Instant::now();
            let alloc_start = allocation_snapshot();
            let profiled = env.read(|txn| {
                txn.execute_query_profiled(
                    &bench_schema,
                    &query.query,
                    &InputBindings::new(),
                    QueryExecutionOptions {
                        allocation_tracking: config.alloc_tracking,
                        ..QueryExecutionOptions::default()
                    },
                )
            })?;
            let trace_json =
                trace_json_for_report(config, query.name, reports.len(), &profiled.trace)?;
            let result = profiled.result;
            let alloc_delta = allocation_delta(alloc_start, allocation_snapshot());
            let elapsed_nanos = start.elapsed().as_nanos();
            if result.facts != expected {
                return Err(BenchError::new(format!(
                    "JOB {} exact values differ: bumbledb={:?} sqlite={:?}",
                    query.name, result.facts, expected
                )));
            }
            let report = BenchmarkReport {
                scale: facts.len() as u64,
                dataset: "job".to_owned(),
                query: query.name.to_owned(),
                engine: "free_join".to_owned(),
                sqlite_reference: "exact SELECT DISTINCT".to_owned(),
                git_commit: option_env!("GIT_COMMIT").unwrap_or("unknown").to_owned(),
                hardware: config
                    .hardware
                    .clone()
                    .unwrap_or_else(|| "unspecified".to_owned()),
                correctness_fingerprint: fingerprint_rows(&result.facts),
                gate_status: "passed".to_owned(),
                elapsed_nanos,
                sqlite_elapsed_nanos,
                load_nanos,
                result_rows: result.facts.len(),
                allocation_tracking: config.alloc_tracking,
                alloc_calls: alloc_delta.alloc_calls,
                allocated_bytes: alloc_delta.allocated_bytes,
                deallocated_bytes: alloc_delta.deallocated_bytes,
                net_allocated_bytes: alloc_delta.net_bytes,
                trace_json,
            };
            if let Some(budgets) = &budgets {
                check_budget(&report, budgets)?;
            }
            reports.push(report);
        }
    }
    let _ = std::fs::remove_dir_all(&path);
    let _ = std::fs::remove_file(&sqlite_db);
    Ok(reports)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct AllocationBudget {
    alloc_calls: u64,
    allocated_bytes: u64,
}

fn load_budgets(path: &str) -> BenchResult<BTreeMap<String, AllocationBudget>> {
    let text = std::fs::read_to_string(path)?;
    let mut budgets = BTreeMap::new();
    for (line_number, line) in text.lines().enumerate() {
        if line_number == 0 || line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let fields = line.split('\t').collect::<Vec<_>>();
        if fields.len() != 3 {
            return Err(BenchError::new(format!(
                "invalid allocation budget line {} in {path}",
                line_number + 1
            )));
        }
        budgets.insert(
            fields[0].to_owned(),
            AllocationBudget {
                alloc_calls: parse_u64(fields[1], path, line_number + 1)?,
                allocated_bytes: parse_u64(fields[2], path, line_number + 1)?,
            },
        );
    }
    Ok(budgets)
}

fn check_budget(
    report: &BenchmarkReport,
    budgets: &BTreeMap<String, AllocationBudget>,
) -> BenchResult<()> {
    let Some(budget) = budgets.get(&report.query) else {
        return Err(BenchError::new(format!(
            "missing allocation budget for {}",
            report.query
        )));
    };
    if report.alloc_calls > budget.alloc_calls {
        return Err(BenchError::new(format!(
            "JOB {} allocation calls {} exceed budget {}",
            report.query, report.alloc_calls, budget.alloc_calls
        )));
    }
    if report.allocated_bytes > budget.allocated_bytes {
        return Err(BenchError::new(format!(
            "JOB {} allocated bytes {} exceed budget {}",
            report.query, report.allocated_bytes, budget.allocated_bytes
        )));
    }
    Ok(())
}

fn parse_u64(raw: &str, path: &str, line: usize) -> BenchResult<u64> {
    raw.parse()
        .map_err(|_| BenchError::new(format!("invalid integer at {path}:{line}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::{OutputFormat, TraceOutput};

    #[test]
    fn job_schema_and_queries_build() -> BenchResult<()> {
        let schema = schema::job_schema();
        let queries =
            queries::job_queries(&schema).map_err(|error| BenchError::new(error.to_string()))?;
        assert_eq!(queries.len(), 8);
        assert!(
            queries
                .iter()
                .any(|query| query.name == "job_q01_top_production")
        );
        assert!(
            queries
                .iter()
                .all(|query| query.sqlite.contains("SELECT DISTINCT"))
        );
        Ok(())
    }

    #[test]
    fn job_sample_runs_against_sqlite_exact_values() -> BenchResult<()> {
        let dir = std::env::temp_dir().join(format!("bumbledb-job-test-{}", std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
        std::fs::create_dir_all(&dir)?;
        for (file, contents) in [
            ("comp_cast_type.csv", "1,cast\n"),
            ("company_type.csv", "1,production companies\n"),
            ("info_type.csv", "1,top 250 rank\n"),
            ("kind_type.csv", "1,movie\n"),
            ("link_type.csv", "1,sequel\n"),
            ("role_type.csv", "1,actor\n"),
            ("keyword.csv", "1,hero,\n"),
            ("company_name.csv", "1,Acme,[us],0,,\n"),
            ("char_name.csv", "1,Hero,,0,,\n"),
            ("name.csv", "1,Jane,,0,m,,,\n"),
            ("title.csv", "1,Movie,,1,2012,0,,0,0,60,\n"),
            ("aka_name.csv", "1,1,Alias,,,,\n"),
            ("cast_info.csv", "1,1,1,1,,1,1\n"),
            ("movie_companies.csv", "1,1,1,1,\n"),
            ("movie_info.csv", "1,1,1,info,\n"),
            ("movie_info_idx.csv", "1,1,1,10,\n"),
            ("movie_keyword.csv", "1,1,1\n"),
            ("movie_link.csv", "1,1,1,1\n"),
        ] {
            std::fs::write(dir.join(file), contents)?;
        }
        let config = Config {
            preset: "job-sample".to_owned(),
            format: OutputFormat::Json,
            repeats: 1,
            warmup: 0,
            hardware: None,
            job_dir: Some(dir.to_string_lossy().to_string()),
            open_limit: None,
            queries: vec!["job_q01_top_production".to_owned()],
            alloc_tracking: false,
            allocation_budget: None,
            trace_output: TraceOutput::Inline,
            profile_query_label: None,
        };

        let reports = run_job(&config)?;

        std::fs::remove_dir_all(dir)?;
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].dataset, "job");
        assert_eq!(reports[0].query, "job_q01_top_production");
        assert!(reports[0].gate_status.starts_with("passed"));
        Ok(())
    }

    #[test]
    fn job_allocation_budget_detects_regression() {
        let report = BenchmarkReport {
            scale: 1,
            dataset: "job".to_owned(),
            query: "q".to_owned(),
            engine: "free_join".to_owned(),
            sqlite_reference: "exact SELECT DISTINCT".to_owned(),
            git_commit: "unknown".to_owned(),
            hardware: "test".to_owned(),
            correctness_fingerprint: "fingerprint".to_owned(),
            gate_status: "passed".to_owned(),
            elapsed_nanos: 1,
            sqlite_elapsed_nanos: 1,
            load_nanos: 1,
            result_rows: 1,
            allocation_tracking: true,
            alloc_calls: 11,
            allocated_bytes: 10,
            deallocated_bytes: 0,
            net_allocated_bytes: 10,
            trace_json: "{\"enabled\":false}".to_owned(),
        };
        let budgets = BTreeMap::from([(
            "q".to_owned(),
            AllocationBudget {
                alloc_calls: 10,
                allocated_bytes: 10,
            },
        )]);

        assert!(check_budget(&report, &budgets).is_err());
    }
}
