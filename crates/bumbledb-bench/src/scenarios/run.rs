use std::path::Path;

use super::load::load;
use super::run_query::{gate, run_query};
use super::{QueryReport, Scenario, all, render};
use crate::harness::Protocol;

/// Runs every scenario (or the selected subset): load, gate, time,
/// report. Returns the rendered markdown; the caller writes artifacts.
///
/// # Errors
///
/// Load/prepare/translate failures and oracle disagreements, as
/// messages naming the scenario and query.
pub fn run(
    dir: &Path,
    seed: u64,
    proto: Protocol,
    only: Option<&[String]>,
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
            reports.push(run_query(&stores, &scenario, &sq, seed, proto)?);
        }
    }
    if reports.is_empty() {
        return Err("no scenario selected".to_owned());
    }
    Ok((render(&reports, proto), reports))
}

/// Gates one scenario without timing anything: load, then the oracle
/// gate for every query × param set × `SQLite` lane — the world
/// packets' smoke-test entry (correctness only; zero measured windows).
///
/// # Errors
///
/// Load/prepare/translate failures and oracle disagreements, as
/// messages naming the scenario, query, and lane.
pub fn gate_scenario(dir: &Path, scenario: &Scenario, seed: u64) -> Result<(), String> {
    let stores = load(dir, scenario, seed)?;
    for sq in (scenario.queries)() {
        eprintln!("scenario {}: gate {}", scenario.name, sq.name);
        gate(&stores, scenario, &sq, seed)?;
    }
    Ok(())
}
