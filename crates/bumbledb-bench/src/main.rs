use std::fmt::Write as _;
use std::fs::File;
use std::hint::black_box;
use std::io::Write as IoWrite;
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use bumbledb_core::datalog::parse_and_typecheck;
use bumbledb_core::encoding::{DecimalRaw, TimestampMicros};
use bumbledb_core::schema::{
    FieldDescriptor, IndexDescriptor, PrimaryKeyDescriptor, RelationDescriptor, RelationKind,
    SchemaDescriptor, ValueType,
};
use bumbledb_lmdb::{
    AllocationPhaseStats, Environment, InputBindings, PlanCounters, QueryAllocationStats,
    QueryOutput, QueryTimings, ResultColumn, Row, StorageSchema, Value,
};
use rusqlite::{Connection, params_from_iter};

mod open;

#[cfg(feature = "alloc-profile")]
mod alloc_profile {
    use std::alloc::{GlobalAlloc, Layout, System};

    pub struct CountingAllocator;

    // SAFETY: this allocator forwards all operations to the standard system
    // allocator and only records successful operations with lock-free atomics.
    unsafe impl GlobalAlloc for CountingAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            // SAFETY: forwarding the exact layout to the system allocator.
            let ptr = unsafe { System.alloc(layout) };
            if !ptr.is_null() {
                bumbledb_lmdb::allocation::record_alloc(layout.size());
            }
            ptr
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            bumbledb_lmdb::allocation::record_dealloc(layout.size());
            // SAFETY: forwarding the original pointer and layout to the system allocator.
            unsafe { System.dealloc(ptr, layout) };
        }

        unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
            // SAFETY: forwarding the original pointer, layout, and requested new size.
            let new_ptr = unsafe { System.realloc(ptr, layout, new_size) };
            if !new_ptr.is_null() {
                bumbledb_lmdb::allocation::record_realloc(layout.size(), new_size);
            }
            new_ptr
        }
    }
}

#[cfg(feature = "alloc-profile")]
#[global_allocator]
static GLOBAL_ALLOCATOR: alloc_profile::CountingAllocator = alloc_profile::CountingAllocator;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let Some(config) = Config::from_env()? else {
        return Ok(());
    };
    if config.trace {
        init_tracing(&config)?;
    }
    if !config.format.is_json_only() {
        println!("BumbleDB benchmark suite");
        println!(
            "scale={} repeats={} warmup={} datasets={:?} queries={:?} open_datasets={}",
            config.scale,
            config.repeats,
            config.warmup,
            config.datasets,
            config.queries,
            config.has_open_datasets()
        );
        println!();
    }

    let mut datasets = all_datasets(config.scale);
    datasets.extend(open::open_datasets(&config)?);

    let datasets = datasets
        .into_iter()
        .filter(|dataset| {
            config.datasets.is_empty() || config.datasets.iter().any(|name| name == dataset.name)
        })
        .collect::<Vec<_>>();

    if datasets.is_empty() {
        return Err("no matching datasets".into());
    }

    let mut results = Vec::new();
    for dataset in datasets {
        results.extend(run_dataset(dataset, &config)?);
        if !config.format.is_json_only() {
            println!();
        }
    }

    if results.is_empty() {
        return Err(bench_error("no matching queries"));
    }

    if config.format.includes_markdown() {
        println!("{}", render_markdown_results(&results));
    }
    if config.format.includes_json() {
        println!("{}", render_json_results(&results));
    }

    if config.fail_gates {
        let failures = results
            .iter()
            .filter(|result| !result.gate.passed)
            .collect::<Vec<_>>();
        if !failures.is_empty() {
            return Err(format!("{} benchmark gate(s) failed", failures.len()).into());
        }
    }

    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum OutputFormat {
    Text,
    Markdown,
    Json,
    Both,
}

impl OutputFormat {
    fn includes_text(self) -> bool {
        matches!(self, OutputFormat::Text | OutputFormat::Both)
    }

    fn includes_markdown(self) -> bool {
        matches!(self, OutputFormat::Markdown | OutputFormat::Both)
    }

    fn includes_json(self) -> bool {
        matches!(self, OutputFormat::Json)
    }

    fn is_json_only(self) -> bool {
        self == OutputFormat::Json
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TraceFormat {
    Fmt,
    Json,
    Chrome,
    Flame,
}

#[derive(Debug)]
struct Config {
    scale: u64,
    repeats: u64,
    warmup: u64,
    datasets: Vec<String>,
    queries: Vec<String>,
    imdb_dir: Option<String>,
    tpch_dir: Option<String>,
    lahman_dir: Option<String>,
    ldbc_dir: Option<String>,
    trace: bool,
    trace_output: Option<String>,
    trace_format: TraceFormat,
    format: OutputFormat,
    fail_gates: bool,
}

impl Config {
    fn from_env() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        Self::from_args(std::env::args().skip(1))
    }

    fn from_args(
        args: impl IntoIterator<Item = String>,
    ) -> Result<Option<Self>, Box<dyn std::error::Error>> {
        let mut scale = 200;
        let mut repeats = 10;
        let mut warmup = 0;
        let mut datasets = Vec::new();
        let mut queries = Vec::new();
        let mut imdb_dir = None;
        let mut tpch_dir = None;
        let mut lahman_dir = None;
        let mut ldbc_dir = None;
        let mut trace = false;
        let mut trace_output = None;
        let mut trace_format = TraceFormat::Fmt;
        let mut format = OutputFormat::Text;
        let mut fail_gates = false;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--scale" => {
                    scale = next_arg(&mut args, "--scale")?
                        .parse()
                        .map_err(|error| bench_error(format!("invalid --scale: {error}")))?
                }
                "--repeats" => {
                    repeats = next_arg(&mut args, "--repeats")?
                        .parse()
                        .map_err(|error| bench_error(format!("invalid --repeats: {error}")))?
                }
                "--warmup" => {
                    warmup = next_arg(&mut args, "--warmup")?
                        .parse()
                        .map_err(|error| bench_error(format!("invalid --warmup: {error}")))?
                }
                "--dataset" => datasets.push(next_arg(&mut args, "--dataset")?),
                "--query" => queries.push(next_arg(&mut args, "--query")?),
                "--imdb-dir" => imdb_dir = Some(next_arg(&mut args, "--imdb-dir")?),
                "--tpch-dir" => tpch_dir = Some(next_arg(&mut args, "--tpch-dir")?),
                "--lahman-dir" => lahman_dir = Some(next_arg(&mut args, "--lahman-dir")?),
                "--ldbc-dir" => ldbc_dir = Some(next_arg(&mut args, "--ldbc-dir")?),
                "--trace" => trace = true,
                "--trace-output" => {
                    trace = true;
                    trace_output = Some(next_arg(&mut args, "--trace-output")?);
                }
                "--trace-format" => {
                    trace = true;
                    trace_format = match next_arg(&mut args, "--trace-format")?.as_str() {
                        "fmt" => TraceFormat::Fmt,
                        "json" => TraceFormat::Json,
                        "chrome" => TraceFormat::Chrome,
                        "flame" => TraceFormat::Flame,
                        other => {
                            return Err(bench_error(format!("unknown --trace-format {other}")));
                        }
                    };
                }
                "--format" => {
                    format = match next_arg(&mut args, "--format")?.as_str() {
                        "text" => OutputFormat::Text,
                        "markdown" => OutputFormat::Markdown,
                        "json" => OutputFormat::Json,
                        "both" => OutputFormat::Both,
                        other => return Err(bench_error(format!("unknown --format {other}"))),
                    }
                }
                "--markdown" => format = OutputFormat::Markdown,
                "--json" => format = OutputFormat::Json,
                "--fail-gates" => fail_gates = true,
                "--help" | "-h" => {
                    println!(
                        "usage: cargo run -p bumbledb-bench --release -- [--scale N] [--repeats N] [--warmup N] [--query NAME] [--trace] [--trace-output PATH] [--trace-format fmt|json|chrome|flame] [--format text|markdown|json|both] [--markdown] [--json] [--fail-gates] [--dataset ledger|sailors|joinstress|tpch|imdb|tpch-open|lahman|ldbc] [--imdb-dir DIR] [--tpch-dir DIR] [--lahman-dir DIR] [--ldbc-dir DIR]"
                    );
                    return Ok(None);
                }
                other => return Err(bench_error(format!("unknown arg {other}"))),
            }
        }
        Ok(Some(Self {
            scale,
            repeats,
            warmup,
            datasets,
            queries,
            imdb_dir,
            tpch_dir,
            lahman_dir,
            ldbc_dir,
            trace,
            trace_output,
            trace_format,
            format,
            fail_gates,
        }))
    }

    fn has_open_datasets(&self) -> bool {
        self.imdb_dir.is_some()
            || self.tpch_dir.is_some()
            || self.lahman_dir.is_some()
            || self.ldbc_dir.is_some()
    }
}

fn init_tracing(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "bumbledb_lmdb=debug".to_owned());
    match config.trace_format {
        TraceFormat::Fmt => {
            if let Some(path) = &config.trace_output {
                let writer = SharedTraceWriter::create(path)?;
                tracing_subscriber::fmt()
                    .with_env_filter(filter)
                    .with_target(true)
                    .with_writer(writer)
                    .try_init()
                    .map_err(|error| {
                        bench_error(format!("failed to initialize tracing: {error}"))
                    })?;
            } else {
                tracing_subscriber::fmt()
                    .with_env_filter(filter)
                    .with_target(true)
                    .try_init()
                    .map_err(|error| {
                        bench_error(format!("failed to initialize tracing: {error}"))
                    })?;
            }
        }
        TraceFormat::Json => {
            if let Some(path) = &config.trace_output {
                let writer = SharedTraceWriter::create(path)?;
                tracing_subscriber::fmt()
                    .json()
                    .with_env_filter(filter)
                    .with_target(true)
                    .with_writer(writer)
                    .try_init()
                    .map_err(|error| {
                        bench_error(format!("failed to initialize tracing: {error}"))
                    })?;
            } else {
                tracing_subscriber::fmt()
                    .json()
                    .with_env_filter(filter)
                    .with_target(true)
                    .try_init()
                    .map_err(|error| {
                        bench_error(format!("failed to initialize tracing: {error}"))
                    })?;
            }
        }
        TraceFormat::Chrome | TraceFormat::Flame => {
            return Err(bench_error(
                "trace format requires an optional profiler dependency that is not enabled",
            ));
        }
    }
    Ok(())
}

#[derive(Clone)]
struct SharedTraceWriter {
    file: Arc<Mutex<File>>,
}

impl SharedTraceWriter {
    fn create(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        Ok(Self {
            file: Arc::new(Mutex::new(File::create(path)?)),
        })
    }
}

struct SharedTraceWriterGuard<'a> {
    file: MutexGuard<'a, File>,
}

impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for SharedTraceWriter {
    type Writer = SharedTraceWriterGuard<'a>;

    fn make_writer(&'a self) -> Self::Writer {
        SharedTraceWriterGuard {
            file: self
                .file
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        }
    }
}

