use std::path::Path;

use bumbledb::obs::{self, Category, TraceEvent};
use bumbledb::Answers;

use super::{Scenario, ScenarioQuery, Stores, Surface};
use crate::families::bind_values;
use crate::trace_out::{self, FlameSummary};

/// The warmups a scenario query gets before its warm traced sample —
/// enough to seat the image cache and the resolved-filter views without
/// the ledger protocol's full band (this is a profile, not a timing).
const WARM_ROUNDS: usize = 8;

/// Captures per-query warm+cold traces for one scenario query, writing
/// each as the pair (`.json` Chrome + `.folded` collapsed stacks) under
/// `<trace_root>/trace/scenarios/<scenario>/<query>.{warm,cold}.*`.
/// Returns the warm flame top-10 (+ phase table when the plan is a join)
/// for the report embed — the same table the ledger read families
/// embed.
///
/// The cold half captures the FRESH PREPARE plus first execute (the
/// plan/COLT-build cost a warm reused prepared query never pays); the
/// warm half captures a steady-state execute after [`WARM_ROUNDS`]. This
/// is the schema-agnostic cold: scenario stores span arbitrary schemas,
/// so the cold half is a fresh prepare rather than the Ledger-specific
/// eviction touch the read families use — the honest warm/cold contrast
/// for a world with no synthetic touch relation.
///
/// # Errors
///
/// Prepare/execute failures and trace I/O, as messages.
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
            // COLD: the fresh prepare + first execute, captured together.
            obs::start_capture();
            let cold_span = obs::span(obs::names::SAMPLE, Category::Harness);
            let mut prepared = db
                .prepare(&q)
                .map_err(|e| format!("{}/{}: prepare: {e:?}", scenario.name, sq.name))?;
            let mut buffer = Answers::new();
            db.read(|snap| snap.execute(&mut prepared, &bind_values(&sets[0]), &mut buffer))
                .map_err(|e| format!("{}/{}: cold execute: {e:?}", scenario.name, sq.name))?;
            cold_span.end();
            emit(&dir, &format!("{}.cold", sq.name), obs::finish_capture(), false)?;

            // WARM: seat the caches on the prepared query, then trace one
            // steady-state execute.
            let mut cursor = 0usize;
            let mut run = || {
                let params = bind_values(&sets[cursor % sets.len()]);
                cursor += 1;
                db.read(|snap| snap.execute(&mut prepared, &params, &mut buffer))
                    .map_err(|e| format!("{}/{}: execute: {e:?}", scenario.name, sq.name))?;
                Ok(buffer.len() as u64)
            };
            for _ in 0..WARM_ROUNDS {
                run()?;
            }
            let (_, warm) = crate::harness::traced_sample(&mut run)?;
            Ok(emit(&dir, &format!("{}.warm", sq.name), warm, true)?.unwrap_or_default())
        }
        Surface::KeyedGet { relation, key } => {
            let statement = key((scenario.schema)());
            // COLD: the first keyed get.
            obs::start_capture();
            let cold_span = obs::span(obs::names::SAMPLE, Category::Harness);
            db.read(|snap| snap.get_dyn(*relation, statement, &sets[0]))
                .map_err(|e| format!("{}/{}: cold get_dyn: {e:?}", scenario.name, sq.name))?;
            cold_span.end();
            emit(&dir, &format!("{}.cold", sq.name), obs::finish_capture(), false)?;

            // WARM: seat, then trace one steady-state get.
            let mut cursor = 0usize;
            let mut run = || {
                let params = &sets[cursor % sets.len()];
                cursor += 1;
                let fact = db
                    .read(|snap| snap.get_dyn(*relation, statement, params))
                    .map_err(|e| format!("{}/{}: get_dyn: {e:?}", scenario.name, sq.name))?;
                Ok(std::hint::black_box(fact).map_or(0, |_| 1))
            };
            for _ in 0..WARM_ROUNDS {
                run()?;
            }
            let (_, warm) = crate::harness::traced_sample(&mut run)?;
            Ok(emit(&dir, &format!("{}.warm", sq.name), warm, true)?.unwrap_or_default())
        }
    }
}

/// Splits one capture, writes the `.json`/`.folded` pair, and — when
/// asked — renders the engine flame top-10 (plus the phase table if the
/// plan produced one) for the report embed.
fn emit(
    dir: &Path,
    stem: &str,
    events: Vec<TraceEvent>,
    want_table: bool,
) -> Result<Option<String>, String> {
    let (engine, harness_events) = trace_out::split_harness(events);
    trace_out::write_trace_pair(dir, stem, &engine, &harness_events)
        .map_err(|e| format!("trace: {e}"))?;
    let table = want_table.then(|| {
        let mut table = FlameSummary::compute(&engine).render_top(10);
        if let Some(phases) = trace_out::render_phase_table(&engine) {
            table.push('\n');
            table.push_str(&phases);
        }
        table
    });
    Ok(table)
}
