use std::path::PathBuf;

use crate::corpus_gen::Scale;
use crate::duralane::DurabilityLane;
use crate::verify::DEFAULT_RANDOM_CASES;

use super::{
    AppPerfArgs, BenchArgs, Cmd, CorpusArgs, CorpusFloatArgs, CurvesArgs, HashProbeArgs, HeapArgs,
    ProfileArgs, ScenarioArgs, StorageArgs, StorageProfile, WritesArgs,
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
        alloc: false,
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
            "--alloc" => args.alloc = true,
            "--ephemeral" | "--nosync" => {
                return Err(format!(
                    "`{flag}` was retired with the engine's no-sync constructor surface \
                     (ENG-008): production stores are durable-only and the bench does not \
                     re-add a weakened lane. Run without the flag."
                ));
            }
            "--proxy-per-rep" => args.proxy_per_rep = true,
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
            "--alloc" if cmd == "scenarios" => args.alloc = true,
            "--alloc" => {
                return Err(format!(
                    "`{cmd}` has no alloc pass — `--alloc` is a `scenarios` mode"
                ));
            }
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
    let mut home_options = false;
    let mut corpus_options = false;
    while let Some(flag) = tokens.next() {
        let flag = flag.to_owned();
        match flag.as_str() {
            "--profile" => {
                args.profile = match tokens.value(&flag)? {
                    "corpus" => StorageProfile::Corpus,
                    "home-costs" => StorageProfile::HomeCosts,
                    other => return Err(format!("unknown storage profile `{other}`")),
                };
            }
            "--rows" => {
                home_options = true;
                args.rows = parse_u64(&flag, tokens.value(&flag)?)?;
            }
            "--samples" => {
                home_options = true;
                args.samples = parse_u32(&flag, tokens.value(&flag)?)?;
            }
            "--scales" => {
                corpus_options = true;
                args.scales = parse_scale_list(&flag, tokens.value(&flag)?)?;
            }
            "--seed" => args.seed = parse_u64(&flag, tokens.value(&flag)?)?,
            "--dir" => args.dir = PathBuf::from(tokens.value(&flag)?),
            "--out" => args.out = Some(PathBuf::from(tokens.value(&flag)?)),
            _ => return Err(unknown("storage", &flag)),
        }
    }
    match args.profile {
        StorageProfile::Corpus if home_options => {
            return Err("--rows/--samples require storage --profile home-costs".into());
        }
        StorageProfile::HomeCosts if corpus_options => {
            return Err("home-costs cannot use --scales".into());
        }
        StorageProfile::HomeCosts => crate::space::variants::validate_home_args(&args)?,
        StorageProfile::Corpus => {}
    }
    Ok(Cmd::Storage(args))
}

fn parse_lane_list(flag: &str, raw: &str) -> Result<Vec<DurabilityLane>, String> {
    if raw.is_empty() {
        return Err(format!("`{flag}` needs at least one lane"));
    }
    raw.split(',')
        .map(|token| match token {
            "durable" => Ok(DurabilityLane::Durable),
            "nosync" => Err(
                "lane `nosync` was retired with the engine's no-sync constructor surface \
                 (ENG-008); only `durable` remains"
                    .to_owned(),
            ),
            other => Err(format!("unknown lane `{other}` (expected durable)")),
        })
        .collect()
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

/// Seed values for the float corpus accept `0x`-prefixed hex — the P11
/// regeneration command pins `--seed 0xB0B`.
fn parse_u64_maybe_hex(flag: &str, raw: &str) -> Result<u64, String> {
    let parsed = raw
        .strip_prefix("0x")
        .or_else(|| raw.strip_prefix("0X"))
        .map_or_else(|| raw.parse(), |hex| u64::from_str_radix(hex, 16));
    parsed.map_err(|_| format!("`{flag}` needs an integer (decimal or 0x-hex), got `{raw}`"))
}

fn parse_corpus_float(tokens: &mut Tokens<'_>) -> Result<Cmd, String> {
    let mut args = CorpusFloatArgs::default();
    while let Some(flag) = tokens.next() {
        let flag = flag.to_owned();
        match flag.as_str() {
            "--seed" => args.seed = parse_u64_maybe_hex(&flag, tokens.value(&flag)?)?,
            "--random" => args.random = parse_u64(&flag, tokens.value(&flag)?)?,
            "--groups" => args.groups = parse_u64(&flag, tokens.value(&flag)?)?,
            "--group-size" => {
                let n = parse_u64(&flag, tokens.value(&flag)?)?;
                if n == 0 {
                    return Err(format!("`{flag}` rejects 0 — a group needs payloads"));
                }
                args.group_size =
                    usize::try_from(n).map_err(|_| format!("`{flag}` is too large, got `{n}`"))?;
            }
            "--out" => args.out = PathBuf::from(tokens.value(&flag)?),
            _ => return Err(unknown("corpus-float", &flag)),
        }
    }
    Ok(Cmd::CorpusFloat(args))
}

fn parse_hash_probe(tokens: &mut Tokens<'_>) -> Result<Cmd, String> {
    let mut args = HashProbeArgs::default();
    while let Some(flag) = tokens.next() {
        let flag = flag.to_owned();
        match flag.as_str() {
            "--seed" => args.seed = parse_u64(&flag, tokens.value(&flag)?)?,
            "--samples" => args.samples = Some(parse_u32(&flag, tokens.value(&flag)?)?),
            "--kat" => args.kat = Some(PathBuf::from(tokens.value(&flag)?)),
            "--out" => args.out = Some(PathBuf::from(tokens.value(&flag)?)),
            _ => return Err(unknown("hash-probe", &flag)),
        }
    }
    Ok(Cmd::HashProbe(args))
}

fn parse_app_perf(tokens: &mut Tokens<'_>) -> Result<Cmd, String> {
    let mut args = AppPerfArgs::default();
    while let Some(flag) = tokens.next() {
        let flag = flag.to_owned();
        match flag.as_str() {
            "--scale" => args.scale = parse_scale(tokens.value(&flag)?)?,
            "--seed" => args.seed = parse_u64(&flag, tokens.value(&flag)?)?,
            "--dir" => args.dir = PathBuf::from(tokens.value(&flag)?),
            "--regimes" => {
                let raw = tokens.value(&flag)?;
                let regimes: Vec<String> = raw.split(',').map(str::to_owned).collect();
                for regime in &regimes {
                    if !matches!(
                        regime.as_str(),
                        "warm" | "cold-open" | "post-write" | "large-result" | "tenant-churn"
                    ) {
                        return Err(format!(
                            "unknown regime `{regime}` (expected warm, cold-open, post-write, \
                             large-result, or tenant-churn; selective runs through `bench \
                             --families`, hosted-contention and maintenance run through the \
                             log lanes)"
                        ));
                    }
                }
                args.regimes = Some(regimes);
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
            "--plan" => args.plan = true,
            _ => return Err(unknown("app-perf", &flag)),
        }
    }
    Ok(Cmd::AppPerf(args))
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
        "corpus-float" => parse_corpus_float(&mut tokens),
        "hash-probe" => parse_hash_probe(&mut tokens),
        "app-perf" => parse_app_perf(&mut tokens),
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