impl IoWrite for SharedTraceWriterGuard<'_> {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.file.flush()
    }
}

fn next_arg(
    args: &mut impl Iterator<Item = String>,
    flag: &'static str,
) -> Result<String, Box<dyn std::error::Error>> {
    args.next()
        .ok_or_else(|| bench_error(format!("missing value for {flag}")))
}

pub(crate) fn bench_error(message: impl Into<String>) -> Box<dyn std::error::Error> {
    Box::new(std::io::Error::new(
        std::io::ErrorKind::InvalidInput,
        message.into(),
    ))
}

pub(crate) struct Dataset {
    name: &'static str,
    schema: SchemaDescriptor,
    rows: Vec<Row>,
    sqlite_schema: &'static str,
    sqlite_insert: SqliteInsert,
    queries: Vec<BenchQuery>,
}

pub(crate) type SqliteInsert = fn(&Connection, &[Row]) -> Result<(), Box<dyn std::error::Error>>;

pub(crate) struct BenchQuery {
    name: &'static str,
    datalog: &'static str,
    inputs: Vec<(&'static str, Value)>,
    sqlite: &'static str,
    sqlite_params: Vec<SqlParam>,
}

#[derive(Clone, Debug)]
struct BenchmarkGate {
    dataset: &'static str,
    query: &'static str,
    max_bumbledb_avg_micros: Option<u64>,
    max_sqlite_ratio: Option<f64>,
    max_iterator_ops: Option<u64>,
    max_materialized_values: Option<u64>,
}

#[derive(Clone, Debug)]
struct BenchmarkRunResult {
    dataset: &'static str,
    query: &'static str,
    rows: usize,
    bumbledb_prepare: Duration,
    sqlite_prepare: Duration,
    bumbledb_warmup: TimingStats,
    sqlite_warmup: TimingStats,
    bumbledb_samples: TimingStats,
    sqlite_samples: TimingStats,
    bumbledb_avg: Duration,
    sqlite_avg: Duration,
    sqlite_ratio: f64,
    chosen_plan: String,
    runtime_kind: String,
    timings: QueryTimings,
    allocations: QueryAllocationStats,
    iterator_ops: u64,
    hash_build_rows: u64,
    hash_probe_rows: u64,
    materialized_values: u64,
    dictionary_reverse_lookups: u64,
    counters: PlanCounters,
    final_output_values: u64,
    output_contains_dictionary_values: bool,
    query_image_build_micros: u128,
    query_image_segment_count: usize,
    query_image_segment_bytes: usize,
    query_image_built_from_segments: bool,
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
    sorted_trie_cache_hits: u64,
    sorted_trie_cache_misses: u64,
    sorted_trie_builds: u64,
    atom_temp_relation_builds: u64,
    hash_probe_calls: u64,
    hash_probe_hits: u64,
    hash_probe_misses: u64,
    hash_rows_returned: u64,
    hash_distinct_emits: u64,
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
    bumbledb_prepare: Duration,
    sqlite_prepare: Duration,
    bumbledb_warmup: TimingStats,
    sqlite_warmup: TimingStats,
    bumbledb_samples: TimingStats,
    sqlite_samples: TimingStats,
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
    build_micros: u128,
    segment_count: usize,
    segment_bytes: usize,
    built_from_segments: bool,
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

fn run_dataset(
    dataset: Dataset,
    config: &Config,
) -> Result<Vec<BenchmarkRunResult>, Box<dyn std::error::Error>> {
    let selected_queries = dataset
        .queries
        .into_iter()
        .filter(|query| {
            config.queries.is_empty() || config.queries.iter().any(|name| name == query.name)
        })
        .collect::<Vec<_>>();
    if selected_queries.is_empty() {
        return Ok(Vec::new());
    }

    let format = config.format;
    if format.includes_text() {
        println!("== {} ==", dataset.name);
        println!("rows={}", dataset.rows.len());
        println!("queries={}", selected_queries.len());
    }

    let bumble_dir = tempfile::tempdir()?;
    let bumble_env = Environment::open(bumble_dir.path())?;
    let bumble_schema = StorageSchema::new(dataset.schema.clone(), bumble_env.max_key_size())?;

    let bumble_load = timed(|| bumble_env.bulk_load(&bumble_schema, dataset.rows.clone()))?;
    if format.includes_text() {
        println!("load.bumbledb={:?}", bumble_load.elapsed);
    }
    let query_image = bumble_env.query_image(&bumble_schema)?;
    let query_image_stats = QueryImageBenchStats {
        build_micros: query_image.stats().build_micros,
        segment_count: query_image.stats().segment_count,
        segment_bytes: query_image.stats().segment_bytes,
        built_from_segments: query_image.stats().built_from_segments,
    };
    if format.includes_text() {
        println!(
            "query_image relation_count={} row_count={} encoded_column_bytes={} segment_count={} segment_bytes={} built_from_segments={} build_micros={}",
            query_image.stats().relation_count,
            query_image.stats().row_count,
            query_image.stats().encoded_column_bytes,
            query_image.stats().segment_count,
            query_image.stats().segment_bytes,
            query_image.stats().built_from_segments,
            query_image.stats().build_micros,
        );
    }

    let mut sqlite = Connection::open_in_memory()?;
    sqlite.execute_batch(dataset.sqlite_schema)?;
    let sqlite_load = timed(|| (dataset.sqlite_insert)(&sqlite, &dataset.rows))?;
    if format.includes_text() {
        println!("load.sqlite={:?}", sqlite_load.elapsed);
    }

    let mut results = Vec::new();
    for query in selected_queries {
        let typed = parse_and_typecheck(bumble_schema.descriptor(), query.datalog)?;
        let inputs = InputBindings::from_values(query.inputs.clone());
        let params = query.sqlite_params.clone();

        let bumble_once =
            timed(|| bumble_env.read(|txn| txn.execute_query(&bumble_schema, &typed, &inputs)))?;
        let bumble_prepare = bumble_once.elapsed;
        let bumble_output = bumble_once.value;
        let sqlite_once = timed(|| sqlite_count(&mut sqlite, query.sqlite, &params))?;
        let sqlite_prepare = sqlite_once.elapsed;
        let sqlite_once = sqlite_once.value;
        if bumble_output.rows.len() != sqlite_once {
            return Err(format!(
                "{}:{} row mismatch bumbledb={} sqlite={}",
                dataset.name,
                query.name,
                bumble_output.rows.len(),
                sqlite_once
            )
            .into());
        }

        let bumble_warmup = timed_samples(config.warmup, || {
            let rows = bumble_env
                .read(|txn| txn.execute_query(&bumble_schema, &typed, &inputs))?
                .rows;
            black_box(rows.len());
            Ok::<_, bumbledb_lmdb::Error>(())
        })?;
        let sqlite_warmup = timed_samples(config.warmup, || {
            let rows = sqlite_count(&mut sqlite, query.sqlite, &params)?;
            black_box(rows);
            Ok::<_, Box<dyn std::error::Error>>(())
        })?;

        let bumble_samples = timed_samples(config.repeats, || {
            let rows = bumble_env
                .read(|txn| txn.execute_query(&bumble_schema, &typed, &inputs))?
                .rows;
            black_box(rows.len());
            Ok::<_, bumbledb_lmdb::Error>(())
        })?;
        let sqlite_samples = timed_samples(config.repeats, || {
            let rows = sqlite_count(&mut sqlite, query.sqlite, &params)?;
            black_box(rows);
            Ok::<_, Box<dyn std::error::Error>>(())
        })?;

        let result = benchmark_result(
            dataset.name,
            &query,
            &bumble_output,
            QueryTimingSamples {
                bumbledb_prepare: bumble_prepare,
                sqlite_prepare,
                bumbledb_warmup: bumble_warmup,
                sqlite_warmup,
                bumbledb_samples: bumble_samples,
                sqlite_samples,
            },
            query_image_stats,
        );
        emit_profile_summary(dataset.name, query.name, &bumble_output);
        if format.includes_text() {
            println!(
                "query={} rows={} bumbledb_prepare={:?} bumbledb_samples={} bumbledb_avg={:?} sqlite_prepare={:?} sqlite_samples={} sqlite_avg={:?} gate={}",
                query.name,
                bumble_output.rows.len(),
                bumble_prepare,
                result.bumbledb_samples.samples,
                result.bumbledb_avg,
                sqlite_prepare,
                result.sqlite_samples.samples,
                result.sqlite_avg,
                if result.gate.passed { "pass" } else { "fail" },
            );
            print_explain(&bumble_output.explain());
            for note in &result.gate.notes {
                println!("  gate_note: {note}");
            }
        }
        results.push(result);
    }

    Ok(results)
}

struct Timed<T> {
    value: T,
    elapsed: Duration,
}

fn timed<T, E>(f: impl FnOnce() -> Result<T, E>) -> Result<Timed<T>, E> {
    let start = Instant::now();
    let value = f()?;
    Ok(Timed {
        value,
        elapsed: start.elapsed(),
    })
}

fn timed_samples<E>(samples: u64, mut f: impl FnMut() -> Result<(), E>) -> Result<TimingStats, E> {
    let mut durations = Vec::with_capacity(samples.min(usize::MAX as u64) as usize);
    for _ in 0..samples {
        let start = Instant::now();
        f()?;
        durations.push(start.elapsed());
    }
    Ok(TimingStats::from_samples(durations))
}

fn duration_avg(duration: Duration, samples: u64) -> Duration {
    if samples == 0 {
        return Duration::ZERO;
    }
    let nanos = duration.as_nanos() / u128::from(samples);
    Duration::from_nanos(nanos.min(u128::from(u64::MAX)) as u64)
}

fn percentile(samples: &[Duration], percentile: u64) -> Duration {
    let index = ((samples.len() as u64 - 1) * percentile).div_ceil(100) as usize;
    samples[index]
}

fn sqlite_count(
    conn: &mut Connection,
    sql: &str,
    params: &[SqlParam],
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt
        .query_map(params_from_iter(params.iter()), |_| Ok(()))?
        .count();
    Ok(rows)
}

fn benchmark_result(
    dataset: &'static str,
    query: &BenchQuery,
    output: &QueryOutput,
    timing: QueryTimingSamples,
    query_image_stats: QueryImageBenchStats,
) -> BenchmarkRunResult {
    let final_output_values = (output.rows.len() * output.columns.len()) as u64;
    let output_contains_dictionary_values = output
        .rows
        .iter()
        .flatten()
        .any(|value| matches!(value, Value::String(_) | Value::Bytes(_)));
    let bumbledb_avg = timing.bumbledb_samples.avg;
    let sqlite_avg = timing.sqlite_samples.avg;
    let sqlite_ratio = duration_ratio(bumbledb_avg, sqlite_avg);
    let gate = evaluate_gate(
        dataset,
        query,
        output,
        bumbledb_avg,
        sqlite_ratio,
        final_output_values,
        output_contains_dictionary_values,
    );
    BenchmarkRunResult {
        dataset,
        query: query.name,
        rows: output.rows.len(),
        bumbledb_prepare: timing.bumbledb_prepare,
        sqlite_prepare: timing.sqlite_prepare,
        bumbledb_warmup: timing.bumbledb_warmup,
        sqlite_warmup: timing.sqlite_warmup,
        bumbledb_samples: timing.bumbledb_samples,
        sqlite_samples: timing.sqlite_samples,
        bumbledb_avg,
        sqlite_avg,
        sqlite_ratio,
        chosen_plan: output.plan.optimizer.chosen.clone(),
        runtime_kind: format!("{:?}", output.plan.runtime_kind),
        timings: output.plan.timings,
        allocations: output.plan.allocations,
        iterator_ops: output.plan.free_join.estimates.iterator_ops,
        hash_build_rows: output.plan.free_join.estimates.hash_build_rows,
        hash_probe_rows: output.plan.free_join.estimates.hash_probe_rows,
        materialized_values: output.plan.counters.materialized_output_values,
        dictionary_reverse_lookups: output.plan.counters.dictionary_reverse_lookups,
        counters: output.plan.counters.clone(),
        final_output_values,
        output_contains_dictionary_values,
        query_image_build_micros: query_image_stats.build_micros,
        query_image_segment_count: query_image_stats.segment_count,
        query_image_segment_bytes: query_image_stats.segment_bytes,
        query_image_built_from_segments: query_image_stats.built_from_segments,
        query_image_cache_cached_images: output.plan.query_image_cache.cached_images,
        query_image_cache_hits: output.plan.query_image_cache.hits,
        query_image_cache_misses: output.plan.query_image_cache.misses,
        query_image_cache_builds: output.plan.query_image_cache.builds,
        query_image_cache_build_micros: output.plan.query_image_cache.build_micros,
        planner_stats_cached_relations: output.plan.planner_stats.cached_relations,
        planner_stats_hits: output.plan.planner_stats.hits,
        planner_stats_misses: output.plan.planner_stats.misses,
        planner_stats_builds: output.plan.planner_stats.builds,
        planner_stats_build_micros: output.plan.planner_stats.build_micros,
        sorted_trie_cache_hits: output.plan.counters.sorted_trie_cache_hits,
        sorted_trie_cache_misses: output.plan.counters.sorted_trie_cache_misses,
        sorted_trie_builds: output.plan.counters.sorted_trie_builds,
        atom_temp_relation_builds: output.plan.counters.atom_temp_relation_builds,
        hash_probe_calls: output.plan.counters.hash_probe_calls,
        hash_probe_hits: output.plan.counters.hash_probe_hits,
        hash_probe_misses: output.plan.counters.hash_probe_misses,
        hash_rows_returned: output.plan.counters.hash_rows_returned,
        hash_distinct_emits: output.plan.counters.hash_distinct_emits,
        gate,
    }
}

fn emit_profile_summary(dataset: &str, query: &str, output: &QueryOutput) {
    let plan = &output.plan;
    let timings = plan.timings;
    tracing::debug!(
        dataset,
        query,
        rows = output.rows.len(),
        runtime = ?plan.runtime_kind,
        total_micros = timings.total_micros,
        plan_micros = timings.plan_micros,
        execute_micros = timings.execute_micros,
        sink_finish_micros = timings.sink_finish_micros,
        allocations_enabled = plan.allocations.enabled,
        "benchmark query profile"
    );
    for node in &plan.node_timings {
        tracing::debug!(
            dataset,
            query,
            node = node.node.0,
            implementation = ?node.implementation,
            estimated_rows = node.estimated_rows,
            actual_rows = node.actual_rows,
            execute_micros = node.execute_micros,
            "benchmark node profile"
        );
    }
}

fn evaluate_gate(
    dataset: &'static str,
    query: &BenchQuery,
    output: &QueryOutput,
    bumbledb_avg: Duration,
    sqlite_ratio: f64,
    final_output_values: u64,
    output_contains_dictionary_values: bool,
) -> GateOutcome {
    let mut passed = true;
    let mut notes = Vec::new();
    if let Some(gate) = benchmark_gate(dataset, query.name) {
        notes.push(format!("performance gate {}.{}", gate.dataset, gate.query));
        let avg_micros = duration_micros(bumbledb_avg);
        if let Some(max) = gate.max_bumbledb_avg_micros
            && avg_micros > u128::from(max)
        {
            passed = false;
            notes.push(format!("avg {avg_micros}us exceeds {max}us"));
        }
        if let Some(max) = gate.max_sqlite_ratio
            && sqlite_ratio > max
        {
            passed = false;
            notes.push(format!("sqlite ratio {sqlite_ratio:.2} exceeds {max:.2}"));
        }
        if let Some(max) = gate.max_iterator_ops
            && output.plan.free_join.estimates.iterator_ops > max
        {
            passed = false;
            notes.push(format!(
                "iterator_ops {} exceeds {max}",
                output.plan.free_join.estimates.iterator_ops
            ));
        }
        if let Some(max) = gate.max_materialized_values
            && output.plan.counters.materialized_output_values > max
        {
            passed = false;
            notes.push(format!(
                "materialized_output_values {} exceeds {max}",
                output.plan.counters.materialized_output_values
            ));
        }
    } else {
        notes.push("no performance gate configured for query".to_owned());
    }

    let counters = &output.plan.counters;
    if counters.cursor_seeks != 0 || counters.rows_scanned != 0 {
        passed = false;
        notes.push(format!(
            "LMDB scan counters nonzero: cursor_seeks={} rows_scanned={}",
            counters.cursor_seeks, counters.rows_scanned
        ));
    }
    if !output_contains_dictionary_values && counters.dictionary_reverse_lookups != 0 {
        passed = false;
        notes.push(format!(
            "dictionary_reverse_lookups {} without string/bytes output",
            counters.dictionary_reverse_lookups
        ));
    }
    let has_aggregate = output
        .columns
        .iter()
        .any(|column| matches!(column, ResultColumn::Aggregate { .. }));
    if !has_aggregate && counters.materialized_output_values != final_output_values {
        passed = false;
        notes.push(format!(
            "materialized_output_values {} != final output values {}",
            counters.materialized_output_values, final_output_values
        ));
    }
    if has_aggregate
        && query.datalog.contains("count(")
        && counters.materialized_output_values > final_output_values
    {
        passed = false;
        notes.push(format!(
            "count aggregate materialized {} values for {} final values",
            counters.materialized_output_values, final_output_values
        ));
    }

    if passed && notes.is_empty() {
        notes.push("all configured gates passed".to_owned());
    }
    GateOutcome { passed, notes }
}

fn benchmark_gate(dataset: &'static str, query: &'static str) -> Option<BenchmarkGate> {
    let gate = match (dataset, query) {
        ("joinstress", "triangle_count") => BenchmarkGate {
            dataset,
            query,
            max_bumbledb_avg_micros: Some(250_000),
            max_sqlite_ratio: None,
            max_iterator_ops: Some(1_000_000),
            max_materialized_values: Some(1),
        },
        ("ledger", "tag_lookup_join") => BenchmarkGate {
            dataset,
            query,
            max_bumbledb_avg_micros: Some(250_000),
            max_sqlite_ratio: None,
            max_iterator_ops: Some(2_000_000),
            max_materialized_values: None,
        },
        ("sailors", "red_boat_sailors") => BenchmarkGate {
            dataset,
            query,
            max_bumbledb_avg_micros: Some(250_000),
            max_sqlite_ratio: None,
            max_iterator_ops: Some(2_000_000),
            max_materialized_values: None,
        },
        ("tpch", "supplier_nation_orders") => BenchmarkGate {
            dataset,
            query,
            max_bumbledb_avg_micros: Some(250_000),
            max_sqlite_ratio: None,
            max_iterator_ops: Some(2_000_000),
            max_materialized_values: None,
        },
        _ => return None,
    };
    Some(gate)
}

fn render_markdown_results(results: &[BenchmarkRunResult]) -> String {
    let mut out = String::new();
    out.push_str("## Benchmark Results\n\n");
    out.push_str("| dataset | query | rows | bumbledb avg us | sqlite avg us | sqlite ratio | chosen plan | runtime | image build us | image segments | image segment bytes | built from segments | image cache images | image cache hits | image cache misses | image cache builds | image cache build us | planner stats cached | planner stats hits | planner stats misses | planner stats builds | planner stats build us | trie cache hits | trie cache misses | trie builds | atom temp builds | hash calls | hash hits | hash misses | hash rows | hash emits | iterator ops | hash build est | hash probe est | materialized | dict lookups | gate |\n");
    out.push_str("|---|---|---:|---:|---:|---:|---|---|---:|---:|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|\n");
    for result in results {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {:.2} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            markdown_escape(result.dataset),
            markdown_escape(result.query),
            result.rows,
            duration_micros(result.bumbledb_avg),
            duration_micros(result.sqlite_avg),
            result.sqlite_ratio,
            markdown_escape(&result.chosen_plan),
            markdown_escape(&result.runtime_kind),
            result.query_image_build_micros,
            result.query_image_segment_count,
            result.query_image_segment_bytes,
            result.query_image_built_from_segments,
            result.query_image_cache_cached_images,
            result.query_image_cache_hits,
            result.query_image_cache_misses,
            result.query_image_cache_builds,
            result.query_image_cache_build_micros,
            result.planner_stats_cached_relations,
            result.planner_stats_hits,
            result.planner_stats_misses,
            result.planner_stats_builds,
            result.planner_stats_build_micros,
            result.sorted_trie_cache_hits,
            result.sorted_trie_cache_misses,
            result.sorted_trie_builds,
            result.atom_temp_relation_builds,
            result.hash_probe_calls,
            result.hash_probe_hits,
            result.hash_probe_misses,
            result.hash_rows_returned,
            result.hash_distinct_emits,
            result.iterator_ops,
            result.hash_build_rows,
            result.hash_probe_rows,
            result.materialized_values,
            result.dictionary_reverse_lookups,
            if result.gate.passed { "pass" } else { "fail" },
        );
    }
    out.push_str("\n## Phase Timing\n\n");
    out.push_str("| dataset | query | runtime | total us | validate us | normalize us | encode us | image us | plan us | lftj build us | hash index us | execute us | lftj exec us | hash exec us | sink emit us | sink finish us | decode us |\n");
    out.push_str(
        "|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n",
    );
    for result in results {
        let timings = result.timings;
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            markdown_escape(result.dataset),
            markdown_escape(result.query),
            markdown_escape(&result.runtime_kind),
            timings.total_micros,
            timings.validate_inputs_micros,
            timings.normalize_micros,
            timings.encode_inputs_micros,
            timings.query_image_micros,
            timings.plan_micros,
            timings.lftj_build_micros,
            timings.hash_index_micros,
            timings.execute_micros,
            timings.lftj_execute_micros,
            timings.hash_execute_micros,
            timings.sink_emit_micros,
            timings.sink_finish_micros,
            timings.decode_micros,
        );
    }
    out.push_str("\n## Allocation Summary\n\n");
    out.push_str("| dataset | query | enabled | alloc calls | dealloc calls | realloc calls | bytes allocated | bytes deallocated | net bytes | current live bytes | peak live bytes |\n");
    out.push_str("|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|\n");
    for result in results {
        let allocations = result.allocations;
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            markdown_escape(result.dataset),
            markdown_escape(result.query),
            allocations.enabled,
            allocations.alloc_calls,
            allocations.dealloc_calls,
            allocations.realloc_calls,
            allocations.bytes_allocated,
            allocations.bytes_deallocated,
            allocations.net_bytes,
            allocations.current_live_bytes,
            allocations.peak_live_bytes,
        );
    }
    out.push_str("\n## Allocation Phase Detail\n\n");
    out.push_str("| dataset | query | phase | enabled | alloc calls | bytes allocated | net bytes | current live bytes | peak live bytes |\n");
    out.push_str("|---|---|---|---|---:|---:|---:|---:|---:|\n");
    for result in results {
        write_allocation_phase_row(&mut out, result, "total", result.allocations.total);
        write_allocation_phase_row(
            &mut out,
            result,
            "validate_inputs",
            result.allocations.validate_inputs,
        );
        write_allocation_phase_row(&mut out, result, "normalize", result.allocations.normalize);
        write_allocation_phase_row(
            &mut out,
            result,
            "encode_inputs",
            result.allocations.encode_inputs,
        );
        write_allocation_phase_row(
            &mut out,
            result,
            "query_image",
            result.allocations.query_image,
        );
        write_allocation_phase_row(&mut out, result, "plan", result.allocations.plan);
        write_allocation_phase_row(
            &mut out,
            result,
            "lftj_build",
            result.allocations.lftj_build,
        );
        write_allocation_phase_row(
            &mut out,
            result,
            "hash_index",
            result.allocations.hash_index,
        );
        write_allocation_phase_row(&mut out, result, "execute", result.allocations.execute);
        write_allocation_phase_row(
            &mut out,
            result,
            "sink_finish",
            result.allocations.sink_finish,
        );
    }
    out.push_str("\n## Distribution\n\n");
    out.push_str("| dataset | query | bumbledb prepare us | bumbledb warmup samples | bumbledb warmup avg us | bumbledb samples | bumbledb min us | bumbledb p50 us | bumbledb p95 us | bumbledb max us | sqlite prepare us | sqlite warmup samples | sqlite warmup avg us | sqlite samples | sqlite min us | sqlite p50 us | sqlite p95 us | sqlite max us |\n");
    out.push_str("|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|\n");
    for result in results {
        let bumble = result.bumbledb_samples;
        let sqlite = result.sqlite_samples;
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            markdown_escape(result.dataset),
            markdown_escape(result.query),
            duration_micros(result.bumbledb_prepare),
            result.bumbledb_warmup.samples,
            duration_micros(result.bumbledb_warmup.avg),
            bumble.samples,
            duration_micros(bumble.min),
            duration_micros(bumble.p50),
            duration_micros(bumble.p95),
            duration_micros(bumble.max),
            duration_micros(result.sqlite_prepare),
            result.sqlite_warmup.samples,
            duration_micros(result.sqlite_warmup.avg),
            sqlite.samples,
            duration_micros(sqlite.min),
            duration_micros(sqlite.p50),
            duration_micros(sqlite.p95),
            duration_micros(sqlite.max),
        );
    }
    out.push_str("\n## Interpretation Notes\n\n");
    out.push_str("| signal | interpretation |\n");
    out.push_str("|---|---|\n");
    out.push_str("| high image us | QueryImage acquisition or segment build bottleneck |\n");
    out.push_str(
        "| high plan us | stats, variable ordering, or Free Join optimization bottleneck |\n",
    );
    out.push_str("| high lftj/hash build us | cached index lookup/build or atom relation preparation bottleneck |\n");
    out.push_str("| high execute us | runtime traversal/probe bottleneck |\n");
    out.push_str(
        "| high sink finish us | projection, aggregation, sorting, or decode bottleneck |\n",
    );
    out.push_str(
        "| high allocation counts | rerun with alloc-profile and then use a deep heap profiler for callsites |\n",
    );
    out.push_str("\n## Counter Gates\n\n");
    out.push_str("| dataset | query | cursor seeks | rows scanned | final values | materialized values | dictionary output | dictionary lookups | notes |\n");
    out.push_str("|---|---|---:|---:|---:|---:|---|---:|---|\n");
    for result in results {
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            markdown_escape(result.dataset),
            markdown_escape(result.query),
            result.counters.cursor_seeks,
            result.counters.rows_scanned,
            result.final_output_values,
            result.materialized_values,
            result.output_contains_dictionary_values,
            result.dictionary_reverse_lookups,
            markdown_escape(&result.gate.notes.join("; ")),
        );
    }
    out
}

