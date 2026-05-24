#![allow(clippy::result_large_err)]

mod load;
mod queries;
mod schema;
mod sqlite;

use std::path::PathBuf;
use std::time::Instant;

use bumbledb_lmdb::{Environment, InputBindings, StorageSchema};

use crate::cli::Config;
use crate::report::{BenchmarkReport, Counters, fingerprint_rows};
use crate::runner::{BenchError, BenchResult};

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
        let expected = sqlite::query_sqlite(&sqlite_db, query.sqlite).map_err(BenchError::new)?;
        for _ in 0..config.warmup {
            let _ = env.read(|txn| {
                txn.execute_query(&bench_schema, &query.query, &InputBindings::new())
            })?;
        }
        for _ in 0..config.repeats.max(1) {
            let start = Instant::now();
            let result = env.read(|txn| {
                txn.execute_query(&bench_schema, &query.query, &InputBindings::new())
            })?;
            let elapsed_nanos = start.elapsed().as_nanos();
            if result.facts != expected {
                return Err(BenchError::new(format!(
                    "JOB {} exact values differ: bumbledb={:?} sqlite={:?}",
                    query.name, result.facts, expected
                )));
            }
            reports.push(BenchmarkReport {
                scale: config.open_limit.unwrap_or(0) as u64,
                dataset: "job".to_owned(),
                query: query.name.to_owned(),
                plan_mode: "default".to_owned(),
                cover_mode: "dynamic".to_owned(),
                batch_size: 1,
                output_mode: "materialized".to_owned(),
                source_mode: "colt".to_owned(),
                git_commit: option_env!("GIT_COMMIT").unwrap_or("unknown").to_owned(),
                hardware: config
                    .hardware
                    .clone()
                    .unwrap_or_else(|| "unspecified".to_owned()),
                correctness_fingerprint: fingerprint_rows(&result.facts),
                gate_status: format!("passed load_nanos={load_nanos}"),
                elapsed_nanos,
                counters: Counters::default(),
            });
        }
    }
    let _ = std::fs::remove_dir_all(&path);
    let _ = std::fs::remove_file(&sqlite_db);
    Ok(reports)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::OutputFormat;

    #[test]
    fn job_schema_and_queries_build() -> BenchResult<()> {
        let schema = schema::job_schema();
        let queries =
            queries::job_queries(&schema).map_err(|error| BenchError::new(error.to_string()))?;
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
            plan_mode: None,
            cover_mode: None,
            batch_size: None,
            output_mode: None,
            source_mode: None,
            hardware: None,
            job_dir: Some(dir.to_string_lossy().to_string()),
            open_limit: None,
            queries: vec!["job_q01_top_production".to_owned()],
        };

        let reports = run_job(&config)?;

        std::fs::remove_dir_all(dir)?;
        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].dataset, "job");
        assert_eq!(reports[0].query, "job_q01_top_production");
        assert!(reports[0].gate_status.starts_with("passed"));
        Ok(())
    }
}
