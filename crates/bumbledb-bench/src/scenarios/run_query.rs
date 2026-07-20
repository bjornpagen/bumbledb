use std::cell::Cell;

use bumbledb::schema::{SchemaDescriptor, StatementView, ValueType};
use bumbledb::{Answers, RelationId, StatementId, Value};

use super::{LaneOutcome, LaneReport, QueryReport, Scenario, ScenarioQuery, Stores, Surface, Twin};
use crate::compare;
use crate::families::bind_values;
use crate::harness::{self, Protocol, Rotation};
use crate::sqlite_run::{CapOutcome, PreparedFamily, sample_capped};
use crate::translate::{Translated, translate};

/// The gated engine executor — one arm per [`Surface`]: what the gate
/// proved agreement for is exactly what the timing half executes, no
/// re-derivation in between.
#[expect(
    clippy::large_enum_variant,
    reason = "one Engine exists per gated query — boxing the prepared \
              query would put an indirection on the timed path to save \
              bytes nothing is short of"
)]
enum Engine<'d> {
    /// A prepared query, reused across every warmup and sample.
    Prepared(bumbledb::PreparedQuery<'d, SchemaDescriptor>),
    /// The keyed-get point read: `Snapshot::get_dyn` through the resolved
    /// key statement — no query, no plan, no prepared object to hold.
    KeyedGet {
        relation: RelationId,
        statement: StatementId,
    },
}

impl Engine<'_> {
    /// The engine side of one param set, decoded to canonical answers —
    /// the gate's one execution fold over either surface (a keyed get
    /// answers a 0-or-1-row multiset: the full fact in declaration
    /// order).
    fn answers(
        &mut self,
        stores: &Stores,
        types: &[ValueType],
        params: &[Value],
    ) -> Result<Vec<compare::Answer>, String> {
        match self {
            Self::Prepared(prepared) => {
                let mut buffer = Answers::new();
                stores
                    .db
                    .read(|snap| snap.execute(prepared, &bind_values(params), &mut buffer))
                    .map_err(|e| format!("execute: {e:?}"))?;
                Ok(compare::from_answers(&buffer, types))
            }
            Self::KeyedGet {
                relation,
                statement,
            } => stores
                .db
                .read(|snap| snap.get_dyn(*relation, *statement, params))
                .map_err(|e| format!("get_dyn: {e:?}"))?
                .map(|fact| compare::from_fact(&fact))
                .into_iter()
                .collect(),
        }
    }
}

/// The gated pre-timing state: everything [`gate`] proved agreement for,
/// carried into the timing half — the gate/time split makes
/// "oracle-gated before ever timed" a call-order fact.
pub(super) struct Gated<'d> {
    engine: Engine<'d>,
    types: Vec<ValueType>,
    /// The `SQLite` lane list from [`Twin`]: `(lane name, SQL)`.
    lanes: Vec<(&'static str, Translated)>,
    sets: Vec<Vec<Value>>,
}

/// Gates one query: prepares the engine side (per its [`Surface`]),
/// builds the lane list from [`Twin`], and for EVERY param set × EVERY
/// lane compares the result multisets (`compare::multisets`) — a
/// disagreement is an error naming the query, lane, and set, and nothing
/// gets timed. The gate is NEVER capped: correctness is sacred.
pub(super) fn gate<'d>(
    stores: &'d Stores,
    scenario: &Scenario,
    sq: &ScenarioQuery,
    seed: u64,
) -> Result<Gated<'d>, String> {
    let schema = (scenario.schema)();
    let sets = (sq.params)(seed);
    let (mut engine, types, lanes) = match &sq.surface {
        Surface::Query(query) => {
            let query = query();
            let prepared = stores
                .db
                .prepare(&query)
                .map_err(|e| format!("{}/{}: prepare: {e:?}", scenario.name, sq.name))?;
            let types: Vec<ValueType> = prepared
                .predicate()
                .columns
                .iter()
                .map(|column| column.ty.clone())
                .collect();
            let canonical = || {
                translate(&query, schema, &[])
                    .map_err(|e| format!("{}/{}: {e}", scenario.name, sq.name))
            };
            let lanes: Vec<(&'static str, Translated)> = match sq.twin {
                Twin::Canonical => vec![("sqlite", canonical()?)],
                Twin::Tuned(tuned) => vec![("sqlite", canonical()?), ("sqlite-tuned", tuned())],
                Twin::Hand(hand) => vec![("sqlite-hand", hand())],
            };
            (Engine::Prepared(prepared), types, lanes)
        }
        Surface::KeyedGet { relation, key } => {
            let statement = key(schema);
            let StatementView::Key(_, key_statement) = schema.statement(statement) else {
                return Err(format!(
                    "{}/{}: statement {statement:?} is not a key statement",
                    scenario.name, sq.name
                ));
            };
            let types: Vec<ValueType> = schema
                .relation(*relation)
                .fields()
                .iter()
                .map(|field| field.value_type.clone())
                .collect();
            // A keyed get's canonical rendering IS SQLite's best shot —
            // the prepared point SELECT through the UNIQUE index — so no
            // tuned/hand lane exists to declare (asserted by test).
            let Twin::Canonical = sq.twin else {
                return Err(format!(
                    "{}/{}: a keyed-get twin is canonical-only",
                    scenario.name, sq.name
                ));
            };
            let lanes = vec![(
                "sqlite",
                crate::translate::keyed_get(schema, *relation, key_statement),
            )];
            (
                Engine::KeyedGet {
                    relation: *relation,
                    statement,
                },
                types,
                lanes,
            )
        }
    };

    // The oracle gate: agreement on every param set × lane before any
    // timing, uncapped always.
    for (idx, params) in sets.iter().enumerate() {
        let ours = engine
            .answers(stores, &types, params)
            .map_err(|e| format!("{}/{}: {e}", scenario.name, sq.name))?;
        let args: Vec<crate::naive::ParamValue> = params
            .iter()
            .map(|value| crate::naive::ParamValue::Scalar(value.clone()))
            .collect();
        for (lane, translated) in &lanes {
            let mut stmt = stores.conn.prepare_cached(&translated.sql).map_err(|e| {
                format!(
                    "{}/{} lane {lane}: oracle prepare: {e}",
                    scenario.name, sq.name
                )
            })?;
            let theirs = compare::from_sqlite(&mut stmt, &translated.params, &args, &types)
                .map_err(|e| {
                    format!(
                        "{}/{} lane {lane}: oracle execute: {e}",
                        scenario.name, sq.name
                    )
                })?;
            compare::multisets(ours.clone(), theirs).map_err(|mismatch| {
                format!(
                    "{}/{} lane {lane} param set {idx}: ENGINES DISAGREE — not timing a wrong answer\n{mismatch}",
                    scenario.name, sq.name
                )
            })?;
        }
    }
    Ok(Gated {
        engine,
        types,
        lanes,
        sets,
    })
}

