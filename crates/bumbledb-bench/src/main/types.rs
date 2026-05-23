use super::*;

#[derive(Clone, Debug)]
pub(super) struct BenchmarkGate {
    pub(super) dataset: &'static str,
    pub(super) query: &'static str,
    pub(super) max_bumbledb_avg_micros: Option<u64>,
    pub(super) max_sqlite_ratio: Option<f64>,
    pub(super) max_lftj_next_calls: Option<u64>,
    pub(super) max_materialized_values: Option<u64>,
}

#[derive(Clone, Debug)]
pub(super) struct BenchmarkRunResult {
    pub(super) dataset: &'static str,
    pub(super) query: &'static str,
    pub(super) facts: usize,
    pub(super) correctness_mode: String,
    pub(super) bumbledb_correctness_execution: Duration,
    pub(super) sqlite_correctness_execution: Duration,
    pub(super) bumbledb_cold_execution: Duration,
    pub(super) sqlite_cold_execution: Duration,
    pub(super) allocation_scope: String,
    pub(super) query_image_scope: String,
    pub(super) bumbledb_warmup: TimingStats,
    pub(super) sqlite_warmup: TimingStats,
    pub(super) bumbledb_samples: TimingStats,
    pub(super) sqlite_samples: TimingStats,
    pub(super) bumbledb_avg: Duration,
    pub(super) sqlite_avg: Duration,
    pub(super) sqlite_ratio: f64,
    pub(super) query_image_sample_cache_hits: u64,
    pub(super) sqlite_materialized_facts: bool,
    pub(super) timings: QueryTimings,
    pub(super) allocations: QueryAllocationStats,
    pub(super) materialized_values: u64,
    pub(super) dictionary_reverse_lookups: u64,
    pub(super) counters: PlanCounters,
    pub(super) final_output_values: u64,
    pub(super) output_contains_dictionary_values: bool,
    pub(super) query_image_built_during_query: bool,
    pub(super) query_image_cache_cached_images: usize,
    pub(super) query_image_cache_hits: u64,
    pub(super) query_image_cache_misses: u64,
    pub(super) query_image_cache_builds: u64,
    pub(super) query_image_cache_build_micros: u64,
    pub(super) planner_stats_cached_relations: usize,
    pub(super) planner_stats_hits: u64,
    pub(super) planner_stats_misses: u64,
    pub(super) planner_stats_builds: u64,
    pub(super) planner_stats_build_micros: u64,
    pub(super) lftj_lazy_access_slices: u64,
    pub(super) gate: GateOutcome,
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct TimingStats {
    pub(super) samples: u64,
    pub(super) total: Duration,
    pub(super) avg: Duration,
    pub(super) min: Duration,
    pub(super) p50: Duration,
    pub(super) p95: Duration,
    pub(super) max: Duration,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct QueryTimingSamples {
    pub(super) bumbledb_correctness_execution: Duration,
    pub(super) sqlite_correctness_execution: Duration,
    pub(super) bumbledb_cold_execution: Duration,
    pub(super) sqlite_cold_execution: Duration,
    pub(super) bumbledb_warmup: TimingStats,
    pub(super) sqlite_warmup: TimingStats,
    pub(super) bumbledb_samples: TimingStats,
    pub(super) sqlite_samples: TimingStats,
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct CacheHitStats {
    pub(super) query_image_cache_hits: u64,
}

impl TimingStats {
    pub(super) fn from_samples(mut samples: Vec<Duration>) -> Self {
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

#[derive(Clone, Debug)]
pub(super) struct GateOutcome {
    pub(super) passed: bool,
    pub(super) notes: Vec<String>,
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
