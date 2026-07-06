//! Hand-rolled command-line parsing (no clap — the quarantine allows
//! rusqlite only). One flat token walk per subcommand; every error names
//! the offending token.

use std::path::PathBuf;

use crate::gen::Scale;
use crate::verify::DEFAULT_RANDOM_CASES;

/// The corpus identity + location every store-touching command shares.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusArgs {
    pub scale: Scale,
    pub seed: u64,
    /// The digest-keyed cache root (`<dir>/<digest-prefix>/…`).
    pub dir: PathBuf,
}

impl Default for CorpusArgs {
    fn default() -> Self {
        Self {
            scale: Scale::S,
            seed: 1,
            dir: PathBuf::from("bench-data"),
        }
    }
}

/// `bench`'s knobs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BenchArgs {
    pub corpus: CorpusArgs,
    /// Selected family names; `None` = the full suite.
    pub families: Option<Vec<String>>,
    /// Measured-sample override for the read protocol.
    pub samples: Option<u32>,
    pub trace: bool,
    pub alloc: bool,
    pub out: Option<PathBuf>,
    /// Skip the verify-stamp gate; the report is branded UNVERIFIED.
    pub i_am_lying: bool,
}

/// A parsed invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cmd {
    /// Print usage and exit 0.
    Help,
    /// Print the versioned query list to stdout.
    Queries,
    /// Generate + load both stores into the digest-keyed directory.
    Gen(CorpusArgs),
    /// The oracle: compare both engines, stamp on success.
    Verify { corpus: CorpusArgs, cases: u32 },
    /// The timing run (refuses without a fresh stamp).
    Bench(BenchArgs),
    /// One traced warm+cold pair for one family.
    Trace { corpus: CorpusArgs, family: String },
    /// The scenario suites: non-ledger worlds, oracle-gated then timed.
    Scenarios(ScenarioArgs),
}

/// `scenarios`' knobs. Scenarios own their sizes (no scale flag): the
/// corpus identity is (scenario, seed).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScenarioArgs {
    pub seed: u64,
    pub dir: PathBuf,
    /// Selected scenario names; `None` = the full registry.
    pub only: Option<Vec<String>>,
    /// Measured samples per query per engine.
    pub samples: Option<u32>,
    pub out: Option<PathBuf>,
}

impl Default for ScenarioArgs {
    fn default() -> Self {
        Self {
            seed: 1,
            dir: PathBuf::from("bench-data"),
            only: None,
            samples: None,
            out: None,
        }
    }
}

/// The usage text.
#[must_use]
pub fn help() -> String {
    format!(
        "bumbledb-bench {}\n\
         \n\
         The benchmark and oracle suite (docs/architecture/50-validation.md).\n\
         \n\
         USAGE:\n\
         \x20 bumbledb-bench <COMMAND> [FLAGS]\n\
         \n\
         COMMANDS:\n\
         \x20 gen      generate + load both stores into the digest-keyed dir\n\
         \x20 verify   the oracle: families + randomized queries on both engines\n\
         \x20 bench    the timing run (requires a fresh verify stamp)\n\
         \x20 trace    one traced warm+cold pair for one family\n\
         \x20 scenarios non-ledger worlds (joins/graph/olap/points), gated then timed\n\
         \x20 queries  print the versioned query list (QUERIES.md)\n\
         \x20 help     print this text\n\
         \n\
         SHARED FLAGS (gen, verify, bench, trace):\n\
         \x20 --scale S|M|L   corpus scale        (default S)\n\
         \x20 --seed N        corpus seed         (default 1)\n\
         \x20 --dir PATH      corpus cache root   (default bench-data)\n\
         \n\
         VERIFY:\n\
         \x20 --cases N       randomized cases    (default {})\n\
         \n\
         BENCH:\n\
         \x20 --families a,b  run only these families (verdict becomes PARTIAL)\n\
         \x20 --samples N     measured samples per read family (default 256)\n\
         \x20 --trace         capture one traced warm+cold sample per family\n\
         \x20 --alloc         allocation windows (needs the obs feature build)\n\
         \x20 --out PATH      artifact dir (default bench-out/<timestamp>)\n\
         \x20 --i-am-lying    skip the stamp gate; the report reads UNVERIFIED\n\
         \n\
         TRACE:\n\
         \x20 --family NAME   the family to trace (required)\n\
         \n\
         SCENARIOS:\n\
         \x20 --seed N        corpus seed              (default 1)\n\
         \x20 --dir PATH      scratch root             (default bench-data)\n\
         \x20 --only a,b      run only these scenarios (joins graph olap points)\n\
         \x20 --samples N     measured samples/query   (default 64)\n\
         \x20 --out PATH      artifact dir (default bench-out/<timestamp>-scenarios)\n\
         \n\
         EXIT CODES: 0 ok / gate won; 1 verify mismatch or gate loss; 2 usage.\n",
        env!("CARGO_PKG_VERSION"),
        DEFAULT_RANDOM_CASES,
    )
}