fn write_allocation_phase_row(
    out: &mut String,
    result: &BenchmarkRunResult,
    phase: &str,
    stats: AllocationPhaseStats,
) {
    let _ = writeln!(
        out,
        "| {} | {} | {} | {} | {} | {} | {} | {} | {} |",
        markdown_escape(result.dataset),
        markdown_escape(result.query),
        markdown_escape(phase),
        stats.enabled,
        stats.alloc_calls,
        stats.bytes_allocated,
        stats.net_bytes,
        stats.current_live_bytes,
        stats.peak_live_bytes,
    );
}

fn render_json_results(results: &[BenchmarkRunResult]) -> String {
    let mut out = String::new();
    out.push_str("{\"results\":[");
    for (index, result) in results.iter().enumerate() {
        if index > 0 {
            out.push(',');
        }
        let _ = write!(
            out,
            "{{\"dataset\":\"{}\",\"query\":\"{}\",\"rows\":{},\"chosen_plan\":\"{}\",\"runtime\":\"{}\",",
            json_escape(result.dataset),
            json_escape(result.query),
            result.rows,
            json_escape(&result.chosen_plan),
            json_escape(&result.runtime_kind),
        );
        write_timing_stats(&mut out, "bumbledb", result.bumbledb_samples);
        out.push(',');
        write_timing_stats(&mut out, "sqlite", result.sqlite_samples);
        let timings = result.timings;
        let allocations = result.allocations;
        let _ = write!(
            out,
            ",\"prepare\":{{\"bumbledb_us\":{},\"sqlite_us\":{}}},\"warmup\":{{\"bumbledb_samples\":{},\"bumbledb_avg_us\":{},\"sqlite_samples\":{},\"sqlite_avg_us\":{}}},\"phase_timing\":{{\"total_us\":{},\"validate_us\":{},\"normalize_us\":{},\"encode_us\":{},\"image_us\":{},\"plan_us\":{},\"lftj_build_us\":{},\"hash_index_us\":{},\"execute_us\":{},\"lftj_execute_us\":{},\"hash_execute_us\":{},\"sink_emit_us\":{},\"sink_finish_us\":{},\"decode_us\":{}}},\"allocations\":{{\"enabled\":{},\"alloc_calls\":{},\"dealloc_calls\":{},\"realloc_calls\":{},\"bytes_allocated\":{},\"bytes_deallocated\":{},\"net_bytes\":{},\"current_live_bytes\":{},\"peak_live_bytes\":{}",
            duration_micros(result.bumbledb_prepare),
            duration_micros(result.sqlite_prepare),
            result.bumbledb_warmup.samples,
            duration_micros(result.bumbledb_warmup.avg),
            result.sqlite_warmup.samples,
            duration_micros(result.sqlite_warmup.avg),
            timings.total_micros,
            timings.validate_inputs_micros,
            timings.normalize_micros,
            timings.encode_inputs_micros,
            timings.query_image_micros,
            timings.plan_micros,
            timings.lftj_build_micros,
            timings.hash_index_micros,
            timings.execute_micros,
            timings.lftj_execute_micros,
            timings.hash_execute_micros,
            timings.sink_emit_micros,
            timings.sink_finish_micros,
            timings.decode_micros,
            allocations.enabled,
            allocations.alloc_calls,
            allocations.dealloc_calls,
            allocations.realloc_calls,
            allocations.bytes_allocated,
            allocations.bytes_deallocated,
            allocations.net_bytes,
            allocations.current_live_bytes,
            allocations.peak_live_bytes,
        );
        out.push_str(",\"phases\":{");
        write_allocation_phase_json(&mut out, "total", allocations.total, true);
        write_allocation_phase_json(
            &mut out,
            "validate_inputs",
            allocations.validate_inputs,
            false,
        );
        write_allocation_phase_json(&mut out, "normalize", allocations.normalize, false);
        write_allocation_phase_json(&mut out, "encode_inputs", allocations.encode_inputs, false);
        write_allocation_phase_json(&mut out, "query_image", allocations.query_image, false);
        write_allocation_phase_json(&mut out, "plan", allocations.plan, false);
        write_allocation_phase_json(&mut out, "lftj_build", allocations.lftj_build, false);
        write_allocation_phase_json(&mut out, "hash_index", allocations.hash_index, false);
        write_allocation_phase_json(&mut out, "execute", allocations.execute, false);
        write_allocation_phase_json(&mut out, "lftj_execute", allocations.lftj_execute, false);
        write_allocation_phase_json(&mut out, "hash_execute", allocations.hash_execute, false);
        write_allocation_phase_json(&mut out, "sink_finish", allocations.sink_finish, false);
        out.push_str("},\"size_class_allocs\":[");
        for (index, count) in allocations.size_class_allocs.iter().enumerate() {
            if index > 0 {
                out.push(',');
            }
            let _ = write!(out, "{}", count);
        }
        let _ = write!(
            out,
            "]}},\"counters\":{{\"cursor_seeks\":{},\"rows_scanned\":{},\"dictionary_reverse_lookups\":{},\"materialized_output_values\":{}}},\"gate\":{{\"passed\":{},\"notes\":[",
            result.counters.cursor_seeks,
            result.counters.rows_scanned,
            result.dictionary_reverse_lookups,
            result.materialized_values,
            result.gate.passed,
        );
        for (note_index, note) in result.gate.notes.iter().enumerate() {
            if note_index > 0 {
                out.push(',');
            }
            let _ = write!(out, "\"{}\"", json_escape(note));
        }
        out.push_str("]}}");
    }
    out.push_str("]}");
    out
}

