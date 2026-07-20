//! The crud orchestration: ONE linear fold over
//! `duralane::ALL × crud::families()` with the oracle gate and the
//! post-state comparator as mandatory stages of the fold — a family
//! cannot be timed un-gated and a lane cannot finish un-compared,
//! because no code path around the stages exists. Everything here is
//! REPORT-class: no budget gate ever reads a crud number.
//!
//! Stage order per lane: load the durability-paired twin
//! ([`super::corpus::load_stores`]), gate the read query (value-identical
//! multiset agreement on every param set — UNCONDITIONAL, even when
//! `--only` filters `crud_read_point` out: the mixed lane times the same
//! query, so an ungated run could still time a wrong answer), time the
//! selected families in registry order, then judge the post-states of
//! BOTH relations ([`crate::poststate`]) — a divergence is a run-failing
//! `Err`, never a report footnote.

use std::path::Path;

use bumbledb::schema::ValueType;
use bumbledb::{Answers, Db};
use rusqlite::Connection;

use crate::corpus_gen::Scale;
use crate::duralane::{self, DurabilityLane};
use crate::families::bind_values;
use crate::harness::{self, Measurement, Protocol, Rotation};
use crate::sqlite_run::{self, PreparedFamily};
use crate::translate::{self, Translated};
use crate::{clockproxy, compare, poststate, report};

use super::lanes::{self, FreshCursor, read_query};
use super::{CrudSizes, CrudWorld, corpus, families, ids, ops, render, schema};

/// One timed crud comparison row: a family under one durability lane,
/// both engines' percentiles, the p50 ratio, the engine side's summed
/// work count, and the clock-proxy stamp around the pair.
#[derive(Debug, Clone)]
pub struct CrudRow {
    /// The registry name (`crud_read_point`, `crud_insert`, …).
    pub family: &'static str,
    /// The durability lane label (`durable` / `nosync`).
    pub lane: &'static str,
    /// The registry's honest one-line description.
    pub about: &'static str,
    /// Our percentiles, nanoseconds.
    pub ours: harness::Stats,
    /// `SQLite`'s percentiles, nanoseconds.
    pub theirs: harness::Stats,
    /// `ours.p50 / theirs.p50` (lower is better; <1 = bumbledb faster).
    pub ratio_p50: f64,
    /// The engine side's summed per-sample work (the anti-dead-code
    /// contract's counter).
    pub work: u64,
    /// The clock-proxy bracket around the family pair.
    pub ghz: Option<report::GhzReport>,
}

/// The lane loader seam: the fold's ONLY store source. [`run_with`]
/// binds it to [`corpus::load_stores`]; the gate tests bind a loader
/// that poisons the mirror after loading — the fold itself (gate,
/// timing, post-state) is identical either way, so the tests exercise
/// the exact stages the real run takes.
pub(crate) type LaneLoader<'a> =
    dyn Fn(&Path, DurabilityLane) -> Result<(Db<CrudWorld>, Connection), String> + 'a;

/// The CLI entry: the crud run at the one timed OLTP shape
/// (`CrudSizes::of(Scale::S)`). Returns `(markdown, json)`; the caller
/// writes artifacts.
///
/// # Errors
///
/// Device-honesty refusals, unknown `--only` names, load/prepare/
/// translate failures, oracle disagreements, runner errors, and
/// post-state divergences — each a message naming the family and lane.
pub fn run(
    dir: &Path,
    seed: u64,
    samples: Option<u32>,
    only: Option<&[String]>,
) -> Result<(String, String), String> {
    run_with(dir, seed, CrudSizes::of(Scale::S), samples, only)
}

/// [`run`] with the corpus shape explicit — the test entry (`Tiny`
/// smoke runs) and the delegation target.
///
/// # Errors
///
/// As [`run`].
pub fn run_with(
    dir: &Path,
    seed: u64,
    sizes: CrudSizes,
    samples: Option<u32>,
    only: Option<&[String]>,
) -> Result<(String, String), String> {
    fold(dir, seed, sizes, samples, only, &|lane_dir, lane| {
        corpus::load_stores(lane_dir, seed, sizes, lane)
    })
}

