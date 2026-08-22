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