fn write_timing_stats(out: &mut String, name: &str, stats: TimingStats) {
    let _ = write!(
        out,
        "\"{}\":{{\"samples\":{},\"total_us\":{},\"avg_us\":{},\"min_us\":{},\"p50_us\":{},\"p95_us\":{},\"max_us\":{}}}",
        name,
        stats.samples,
        duration_micros(stats.total),
        duration_micros(stats.avg),
        duration_micros(stats.min),
        duration_micros(stats.p50),
        duration_micros(stats.p95),
        duration_micros(stats.max),
    );
}

fn write_allocation_phase_json(
    out: &mut String,
    name: &str,
    stats: AllocationPhaseStats,
    first: bool,
) {
    if !first {
        out.push(',');
    }
    let _ = write!(
        out,
        "\"{}\":{{\"enabled\":{},\"alloc_calls\":{},\"dealloc_calls\":{},\"realloc_calls\":{},\"bytes_allocated\":{},\"bytes_deallocated\":{},\"net_bytes\":{},\"current_live_bytes\":{},\"peak_live_bytes\":{}}}",
        json_escape(name),
        stats.enabled,
        stats.alloc_calls,
        stats.dealloc_calls,
        stats.realloc_calls,
        stats.bytes_allocated,
        stats.bytes_deallocated,
        stats.net_bytes,
        stats.current_live_bytes,
        stats.peak_live_bytes,
    );
}