/// The fold itself: every stage in one function, in stage order, with
/// the store source injected ([`LaneLoader`]) and nothing else — there
/// is no entry that times a family without passing the gate first and
/// the post-state judgment after.
pub(crate) fn fold(
    dir: &Path,
    seed: u64,
    sizes: CrudSizes,
    samples: Option<u32>,
    only: Option<&[String]>,
    load: &LaneLoader<'_>,
) -> Result<(String, String), String> {
    // (1) Device honesty FIRST — every crud lane here is timed, and the
    // rule is symmetric (docs/architecture/60-validation.md § the
    // ramdisk sanction): a RAM-backed target refuses before any store
    // exists.
    crate::devhonesty::assert_disk_backed(dir, "the timed crud lanes")
        .map_err(|refusal| refusal.to_string())?;

    // (2) Unknown `--only` names are an error listing the registry (the
    // bench_preflight precedent) — a typo must not silently run nothing.
    let names: Vec<&str> = families().iter().map(|f| f.name).collect();
    if let Some(only) = only {
        for name in only {
            if !names.contains(&name.as_str()) {
                return Err(format!(
                    "unknown family `{name}` (families: {})",
                    names.join(", ")
                ));
            }
        }
    }

    let mut rows = Vec::new();
    for lane in duralane::ALL {
        let (db, conn) = load(&dir.join("crud").join(lane.label()), lane)?;

        // (3) THE ORACLE GATE — unconditional, before any timed window.
        eprintln!("crud [{}]: gate crud_read_point", lane.label());
        let (translated, types) = gate(&db, &conn, lane, seed, sizes)?;

        // (4) The families, in registry order — the registry IS the run
        // order. One fresh-mint cursor per engine pass per lane
        // ([`LaneRun`]): the two passes mint identical id sequences by
        // construction, and a filtered family skips on BOTH sides at
        // once. ONE counter model per lane, threaded through the stream
        // generators in this same order, so every stream's `prev`
        // accounting describes the store the families actually run over
        // — filtered families never touch the model, exactly as they
        // never touch the store.
        let mut lane_run = LaneRun {
            db: &db,
            conn: &conn,
            seed,
            sizes,
            translated: &translated,
            types: &types,
            ours_cursor: FreshCursor::at_base(sizes),
            theirs_cursor: FreshCursor::at_base(sizes),
            model: ops::CounterModel::at_load(sizes),
        };
        for family in families() {
            if let Some(only) = only
                && !only.iter().any(|n| n == family.name)
            {
                continue;
            }
            let proto = Protocol {
                warmups: family.protocol.warmups,
                samples: samples.unwrap_or(family.protocol.samples),
            };
            eprintln!("crud [{}]: {}", lane.label(), family.name);
            let ((ours, theirs), stamp) = lane_run.time_family(family.name, proto)?;
            #[expect(
                clippy::cast_precision_loss,
                reason = "reporting accepts lossy integer-to-float conversion"
            )]
            let ratio_p50 = ours.stats.p50 as f64 / theirs.stats.p50.max(1) as f64;
            rows.push(CrudRow {
                family: family.name,
                lane: lane.label(),
                about: family.about,
                ours: ours.stats,
                theirs: theirs.stats,
                ratio_p50,
                work: ours.work,
                ghz: Some(report::GhzReport {
                    pre: stamp.pre,
                    post: stamp.post,
                    retried: stamp.retried,
                    contaminated: stamp.contaminated(),
                }),
            });
        }

        // (5) THE POST-STATE FOLD — after ALL selected families of the
        // lane, both relations, run-failing on divergence.
        for rel in [ids::DOC, ids::COUNTER] {
            let name = schema().relation(rel).name();
            let ours = poststate::engine_rows(&db, rel)
                .map_err(|e| format!("crud/{name} [{}]: {e}", lane.label()))?;
            let theirs = poststate::sqlite_rows(&conn, schema().relation(rel))
                .map_err(|e| format!("crud/{name} [{}]: {e}", lane.label()))?;
            poststate::assert_identical("crud", name, ours, theirs)?;
        }
    }

    // (6) Render — both artifacts from the same rows.
    Ok((render::markdown(&rows, seed), render::json(&rows, seed)))
}

