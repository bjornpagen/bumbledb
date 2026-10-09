use std::path::PathBuf;

use crate::oracle::sqlite::verify::DEFAULT_RANDOM_CASES;
use crate::worlds::corpus_gen::Scale;

use super::{
    AppPerfArgs, BenchArgs, Cmd, CorpusArgs, CurvesArgs, HeapArgs, ProfileArgs, ScenarioArgs,
    StorageArgs, WritesArgs,
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
        read_batch: None,
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
            "--read-batch" => {
                let batch = parse_u32(&flag, tokens.value(&flag)?)?;
                if !(1..=crate::harness::MAX_READ_BATCH).contains(&batch) {
                    return Err(format!(
                        "`{flag}` must be between 1 and {}",
                        crate::harness::MAX_READ_BATCH
                    ));
                }
                args.read_batch = std::num::NonZeroU32::new(batch);
            }
            "--out" => args.out = Some(PathBuf::from(tokens.value(&flag)?)),
            "--i-am-lying" => args.i_am_lying = true,
            _ => return Err(unknown("bench", &flag)),
        }
    }
    Ok(Cmd::Bench(args))
}

fn parse_profile(tokens: &mut Tokens<'_>) -> Result<Cmd, String> {
    let mut corpus = CorpusArgs::default();
    let mut family = None;
    let mut seconds = 10;
    let mut out = None;
    while let Some(flag) = tokens.next() {
        let flag = flag.to_owned();
        if corpus_flag(&mut corpus, &flag, tokens)? {
            continue;
        }
        match flag.as_str() {
            "--family" => family = Some(tokens.value(&flag)?.to_owned()),
            "--seconds" => {
                seconds = parse_u32(&flag, tokens.value(&flag)?)?;
                if !(1..=3600).contains(&seconds) {
                    return Err("`--seconds` must be between 1 and 3600".to_owned());
                }
            }
            "--out" => out = Some(PathBuf::from(tokens.value(&flag)?)),
            _ => return Err(unknown("profile", &flag)),
        }
    }
    let family = family
        .filter(|name| !name.is_empty())
        .ok_or_else(|| "`profile` needs `--family NAME`".to_owned())?;
    Ok(Cmd::Profile(ProfileArgs {
        corpus,
        family,
        seconds,
        out,
    }))
}

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

fn parse_storage(tokens: &mut Tokens<'_>) -> Result<Cmd, String> {
    let mut args = StorageArgs::default();
    while let Some(flag) = tokens.next() {
        let flag = flag.to_owned();
        match flag.as_str() {
            "--scales" => args.scales = parse_scale_list(&flag, tokens.value(&flag)?)?,
            "--seed" => args.seed = parse_u64(&flag, tokens.value(&flag)?)?,
            "--dir" => args.dir = PathBuf::from(tokens.value(&flag)?),
            "--out" => args.out = Some(PathBuf::from(tokens.value(&flag)?)),
            _ => return Err(unknown("storage", &flag)),
        }
    }
    Ok(Cmd::Storage(args))
}

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

fn parse_prefix_list(flag: &str, raw: &str) -> Result<Vec<u64>, String> {
    if raw.is_empty() {
        return Err(format!("`{flag}` needs at least one prefix"));
    }
    raw.split(',')
        .map(|token| {
            let n = parse_u64(flag, token)?;
            if n == 0 {
                return Err(format!("`{flag}` rejects 0 — a prefix needs facts"));
            }
            Ok(n)
        })
        .collect()
}

fn parse_heap(tokens: &mut Tokens<'_>) -> Result<Cmd, String> {
    let mut args = HeapArgs::default();
    while let Some(flag) = tokens.next() {
        let flag = flag.to_owned();
        match flag.as_str() {
            "--scale" => args.scale = parse_scale(tokens.value(&flag)?)?,
            "--seed" => args.seed = parse_u64(&flag, tokens.value(&flag)?)?,
            "--dir" => args.dir = PathBuf::from(tokens.value(&flag)?),
            "--samples" => args.samples = Some(parse_u32(&flag, tokens.value(&flag)?)?),
            "--prefixes" => args.prefixes = parse_prefix_list(&flag, tokens.value(&flag)?)?,
            "--out" => args.out = Some(PathBuf::from(tokens.value(&flag)?)),
            _ => return Err(unknown("heap", &flag)),
        }
    }
    Ok(Cmd::Heap(args))
}

