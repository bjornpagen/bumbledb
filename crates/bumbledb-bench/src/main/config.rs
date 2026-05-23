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
        matches!(self, OutputFormat::Json | OutputFormat::Both)
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CompareMode {
    Materialized,
    Facts,
}

impl CompareMode {
    fn as_str(self) -> &'static str {
        match self {
            CompareMode::Materialized => "materialized",
            CompareMode::Facts => "facts",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CacheMode {
    Recompute,
    PreparedPlan,
}

impl CacheMode {
    fn as_str(self) -> &'static str {
        match self {
            CacheMode::Recompute => "recompute",
            CacheMode::PreparedPlan => "prepared-plan",
        }
    }
}

#[derive(Debug)]
struct Config {
    scale: u64,
    open_limit: Option<usize>,
    repeats: u64,
    warmup: u64,
    datasets: Vec<String>,
    queries: Vec<String>,
    imdb_dir: Option<String>,
    job_dir: Option<String>,
    tpch_dir: Option<String>,
    lahman_dir: Option<String>,
    ldbc_dir: Option<String>,
    preset: Option<String>,
    trace: bool,
    trace_output: Option<String>,
    trace_format: TraceFormat,
    format: OutputFormat,
    compare_mode: CompareMode,
    cache_mode: CacheMode,
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
        let mut open_limit = Some(DEFAULT_OPEN_LIMIT);
        let mut repeats = 10;
        let mut warmup = 0;
        let mut datasets = Vec::new();
        let mut queries = Vec::new();
        let mut imdb_dir = None;
        let mut job_dir = None;
        let mut tpch_dir = None;
        let mut lahman_dir = None;
        let mut ldbc_dir = None;
        let mut preset = None;
        let mut trace = false;
        let mut trace_output = None;
        let mut trace_format = TraceFormat::Fmt;
        let mut format = OutputFormat::Text;
        let mut format_explicit = false;
        let mut compare_mode = CompareMode::Materialized;
        let mut cache_mode = CacheMode::PreparedPlan;
        let mut fail_gates = false;
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--scale" => {
                    scale = next_arg(&mut args, "--scale")?
                        .parse()
                        .map_err(|error| bench_error(format!("invalid --scale: {error}")))?
                }
                "--open-limit" => {
                    open_limit = Some(
                        next_arg(&mut args, "--open-limit")?
                            .parse()
                            .map_err(|error| {
                                bench_error(format!("invalid --open-limit: {error}"))
                            })?,
                    )
                }
                "--open-full" => open_limit = None,
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
                "--job-dir" => job_dir = Some(next_arg(&mut args, "--job-dir")?),
                "--tpch-dir" => tpch_dir = Some(next_arg(&mut args, "--tpch-dir")?),
                "--lahman-dir" => lahman_dir = Some(next_arg(&mut args, "--lahman-dir")?),
                "--ldbc-dir" => ldbc_dir = Some(next_arg(&mut args, "--ldbc-dir")?),
                "--preset" => preset = Some(next_arg(&mut args, "--preset")?),
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
                    format_explicit = true;
                    format = match next_arg(&mut args, "--format")?.as_str() {
                        "text" => OutputFormat::Text,
                        "markdown" => OutputFormat::Markdown,
                        "json" => OutputFormat::Json,
                        "both" => OutputFormat::Both,
                        other => return Err(bench_error(format!("unknown --format {other}"))),
                    }
                }
                "--compare-mode" => {
                    compare_mode = match next_arg(&mut args, "--compare-mode")?.as_str() {
                        "materialized" => CompareMode::Materialized,
                        "facts" => CompareMode::Facts,
                        other => {
                            return Err(bench_error(format!("unknown --compare-mode {other}")));
                        }
                    }
                }
                "--cache-mode" => {
                    cache_mode = match next_arg(&mut args, "--cache-mode")?.as_str() {
                        "recompute" => CacheMode::Recompute,
                        "prepared-plan" => CacheMode::PreparedPlan,
                        other => return Err(bench_error(format!("unknown --cache-mode {other}"))),
                    }
                }
                "--markdown" => {
                    format_explicit = true;
                    format = OutputFormat::Markdown;
                }
                "--json" => {
                    format_explicit = true;
                    format = OutputFormat::Json;
                }
                "--fail-gates" => fail_gates = true,
                "--help" | "-h" => {
                    println!(
                        "usage: cargo run -p bumbledb-bench --release -- [--preset quick|nonjob|job|job-sample|job-full] [--scale N] [--open-limit N|--open-full] [--repeats N] [--warmup N] [--query NAME] [--trace] [--trace-output PATH] [--trace-format fmt|json|chrome|flame] [--format text|markdown|json|both] [--compare-mode materialized|facts] [--cache-mode recompute|prepared-plan] [--markdown] [--json] [--fail-gates] [--dataset ledger|sailors|joinstress|tpch|imdb|job|tpch-open|lahman|ldbc] [--imdb-dir DIR] [--job-dir DIR] [--tpch-dir DIR] [--lahman-dir DIR] [--ldbc-dir DIR]"
                    );
                    return Ok(None);
                }
                other => return Err(bench_error(format!("unknown arg {other}"))),
            }
        }
        let mut config = Self {
            scale,
            open_limit,
            repeats,
            warmup,
            datasets,
            queries,
            imdb_dir,
            job_dir,
            tpch_dir,
            lahman_dir,
            ldbc_dir,
            preset,
            trace,
            trace_output,
            trace_format,
            format,
            compare_mode,
            cache_mode,
            fail_gates,
        };
        config.apply_preset(format_explicit)?;
        Ok(Some(config))
    }

    fn apply_preset(&mut self, format_explicit: bool) -> Result<(), Box<dyn std::error::Error>> {
        let Some(preset) = self.preset.as_deref() else {
            return Ok(());
        };
        match preset {
            "quick" => {
                self.scale = 2000;
                self.repeats = 10;
                self.warmup = 0;
                self.datasets = vec![
                    "ledger".to_owned(),
                    "sailors".to_owned(),
                    "joinstress".to_owned(),
                    "tpch".to_owned(),
                ];
                if !format_explicit {
                    self.format = OutputFormat::Markdown;
                }
            }
            "nonjob" => {
                self.scale = 10000;
                self.repeats = 30;
                self.warmup = 2;
                self.datasets = vec![
                    "ledger".to_owned(),
                    "sailors".to_owned(),
                    "joinstress".to_owned(),
                    "tpch".to_owned(),
                ];
                if !format_explicit {
                    self.format = OutputFormat::Json;
                }
            }
            "job" => {
                self.repeats = 30;
                self.warmup = 2;
                self.datasets = vec!["job".to_owned()];
                self.open_limit = Some(self.open_limit.unwrap_or(DEFAULT_OPEN_LIMIT));
                if !format_explicit {
                    self.format = OutputFormat::Json;
                }
                if self.job_dir.is_none() {
                    self.job_dir = std::env::var("BUMBLED_JOB_DIR").ok();
                }
            }
            "job-sample" => {
                self.repeats = 30;
                self.warmup = 2;
                self.datasets = vec!["job".to_owned()];
                self.open_limit = Some(self.open_limit.unwrap_or(DEFAULT_OPEN_LIMIT));
                if !format_explicit {
                    self.format = OutputFormat::Json;
                }
                if self.job_dir.is_none() {
                    self.job_dir = std::env::var("BUMBLED_JOB_DIR").ok();
                }
            }
            "job-full" => {
                self.repeats = 30;
                self.warmup = 2;
                self.datasets = vec!["job".to_owned()];
                self.open_limit = None;
                if !format_explicit {
                    self.format = OutputFormat::Json;
                }
                if self.job_dir.is_none() {
                    self.job_dir = std::env::var("BUMBLED_JOB_DIR").ok();
                }
            }
            other => return Err(bench_error(format!("unknown --preset {other}"))),
        }
        Ok(())
    }

    fn has_open_datasets(&self) -> bool {
        self.imdb_dir.is_some()
            || self.job_dir.is_some()
            || self.tpch_dir.is_some()
            || self.lahman_dir.is_some()
            || self.ldbc_dir.is_some()
    }
}
