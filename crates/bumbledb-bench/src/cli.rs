use crate::runner::{BenchError, BenchResult};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum OutputFormat {
    Json,
    Markdown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum TraceOutput {
    Inline,
    File,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Config {
    pub(crate) preset: String,
    pub(crate) format: OutputFormat,
    pub(crate) repeats: usize,
    pub(crate) warmup: usize,
    pub(crate) hardware: Option<String>,
    pub(crate) job_dir: Option<String>,
    pub(crate) open_limit: Option<usize>,
    pub(crate) queries: Vec<String>,
    pub(crate) alloc_tracking: bool,
    pub(crate) trace_output: TraceOutput,
    pub(crate) profile_query_label: Option<String>,
}

impl Config {
    pub(crate) fn from_env() -> BenchResult<Self> {
        Self::from_args(std::env::args().skip(1))
    }

    pub(crate) fn from_args(args: impl IntoIterator<Item = String>) -> BenchResult<Self> {
        let mut config = Self::default();
        let mut args = args.into_iter();
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--preset" => config.preset = next_arg(&mut args, "--preset")?,
                "--format" => config.format = parse_format(&next_arg(&mut args, "--format")?)?,
                "--repeats" => config.repeats = parse_usize(&next_arg(&mut args, "--repeats")?)?,
                "--warmup" => config.warmup = parse_usize(&next_arg(&mut args, "--warmup")?)?,
                "--hardware" => config.hardware = Some(next_arg(&mut args, "--hardware")?),
                "--job-dir" => config.job_dir = Some(next_arg(&mut args, "--job-dir")?),
                "--open-limit" => {
                    config.open_limit = Some(parse_usize(&next_arg(&mut args, "--open-limit")?)?)
                }
                "--open-full" => config.open_limit = None,
                "--query" => config.queries.push(next_arg(&mut args, "--query")?),
                "--alloc" => {
                    config.alloc_tracking = parse_on_off(&next_arg(&mut args, "--alloc")?)?
                }
                "--trace-output" => {
                    config.trace_output =
                        parse_trace_output(&next_arg(&mut args, "--trace-output")?)?
                }
                "--profile-query-label" => {
                    config.profile_query_label = Some(next_arg(&mut args, "--profile-query-label")?)
                }
                other => return Err(BenchError::new(format!("unknown argument {other}"))),
            }
        }
        Ok(config)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            preset: "quick".to_owned(),
            format: OutputFormat::Json,
            repeats: 1,
            warmup: 0,
            hardware: None,
            job_dir: None,
            open_limit: Some(10_000),
            queries: Vec::new(),
            alloc_tracking: false,
            trace_output: TraceOutput::Inline,
            profile_query_label: None,
        }
    }
}

fn next_arg(args: &mut impl Iterator<Item = String>, flag: &str) -> BenchResult<String> {
    args.next()
        .ok_or_else(|| BenchError::new(format!("missing value for {flag}")))
}

fn parse_format(value: &str) -> BenchResult<OutputFormat> {
    match value {
        "json" => Ok(OutputFormat::Json),
        "markdown" | "md" => Ok(OutputFormat::Markdown),
        _ => Err(BenchError::new(format!("unknown format {value}"))),
    }
}

fn parse_usize(value: &str) -> BenchResult<usize> {
    value
        .parse()
        .map_err(|_| BenchError::new(format!("invalid integer {value}")))
}

fn parse_on_off(value: &str) -> BenchResult<bool> {
    match value {
        "on" => Ok(true),
        "off" => Ok(false),
        _ => Err(BenchError::new(format!("expected on or off, got {value}"))),
    }
}

fn parse_trace_output(value: &str) -> BenchResult<TraceOutput> {
    match value {
        "inline" => Ok(TraceOutput::Inline),
        "file" => Ok(TraceOutput::File),
        _ => Err(BenchError::new(format!("unknown trace output {value}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cli_options() -> BenchResult<()> {
        let config = Config::from_args(
            [
                "--preset",
                "quick",
                "--format",
                "json",
                "--repeats",
                "2",
                "--warmup",
                "1",
                "--job-dir",
                "data/job",
                "--open-limit",
                "1000",
                "--query",
                "job_q01_top_production",
                "--alloc",
                "on",
                "--trace-output",
                "file",
                "--profile-query-label",
                "q01-profile",
            ]
            .into_iter()
            .map(str::to_owned),
        )?;

        assert_eq!(config.repeats, 2);
        assert_eq!(config.warmup, 1);
        assert_eq!(config.job_dir.as_deref(), Some("data/job"));
        assert_eq!(config.open_limit, Some(1000));
        assert_eq!(config.queries, vec!["job_q01_top_production"]);
        assert!(config.alloc_tracking);
        assert_eq!(config.trace_output, TraceOutput::File);
        assert_eq!(config.profile_query_label.as_deref(), Some("q01-profile"));
        Ok(())
    }
}