/// One lane's timing context: both stores, the gate's translation and
/// output signature (reused by the read-point twin), the pair of
/// lane-scoped fresh-mint cursors the insert-bearing families advance
/// in lockstep, and the lane's single evolving counter model every
/// stream generator draws from and writes back to.
struct LaneRun<'l> {
    db: &'l Db<CrudWorld>,
    conn: &'l Connection,
    seed: u64,
    sizes: CrudSizes,
    translated: &'l Translated,
    types: &'l [ValueType],
    ours_cursor: FreshCursor,
    theirs_cursor: FreshCursor,
    model: ops::CounterModel,
}

impl LaneRun<'_> {
    /// One family pair under the clock proxy: the engine runner then
    /// the `SQLite` runner over the ONE shared op stream (derived here
    /// from `(seed, sizes, count)` and the lane's evolving counter
    /// model), stamped as a block (the `driver/write_families.rs`
    /// shape).
    fn time_family(
        &mut self,
        name: &'static str,
        proto: Protocol,
    ) -> Result<((Measurement, Measurement), clockproxy::GhzStamp), String> {
        let count =
            usize::try_from(proto.warmups + proto.samples).expect("protocol counts are small");
        let (db, conn, seed, sizes) = (self.db, self.conn, self.seed, self.sizes);
        match name {
            "crud_read_point" => clockproxy::stamped(|| {
                Ok((
                    read_point_ours(db, proto, seed, sizes)?,
                    read_point_theirs(conn, proto, seed, sizes, self.translated, self.types)?,
                ))
            }),
            "crud_insert" => self.insert_pair(proto, 1),
            "crud_insert_10" => self.insert_pair(proto, 10),
            "crud_insert_100" => self.insert_pair(proto, 100),
            "crud_insert_1k" => self.insert_pair(proto, 1_000),
            "crud_update" => {
                let stream = ops::update_stream(seed, sizes, count, &mut self.model);
                clockproxy::stamped(|| {
                    Ok((
                        lanes::update_bumbledb(db, proto, &stream)?,
                        lanes::update_sqlite(conn, proto, &stream)?,
                    ))
                })
            }
            "crud_update_hot" => {
                let stream = ops::hot_update_stream(count, &mut self.model);
                clockproxy::stamped(|| {
                    Ok((
                        lanes::update_bumbledb(db, proto, &stream)?,
                        lanes::update_sqlite(conn, proto, &stream)?,
                    ))
                })
            }
            "crud_upsert" => {
                let stream = ops::upsert_stream(seed, sizes, count, &mut self.model);
                clockproxy::stamped(|| {
                    Ok((
                        lanes::upsert_bumbledb(db, proto, &stream)?,
                        lanes::upsert_sqlite(conn, proto, &stream)?,
                    ))
                })
            }
            "crud_rmw" => {
                let keys = ops::rmw_stream(seed, sizes, count, &mut self.model);
                clockproxy::stamped(|| {
                    Ok((
                        lanes::rmw_bumbledb(db, proto, &keys)?,
                        lanes::rmw_sqlite(conn, proto, &keys)?,
                    ))
                })
            }
            "crud_delete" => clockproxy::stamped(|| {
                Ok((
                    lanes::delete_bumbledb(db, proto, seed, sizes)?,
                    lanes::delete_sqlite(conn, proto, sizes)?,
                ))
            }),
            "crud_mixed_90_10" => clockproxy::stamped(|| {
                Ok((
                    lanes::mixed_bumbledb(db, proto, seed, sizes, &mut self.ours_cursor)?,
                    lanes::mixed_sqlite(conn, proto, seed, sizes, &mut self.theirs_cursor)?,
                ))
            }),
            other => unreachable!("the registry names are exhaustive: {other}"),
        }
    }

    /// One insert-ladder family pair, both engines' runners advancing
    /// their own lane-scoped mint cursors.
    fn insert_pair(
        &mut self,
        proto: Protocol,
        per_commit: u64,
    ) -> Result<((Measurement, Measurement), clockproxy::GhzStamp), String> {
        let (db, conn, seed) = (self.db, self.conn, self.seed);
        clockproxy::stamped(|| {
            Ok((
                lanes::insert_bumbledb(db, proto, seed, per_commit, &mut self.ours_cursor)?,
                lanes::insert_sqlite(conn, proto, seed, per_commit, &mut self.theirs_cursor)?,
            ))
        })
    }
}