fn parse_app_perf(tokens: &mut Tokens<'_>) -> Result<Cmd, String> {
    let mut args = AppPerfArgs::default();
    while let Some(flag) = tokens.next() {
        let flag = flag.to_owned();
        match flag.as_str() {
            "--scale" => args.scale = parse_scale(tokens.value(&flag)?)?,
            "--seed" => args.seed = parse_u64(&flag, tokens.value(&flag)?)?,
            "--regimes" => {
                args.regimes = Some(
                    tokens
                        .value(&flag)?
                        .split(',')
                        .map(crate::harness::appperf::Regime::parse)
                        .collect::<Result<_, _>>()?,
                );
            }
            "--samples" => args.samples = Some(parse_u32(&flag, tokens.value(&flag)?)?),
            "--tenants" => {
                args.tenants = parse_u32(&flag, tokens.value(&flag)?)?;
                if args.tenants < 2 {
                    return Err(format!(
                        "`{flag}` needs at least 2 — churn needs a hot and a cold tenant"
                    ));
                }
            }
            "--out" => args.out = Some(PathBuf::from(tokens.value(&flag)?)),
            _ => return Err(unknown("app-perf", &flag)),
        }
    }
    Ok(Cmd::AppPerf(args))
}

fn parse_micro(tokens: &mut Tokens<'_>) -> Result<Cmd, String> {
    use crate::harness::micro::{Levels, MicroArgs};
    let mut args = MicroArgs::default();
    while let Some(flag) = tokens.next() {
        let flag = flag.to_owned();
        match flag.as_str() {
            "--levels" => {
                args.levels = match tokens.value(&flag)? {
                    "all" => Levels::All,
                    "" => return Err(format!("`{flag}` needs `all` or level names")),
                    names => Levels::Named(names.split(',').map(str::to_owned).collect()),
                };
            }
            "--elements" => {
                let n = parse_u64(&flag, tokens.value(&flag)?)?;
                args.elements = usize::try_from(n)
                    .ok()
                    .filter(|n| (1..=1 << 24).contains(n))
                    .ok_or_else(|| format!("`{flag}` must be between 1 and 16777216"))?;
            }
            "--float-rows" => {
                args.float_rows = parse_u64(&flag, tokens.value(&flag)?)?;
                if args.float_rows == 0 {
                    return Err(format!("`{flag}` rejects 0 — the folds need rows"));
                }
            }
            "--samples" => {
                args.samples = parse_u32(&flag, tokens.value(&flag)?)?;
                if args.samples == 0 {
                    return Err(format!("`{flag}` rejects 0"));
                }
            }
            "--seed" => args.seed = parse_u64(&flag, tokens.value(&flag)?)?,
            "--dir" => args.dir = PathBuf::from(tokens.value(&flag)?),
            "--out" => args.out = Some(PathBuf::from(tokens.value(&flag)?)),
            _ => return Err(unknown("micro", &flag)),
        }
    }
    Ok(Cmd::Micro(args))
}

/// # Errors
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
        "profile" => parse_profile(&mut tokens),
        "scenarios" => parse_scenarios(&mut tokens),
        "crud" => parse_crud(&mut tokens),
        "lawful" => parse_lawful(&mut tokens),
        "storage" => parse_storage(&mut tokens),
        "writes" => parse_writes(&mut tokens),
        "curves" => parse_curves(&mut tokens),
        "heap" => parse_heap(&mut tokens),
        "micro" => parse_micro(&mut tokens),
        "app-perf" => parse_app_perf(&mut tokens),
        other => Err(format!("unknown command `{other}`")),
    }
}
