use std::path::{Path, PathBuf};

use bumbledb::schema::ValueType;
use bumbledb::{Answers, Db};
use rusqlite::Connection;

use crate::corpus_gen::Scale;
use crate::duralane::{self, DurabilityLane};
use crate::families::bind_values;
use crate::harness::{self, Measurement, Protocol, Rotation};
use crate::sqlite_run::{self, PreparedFamily};
use crate::translate::{self, Translated};
use crate::{clockproxy, compare, poststate, report, trace_out};

use super::lanes::{self, MintCursor, read_query};
use super::{CrudSizes, CrudWorld, corpus, families, ids, ops, render, schema};

#[derive(Debug, Clone)]
pub struct CrudRow {
    pub family: &'static str,

    pub lane: &'static str,

    pub about: &'static str,

    pub ours: harness::Stats,

    pub theirs: harness::Stats,

    pub ratio_p50: f64,

    pub work: u64,

    pub ghz: Option<report::GhzReport>,

    pub flame: Option<String>,
}

/// [`run_with`] binds it to [`corpus::load_stores`]; the gate tests bind a
/// loader that poisons the mirror after loading — the fold itself (gate,
/// timing, post-state) is identical either way, so the tests exercise the exact
/// stages the real run takes.
pub(crate) type LaneLoader<'a> =
    dyn Fn(&Path, DurabilityLane) -> Result<(Db<CrudWorld>, Connection), String> + 'a;

/// # Errors
pub fn run(
    dir: &Path,
    seed: u64,
    samples: Option<u32>,
    only: Option<&[String]>,
    trace_root: Option<&Path>,
) -> Result<(String, String), String> {
    run_with(
        dir,
        seed,
        CrudSizes::of(Scale::S),
        samples,
        only,
        trace_root,
    )
}

/// # Errors
pub fn run_with(
    dir: &Path,
    seed: u64,
    sizes: CrudSizes,
    samples: Option<u32>,
    only: Option<&[String]>,
    trace_root: Option<&Path>,
) -> Result<(String, String), String> {
    fold(
        dir,
        seed,
        sizes,
        samples,
        only,
        trace_root,
        &|lane_dir, lane| corpus::load_stores(lane_dir, seed, sizes, lane),
    )
}

pub(crate) fn fold(
    dir: &Path,
    seed: u64,
    sizes: CrudSizes,
    samples: Option<u32>,
    only: Option<&[String]>,
    trace_root: Option<&Path>,
    load: &LaneLoader<'_>,
) -> Result<(String, String), String> {
    // ramdisk sanction): a RAM-backed target refuses before any store

    crate::devhonesty::assert_disk_backed(dir, "the timed crud lanes")
        .map_err(|refusal| refusal.to_string())?;

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

        let mut lane_run = LaneRun {
            db: &db,
            conn: &conn,
            seed,
            sizes,
            translated: &translated,
            types: &types,
            ours_cursor: MintCursor::at_base(sizes),
            theirs_cursor: MintCursor::at_base(sizes),
            model: ops::CounterModel::at_load(sizes),
            trace_dir: trace_root.map(|root| root.join("trace").join("crud").join(lane.label())),
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
            let (ours, theirs, stamp, flame) = lane_run.time_family(family.name, proto)?;
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
                ghz: Some(stamp.into()),
                flame,
            });
        }

        // (5) THE POST-STATE FOLD — after ALL selected families of the

        for rel in [ids::DOC, ids::COUNTER] {
            let name = schema().relation(rel).name();
            let ours = poststate::engine_rows(&db, rel)
                .map_err(|e| format!("crud/{name} [{}]: {e}", lane.label()))?;
            let theirs = poststate::sqlite_rows(&conn, schema().relation(rel))
                .map_err(|e| format!("crud/{name} [{}]: {e}", lane.label()))?;
            poststate::assert_identical("crud", name, ours, theirs)?;
        }
    }

    Ok((render::markdown(&rows, seed), render::json(&rows, seed)))
}

struct LaneRun<'l> {
    db: &'l Db<CrudWorld>,
    conn: &'l Connection,
    seed: u64,
    sizes: CrudSizes,
    translated: &'l Translated,
    types: &'l [ValueType],
    ours_cursor: MintCursor,
    theirs_cursor: MintCursor,
    model: ops::CounterModel,

    trace_dir: Option<PathBuf>,
}

type FamilyOutcome = (
    Measurement,
    Measurement,
    clockproxy::GhzStamp,
    Option<String>,
);

