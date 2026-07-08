use bumbledb::schema::ValueType;
use bumbledb::ResultBuffer;

use super::{QueryReport, Scenario, ScenarioQuery, Stores};
use crate::compare;
use crate::harness::{self, Protocol, Rotation};
use crate::sqlite_run::PreparedFamily;
use crate::translate::translate;

/// Gates then times one query on both engines. The gate compares every
/// param set's result multisets (`compare::multisets`) — a disagreement
/// is an error naming the query and set, and nothing gets timed.
pub(super) fn run_query(
    stores: &Stores,
    scenario: &Scenario,
    sq: &ScenarioQuery,
    seed: u64,
    proto: Protocol,
) -> Result<QueryReport, String> {
    let schema = (scenario.schema)();
    let query = (sq.query)();
    let sets = (sq.params)(seed);
    let mut prepared = stores
        .db
        .prepare(&query)
        .map_err(|e| format!("{}/{}: prepare: {e:?}", scenario.name, sq.name))?;
    let types: Vec<ValueType> = prepared.column_types().cloned().collect();
    let translated =
        translate(&query, schema).map_err(|e| format!("{}/{}: {e}", scenario.name, sq.name))?;

    // The oracle gate: agreement on every param set before any timing.
    for (idx, params) in sets.iter().enumerate() {
        let mut buffer = ResultBuffer::new();
        stores
            .db
            .read(|snap| snap.execute(&mut prepared, params, &mut buffer))
            .map_err(|e| format!("{}/{}: execute: {e:?}", scenario.name, sq.name))?;
        let ours = compare::from_buffer(&buffer, &types);
        let mut stmt = stores
            .conn
            .prepare_cached(&translated.sql)
            .map_err(|e| format!("{}/{}: oracle prepare: {e}", scenario.name, sq.name))?;
        let theirs = compare::from_sqlite(&mut stmt, &translated.params, params, &types)
            .map_err(|e| format!("{}/{}: oracle execute: {e}", scenario.name, sq.name))?;
        compare::multisets(ours, theirs).map_err(|mismatch| {
            format!(
                "{}/{} param set {idx}: ENGINES DISAGREE — not timing a wrong answer\n{mismatch}",
                scenario.name, sq.name
            )
        })?;
    }

    // Timing, the ledger protocol: rotation across param sets, medians.
    let mut rotation = Rotation::new(sets.clone());
    let mut buffer = ResultBuffer::new();
    let db = &stores.db;
    let ours = harness::measure(proto, || {
        let params = rotation.next_set().to_vec();
        db.read(|snap| snap.execute(&mut prepared, &params, &mut buffer))
            .map_err(|e| format!("execute: {e:?}"))?;
        Ok(buffer.len() as u64)
    })?;

    let mut sqlite_family = PreparedFamily::new(&stores.conn, &translated, types)?;
    let mut rotation = Rotation::new(sets);
    let theirs = harness::measure(proto, || {
        crate::sqlite_run::sample(&mut sqlite_family, rotation.next_set())
    })?;

    #[allow(clippy::cast_precision_loss)]
    let ratio_p50 = ours.stats.p50 as f64 / theirs.stats.p50.max(1) as f64;
    Ok(QueryReport {
        scenario: scenario.name,
        name: sq.name,
        about: sq.about,
        rows: ours.work / u64::from(proto.samples.max(1)),
        ours: ours.stats,
        theirs: theirs.stats,
        ratio_p50,
    })
}
