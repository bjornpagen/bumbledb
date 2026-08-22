use std::path::Path;

use bumbledb::Answers;
use bumbledb::obs::TraceEvent;

use super::{Scenario, ScenarioQuery, Stores, Surface};
use crate::families::bind_values;
use crate::trace_out;

/// The warmups a scenario query gets before its warm traced sample — enough to
/// seat the image cache and the resolved-filter views without the ledger
/// protocol's full band (this is a profile, not a timing).
const WARM_ROUNDS: usize = 8;

/// # Errors
pub(super) fn capture_query(
    stores: &Stores,
    scenario: &Scenario,
    sq: &ScenarioQuery,
    seed: u64,
    trace_root: &Path,
) -> Result<String, String> {
    let sets = (sq.params)(seed);
    if sets.is_empty() {
        return Err(format!(
            "{}/{}: no param sets to trace",
            scenario.name, sq.name
        ));
    }
    let dir = trace_root
        .join("trace")
        .join("scenarios")
        .join(scenario.name);
    let db = &stores.db;

    match &sq.surface {
        Surface::Query(query) => {
            let q = query();

            let mut buffer = Answers::new();
            let mut prepared = None;
            let (_, cold) = crate::harness::traced_sample(&mut || {
                let mut p = db
                    .prepare(&q)
                    .map_err(|e| format!("{}/{}: prepare: {e:?}", scenario.name, sq.name))?;
                db.read(|snap| snap.execute(&mut p, &bind_values(&sets[0]), &mut buffer))
                    .map_err(|e| format!("{}/{}: cold execute: {e:?}", scenario.name, sq.name))?;
                prepared = Some(p);
                Ok(buffer.len() as u64)
            })?;
            emit(&dir, &format!("{}.cold", sq.name), cold, false)?;
            let mut prepared = prepared.expect("the cold sample prepared the query");

            let mut run = |index: usize| {
                let params = bind_values(&sets[index]);
                db.read(|snap| snap.execute(&mut prepared, &params, &mut buffer))
                    .map_err(|e| format!("{}/{}: execute: {e:?}", scenario.name, sq.name))?;
                Ok(buffer.len() as u64)
            };
            for round in 0..WARM_ROUNDS {
                run(round % sets.len())?;
            }
            let median = median_param(sets.len(), &mut |index| timed(|| run(index)))?;
            let (_, warm) = crate::harness::traced_sample(&mut || run(median))?;
            Ok(emit(&dir, &format!("{}.warm", sq.name), warm, true)?.unwrap_or_default())
        }
        Surface::KeyedGet { relation, key } => {
            let statement = key((scenario.schema)());

            let (_, cold) = crate::harness::traced_sample(&mut || {
                let fact = db
                    .read(|snap| snap.get_dyn(*relation, statement, &sets[0]))
                    .map_err(|e| format!("{}/{}: cold get_dyn: {e:?}", scenario.name, sq.name))?;
                Ok(std::hint::black_box(fact).map_or(0, |_| 1))
            })?;
            emit(&dir, &format!("{}.cold", sq.name), cold, false)?;

            let run = |index: usize| {
                let fact = db
                    .read(|snap| snap.get_dyn(*relation, statement, &sets[index]))
                    .map_err(|e| format!("{}/{}: get_dyn: {e:?}", scenario.name, sq.name))?;
                Ok(std::hint::black_box(fact).map_or(0, |_| 1))
            };
            for round in 0..WARM_ROUNDS {
                run(round % sets.len())?;
            }
            let median = median_param(sets.len(), &mut |index| timed(|| run(index)))?;
            let (_, warm) = crate::harness::traced_sample(&mut || run(median))?;
            Ok(emit(&dir, &format!("{}.warm", sq.name), warm, true)?.unwrap_or_default())
        }
    }
}

fn timed(mut run: impl FnMut() -> Result<u64, String>) -> Result<u64, String> {
    let start = std::time::Instant::now();
    run()?;
    Ok(u64::try_from(start.elapsed().as_nanos()).unwrap_or(u64::MAX))
}

/// The median-cost draw's index, each set costed min-of-3 after the warm
/// rounds: the traced execute prices the draw the lane's p50 describes —
/// tracing whichever draw the cursor landed on pinned Zipf heads and misses
/// instead (the 2026-07-25 baseline's 20-150x traced-vs-timed divergences on
/// the joins world).
pub(super) fn median_param(
    count: usize,
    cost: &mut dyn FnMut(usize) -> Result<u64, String>,
) -> Result<usize, String> {
    let mut costs: Vec<(u64, usize)> = Vec::with_capacity(count);
    for index in 0..count {
        let mut best = u64::MAX;
        for _ in 0..3 {
            best = best.min(cost(index)?);
        }
        costs.push((best, index));
    }
    costs.sort_unstable();
    Ok(costs[count / 2].1)
}

fn emit(
    dir: &Path,
    stem: &str,
    events: Vec<TraceEvent>,
    want_table: bool,
) -> Result<Option<String>, String> {
    let table = trace_out::emit_pair(dir, stem, events)?;
    Ok(want_table.then_some(table))
}
