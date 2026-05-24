use crate::runner::{BenchError, BenchResult};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum OutputFormat {
    Json,
    Markdown,
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
            ]
            .into_iter()
            .map(str::to_owned),
        )?;

        assert_eq!(config.repeats, 2);
        assert_eq!(config.warmup, 1);
        assert_eq!(config.job_dir.as_deref(), Some("data/job"));
        assert_eq!(config.open_limit, Some(1000));
        assert_eq!(config.queries, vec!["job_q01_top_production"]);
        Ok(())
    }
}
