use std::path::Path;

use super::load::load;
use super::run_query::{gate, run_query};
use super::{QueryModes, QueryReport, Scenario, all, render};
use crate::harness::Protocol;

/// # Errors
pub fn run(
    dir: &Path,
    seed: u64,
    proto: Protocol,
    only: Option<&[String]>,
    modes: &QueryModes,
) -> Result<(String, Vec<QueryReport>), String> {
    let mut reports = Vec::new();
    for scenario in all() {
        if let Some(only) = only
            && !only.iter().any(|n| n == scenario.name)
        {
            continue;
        }
        let stores = load(dir, &scenario, seed)?;
        for sq in (scenario.queries)() {
            eprintln!("scenario {}: {}", scenario.name, sq.name);
            reports.push(run_query(&stores, &scenario, &sq, seed, proto, modes)?);
        }
    }
    if reports.is_empty() {
        return Err("no scenario selected".to_owned());
    }
    Ok((render(&reports, proto), reports))
}

/// # Errors
pub fn gate_scenario(dir: &Path, scenario: &Scenario, seed: u64) -> Result<(), String> {
    let stores = load(dir, scenario, seed)?;
    for sq in (scenario.queries)() {
        eprintln!("scenario {}: gate {}", scenario.name, sq.name);
        gate(&stores, scenario, &sq, seed)?;
    }
    Ok(())
}

/// Fresh diagnostic corpus, ordinary independent oracle, then only the
/// selected query in the native window. Never erase a previous capture.
pub(crate) fn profile(
    dir: &Path,
    args: &crate::cli::ProfileArgs,
) -> Result<Option<crate::driver::profile::ProfileResult>, String> {
    for scenario in all() {
        let Some(query) = (scenario.queries)()
            .into_iter()
            .find(|q| q.name == args.family)
        else {
            continue;
        };
        if args.corpus.scale != crate::corpus_gen::Scale::S {
            return Err("scenario profile corpora have one fixed scale; use --scale S".to_owned());
        }
        if let Some(parent) = dir.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("profile parent: {e}"))?;
        }
        std::fs::create_dir(dir).map_err(|e| format!("fresh profile corpus: {e}"))?;
        let stores = load(dir, &scenario, args.corpus.seed)?;
        return super::run_query::profile(&stores, &scenario, &query, args).map(Some);
    }
    Ok(None)
}
