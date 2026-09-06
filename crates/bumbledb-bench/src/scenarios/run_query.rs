use std::cell::Cell;

use bumbledb::schema::{SchemaDescriptor, StatementView, ValueType};
use bumbledb::{Answers, RelationId, StatementId, Value};

use super::{
    LaneOutcome, LaneReport, QueryModes, QueryReport, Scenario, ScenarioQuery, Stores, Surface,
    Twin,
};
use crate::compare;
use crate::families::bind_values;
use crate::harness::{self, Modes, Protocol, Rotation};
use crate::sqlite_run::{CapOutcome, PreparedFamily, sample_capped};
use crate::translate::{Translated, translate};

#[expect(
    clippy::large_enum_variant,
    reason = "one Engine exists per gated query — boxing the prepared \
              query would put an indirection on the timed path to save \
              bytes nothing is short of"
)]
enum Engine {
    Prepared(bumbledb::PreparedQuery<SchemaDescriptor>),

    KeyedGet {
        relation: RelationId,
        statement: StatementId,
    },
}

impl Engine {
    /// The same operation in timing, allocation and native-sampling windows.
    fn sample(
        &mut self,
        stores: &Stores,
        params: &[Value],
        buffer: &mut Answers,
    ) -> Result<u64, String> {
        match self {
            Self::Prepared(prepared) => {
                let params = bind_values(params);
                stores
                    .db
                    .read(crate::harness::bench_work(), |snap| {
                        snap.execute(prepared, &params, buffer)
                    })
                    .map_err(|e| format!("execute: {e:?}"))?;
                Ok(std::hint::black_box(buffer).len() as u64)
            }
            Self::KeyedGet {
                relation,
                statement,
            } => {
                let fact = stores
                    .db
                    .read(crate::harness::bench_work(), |snap| {
                        snap.get_dyn(*relation, *statement, params)
                    })
                    .map_err(|e| format!("get_dyn: {e:?}"))?;
                Ok(std::hint::black_box(fact).map_or(0, |_| 1))
            }
        }
    }

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
                    .read(crate::harness::bench_work(), |snap| {
                        snap.execute(prepared, &bind_values(params), &mut buffer)
                    })
                    .map_err(|e| format!("execute: {e:?}"))?;
                Ok(compare::from_answers(&buffer, types))
            }
            Self::KeyedGet {
                relation,
                statement,
            } => Ok(stores
                .db
                .read(crate::harness::bench_work(), |snap| {
                    snap.get_dyn(*relation, *statement, params)
                })
                .map_err(|e| format!("get_dyn: {e:?}"))?
                .map(|fact| compare::from_fact(&fact))
                .into_iter()
                .collect()),
        }
    }
}

/// The gated pre-timing state: everything [`gate`] proved agreement for,
/// carried into the timing half — the gate/time split makes "oracle-gated
/// before ever timed" a call-order fact.
pub(super) struct Gated {
    engine: Engine,
    types: Vec<ValueType>,

    lanes: Vec<(&'static str, Translated)>,
    sets: Vec<Vec<Value>>,
}

pub(super) fn gate(
    stores: &Stores,
    scenario: &Scenario,
    sq: &ScenarioQuery,
    seed: u64,
) -> Result<Gated, String> {
    let schema = (scenario.schema)();
    let sets = (sq.params)(seed);
    let (mut engine, types, lanes) = match &sq.surface {
        Surface::Query(query) => {
            let query = query();
            let prepared = stores
                .db
                .prepare(&query, crate::harness::bench_work())
                .map_err(|e| format!("{}/{}: prepare: {e:?}", scenario.name, sq.name))?;
            let types: Vec<ValueType> = prepared
                .signature()
                .columns
                .iter()
                .map(|column| *column.ty())
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
                .map(|field| field.value_type)
                .collect();

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

    // The oracle gate checks every parameter set and lane before timing.
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

/// Gates then times one query: the engine side under the ledger protocol, then
/// every `SQLite` lane — uncapped lanes exactly as before; capped lanes
/// pre-flight one untimed sample per param set and report
/// [`LaneOutcome::ExceededCap`] the moment any sample trips (no censored
/// percentiles can exist). The optional allocation pass ([`QueryModes`])
/// run after timing, each a separate scoped window.
pub(super) fn run_query(
    stores: &Stores,
    scenario: &Scenario,
    sq: &ScenarioQuery,
    seed: u64,
    proto: Protocol,
    modes: &QueryModes,
) -> Result<QueryReport, String> {
    let Gated {
        mut engine,
        types,
        lanes,
        sets,
    } = gate(stores, scenario, sq, seed)?;

    // Timing, the ledger protocol: rotation across param sets, medians.
    let mut rotation = Rotation::new(sets.clone());
    let mut buffer = Answers::new();
    let ours = harness::measure(proto, || {
        engine.sample(stores, rotation.next_set(), &mut buffer)
    })?;

    // alloc window over the same protocol, so `scenarios --alloc` scopes

    let alloc = if modes.alloc {
        let mut rotation = Rotation::new(sets.clone());
        let alloc_modes = Modes {
            alloc_window: true,
            ..Modes::default()
        };
        let mut buffer = Answers::new();
        let measured = harness::measure_batched(proto, alloc_modes, 1, || {
            engine.sample(stores, rotation.next_set(), &mut buffer)
        })?;
        measured.alloc.map(crate::report::AllocReport::from)
    } else {
        None
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
        alloc,
    })
}

pub(super) fn profile(
    stores: &Stores,
    scenario: &Scenario,
    query: &ScenarioQuery,
    args: &crate::cli::ProfileArgs,
) -> Result<crate::driver::profile::ProfileResult, String> {
    let Gated {
        mut engine,
        types,
        sets,
        ..
    } = gate(stores, scenario, query, args.corpus.seed)?;
    let surface = match &engine {
        Engine::Prepared(prepared) => prepared.rendered_query().to_owned(),
        Engine::KeyedGet {
            relation,
            statement,
        } => format!("get({relation:?}, {statement:?})"),
    };
    let input_digest = crate::driver::profile::input_fingerprint(&(
        bumbledb::schema::fingerprint::fingerprint(stores.db.schema()),
        surface,
        &sets,
    ));
    let mut expected = Vec::with_capacity(sets.len());
    let mut digest = bumbledb::digest::Digest::new();
    for params in &sets {
        let mut answers = engine.answers(stores, &types, params)?;
        expected.push(answers.len() as u64);
        answers.sort_unstable();
        let encoded = format!("{answers:?}");
        digest.update(&(encoded.len() as u64).to_le_bytes());
        digest.update(encoded.as_bytes());
    }
    let mut buffer = Answers::new();
    crate::driver::profile::profile_cycles(
        args,
        &expected,
        input_digest,
        crate::corpus_gen::digest_hex(&digest.finalize()),
        |index| engine.sample(stores, &sets[index], &mut buffer),
    )
}
