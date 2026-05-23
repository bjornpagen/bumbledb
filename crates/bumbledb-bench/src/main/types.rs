#[derive(Clone, Debug)]
struct BenchmarkGate {
    dataset: &'static str,
    query: &'static str,
    max_bumbledb_avg_micros: Option<u64>,
    max_sqlite_ratio: Option<f64>,
    max_lftj_next_calls: Option<u64>,
    max_materialized_values: Option<u64>,
}

#[derive(Clone, Debug)]
struct BenchmarkRunResult {
    dataset: &'static str,
    query: &'static str,
    facts: usize,
    correctness_mode: String,
    bumbledb_correctness_execution: Duration,
    sqlite_correctness_execution: Duration,
    bumbledb_cold_execution: Duration,
    sqlite_cold_execution: Duration,
    allocation_scope: String,
    query_image_scope: String,
    bumbledb_warmup: TimingStats,
    sqlite_warmup: TimingStats,
    bumbledb_samples: TimingStats,
    sqlite_samples: TimingStats,
    bumbledb_avg: Duration,
    sqlite_avg: Duration,
    sqlite_ratio: f64,
    query_image_sample_cache_hits: u64,
    sqlite_materialized_facts: bool,
    timings: QueryTimings,
    allocations: QueryAllocationStats,
    materialized_values: u64,
    dictionary_reverse_lookups: u64,
    counters: PlanCounters,
    final_output_values: u64,
    output_contains_dictionary_values: bool,
    query_image_build_micros: u128,
    query_image_built_during_query: bool,
    query_image_cache_cached_images: usize,
    query_image_cache_hits: u64,
    query_image_cache_misses: u64,
    query_image_cache_builds: u64,
    query_image_cache_build_micros: u64,
    planner_stats_cached_relations: usize,
    planner_stats_hits: u64,
    planner_stats_misses: u64,
    planner_stats_builds: u64,
    planner_stats_build_micros: u64,
    lftj_lazy_access_slices: u64,
    query_image_relation_count: usize,
    query_image_fact_count: usize,
    query_image_encoded_column_bytes: usize,
    gate: GateOutcome,
}

#[derive(Clone, Copy, Debug, Default)]
struct TimingStats {
    samples: u64,
    total: Duration,
    avg: Duration,
    min: Duration,
    p50: Duration,
    p95: Duration,
    max: Duration,
}

#[derive(Clone, Copy, Debug)]
struct QueryTimingSamples {
    bumbledb_correctness_execution: Duration,
    sqlite_correctness_execution: Duration,
    bumbledb_cold_execution: Duration,
    sqlite_cold_execution: Duration,
    bumbledb_warmup: TimingStats,
    sqlite_warmup: TimingStats,
    bumbledb_samples: TimingStats,
    sqlite_samples: TimingStats,
}

#[derive(Clone, Copy, Debug, Default)]
struct CacheHitStats {
    query_image_cache_hits: u64,
}

impl TimingStats {
    fn from_samples(mut samples: Vec<Duration>) -> Self {
        if samples.is_empty() {
            return Self::default();
        }
        samples.sort();
        let total = samples.iter().copied().sum::<Duration>();
        let sample_count = samples.len() as u64;
        Self {
            samples: sample_count,
            total,
            avg: duration_avg(total, sample_count),
            min: samples[0],
            p50: percentile(&samples, 50),
            p95: percentile(&samples, 95),
            max: samples[samples.len() - 1],
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct QueryImageBenchStats {
    relation_count: usize,
    fact_count: usize,
    encoded_column_bytes: usize,
    build_micros: u128,
}

impl QueryImageBenchStats {
    fn empty() -> Self {
        Self {
            relation_count: 0,
            fact_count: 0,
            encoded_column_bytes: 0,
            build_micros: 0,
        }
    }
}

#[derive(Clone, Debug)]
struct GateOutcome {
    passed: bool,
    notes: Vec<String>,
}

#[derive(Clone)]
pub(crate) enum SqlParam {
    I64(i64),
}

impl rusqlite::ToSql for SqlParam {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        match self {
            SqlParam::I64(value) => Ok((*value).into()),
        }
    }
}