fn duration_ratio(left: Duration, right: Duration) -> f64 {
    let right = right.as_nanos();
    if right == 0 {
        return f64::INFINITY;
    }
    left.as_nanos() as f64 / right as f64
}

fn duration_micros(duration: Duration) -> u128 {
    duration.as_micros()
}

fn markdown_escape(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn json_escape(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if ch.is_control() => {
                let _ = write!(out, "\\u{:04x}", ch as u32);
            }
            ch => out.push(ch),
        }
    }
    out
}

fn print_explain(explain: &str) {
    for line in explain.lines() {
        if line.contains("relation=")
            || line.contains("runtime_kind")
            || line.contains("query_timing")
            || line.contains("allocation_summary")
            || line.contains("variable_estimate")
            || line.contains("missing_index")
            || line.contains("query_image_cache")
            || line.contains("planner_stats")
            || line.contains("chosen_plan")
            || line.contains("candidate_plan")
            || line.contains("free_join_estimates")
            || line.contains("free_join_node")
            || line.contains("node_rows")
            || line.contains("node_timing")
            || line.contains("free_join_subatom")
            || line.contains("rows_scanned")
            || line.contains("cursor_seeks")
            || line.contains("trie_intersections")
            || line.contains("variable_candidates")
            || line.contains("decoded_values")
            || line.contains("dictionary_reverse_lookups")
            || line.contains("encoded_comparisons_evaluated")
            || line.contains("decoded_comparisons_evaluated")
            || line.contains("materialized_output_values")
            || line.contains("trie_open")
            || line.contains("trie_up")
            || line.contains("trie_next")
            || line.contains("trie_seek")
            || line.contains("trie_key_reads")
            || line.contains("sorted_trie_cache")
            || line.contains("sorted_trie_build")
            || line.contains("atom_temp_relation")
            || line.contains("hash_index")
            || line.contains("hash_probe")
            || line.contains("hash_rows")
            || line.contains("hash_distinct")
            || line.contains("output_rows")
        {
            println!("  {line}");
        }
    }
}

fn all_datasets(scale: u64) -> Vec<Dataset> {
    vec![
        ledger_dataset(scale),
        sailors_dataset(scale),
        join_stress_dataset(scale),
        tpch_dataset(scale),
    ]
}

fn ledger_dataset(scale: u64) -> Dataset {
    Dataset {
        name: "ledger",
        schema: bumbledb_lmdb::benchmark::benchmark_schema(),
        rows: bumbledb_lmdb::benchmark::benchmark_rows(scale),
        sqlite_schema: r#"
            CREATE TABLE holder (id INTEGER PRIMARY KEY, name TEXT NOT NULL);
            CREATE TABLE account (id INTEGER PRIMARY KEY, holder INTEGER NOT NULL, currency INTEGER NOT NULL);
            CREATE TABLE instrument (id INTEGER PRIMARY KEY, symbol TEXT NOT NULL);
            CREATE TABLE journal_entry (id INTEGER PRIMARY KEY, source INTEGER NOT NULL, created_at INTEGER NOT NULL);
            CREATE TABLE posting (id INTEGER PRIMARY KEY, entry INTEGER NOT NULL, account INTEGER NOT NULL, instrument INTEGER NOT NULL, amount INTEGER NOT NULL, at INTEGER NOT NULL);
            CREATE TABLE posting_tag (posting INTEGER NOT NULL, tag INTEGER NOT NULL, PRIMARY KEY (posting, tag));
            CREATE INDEX account_holder ON account(holder, id);
            CREATE INDEX posting_account ON posting(account, id);
            CREATE INDEX posting_at ON posting(at, id);
            CREATE INDEX posting_instrument ON posting(instrument, id);
            CREATE INDEX posting_tag_tag ON posting_tag(tag, posting);
        "#,
        sqlite_insert: insert_ledger_sqlite,
        queries: vec![
            BenchQuery {
                name: "postings_for_holder_range",
                datalog: r#"
                    find ?posting ?amount
                    where
                      Posting(id: ?posting, account: ?account, amount: ?amount, at: ?t)
                      Account(id: ?account, holder: $holder)
                      ?t >= $start
                      ?t < $end
                "#,
                inputs: vec![
                    ("holder", Value::Ref(1)),
                    ("start", Value::Timestamp(TimestampMicros(0))),
                    (
                        "end",
                        Value::Timestamp(TimestampMicros((scale as i64 * 3 + 1) * 10)),
                    ),
                ],
                sqlite: r#"
                    SELECT p.id, p.amount FROM posting p
                    JOIN account a ON a.id = p.account
                    WHERE a.holder = ?1 AND p.at >= ?2 AND p.at < ?3
                "#,
                sqlite_params: vec![
                    SqlParam::I64(1),
                    SqlParam::I64(0),
                    SqlParam::I64((scale as i64 * 3 + 1) * 10),
                ],
            },
            BenchQuery {
                name: "balances_by_instrument",
                datalog: r#"
                    find ?instrument sum(?amount)
                    where
                      Posting(id: ?posting, account: ?account, instrument: ?instrument, amount: ?amount, at: ?t)
                      Account(id: ?account, holder: $holder)
                "#,
                inputs: vec![("holder", Value::Ref(1))],
                sqlite: r#"
                    SELECT p.instrument, SUM(p.amount) FROM posting p
                    JOIN account a ON a.id = p.account
                    WHERE a.holder = ?1
                    GROUP BY p.instrument
                "#,
                sqlite_params: vec![SqlParam::I64(1)],
            },
            BenchQuery {
                name: "tag_lookup_join",
                datalog: r#"
                    find ?posting ?account
                    where
                      PostingTag(posting: ?posting, tag: $tag)
                      Posting(id: ?posting, account: ?account)
                "#,
                inputs: vec![("tag", Value::Symbol(1))],
                sqlite: r#"
                    SELECT p.id, p.account FROM posting_tag t
                    JOIN posting p ON p.id = t.posting
                    WHERE t.tag = ?1
                "#,
                sqlite_params: vec![SqlParam::I64(1)],
            },
        ],
    }
}