struct Tokens<'a> {
    args: &'a [String],
    index: usize,
}

impl Tokens<'_> {
    fn next(&mut self) -> Option<&str> {
        let token = self.args.get(self.index)?;
        self.index += 1;
        Some(token)
    }

    fn value(&mut self, flag: &str) -> Result<&str, String> {
        self.next().ok_or_else(|| format!("`{flag}` needs a value"))
    }
}

fn parse_scale(raw: &str) -> Result<Scale, String> {
    match raw {
        "S" => Ok(Scale::S),
        "M" => Ok(Scale::M),
        "L" => Ok(Scale::L),
        other => Err(format!("unknown scale `{other}` (expected S, M, or L)")),
    }
}

fn parse_u64(flag: &str, raw: &str) -> Result<u64, String> {
    raw.parse()
        .map_err(|_| format!("`{flag}` needs an integer, got `{raw}`"))
}

fn parse_u32(flag: &str, raw: &str) -> Result<u32, String> {
    raw.parse()
        .map_err(|_| format!("`{flag}` needs an integer, got `{raw}`"))
}

/// Tries the shared corpus flags; `Ok(true)` = consumed.
fn corpus_flag(
    corpus: &mut CorpusArgs,
    flag: &str,
    tokens: &mut Tokens<'_>,
) -> Result<bool, String> {
    match flag {
        "--scale" => {
            corpus.scale = parse_scale(tokens.value(flag)?)?;
            Ok(true)
        }
        "--seed" => {
            corpus.seed = parse_u64(flag, tokens.value(flag)?)?;
            Ok(true)
        }
        "--dir" => {
            corpus.dir = PathBuf::from(tokens.value(flag)?);
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn unknown(cmd: &str, flag: &str) -> String {
    format!("unknown flag `{flag}` for `{cmd}`")
}

fn parse_gen(tokens: &mut Tokens<'_>) -> Result<Cmd, String> {
    let mut corpus = CorpusArgs::default();
    while let Some(flag) = tokens.next() {
        let flag = flag.to_owned();
        if !corpus_flag(&mut corpus, &flag, tokens)? {
            return Err(unknown("gen", &flag));
        }
    }
    Ok(Cmd::Gen(corpus))
}

fn parse_verify(tokens: &mut Tokens<'_>) -> Result<Cmd, String> {
    let mut corpus = CorpusArgs::default();
    let mut cases = DEFAULT_RANDOM_CASES;
    while let Some(flag) = tokens.next() {
        let flag = flag.to_owned();
        if corpus_flag(&mut corpus, &flag, tokens)? {
            continue;
        }
        match flag.as_str() {
            "--cases" => cases = parse_u32(&flag, tokens.value(&flag)?)?,
            _ => return Err(unknown("verify", &flag)),
        }
    }
    Ok(Cmd::Verify { corpus, cases })
}

fn parse_bench(tokens: &mut Tokens<'_>) -> Result<Cmd, String> {
    let mut args = BenchArgs {
        corpus: CorpusArgs::default(),
        families: None,
        samples: None,
        trace: false,
        alloc: false,
        out: None,
        i_am_lying: false,
    };
    while let Some(flag) = tokens.next() {
        let flag = flag.to_owned();
        if corpus_flag(&mut args.corpus, &flag, tokens)? {
            continue;
        }
        match flag.as_str() {
            "--families" => {
                args.families = Some(tokens.value(&flag)?.split(',').map(str::to_owned).collect());
            }
            "--samples" => args.samples = Some(parse_u32(&flag, tokens.value(&flag)?)?),
            "--trace" => args.trace = true,
            "--alloc" => args.alloc = true,
            "--out" => args.out = Some(PathBuf::from(tokens.value(&flag)?)),
            "--i-am-lying" => args.i_am_lying = true,
            _ => return Err(unknown("bench", &flag)),
        }
    }
    Ok(Cmd::Bench(args))
}

fn parse_trace(tokens: &mut Tokens<'_>) -> Result<Cmd, String> {
    let mut corpus = CorpusArgs::default();
    let mut family = None;
    while let Some(flag) = tokens.next() {
        let flag = flag.to_owned();
        if corpus_flag(&mut corpus, &flag, tokens)? {
            continue;
        }
        match flag.as_str() {
            "--family" => family = Some(tokens.value(&flag)?.to_owned()),
            _ => return Err(unknown("trace", &flag)),
        }
    }
    let family = family.ok_or_else(|| "`trace` needs `--family NAME`".to_owned())?;
    Ok(Cmd::Trace { corpus, family })
}

/// Parses raw arguments (without the program name).
///
/// # Errors
///
/// A human-readable message naming the offending token.
fn parse_scenarios(tokens: &mut Tokens<'_>) -> Result<Cmd, String> {
    let mut args = ScenarioArgs::default();
    while let Some(flag) = tokens.next() {
        let flag = flag.to_owned();
        match flag.as_str() {
            "--seed" => args.seed = parse_u64(&flag, tokens.value(&flag)?)?,
            "--dir" => args.dir = PathBuf::from(tokens.value(&flag)?),
            "--only" => {
                args.only = Some(tokens.value(&flag)?.split(',').map(str::to_owned).collect());
            }
            "--samples" => args.samples = Some(parse_u32(&flag, tokens.value(&flag)?)?),
            "--out" => args.out = Some(PathBuf::from(tokens.value(&flag)?)),
            _ => return Err(unknown("scenarios", &flag)),
        }
    }
    Ok(Cmd::Scenarios(args))
}

/// Parses one invocation.
///
/// # Errors
///
/// A usage message naming the offending token.
pub fn parse(args: &[String]) -> Result<Cmd, String> {
    let mut tokens = Tokens { args, index: 0 };
    let Some(command) = tokens.next() else {
        return Ok(Cmd::Help);
    };
    match command {
        "help" => match tokens.next() {
            None => Ok(Cmd::Help),
            Some(extra) => Err(format!("unexpected argument after `help`: `{extra}`")),
        },
        "queries" => match tokens.next() {
            None => Ok(Cmd::Queries),
            Some(extra) => Err(format!("unexpected argument after `queries`: `{extra}`")),
        },
        "gen" => parse_gen(&mut tokens),
        "verify" => parse_verify(&mut tokens),
        "bench" => parse_bench(&mut tokens),
        "trace" => parse_trace(&mut tokens),
        "scenarios" => parse_scenarios(&mut tokens),
        other => Err(format!("unknown command `{other}`")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(args: &[&str]) -> Vec<String> {
        args.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn help_and_queries_parse() {
        assert_eq!(parse(&argv(&["help"])), Ok(Cmd::Help));
        assert_eq!(parse(&[]), Ok(Cmd::Help));
        assert_eq!(parse(&argv(&["queries"])), Ok(Cmd::Queries));
    }

    #[test]
    fn gen_parses_the_shared_flags() {
        let cmd = parse(&argv(&[
            "gen", "--scale", "M", "--seed", "7", "--dir", "/tmp/x",
        ]))
        .expect("parses");
        assert_eq!(
            cmd,
            Cmd::Gen(CorpusArgs {
                scale: Scale::M,
                seed: 7,
                dir: PathBuf::from("/tmp/x"),
            })
        );
        let err = parse(&argv(&["gen", "--scale", "XXL"])).unwrap_err();
        assert!(err.contains("XXL"), "{err}");
    }

    #[test]
    fn verify_parses_cases() {
        let cmd = parse(&argv(&["verify", "--cases", "50"])).expect("parses");
        assert_eq!(
            cmd,
            Cmd::Verify {
                corpus: CorpusArgs::default(),
                cases: 50,
            }
        );
        let err = parse(&argv(&["verify", "--cases"])).unwrap_err();
        assert!(err.contains("--cases"), "{err}");
    }

    #[test]
    fn bench_parses_every_knob() {
        let cmd = parse(&argv(&[
            "bench",
            "--families",
            "point,fk_walk",
            "--samples",
            "8",
            "--trace",
            "--alloc",
            "--out",
            "artifacts",
            "--i-am-lying",
        ]))
        .expect("parses");
        assert_eq!(
            cmd,
            Cmd::Bench(BenchArgs {
                corpus: CorpusArgs::default(),
                families: Some(vec!["point".to_owned(), "fk_walk".to_owned()]),
                samples: Some(8),
                trace: true,
                alloc: true,
                out: Some(PathBuf::from("artifacts")),
                i_am_lying: true,
            })
        );
        let err = parse(&argv(&["bench", "--frobnicate"])).unwrap_err();
        assert!(err.contains("--frobnicate"), "{err}");
    }

    #[test]
    fn trace_requires_a_family() {
        let cmd = parse(&argv(&["trace", "--family", "skew"])).expect("parses");
        assert_eq!(
            cmd,
            Cmd::Trace {
                corpus: CorpusArgs::default(),
                family: "skew".to_owned(),
            }
        );
        let err = parse(&argv(&["trace"])).unwrap_err();
        assert!(err.contains("--family"), "{err}");
    }

    #[test]
    fn garbage_names_the_offending_token() {
        let err = parse(&argv(&["frobnicate"])).unwrap_err();
        assert!(err.contains("frobnicate"), "{err}");
        let err = parse(&argv(&["help", "me"])).unwrap_err();
        assert!(err.contains("me"), "{err}");
    }

    #[test]
    fn help_text_names_the_binary_and_version() {
        let text = help();
        assert!(text.contains("bumbledb-bench"));
        assert!(text.contains(env!("CARGO_PKG_VERSION")));
        for command in ["gen", "verify", "bench", "trace", "queries"] {
            assert!(text.contains(command), "{command}");
        }
    }
}