impl LaneRun<'_> {
    #[expect(
        clippy::too_many_lines,
        reason = "one match arm per registered family: the registry IS the run order"
    )]
    fn time_family(
        &mut self,
        name: &'static str,
        proto: Protocol,
    ) -> Result<FamilyOutcome, String> {
        let count =
            usize::try_from(proto.warmups + proto.samples).expect("protocol counts are small");
        let extra = usize::from(self.trace_dir.is_some());
        let dir = self.trace_dir.clone();
        let dir = dir.as_deref();
        let (db, conn, seed, sizes) = (self.db, self.conn, self.seed, self.sizes);
        match name {
            "crud_read_point" => {
                let ((ours, theirs), stamp) = clockproxy::stamped(|| {
                    Ok((
                        read_point_ours(db, proto, seed, sizes)?,
                        read_point_theirs(conn, proto, seed, sizes, self.translated, self.types)?,
                    ))
                })?;
                let (translated, types) = (self.translated, self.types);
                let flame = trace_out::traced_twin(
                    dir,
                    name,
                    &mut |p| read_point_ours(db, p, seed, sizes),
                    &mut |p| read_point_theirs(conn, p, seed, sizes, translated, types),
                )?;
                Ok((ours, theirs, stamp, flame))
            }
            "crud_insert" => self.insert_pair(name, proto, 1),
            "crud_insert_10" => self.insert_pair(name, proto, 10),
            "crud_insert_100" => self.insert_pair(name, proto, 100),
            "crud_insert_1k" => self.insert_pair(name, proto, 1_000),
            "crud_update" | "crud_update_hot" => {
                let stream = if name == "crud_update" {
                    ops::update_stream(seed, sizes, count + extra, &mut self.model)
                } else {
                    ops::hot_update_stream(count + extra, &mut self.model)
                };
                let (timed, spare) = stream.split_at(count);
                let ((ours, theirs), stamp) = clockproxy::stamped(|| {
                    Ok((
                        lanes::update_bumbledb(db, proto, timed)?,
                        lanes::update_sqlite(conn, proto, timed)?,
                    ))
                })?;
                let flame = trace_out::traced_twin(
                    dir,
                    name,
                    &mut |p| lanes::update_bumbledb(db, p, spare),
                    &mut |p| lanes::update_sqlite(conn, p, spare),
                )?;
                Ok((ours, theirs, stamp, flame))
            }
            "crud_upsert" => {
                let stream = ops::upsert_stream(seed, sizes, count + extra, &mut self.model);
                let (timed, spare) = stream.split_at(count);
                let ((ours, theirs), stamp) = clockproxy::stamped(|| {
                    Ok((
                        lanes::upsert_bumbledb(db, proto, timed)?,
                        lanes::upsert_sqlite(conn, proto, timed)?,
                    ))
                })?;
                let flame = trace_out::traced_twin(
                    dir,
                    name,
                    &mut |p| lanes::upsert_bumbledb(db, p, spare),
                    &mut |p| lanes::upsert_sqlite(conn, p, spare),
                )?;
                Ok((ours, theirs, stamp, flame))
            }
            "crud_rmw" => {
                let keys = ops::rmw_stream(seed, sizes, count + extra, &mut self.model);
                let (timed, spare) = keys.split_at(count);
                let ((ours, theirs), stamp) = clockproxy::stamped(|| {
                    Ok((
                        lanes::rmw_bumbledb(db, proto, timed)?,
                        lanes::rmw_sqlite(conn, proto, timed)?,
                    ))
                })?;
                let flame = trace_out::traced_twin(
                    dir,
                    name,
                    &mut |p| lanes::rmw_bumbledb(db, p, spare),
                    &mut |p| lanes::rmw_sqlite(conn, p, spare),
                )?;
                Ok((ours, theirs, stamp, flame))
            }
            "crud_delete" => {
                let rows = ops::delete_rows(seed, sizes, count + extra);
                let ids: Vec<u64> = (0..count + extra)
                    .map(|i| sizes.docs + u64::try_from(i).expect("protocol counts are small"))
                    .collect();
                let (timed_rows, spare_rows) = rows.split_at(count);
                let (timed_ids, spare_ids) = ids.split_at(count);
                let ((ours, theirs), stamp) = clockproxy::stamped(|| {
                    Ok((
                        lanes::delete_bumbledb(db, proto, timed_rows)?,
                        lanes::delete_sqlite(conn, proto, timed_ids)?,
                    ))
                })?;
                let flame = trace_out::traced_twin(
                    dir,
                    name,
                    &mut |p| lanes::delete_bumbledb(db, p, spare_rows),
                    &mut |p| lanes::delete_sqlite(conn, p, spare_ids),
                )?;
                Ok((ours, theirs, stamp, flame))
            }
            "crud_mixed_90_10" => {
                let ((ours, theirs), stamp) = clockproxy::stamped(|| {
                    Ok((
                        lanes::mixed_bumbledb(db, proto, seed, sizes, &mut self.ours_cursor)?,
                        lanes::mixed_sqlite(conn, proto, seed, sizes, &mut self.theirs_cursor)?,
                    ))
                })?;
                let (ours_cursor, theirs_cursor) = (&mut self.ours_cursor, &mut self.theirs_cursor);
                let flame = trace_out::traced_twin(
                    dir,
                    name,
                    &mut |p| lanes::mixed_bumbledb(db, p, seed, sizes, ours_cursor),
                    &mut |p| lanes::mixed_sqlite(conn, p, seed, sizes, theirs_cursor),
                )?;
                Ok((ours, theirs, stamp, flame))
            }
            other => unreachable!("the registry names are exhaustive: {other}"),
        }
    }

    fn insert_pair(
        &mut self,
        name: &'static str,
        proto: Protocol,
        per_commit: u64,
    ) -> Result<FamilyOutcome, String> {
        let (db, conn, seed) = (self.db, self.conn, self.seed);
        let ((ours, theirs), stamp) = clockproxy::stamped(|| {
            Ok((
                lanes::insert_bumbledb(db, proto, seed, per_commit, &mut self.ours_cursor)?,
                lanes::insert_sqlite(conn, proto, seed, per_commit, &mut self.theirs_cursor)?,
            ))
        })?;
        let dir = self.trace_dir.clone();
        let (ours_cursor, theirs_cursor) = (&mut self.ours_cursor, &mut self.theirs_cursor);
        let flame = trace_out::traced_twin(
            dir.as_deref(),
            name,
            &mut |p| lanes::insert_bumbledb(db, p, seed, per_commit, ours_cursor),
            &mut |p| lanes::insert_sqlite(conn, p, seed, per_commit, theirs_cursor),
        )?;
        Ok((ours, theirs, stamp, flame))
    }
}

/// Returns the canonical translation and the output signature the timing half
/// reuses (the gate/time split makes "oracle-gated before ever timed" a
/// call-order fact).
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
        .signature()
        .columns
        .iter()
        .map(|column| *column.ty())
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
