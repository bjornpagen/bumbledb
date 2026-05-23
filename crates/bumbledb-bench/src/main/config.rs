use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum OutputFormat {
    Text,
    Markdown,
    Json,
    Both,
}

impl OutputFormat {
    pub(super) fn includes_text(self) -> bool {
        matches!(self, OutputFormat::Text | OutputFormat::Both)
    }

    pub(super) fn includes_markdown(self) -> bool {
        matches!(self, OutputFormat::Markdown | OutputFormat::Both)
    }

    pub(super) fn includes_json(self) -> bool {
        matches!(self, OutputFormat::Json | OutputFormat::Both)
    }

    pub(super) fn is_json_only(self) -> bool {
        self == OutputFormat::Json
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TraceFormat {
    Fmt,
    Json,
    Chrome,
    Flame,
}

#[derive(Debug)]
pub(super) struct Config {
    pub(super) scale: u64,
    pub(super) open_limit: Option<usize>,
    pub(super) repeats: u64,
    pub(super) warmup: u64,
    pub(super) datasets: Vec<String>,
    pub(super) queries: Vec<String>,
    pub(super) imdb_dir: Option<String>,
    pub(super) job_dir: Option<String>,
    pub(super) tpch_dir: Option<String>,
    pub(super) lahman_dir: Option<String>,
    pub(super) ldbc_dir: Option<String>,
    pub(super) preset: Option<String>,
    pub(super) trace: bool,
    pub(super) trace_output: Option<String>,
    pub(super) trace_format: TraceFormat,
    pub(super) format: OutputFormat,
    pub(super) fail_gates: bool,
}

impl Config {
    pub(super) fn from_env() -> Result<Option<Self>, Box<dyn std::error::Error>> {
        Self::from_args(std::env::args().skip(1))
    }

    pub(super) fn from_args(
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
                        "usage: cargo run -p bumbledb-bench --release -- [--preset quick|nonjob|job|job-sample|job-full] [--scale N] [--open-limit N|--open-full] [--repeats N] [--warmup N] [--query NAME] [--trace] [--trace-output PATH] [--trace-format fmt|json|chrome|flame] [--format text|markdown|json|both] [--markdown] [--json] [--fail-gates] [--dataset ledger|sailors|joinstress|tpch|imdb|job|tpch-open|lahman|ldbc] [--imdb-dir DIR] [--job-dir DIR] [--tpch-dir DIR] [--lahman-dir DIR] [--ldbc-dir DIR]"
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

    pub(super) fn has_open_datasets(&self) -> bool {
        self.imdb_dir.is_some()
            || self.job_dir.is_some()
            || self.tpch_dir.is_some()
            || self.lahman_dir.is_some()
            || self.ldbc_dir.is_some()
    }
}