/// Gates then times one query: the engine side under the ledger
/// protocol, then every `SQLite` lane — uncapped lanes exactly as
/// before; capped lanes pre-flight one untimed sample per param set and
/// report [`LaneOutcome::ExceededCap`] the moment any sample trips (no
/// censored percentiles can exist).
pub(super) fn run_query(
    stores: &Stores,
    scenario: &Scenario,
    sq: &ScenarioQuery,
    seed: u64,
    proto: Protocol,
) -> Result<QueryReport, String> {
    let Gated {
        mut engine,
        types,
        lanes,
        sets,
    } = gate(stores, scenario, sq, seed)?;

    // Timing, the ledger protocol: rotation across param sets, medians.
    let mut rotation = Rotation::new(sets.clone());
    let db = &stores.db;
    let ours = match &mut engine {
        Engine::Prepared(prepared) => {
            let mut buffer = Answers::new();
            harness::measure(proto, || {
                let params = bind_values(rotation.next_set());
                db.read(|snap| snap.execute(prepared, &params, &mut buffer))
                    .map_err(|e| format!("execute: {e:?}"))?;
                Ok(buffer.len() as u64)
            })?
        }
        Engine::KeyedGet {
            relation,
            statement,
        } => harness::measure(proto, || {
            let fact = db
                .read(|snap| snap.get_dyn(*relation, *statement, rotation.next_set()))
                .map_err(|e| format!("get_dyn: {e:?}"))?;
            // The decoded fact is the work — black-boxed so the owned
            // values are never dead code; count mirrors the row contract.
            Ok(std::hint::black_box(fact).map_or(0, |_| 1))
        })?,
    };

    #[expect(
        clippy::cast_precision_loss,
        reason = "reporting accepts lossy integer-to-float conversion"
    )]
    let ratio = |theirs_p50: u64| ours.stats.p50 as f64 / theirs_p50.max(1) as f64;

    let mut lane_reports = Vec::with_capacity(lanes.len());
    for (lane, translated) in &lanes {
        let mut family = PreparedFamily::new(&stores.conn, translated, types.clone())?;
        let outcome = match sq.cap {
            None => {
                // Uncapped: exactly the pre-cap protocol, no handler ever
                // installed.
                let mut rotation = Rotation::new(sets.clone());
                let theirs = harness::measure(proto, || {
                    crate::sqlite_run::sample(&mut family, rotation.next_set())
                })?;
                LaneOutcome::Timed {
                    stats: theirs.stats,
                    ratio_p50: ratio(theirs.stats.p50),
                }
            }
            Some(cap) => {
                // Pre-flight: one untimed capped sample per param set —
                // a lane that cannot finish never enters a timed window.
                let mut preflight_tripped = false;
                for params in &sets {
                    if sample_capped(&mut family, &stores.conn, cap, params)? == CapOutcome::Tripped
                    {
                        preflight_tripped = true;
                        break;
                    }
                }
                if preflight_tripped {
                    LaneOutcome::ExceededCap { cap }
                } else {
                    let mut rotation = Rotation::new(sets.clone());
                    let tripped = Cell::new(false);
                    let conn = &stores.conn;
                    let measured = harness::measure(proto, || {
                        match sample_capped(&mut family, conn, cap, rotation.next_set())? {
                            CapOutcome::Done(count) => Ok(count),
                            CapOutcome::Tripped => {
                                tripped.set(true);
                                Err("cap tripped".into())
                            }
                        }
                    });
                    if tripped.get() {
                        LaneOutcome::ExceededCap { cap }
                    } else {
                        let theirs = measured?;
                        LaneOutcome::Timed {
                            stats: theirs.stats,
                            ratio_p50: ratio(theirs.stats.p50),
                        }
                    }
                }
            }
        };
        lane_reports.push(LaneReport { lane, outcome });
    }

    Ok(QueryReport {
        scenario: scenario.name,
        name: sq.name,
        about: sq.about,
        answers: ours.work / u64::from(proto.samples.max(1)),
        ours: ours.stats,
        lanes: lane_reports,
    })
}
