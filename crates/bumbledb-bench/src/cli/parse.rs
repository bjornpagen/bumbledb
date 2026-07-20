use std::path::PathBuf;

use crate::corpus_gen::Scale;
use crate::lanes::writes::DurabilityLane;
use crate::verify::DEFAULT_RANDOM_CASES;

use super::{
    BenchArgs, ChurnArgs, Cmd, CorpusArgs, CurvesArgs, ScenarioArgs, StorageArgs, SweepArgs,
    WritesArgs,
};

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

/// A comma-separated scale list; the error names the bad token, an
/// empty list is rejected.
fn parse_scale_list(flag: &str, raw: &str) -> Result<Vec<Scale>, String> {
    if raw.is_empty() {
        return Err(format!("`{flag}` needs at least one scale"));
    }
    raw.split(',').map(parse_scale).collect()
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

fn parse_verify_store(tokens: &mut Tokens<'_>) -> Result<Cmd, String> {
    let mut corpus = CorpusArgs::default();
    while let Some(flag) = tokens.next() {
        let flag = flag.to_owned();
        if !corpus_flag(&mut corpus, &flag, tokens)? {
            return Err(unknown("verify-store", &flag));
        }
    }
    Ok(Cmd::VerifyStore(corpus))
}

fn parse_bench(tokens: &mut Tokens<'_>) -> Result<Cmd, String> {
    let mut args = BenchArgs {
        corpus: CorpusArgs::default(),
        families: None,
        samples: None,
        trace: false,
        alloc: false,
        ephemeral: false,
        proxy_per_rep: false,
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
            "--ephemeral" => args.ephemeral = true,
            "--proxy-per-rep" => args.proxy_per_rep = true,
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

/// The shared world-args walk (`scenarios`, `crud`, `lawful` — one
/// flag vocabulary, [`ScenarioArgs`]); an unknown flag names `cmd`.
fn parse_world(cmd: &str, tokens: &mut Tokens<'_>) -> Result<ScenarioArgs, String> {
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
            _ => return Err(unknown(cmd, &flag)),
        }
    }
    Ok(args)
}

fn parse_scenarios(tokens: &mut Tokens<'_>) -> Result<Cmd, String> {
    Ok(Cmd::Scenarios(parse_world("scenarios", tokens)?))
}

fn parse_crud(tokens: &mut Tokens<'_>) -> Result<Cmd, String> {
    Ok(Cmd::Crud(parse_world("crud", tokens)?))
}

fn parse_lawful(tokens: &mut Tokens<'_>) -> Result<Cmd, String> {
    Ok(Cmd::Lawful(parse_world("lawful", tokens)?))
}

fn parse_sweep_commit(tokens: &mut Tokens<'_>) -> Result<Cmd, String> {
    let mut args = SweepArgs::default();
    while let Some(flag) = tokens.next() {
        let flag = flag.to_owned();
        match flag.as_str() {
            "--sizes" => {
                args.sizes = Some(
                    tokens
                        .value(&flag)?
                        .split(',')
                        .map(|raw| parse_u64(&flag, raw))
                        .collect::<Result<_, _>>()?,
                );
            }
            "--samples" => args.samples = Some(parse_u32(&flag, tokens.value(&flag)?)?),
            "--seed" => args.seed = parse_u64(&flag, tokens.value(&flag)?)?,
            "--dir" => args.dir = PathBuf::from(tokens.value(&flag)?),
            _ => return Err(unknown("sweep-commit", &flag)),
        }
    }
    Ok(Cmd::SweepCommit(args))
}

fn parse_storage(tokens: &mut Tokens<'_>) -> Result<Cmd, String> {
    let mut args = StorageArgs::default();
    while let Some(flag) = tokens.next() {
        let flag = flag.to_owned();
        match flag.as_str() {
            "--scales" => args.scales = parse_scale_list(&flag, tokens.value(&flag)?)?,
            "--seed" => args.seed = parse_u64(&flag, tokens.value(&flag)?)?,
            "--dir" => args.dir = PathBuf::from(tokens.value(&flag)?),
            "--churn-dir" => args.churn_dir = Some(PathBuf::from(tokens.value(&flag)?)),
            "--out" => args.out = Some(PathBuf::from(tokens.value(&flag)?)),
            _ => return Err(unknown("storage", &flag)),
        }
    }
    Ok(Cmd::Storage(args))
}

/// A comma-separated durability-lane list; unknown lane tokens are
/// named, an empty list is rejected.
fn parse_lane_list(flag: &str, raw: &str) -> Result<Vec<DurabilityLane>, String> {
    if raw.is_empty() {
        return Err(format!("`{flag}` needs at least one lane"));
    }
    raw.split(',')
        .map(|token| match token {
            "durable" => Ok(DurabilityLane::Durable),
            "nosync" => Ok(DurabilityLane::NoSync),
            other => Err(format!(
                "unknown lane `{other}` (expected durable or nosync)"
            )),
        })
        .collect()
}

/// A comma-separated batch-size list; zero and empty are rejected.
fn parse_batch_list(flag: &str, raw: &str) -> Result<Vec<u32>, String> {
    if raw.is_empty() {
        return Err(format!("`{flag}` needs at least one batch size"));
    }
    raw.split(',')
        .map(|token| {
            let batch = parse_u32(flag, token)?;
            if batch == 0 {
                return Err(format!(
                    "`{flag}` rejects 0 — a commit needs at least one row"
                ));
            }
            Ok(batch)
        })
        .collect()
}

fn parse_writes(tokens: &mut Tokens<'_>) -> Result<Cmd, String> {
    let mut args = WritesArgs::default();
    while let Some(flag) = tokens.next() {
        let flag = flag.to_owned();
        match flag.as_str() {
            "--scale" => args.scale = parse_scale(tokens.value(&flag)?)?,
            "--seed" => args.seed = parse_u64(&flag, tokens.value(&flag)?)?,
            "--dir" => args.dir = PathBuf::from(tokens.value(&flag)?),
            "--lanes" => args.lanes = parse_lane_list(&flag, tokens.value(&flag)?)?,
            "--batches" => args.batches = parse_batch_list(&flag, tokens.value(&flag)?)?,
            "--samples" => args.samples = Some(parse_u32(&flag, tokens.value(&flag)?)?),
            "--out" => args.out = Some(PathBuf::from(tokens.value(&flag)?)),
            _ => return Err(unknown("writes", &flag)),
        }
    }
    Ok(Cmd::Writes(args))
}

fn parse_curves(tokens: &mut Tokens<'_>) -> Result<Cmd, String> {
    let mut args = CurvesArgs::default();
    while let Some(flag) = tokens.next() {
        let flag = flag.to_owned();
        match flag.as_str() {
            "--scales" => args.scales = parse_scale_list(&flag, tokens.value(&flag)?)?,
            "--families" => {
                args.families = Some(tokens.value(&flag)?.split(',').map(str::to_owned).collect());
            }
            "--seed" => args.seed = parse_u64(&flag, tokens.value(&flag)?)?,
            "--dir" => args.dir = PathBuf::from(tokens.value(&flag)?),
            "--samples" => args.samples = Some(parse_u32(&flag, tokens.value(&flag)?)?),
            "--cap-ms" => args.cap_ms = parse_u64(&flag, tokens.value(&flag)?)?,
            "--warmth" => args.warmth = true,
            "--out" => args.out = Some(PathBuf::from(tokens.value(&flag)?)),
            _ => return Err(unknown("curves", &flag)),
        }
    }
    Ok(Cmd::Curves(args))
}

fn parse_churn(tokens: &mut Tokens<'_>) -> Result<Cmd, String> {
    let mut args = ChurnArgs::default();
    while let Some(flag) = tokens.next() {
        let flag = flag.to_owned();
        if corpus_flag(&mut args.corpus, &flag, tokens)? {
            continue;
        }
        match flag.as_str() {
            "--cycles" => args.cycles = parse_u64(&flag, tokens.value(&flag)?)?,
            "--sample-every" => args.sample_every = parse_u64(&flag, tokens.value(&flag)?)?,
            "--vacuum-every" => args.vacuum_every = parse_u64(&flag, tokens.value(&flag)?)?,
            "--analyze-every" => args.analyze_every = parse_u64(&flag, tokens.value(&flag)?)?,
            "--runs" => {
                args.runs = Some(tokens.value(&flag)?.split(',').map(str::to_owned).collect());
            }
            "--out" => args.out = Some(PathBuf::from(tokens.value(&flag)?)),
            _ => return Err(unknown("churn", &flag)),
        }
    }
    Ok(Cmd::Churn(args))
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
        "verify-store" => parse_verify_store(&mut tokens),
        "bench" => parse_bench(&mut tokens),
        "trace" => parse_trace(&mut tokens),
        "scenarios" => parse_scenarios(&mut tokens),
        "crud" => parse_crud(&mut tokens),
        "lawful" => parse_lawful(&mut tokens),
        "sweep-commit" => parse_sweep_commit(&mut tokens),
        "storage" => parse_storage(&mut tokens),
        "writes" => parse_writes(&mut tokens),
        "curves" => parse_curves(&mut tokens),
        "churn" => parse_churn(&mut tokens),
        "merge" => {
            let mut dirs = Vec::new();
            while let Some(token) = tokens.next() {
                dirs.push(PathBuf::from(token));
            }
            if dirs.is_empty() {
                return Err("`merge` needs at least one run directory".to_owned());
            }
            Ok(Cmd::Merge { dirs })
        }
        other => Err(format!("unknown command `{other}`")),
    }
}