fn sailors_dataset(scale: u64) -> Dataset {
    let sailors = scale.max(10);
    Dataset {
        name: "sailors",
        schema: SchemaDescriptor::new(
            "SailorsDb",
            vec![
                RelationDescriptor::new(
                    "Sailor",
                    RelationKind::Entity,
                    vec![
                        id_field("SailorId", "Sailor"),
                        FieldDescriptor::new("name", ValueType::String),
                        FieldDescriptor::new("rating", ValueType::U64).range_indexed(),
                        FieldDescriptor::new("age", ValueType::I64),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                ),
                RelationDescriptor::new(
                    "Boat",
                    RelationKind::Entity,
                    vec![
                        id_field("BoatId", "Boat"),
                        FieldDescriptor::new("name", ValueType::String),
                        FieldDescriptor::new(
                            "color",
                            ValueType::Symbol {
                                name: "Color".to_owned(),
                            },
                        ),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                )
                .with_index(IndexDescriptor::equality("by_color", ["color", "id"])),
                RelationDescriptor::new(
                    "Reserve",
                    RelationKind::Edge,
                    vec![
                        ref_field("SailorId", "sailor", "Sailor"),
                        ref_field("BoatId", "boat", "Boat"),
                        FieldDescriptor::new("day", ValueType::TimestampMicros).range_indexed(),
                    ],
                    PrimaryKeyDescriptor::new(["sailor", "boat", "day"]),
                ),
            ],
        ),
        rows: sailors_rows(sailors),
        sqlite_schema: r#"
            CREATE TABLE sailor (id INTEGER PRIMARY KEY, name TEXT NOT NULL, rating INTEGER NOT NULL, age INTEGER NOT NULL);
            CREATE TABLE boat (id INTEGER PRIMARY KEY, name TEXT NOT NULL, color INTEGER NOT NULL);
            CREATE TABLE reserve (sailor INTEGER NOT NULL, boat INTEGER NOT NULL, day INTEGER NOT NULL, PRIMARY KEY (sailor, boat, day));
            CREATE INDEX sailor_rating ON sailor(rating, id);
            CREATE INDEX boat_color ON boat(color, id);
            CREATE INDEX reserve_sailor ON reserve(sailor, boat, day);
            CREATE INDEX reserve_boat ON reserve(boat, sailor, day);
            CREATE INDEX reserve_day ON reserve(day, sailor, boat);
        "#,
        sqlite_insert: insert_sailors_sqlite,
        queries: vec![
            BenchQuery {
                name: "red_boat_sailors",
                datalog: r#"
                    find ?sailor ?rating
                    where
                      Reserve(sailor: ?sailor, boat: ?boat)
                      Boat(id: ?boat, color: $color)
                      Sailor(id: ?sailor, rating: ?rating)
                "#,
                inputs: vec![("color", Value::Symbol(1))],
                sqlite: r#"
                    SELECT DISTINCT s.id, s.rating FROM reserve r
                    JOIN boat b ON b.id = r.boat
                    JOIN sailor s ON s.id = r.sailor
                    WHERE b.color = ?1
                "#,
                sqlite_params: vec![SqlParam::I64(1)],
            },
            BenchQuery {
                name: "sailor_range_reserves",
                datalog: r#"
                    find ?boat ?day
                    where
                      Reserve(sailor: $sailor, boat: ?boat, day: ?day)
                      ?day >= $start
                      ?day < $end
                "#,
                inputs: vec![
                    ("sailor", Value::Ref(1)),
                    ("start", Value::Timestamp(TimestampMicros(0))),
                    ("end", Value::Timestamp(TimestampMicros(10_000_000))),
                ],
                sqlite: "SELECT boat, day FROM reserve WHERE sailor = ?1 AND day >= ?2 AND day < ?3",
                sqlite_params: vec![
                    SqlParam::I64(1),
                    SqlParam::I64(0),
                    SqlParam::I64(10_000_000),
                ],
            },
            BenchQuery {
                name: "high_rating_red_boats",
                datalog: r#"
                    find ?sailor ?boat
                    where
                      Sailor(id: ?sailor, rating: ?rating)
                      Reserve(sailor: ?sailor, boat: ?boat)
                      Boat(id: ?boat, color: $color)
                      ?rating >= $min_rating
                "#,
                inputs: vec![("color", Value::Symbol(1)), ("min_rating", Value::U64(7))],
                sqlite: r#"
                    SELECT DISTINCT s.id, b.id FROM sailor s
                    JOIN reserve r ON r.sailor = s.id
                    JOIN boat b ON b.id = r.boat
                    WHERE b.color = ?1 AND s.rating >= ?2
                "#,
                sqlite_params: vec![SqlParam::I64(1), SqlParam::I64(7)],
            },
        ],
    }
}

fn join_stress_dataset(scale: u64) -> Dataset {
    let n = scale.max(20);
    Dataset {
        name: "joinstress",
        schema: SchemaDescriptor::new(
            "JoinStressDb",
            vec![
                RelationDescriptor::new(
                    "A",
                    RelationKind::Entity,
                    vec![
                        id_field("AId", "A"),
                        FieldDescriptor::new(
                            "k",
                            ValueType::Symbol {
                                name: "K".to_owned(),
                            },
                        ),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                ),
                RelationDescriptor::new(
                    "B",
                    RelationKind::Entity,
                    vec![
                        id_field("BId", "B"),
                        ref_field("AId", "a", "A"),
                        FieldDescriptor::new(
                            "k",
                            ValueType::Symbol {
                                name: "K".to_owned(),
                            },
                        ),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                ),
                RelationDescriptor::new(
                    "C",
                    RelationKind::Entity,
                    vec![
                        id_field("CId", "C"),
                        ref_field("BId", "b", "B"),
                        FieldDescriptor::new(
                            "k",
                            ValueType::Symbol {
                                name: "K".to_owned(),
                            },
                        ),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                ),
                RelationDescriptor::new(
                    "D",
                    RelationKind::Entity,
                    vec![
                        id_field("DId", "D"),
                        ref_field("CId", "c", "C"),
                        FieldDescriptor::new(
                            "k",
                            ValueType::Symbol {
                                name: "K".to_owned(),
                            },
                        ),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                ),
                RelationDescriptor::new(
                    "EdgeAB",
                    RelationKind::Edge,
                    vec![ref_field("AId", "a", "A"), ref_field("BId", "b", "B")],
                    PrimaryKeyDescriptor::new(["a", "b"]),
                ),
                RelationDescriptor::new(
                    "EdgeAC",
                    RelationKind::Edge,
                    vec![ref_field("AId", "a", "A"), ref_field("CId", "c", "C")],
                    PrimaryKeyDescriptor::new(["a", "c"]),
                ),
                RelationDescriptor::new(
                    "EdgeBC",
                    RelationKind::Edge,
                    vec![ref_field("BId", "b", "B"), ref_field("CId", "c", "C")],
                    PrimaryKeyDescriptor::new(["b", "c"]),
                ),
            ],
        ),
        rows: join_stress_rows(n),
        sqlite_schema: r#"
            CREATE TABLE a (id INTEGER PRIMARY KEY, k INTEGER NOT NULL);
            CREATE TABLE b (id INTEGER PRIMARY KEY, a INTEGER NOT NULL, k INTEGER NOT NULL);
            CREATE TABLE c (id INTEGER PRIMARY KEY, b INTEGER NOT NULL, k INTEGER NOT NULL);
            CREATE TABLE d (id INTEGER PRIMARY KEY, c INTEGER NOT NULL, k INTEGER NOT NULL);
            CREATE TABLE edge_ab (a INTEGER NOT NULL, b INTEGER NOT NULL, PRIMARY KEY (a, b));
            CREATE TABLE edge_ac (a INTEGER NOT NULL, c INTEGER NOT NULL, PRIMARY KEY (a, c));
            CREATE TABLE edge_bc (b INTEGER NOT NULL, c INTEGER NOT NULL, PRIMARY KEY (b, c));
            CREATE INDEX b_a ON b(a, id);
            CREATE INDEX c_b ON c(b, id);
            CREATE INDEX d_c ON d(c, id);
            CREATE INDEX edge_ab_b ON edge_ab(b, a);
            CREATE INDEX edge_ac_c ON edge_ac(c, a);
            CREATE INDEX edge_bc_c ON edge_bc(c, b);
        "#,
        sqlite_insert: insert_join_stress_sqlite,
        queries: vec![
            BenchQuery {
                name: "chain4_from_a",
                datalog: r#"
                    find ?d
                    where
                      A(id: $a)
                      B(id: ?b, a: $a)
                      C(id: ?c, b: ?b)
                      D(id: ?d, c: ?c)
                "#,
                inputs: vec![("a", Value::Id(1))],
                sqlite: "SELECT d.id FROM a JOIN b ON b.a = a.id JOIN c ON c.b = b.id JOIN d ON d.c = c.id WHERE a.id = ?1",
                sqlite_params: vec![SqlParam::I64(1)],
            },
            BenchQuery {
                name: "triangle_count",
                datalog: r#"
                    find count(?a)
                    where
                      EdgeAB(a: ?a, b: ?b)
                      EdgeAC(a: ?a, c: ?c)
                      EdgeBC(b: ?b, c: ?c)
                "#,
                inputs: vec![],
                sqlite: "SELECT COUNT(eab.a) FROM edge_ab eab JOIN edge_ac eac ON eac.a = eab.a JOIN edge_bc ebc ON ebc.b = eab.b AND ebc.c = eac.c",
                sqlite_params: vec![],
            },
        ],
    }
}

fn tpch_dataset(scale: u64) -> Dataset {
    let n = scale.max(20);
    Dataset {
        name: "tpch",
        schema: SchemaDescriptor::new(
            "TpchSubsetDb",
            vec![
                RelationDescriptor::new(
                    "Customer",
                    RelationKind::Entity,
                    vec![
                        id_field("CustomerId", "Customer"),
                        FieldDescriptor::new(
                            "nation",
                            ValueType::Symbol {
                                name: "Nation".to_owned(),
                            },
                        ),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                )
                .with_index(IndexDescriptor::equality("by_nation", ["nation", "id"])),
                RelationDescriptor::new(
                    "Supplier",
                    RelationKind::Entity,
                    vec![
                        id_field("SupplierId", "Supplier"),
                        FieldDescriptor::new(
                            "nation",
                            ValueType::Symbol {
                                name: "Nation".to_owned(),
                            },
                        ),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                )
                .with_index(IndexDescriptor::equality("by_nation", ["nation", "id"])),
                RelationDescriptor::new(
                    "Part",
                    RelationKind::Entity,
                    vec![
                        id_field("PartId", "Part"),
                        FieldDescriptor::new(
                            "brand",
                            ValueType::Symbol {
                                name: "Brand".to_owned(),
                            },
                        ),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                ),
                RelationDescriptor::new(
                    "Orders",
                    RelationKind::Entity,
                    vec![
                        id_field("OrderId", "Orders"),
                        ref_field("CustomerId", "customer", "Customer"),
                        FieldDescriptor::new("order_date", ValueType::TimestampMicros)
                            .range_indexed(),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                ),
                RelationDescriptor::new(
                    "LineItem",
                    RelationKind::Entity,
                    vec![
                        id_field("LineItemId", "LineItem"),
                        ref_field("OrderId", "order", "Orders"),
                        ref_field("PartId", "part", "Part"),
                        ref_field("SupplierId", "supplier", "Supplier"),
                        FieldDescriptor::new("quantity", ValueType::I64),
                        FieldDescriptor::new("extended_price", ValueType::Decimal { scale: 2 }),
                        FieldDescriptor::new("ship_date", ValueType::TimestampMicros)
                            .range_indexed(),
                    ],
                    PrimaryKeyDescriptor::new(["id"]),
                ),
            ],
        ),
        rows: tpch_rows(n),
        sqlite_schema: r#"
            CREATE TABLE customer (id INTEGER PRIMARY KEY, nation INTEGER NOT NULL);
            CREATE TABLE supplier (id INTEGER PRIMARY KEY, nation INTEGER NOT NULL);
            CREATE TABLE part (id INTEGER PRIMARY KEY, brand INTEGER NOT NULL);
            CREATE TABLE orders (id INTEGER PRIMARY KEY, customer INTEGER NOT NULL, order_date INTEGER NOT NULL);
            CREATE TABLE lineitem (id INTEGER PRIMARY KEY, ord INTEGER NOT NULL, part INTEGER NOT NULL, supplier INTEGER NOT NULL, quantity INTEGER NOT NULL, extended_price INTEGER NOT NULL, ship_date INTEGER NOT NULL);
            CREATE INDEX orders_customer ON orders(customer, id);
            CREATE INDEX lineitem_order ON lineitem(ord, id);
            CREATE INDEX lineitem_supplier ON lineitem(supplier, id);
            CREATE INDEX lineitem_ship_date ON lineitem(ship_date, id);
            CREATE INDEX supplier_nation ON supplier(nation, id);
        "#,
        sqlite_insert: insert_tpch_sqlite,
        queries: vec![
            BenchQuery {
                name: "revenue_by_customer_range",
                datalog: r#"
                    find ?customer sum(?price)
                    where
                      Customer(id: ?customer, nation: $nation)
                      Orders(id: ?order, customer: ?customer)
                      LineItem(order: ?order, extended_price: ?price, ship_date: ?ship)
                      ?ship >= $start
                      ?ship < $end
                "#,
                inputs: vec![
                    ("nation", Value::Symbol(1)),
                    ("start", Value::Timestamp(TimestampMicros(0))),
                    ("end", Value::Timestamp(TimestampMicros(1_000_000_000))),
                ],
                sqlite: r#"
                    SELECT c.id, SUM(l.extended_price) FROM customer c
                    JOIN orders o ON o.customer = c.id
                    JOIN lineitem l ON l.ord = o.id
                    WHERE c.nation = ?1 AND l.ship_date >= ?2 AND l.ship_date < ?3
                    GROUP BY c.id
                "#,
                sqlite_params: vec![
                    SqlParam::I64(1),
                    SqlParam::I64(0),
                    SqlParam::I64(1_000_000_000),
                ],
            },
            BenchQuery {
                name: "supplier_nation_orders",
                datalog: r#"
                    find ?line ?order
                    where
                      Supplier(id: ?supplier, nation: $nation)
                      LineItem(id: ?line, order: ?order, supplier: ?supplier)
                      Orders(id: ?order, customer: ?customer)
                "#,
                inputs: vec![("nation", Value::Symbol(2))],
                sqlite: r#"
                    SELECT l.id, o.id FROM supplier s
                    JOIN lineitem l ON l.supplier = s.id
                    JOIN orders o ON o.id = l.ord
                    WHERE s.nation = ?1
                "#,
                sqlite_params: vec![SqlParam::I64(2)],
            },
        ],
    }
}

fn sailors_rows(sailors: u64) -> Vec<Row> {
    let mut rows = Vec::new();
    for sid in 1..=sailors {
        rows.push(Row::new(
            "Sailor",
            [
                ("id", Value::Id(sid)),
                ("name", Value::String(format!("sailor-{sid}"))),
                ("rating", Value::U64((sid % 10) + 1)),
                ("age", Value::I64(18 + (sid % 50) as i64)),
            ],
        ));
    }
    let boats = (sailors / 4).max(10);
    for bid in 1..=boats {
        rows.push(Row::new(
            "Boat",
            [
                ("id", Value::Id(bid)),
                ("name", Value::String(format!("boat-{bid}"))),
                ("color", Value::Symbol((bid % 3) + 1)),
            ],
        ));
    }
    let mut seen = std::collections::BTreeSet::new();
    for sid in 1..=sailors {
        for offset in 0..5 {
            let bid = ((sid + offset * 7) % boats) + 1;
            let day = ((sid * 10 + offset) as i64) * 86_400;
            if seen.insert((sid, bid, day)) {
                rows.push(Row::new(
                    "Reserve",
                    [
                        ("sailor", Value::Ref(sid)),
                        ("boat", Value::Ref(bid)),
                        ("day", Value::Timestamp(TimestampMicros(day))),
                    ],
                ));
            }
        }
    }
    rows
}

fn join_stress_rows(n: u64) -> Vec<Row> {
    let mut rows = Vec::new();
    for id in 1..=n {
        rows.push(Row::new(
            "A",
            [("id", Value::Id(id)), ("k", Value::Symbol(id % 10))],
        ));
        rows.push(Row::new(
            "B",
            [
                ("id", Value::Id(id)),
                ("a", Value::Ref(((id - 1) % n) + 1)),
                ("k", Value::Symbol(id % 10)),
            ],
        ));
        rows.push(Row::new(
            "C",
            [
                ("id", Value::Id(id)),
                ("b", Value::Ref(((id - 1) % n) + 1)),
                ("k", Value::Symbol(id % 10)),
            ],
        ));
        rows.push(Row::new(
            "D",
            [
                ("id", Value::Id(id)),
                ("c", Value::Ref(((id - 1) % n) + 1)),
                ("k", Value::Symbol(id % 10)),
            ],
        ));
    }
    let mut ab = std::collections::BTreeSet::new();
    let mut ac = std::collections::BTreeSet::new();
    let mut bc = std::collections::BTreeSet::new();
    for a in 1..=n {
        for offset in 0..3 {
            let b = ((a + offset) % n) + 1;
            let c = ((a + offset * 2) % n) + 1;
            if ab.insert((a, b)) {
                rows.push(Row::new(
                    "EdgeAB",
                    [("a", Value::Ref(a)), ("b", Value::Ref(b))],
                ));
            }
            if ac.insert((a, c)) {
                rows.push(Row::new(
                    "EdgeAC",
                    [("a", Value::Ref(a)), ("c", Value::Ref(c))],
                ));
            }
            if bc.insert((b, c)) {
                rows.push(Row::new(
                    "EdgeBC",
                    [("b", Value::Ref(b)), ("c", Value::Ref(c))],
                ));
            }
        }
    }
    rows
}

fn tpch_rows(n: u64) -> Vec<Row> {
    let mut rows = Vec::new();
    for id in 1..=n {
        rows.push(Row::new(
            "Customer",
            [
                ("id", Value::Id(id)),
                ("nation", Value::Symbol((id % 5) + 1)),
            ],
        ));
        rows.push(Row::new(
            "Supplier",
            [
                ("id", Value::Id(id)),
                ("nation", Value::Symbol((id % 7) + 1)),
            ],
        ));
        rows.push(Row::new(
            "Part",
            [
                ("id", Value::Id(id)),
                ("brand", Value::Symbol((id % 11) + 1)),
            ],
        ));
        rows.push(Row::new(
            "Orders",
            [
                ("id", Value::Id(id)),
                ("customer", Value::Ref(((id - 1) % n) + 1)),
                (
                    "order_date",
                    Value::Timestamp(TimestampMicros(id as i64 * 10)),
                ),
            ],
        ));
    }
    let mut line = 1;
    for order in 1..=n {
        for offset in 0..4 {
            rows.push(Row::new(
                "LineItem",
                [
                    ("id", Value::Id(line)),
                    ("order", Value::Ref(order)),
                    ("part", Value::Ref(((order + offset) % n) + 1)),
                    ("supplier", Value::Ref(((order + offset * 3) % n) + 1)),
                    ("quantity", Value::I64((offset + 1) as i64)),
                    (
                        "extended_price",
                        Value::Decimal(DecimalRaw(line as i128 * 100)),
                    ),
                    (
                        "ship_date",
                        Value::Timestamp(TimestampMicros(line as i64 * 10)),
                    ),
                ],
            ));
            line += 1;
        }
    }
    rows
}

fn insert_ledger_sqlite(conn: &Connection, rows: &[Row]) -> Result<(), Box<dyn std::error::Error>> {
    let tx = conn.unchecked_transaction()?;
    for row in rows {
        match row.relation() {
            "Holder" => {
                tx.execute(
                    "INSERT INTO holder (id, name) VALUES (?1, ?2)",
                    rusqlite::params![id(row, "id")?, text(row, "name")?],
                )?;
            }
            "Account" => {
                tx.execute(
                    "INSERT INTO account (id, holder, currency) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id(row, "id")?, rf(row, "holder")?, symbol(row, "currency")?],
                )?;
            }
            "Instrument" => {
                tx.execute(
                    "INSERT INTO instrument (id, symbol) VALUES (?1, ?2)",
                    rusqlite::params![id(row, "id")?, text(row, "symbol")?],
                )?;
            }
            "JournalEntry" => {
                tx.execute(
                    "INSERT INTO journal_entry (id, source, created_at) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id(row, "id")?, rf(row, "source")?, ts(row, "created_at")?],
                )?;
            }
            "Posting" => {
                tx.execute("INSERT INTO posting (id, entry, account, instrument, amount, at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", rusqlite::params![id(row, "id")?, rf(row, "entry")?, rf(row, "account")?, rf(row, "instrument")?, dec(row, "amount")?, ts(row, "at")?])?;
            }
            "PostingTag" => {
                tx.execute(
                    "INSERT INTO posting_tag (posting, tag) VALUES (?1, ?2)",
                    rusqlite::params![rf(row, "posting")?, symbol(row, "tag")?],
                )?;
            }
            _ => {}
        }
    }
    tx.commit()?;
    Ok(())
}

fn insert_sailors_sqlite(
    conn: &Connection,
    rows: &[Row],
) -> Result<(), Box<dyn std::error::Error>> {
    let tx = conn.unchecked_transaction()?;
    for row in rows {
        match row.relation() {
            "Sailor" => {
                tx.execute(
                    "INSERT INTO sailor (id, name, rating, age) VALUES (?1, ?2, ?3, ?4)",
                    rusqlite::params![
                        id(row, "id")?,
                        text(row, "name")?,
                        u64v(row, "rating")?,
                        i64v(row, "age")?
                    ],
                )?;
            }
            "Boat" => {
                tx.execute(
                    "INSERT INTO boat (id, name, color) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id(row, "id")?, text(row, "name")?, symbol(row, "color")?],
                )?;
            }
            "Reserve" => {
                tx.execute(
                    "INSERT INTO reserve (sailor, boat, day) VALUES (?1, ?2, ?3)",
                    rusqlite::params![rf(row, "sailor")?, rf(row, "boat")?, ts(row, "day")?],
                )?;
            }
            _ => {}
        }
    }
    tx.commit()?;
    Ok(())
}

fn insert_join_stress_sqlite(
    conn: &Connection,
    rows: &[Row],
) -> Result<(), Box<dyn std::error::Error>> {
    let tx = conn.unchecked_transaction()?;
    for row in rows {
        match row.relation() {
            "A" => {
                tx.execute(
                    "INSERT INTO a (id, k) VALUES (?1, ?2)",
                    rusqlite::params![id(row, "id")?, symbol(row, "k")?],
                )?;
            }
            "B" => {
                tx.execute(
                    "INSERT INTO b (id, a, k) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id(row, "id")?, rf(row, "a")?, symbol(row, "k")?],
                )?;
            }
            "C" => {
                tx.execute(
                    "INSERT INTO c (id, b, k) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id(row, "id")?, rf(row, "b")?, symbol(row, "k")?],
                )?;
            }
            "D" => {
                tx.execute(
                    "INSERT INTO d (id, c, k) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id(row, "id")?, rf(row, "c")?, symbol(row, "k")?],
                )?;
            }
            "EdgeAB" => {
                tx.execute(
                    "INSERT INTO edge_ab (a, b) VALUES (?1, ?2)",
                    rusqlite::params![rf(row, "a")?, rf(row, "b")?],
                )?;
            }
            "EdgeAC" => {
                tx.execute(
                    "INSERT INTO edge_ac (a, c) VALUES (?1, ?2)",
                    rusqlite::params![rf(row, "a")?, rf(row, "c")?],
                )?;
            }
            "EdgeBC" => {
                tx.execute(
                    "INSERT INTO edge_bc (b, c) VALUES (?1, ?2)",
                    rusqlite::params![rf(row, "b")?, rf(row, "c")?],
                )?;
            }
            _ => {}
        }
    }
    tx.commit()?;
    Ok(())
}

fn insert_tpch_sqlite(conn: &Connection, rows: &[Row]) -> Result<(), Box<dyn std::error::Error>> {
    let tx = conn.unchecked_transaction()?;
    for row in rows {
        match row.relation() {
            "Customer" => {
                tx.execute(
                    "INSERT INTO customer (id, nation) VALUES (?1, ?2)",
                    rusqlite::params![id(row, "id")?, symbol(row, "nation")?],
                )?;
            }
            "Supplier" => {
                tx.execute(
                    "INSERT INTO supplier (id, nation) VALUES (?1, ?2)",
                    rusqlite::params![id(row, "id")?, symbol(row, "nation")?],
                )?;
            }
            "Part" => {
                tx.execute(
                    "INSERT INTO part (id, brand) VALUES (?1, ?2)",
                    rusqlite::params![id(row, "id")?, symbol(row, "brand")?],
                )?;
            }
            "Orders" => {
                tx.execute(
                    "INSERT INTO orders (id, customer, order_date) VALUES (?1, ?2, ?3)",
                    rusqlite::params![id(row, "id")?, rf(row, "customer")?, ts(row, "order_date")?],
                )?;
            }
            "LineItem" => {
                tx.execute("INSERT INTO lineitem (id, ord, part, supplier, quantity, extended_price, ship_date) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)", rusqlite::params![id(row, "id")?, rf(row, "order")?, rf(row, "part")?, rf(row, "supplier")?, i64v(row, "quantity")?, dec(row, "extended_price")?, ts(row, "ship_date")?])?;
            }
            _ => {}
        }
    }
    tx.commit()?;
    Ok(())
}

pub(crate) fn id(row: &Row, field: &str) -> Result<i64, Box<dyn std::error::Error>> {
    match required_value(row, field)? {
        Value::Id(v) => Ok(*v as i64),
        other => Err(unexpected_value(field, "id", other)),
    }
}

pub(crate) fn rf(row: &Row, field: &str) -> Result<i64, Box<dyn std::error::Error>> {
    match required_value(row, field)? {
        Value::Ref(v) => Ok(*v as i64),
        other => Err(unexpected_value(field, "ref", other)),
    }
}

pub(crate) fn symbol(row: &Row, field: &str) -> Result<i64, Box<dyn std::error::Error>> {
    match required_value(row, field)? {
        Value::Symbol(v) => Ok(*v as i64),
        other => Err(unexpected_value(field, "symbol", other)),
    }
}

pub(crate) fn dec(row: &Row, field: &str) -> Result<i64, Box<dyn std::error::Error>> {
    match required_value(row, field)? {
        Value::Decimal(DecimalRaw(v)) => Ok(*v as i64),
        other => Err(unexpected_value(field, "decimal", other)),
    }
}

pub(crate) fn ts(row: &Row, field: &str) -> Result<i64, Box<dyn std::error::Error>> {
    match required_value(row, field)? {
        Value::Timestamp(TimestampMicros(v)) => Ok(*v),
        other => Err(unexpected_value(field, "timestamp", other)),
    }
}

pub(crate) fn u64v(row: &Row, field: &str) -> Result<i64, Box<dyn std::error::Error>> {
    match required_value(row, field)? {
        Value::U64(v) => Ok(*v as i64),
        other => Err(unexpected_value(field, "u64", other)),
    }
}

pub(crate) fn i64v(row: &Row, field: &str) -> Result<i64, Box<dyn std::error::Error>> {
    match required_value(row, field)? {
        Value::I64(v) => Ok(*v),
        other => Err(unexpected_value(field, "i64", other)),
    }
}

pub(crate) fn text(row: &Row, field: &str) -> Result<String, Box<dyn std::error::Error>> {
    match required_value(row, field)? {
        Value::String(v) => Ok(v.clone()),
        other => Err(unexpected_value(field, "string", other)),
    }
}

fn required_value<'a>(row: &'a Row, field: &str) -> Result<&'a Value, Box<dyn std::error::Error>> {
    row.value(field)
        .ok_or_else(|| bench_error(format!("missing field {field}")))
}

fn unexpected_value(field: &str, expected: &str, actual: &Value) -> Box<dyn std::error::Error> {
    bench_error(format!("expected {expected} {field}, got {actual:?}"))
}

pub(crate) fn id_field(id_type: &str, relation: &str) -> FieldDescriptor {
    FieldDescriptor::new(
        "id",
        ValueType::Id {
            name: id_type.to_owned(),
            relation: relation.to_owned(),
        },
    )
}

pub(crate) fn ref_field(id_type: &str, field: &str, target: &str) -> FieldDescriptor {
    FieldDescriptor::new(
        field,
        ValueType::Ref {
            name: id_type.to_owned(),
            target_relation: target.to_owned(),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_renderer_emits_gate_tables() {
        let sample_stats = TimingStats::from_samples(vec![
            Duration::from_micros(9),
            Duration::from_micros(10),
            Duration::from_micros(11),
        ]);
        let result = BenchmarkRunResult {
            dataset: "joinstress",
            query: "triangle_count",
            rows: 1,
            bumbledb_prepare: Duration::from_micros(20),
            sqlite_prepare: Duration::from_micros(12),
            bumbledb_warmup: TimingStats::from_samples(vec![Duration::from_micros(13)]),
            sqlite_warmup: TimingStats::from_samples(vec![Duration::from_micros(8)]),
            bumbledb_samples: sample_stats,
            sqlite_samples: sample_stats,
            bumbledb_avg: Duration::from_micros(10),
            sqlite_avg: Duration::from_micros(5),
            sqlite_ratio: 2.0,
            chosen_plan: "pure_lftj".to_owned(),
            runtime_kind: "Lftj".to_owned(),
            timings: QueryTimings {
                total_micros: 10,
                execute_micros: 4,
                sink_finish_micros: 1,
                ..QueryTimings::default()
            },
            allocations: QueryAllocationStats::default(),
            iterator_ops: 7,
            hash_build_rows: 0,
            hash_probe_rows: 0,
            materialized_values: 1,
            dictionary_reverse_lookups: 0,
            counters: PlanCounters {
                output_rows: 1,
                materialized_output_values: 1,
                ..PlanCounters::default()
            },
            final_output_values: 1,
            output_contains_dictionary_values: false,
            query_image_build_micros: 3,
            query_image_segment_count: 4,
            query_image_segment_bytes: 128,
            query_image_built_from_segments: true,
            query_image_cache_cached_images: 1,
            query_image_cache_hits: 1,
            query_image_cache_misses: 1,
            query_image_cache_builds: 1,
            query_image_cache_build_micros: 3,
            planner_stats_cached_relations: 1,
            planner_stats_hits: 2,
            planner_stats_misses: 1,
            planner_stats_builds: 1,
            planner_stats_build_micros: 9,
            sorted_trie_cache_hits: 1,
            sorted_trie_cache_misses: 1,
            sorted_trie_builds: 1,
            atom_temp_relation_builds: 1,
            hash_probe_calls: 1,
            hash_probe_hits: 1,
            hash_probe_misses: 0,
            hash_rows_returned: 1,
            hash_distinct_emits: 1,
            gate: GateOutcome {
                passed: true,
                notes: vec!["ok".to_owned()],
            },
        };

        let markdown = render_markdown_results(&[result]);
        assert!(markdown.contains("| joinstress | triangle_count |"));
        assert!(markdown.contains("## Phase Timing"));
        assert!(markdown.contains("## Allocation Summary"));
        assert!(markdown.contains("## Allocation Phase Detail"));
        assert!(markdown.contains("## Distribution"));
        assert!(markdown.contains("| dataset | query | cursor seeks |"));
        assert!(
            markdown.contains("| joinstress | triangle_count | 0 | 0 | 1 | 1 | false | 0 | ok |")
        );
    }

    #[test]
    fn json_renderer_emits_structured_results() {
        let result = BenchmarkRunResult {
            dataset: "ledger",
            query: "tag_lookup_join",
            rows: 2,
            bumbledb_prepare: Duration::from_micros(20),
            sqlite_prepare: Duration::from_micros(10),
            bumbledb_warmup: TimingStats::from_samples(vec![Duration::from_micros(11)]),
            sqlite_warmup: TimingStats::from_samples(vec![Duration::from_micros(7)]),
            bumbledb_samples: TimingStats::from_samples(vec![Duration::from_micros(9)]),
            sqlite_samples: TimingStats::from_samples(vec![Duration::from_micros(3)]),
            bumbledb_avg: Duration::from_micros(9),
            sqlite_avg: Duration::from_micros(3),
            sqlite_ratio: 3.0,
            chosen_plan: "hash_probe".to_owned(),
            runtime_kind: "HashProbe".to_owned(),
            timings: QueryTimings {
                total_micros: 20,
                hash_execute_micros: 4,
                ..QueryTimings::default()
            },
            allocations: QueryAllocationStats::default(),
            iterator_ops: 1,
            hash_build_rows: 1,
            hash_probe_rows: 1,
            materialized_values: 2,
            dictionary_reverse_lookups: 0,
            counters: PlanCounters::default(),
            final_output_values: 2,
            output_contains_dictionary_values: false,
            query_image_build_micros: 1,
            query_image_segment_count: 1,
            query_image_segment_bytes: 1,
            query_image_built_from_segments: true,
            query_image_cache_cached_images: 1,
            query_image_cache_hits: 1,
            query_image_cache_misses: 0,
            query_image_cache_builds: 1,
            query_image_cache_build_micros: 1,
            planner_stats_cached_relations: 1,
            planner_stats_hits: 1,
            planner_stats_misses: 0,
            planner_stats_builds: 1,
            planner_stats_build_micros: 1,
            sorted_trie_cache_hits: 0,
            sorted_trie_cache_misses: 0,
            sorted_trie_builds: 0,
            atom_temp_relation_builds: 0,
            hash_probe_calls: 1,
            hash_probe_hits: 1,
            hash_probe_misses: 0,
            hash_rows_returned: 2,
            hash_distinct_emits: 2,
            gate: GateOutcome {
                passed: true,
                notes: vec!["ok".to_owned()],
            },
        };

        let json = render_json_results(&[result]);
        assert!(json.contains("\"dataset\":\"ledger\""));
        assert!(json.contains("\"runtime\":\"HashProbe\""));
        assert!(json.contains("\"phase_timing\""));
        assert!(json.contains("\"allocations\""));
        assert!(json.contains("\"phases\""));
        assert!(json.contains("\"size_class_allocs\""));
    }

    #[test]
    fn cli_parser_accepts_repeated_query_filters() -> Result<(), Box<dyn std::error::Error>> {
        let config = Config::from_args(
            [
                "--dataset",
                "ledger",
                "--query",
                "tag_lookup_join",
                "--query",
                "balances_by_instrument",
                "--warmup",
                "2",
                "--format",
                "json",
            ]
            .into_iter()
            .map(str::to_owned),
        )?
        .ok_or_else(|| bench_error("expected config"))?;

        assert_eq!(config.datasets, vec!["ledger"]);
        assert_eq!(
            config.queries,
            vec!["tag_lookup_join", "balances_by_instrument"]
        );
        assert_eq!(config.warmup, 2);
        assert_eq!(config.format, OutputFormat::Json);
        Ok(())
    }

    #[test]
    fn cli_parser_rejects_invalid_numbers() {
        let result = Config::from_args(["--repeats", "nope"].into_iter().map(str::to_owned));
        assert!(result.is_err());
    }

    #[test]
    fn cli_parser_accepts_trace_output_without_default_subscriber()
    -> Result<(), Box<dyn std::error::Error>> {
        let config = Config::from_args(
            ["--trace-output", "trace.log", "--trace-format", "json"]
                .into_iter()
                .map(str::to_owned),
        )?
        .ok_or_else(|| bench_error("expected config"))?;

        assert!(config.trace);
        assert_eq!(config.trace_output.as_deref(), Some("trace.log"));
        assert_eq!(config.trace_format, TraceFormat::Json);
        Ok(())
    }

    #[cfg(feature = "alloc-profile")]
    #[test]
    fn allocation_profile_records_known_vector() {
        let before = bumbledb_lmdb::allocation::snapshot();
        let values = vec![42u8; 4096];
        black_box(&values);
        let after = bumbledb_lmdb::allocation::snapshot();
        let delta = bumbledb_lmdb::allocation::delta(before, after);

        assert!(delta.enabled);
        assert!(delta.alloc_calls > 0);
        assert!(delta.bytes_allocated >= 4096);
        assert!(delta.current_live_bytes >= 4096);
    }

    #[test]
    fn focused_gate_definitions_are_present() {
        assert!(benchmark_gate("joinstress", "triangle_count").is_some());
        assert!(benchmark_gate("ledger", "tag_lookup_join").is_some());
        assert!(benchmark_gate("sailors", "red_boat_sailors").is_some());
        assert!(benchmark_gate("tpch", "supplier_nation_orders").is_some());
        assert!(benchmark_gate("ledger", "unknown").is_none());
    }

    #[test]
    fn duration_ratio_handles_zero_sqlite_time() {
        assert!(duration_ratio(Duration::from_micros(1), Duration::ZERO).is_infinite());
    }
}