/// The oracle gate for the read query (the `scenarios/run_query.rs`
/// wiring, copied faithfully): prepare on both engines, and for EVERY
/// param set from [`ops::read_keys`] compare the result multisets —
/// a disagreement errs naming the family, set index, and lane, and
/// nothing gets timed. Returns the canonical translation and the
/// output signature the timing half reuses (the gate/time split makes
/// "oracle-gated before ever timed" a call-order fact).
fn gate(
    db: &Db<CrudWorld>,
    conn: &Connection,
    lane: DurabilityLane,
    seed: u64,
    sizes: CrudSizes,
) -> Result<(Translated, Vec<ValueType>), String> {
    let query = read_query();
    let mut prepared = db
        .prepare(&query)
        .map_err(|e| format!("crud/crud_read_point [{}]: prepare: {e:?}", lane.label()))?;
    let types: Vec<ValueType> = prepared
        .predicate()
        .columns
        .iter()
        .map(|column| column.ty.clone())
        .collect();
    let translated = translate::translate(&query, schema(), &[])
        .map_err(|e| format!("crud/crud_read_point [{}]: {e}", lane.label()))?;
    for (i, params) in ops::read_keys(seed, sizes).iter().enumerate() {
        let mut buffer = Answers::new();
        db.read(|snap| snap.execute(&mut prepared, &bind_values(params), &mut buffer))
            .map_err(|e| format!("crud/crud_read_point [{}]: execute: {e:?}", lane.label()))?;
        let ours = compare::from_answers(&buffer, &types);
        let args: Vec<crate::naive::ParamValue> = params
            .iter()
            .map(|value| crate::naive::ParamValue::Scalar(value.clone()))
            .collect();
        let mut stmt = conn.prepare_cached(&translated.sql).map_err(|e| {
            format!(
                "crud/crud_read_point [{}]: oracle prepare: {e}",
                lane.label()
            )
        })?;
        let theirs =
            compare::from_sqlite(&mut stmt, &translated.params, &args, &types).map_err(|e| {
                format!(
                    "crud/crud_read_point [{}]: oracle execute: {e}",
                    lane.label()
                )
            })?;
        compare::multisets(ours, theirs).map_err(|mismatch| {
            format!(
                "crud/crud_read_point set {i} [{}]: ENGINES DISAGREE — not timing a wrong answer\n{mismatch}",
                lane.label()
            )
        })?;
    }
    Ok((translated, types))
}

/// `crud_read_point`, engine side: the prepared read query under the
/// gate-style rotation over [`ops::read_keys`] (3 hits + 1 miss) —
/// the `run_query.rs` timing shape.
fn read_point_ours(
    db: &Db<CrudWorld>,
    proto: Protocol,
    seed: u64,
    sizes: CrudSizes,
) -> Result<Measurement, String> {
    let query = read_query();
    let mut prepared = db
        .prepare(&query)
        .map_err(|e| format!("crud_read_point: prepare: {e:?}"))?;
    let mut rotation = Rotation::new(ops::read_keys(seed, sizes));
    let mut buffer = Answers::new();
    harness::measure(proto, || {
        let params = bind_values(rotation.next_set());
        db.read(|snap| snap.execute(&mut prepared, &params, &mut buffer))
            .map_err(|e| format!("crud_read_point: execute: {e:?}"))?;
        Ok(buffer.len() as u64)
    })
}

/// `crud_read_point`, `SQLite` side: the gate's canonical translation on
/// one reused prepared statement over the IDENTICAL rotation.
fn read_point_theirs(
    conn: &Connection,
    proto: Protocol,
    seed: u64,
    sizes: CrudSizes,
    translated: &Translated,
    types: &[ValueType],
) -> Result<Measurement, String> {
    let mut family = PreparedFamily::new(conn, translated, types.to_vec())?;
    let mut rotation = Rotation::new(ops::read_keys(seed, sizes));
    harness::measure(proto, || {
        sqlite_run::sample(&mut family, rotation.next_set())
    })
}
