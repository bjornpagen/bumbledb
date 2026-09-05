use bumbledb::FieldId;

use crate::compare::Owned;
use crate::corpus_gen::Scale;
use crate::duralane::{self, DurabilityLane};
use crate::harness::Protocol;
use crate::poststate;

use super::lanes::{self, MintCursor};
use super::{CrudSizes, ids, ops};

fn scratch(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("bumbledb-crud-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

#[test]
fn the_crud_schema_validates_and_names_its_ids() {
    let schema = super::schema();
    assert_eq!(schema.relations().len(), 2, "two crud relations");
    assert_eq!(schema.relation(ids::DOC).name(), "Doc");
    assert_eq!(schema.relation(ids::COUNTER).name(), "Counter");
    let keys = schema.keys();
    assert!(
        keys.iter()
            .any(|key| key.relation == ids::DOC && *key.projection == [FieldId(1)]),
        "Doc(key) -> Doc is sealed"
    );
    assert!(
        keys.iter()
            .any(|key| key.relation == ids::COUNTER && *key.projection == [FieldId(0)]),
        "Counter(key) -> Counter is sealed"
    );
}

#[test]
fn the_twin_stores_load_value_identical_at_tiny() {
    let sizes = CrudSizes::of(Scale::Tiny);
    for lane in duralane::ALL {
        let dir = scratch(&format!("twin-{}", lane.label()));
        let (db, conn) = super::corpus::load_stores(&dir, 7, sizes, lane).unwrap_or_else(|e| {
            panic!("{}: {e}", lane.label());
        });
        for (rel, expected) in [
            (ids::DOC, sizes.docs + sizes.delete_pool),
            (ids::COUNTER, sizes.counters),
        ] {
            let name = super::schema().relation(rel).name();
            let ours = poststate::engine_rows(&db, rel).expect("engine rows");
            let theirs =
                poststate::sqlite_rows(&conn, super::schema().relation(rel)).expect("mirror rows");
            assert_eq!(ours.len() as u64, expected, "{name}: engine row count");
            assert_eq!(theirs.len() as u64, expected, "{name}: mirror row count");
            poststate::assert_identical("crud", name, ours, theirs).expect(name);
        }
        drop((db, conn));
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[test]
fn the_lane_parity_assertion_catches_a_mismatched_synchronous() {
    let dir = scratch("parity-mismatch");
    let conn = rusqlite::Connection::open(dir.join("durable.sqlite")).expect("open");
    DurabilityLane::Durable.configure(&conn).expect("configure");
    DurabilityLane::Durable
        .assert_parity(&conn)
        .expect("a configured durable mirror passes its own readback");
    // Weaken the connection by hand (the retired nosync lane's pragma set):
    // the durable parity readback must convict it before any timing.
    conn.pragma_update(None, "synchronous", "OFF")
        .expect("pragma");
    let err = DurabilityLane::Durable
        .assert_parity(&conn)
        .expect_err("a weakened mirror is not a durable twin");
    assert!(err.contains("synchronous"), "{err}");
    drop(conn);
    let _ = std::fs::remove_dir_all(&dir);
}

const SEED: u64 = 1;

/// The tiny per-family protocol: 1 warmup + 2 measured samples = 3 closure
/// invocations (the delete pool at Tiny, 256, covers it).
const TINY_PROTO: Protocol = Protocol {
    warmups: 1,
    samples: 2,
};

const COUNT: usize = 3;

fn assert_twins_identical(db: &bumbledb::Db<super::CrudWorld>, conn: &rusqlite::Connection) {
    for rel in [ids::DOC, ids::COUNTER] {
        let name = super::schema().relation(rel).name();
        let ours = poststate::engine_rows(db, rel).expect("engine rows");
        let theirs =
            poststate::sqlite_rows(conn, super::schema().relation(rel)).expect("mirror rows");
        poststate::assert_identical("crud", name, ours, theirs).expect(name);
    }
}

fn expected_mixed_work() -> u64 {
    let mut work = 0u64;
    let mut cursor = 0usize;
    for invocation in 0..COUNT {
        for _ in 0..9 {
            let hit = cursor % 4 != 3;
            if invocation >= 1 && hit {
                work += 1;
            }
            cursor += 1;
        }
        if invocation >= 1 {
            work += 1;
        }
    }
    work
}

#[test]
fn every_crud_write_family_leaves_the_twins_value_identical() {
    let sizes = CrudSizes::of(Scale::Tiny);
    let dir = scratch("families-durable");
    let (db, conn) =
        super::corpus::load_stores(&dir, SEED, sizes, DurabilityLane::Durable).expect("load");

    let mut ours_cursor = MintCursor::at_base(sizes);
    let mut theirs_cursor = MintCursor::at_base(sizes);

    for per_commit in [1u64, 10, 100, 1_000] {
        let ours = lanes::insert_bumbledb(&db, TINY_PROTO, SEED, per_commit, &mut ours_cursor)
            .expect("insert engine");
        let theirs = lanes::insert_sqlite(&conn, TINY_PROTO, SEED, per_commit, &mut theirs_cursor)
            .expect("insert sqlite");
        assert_eq!(
            ours.work,
            per_commit * 2,
            "insert x{per_commit}: engine work"
        );
        assert_eq!(
            theirs.work,
            per_commit * 2,
            "insert x{per_commit}: mirror work"
        );
    }
    assert_eq!(ours_cursor, theirs_cursor, "the cursors stay in lockstep");

    let mut model = ops::CounterModel::at_load(sizes);

    let stream = ops::update_stream(SEED, sizes, COUNT, &mut model);
    let ours = lanes::update_bumbledb(&db, TINY_PROTO, &stream).expect("update engine");
    let theirs = lanes::update_sqlite(&conn, TINY_PROTO, &stream).expect("update sqlite");
    assert_eq!(ours.work, 2, "update: engine work");
    assert_eq!(theirs.work, 2, "update: mirror work");

    let stream = ops::hot_update_stream(COUNT, &mut model);
    let ours = lanes::update_bumbledb(&db, TINY_PROTO, &stream).expect("hot engine");
    let theirs = lanes::update_sqlite(&conn, TINY_PROTO, &stream).expect("hot sqlite");
    assert_eq!(ours.work, 2, "hot: engine work");
    assert_eq!(theirs.work, 2, "hot: mirror work");

    let stream = ops::upsert_stream(SEED, sizes, COUNT, &mut model);
    let ours = lanes::upsert_bumbledb(&db, TINY_PROTO, &stream).expect("upsert engine");
    let theirs = lanes::upsert_sqlite(&conn, TINY_PROTO, &stream).expect("upsert sqlite");
    assert_eq!(ours.work, 2, "upsert: engine work");
    assert_eq!(theirs.work, 2, "upsert: mirror work");

    let keys = ops::rmw_stream(SEED, sizes, COUNT, &mut model);
    let ours = lanes::rmw_bumbledb(&db, TINY_PROTO, &keys).expect("rmw engine");
    let theirs = lanes::rmw_sqlite(&conn, TINY_PROTO, &keys).expect("rmw sqlite");
    assert_eq!(ours.work, 2, "rmw: engine work");
    assert_eq!(theirs.work, 2, "rmw: mirror work");

    let rows = ops::delete_rows(SEED, sizes, COUNT);
    let ids: Vec<u64> = (0..COUNT as u64).map(|i| sizes.docs + i).collect();
    let ours = lanes::delete_bumbledb(&db, TINY_PROTO, &rows).expect("delete engine");
    let theirs = lanes::delete_sqlite(&conn, TINY_PROTO, &ids).expect("delete sqlite");
    assert_eq!(ours.work, 2, "delete: engine work");
    assert_eq!(theirs.work, 2, "delete: mirror work");

    let ours = lanes::mixed_bumbledb(&db, TINY_PROTO, SEED, sizes, &mut ours_cursor)
        .expect("mixed engine");
    let theirs = lanes::mixed_sqlite(&conn, TINY_PROTO, SEED, sizes, &mut theirs_cursor)
        .expect("mixed sqlite");
    assert_eq!(ours.work, expected_mixed_work(), "mixed: engine work");
    assert_eq!(theirs.work, expected_mixed_work(), "mixed: mirror work");
    assert_eq!(ours_cursor, theirs_cursor, "the cursors end in lockstep");

    assert_twins_identical(&db, &conn);
    drop((db, conn));
    let _ = std::fs::remove_dir_all(&dir);
}

/// The family battery on a second independently-loaded durable twin pair:
/// the lockstep-cursor protocol is store-instance independent (the old
/// second durability lane is gone — ENG-008; this keeps the second-instance
/// coverage the nosync twin used to provide).
#[test]
fn a_second_twin_pair_runs_the_same_families_identically() {
    let sizes = CrudSizes::of(Scale::Tiny);
    let dir = scratch("families-second");
    let (db, conn) =
        super::corpus::load_stores(&dir, SEED, sizes, DurabilityLane::Durable).expect("load");
    let mut ours_cursor = MintCursor::at_base(sizes);
    let mut theirs_cursor = MintCursor::at_base(sizes);

    lanes::insert_bumbledb(&db, TINY_PROTO, SEED, 1, &mut ours_cursor).expect("insert engine");
    lanes::insert_sqlite(&conn, TINY_PROTO, SEED, 1, &mut theirs_cursor).expect("insert sqlite");

    let mut model = ops::CounterModel::at_load(sizes);
    let stream = ops::upsert_stream(SEED, sizes, COUNT, &mut model);
    lanes::upsert_bumbledb(&db, TINY_PROTO, &stream).expect("upsert engine");
    lanes::upsert_sqlite(&conn, TINY_PROTO, &stream).expect("upsert sqlite");

    let keys = ops::rmw_stream(SEED, sizes, COUNT, &mut model);
    lanes::rmw_bumbledb(&db, TINY_PROTO, &keys).expect("rmw engine");
    lanes::rmw_sqlite(&conn, TINY_PROTO, &keys).expect("rmw sqlite");

    let rows = ops::delete_rows(SEED, sizes, COUNT);
    let ids: Vec<u64> = (0..COUNT as u64).map(|i| sizes.docs + i).collect();
    lanes::delete_bumbledb(&db, TINY_PROTO, &rows).expect("delete engine");
    lanes::delete_sqlite(&conn, TINY_PROTO, &ids).expect("delete sqlite");

    assert_twins_identical(&db, &conn);
    drop((db, conn));
    let _ = std::fs::remove_dir_all(&dir);
}

/// The delete lane's refusal contract, falsified from both sides: deleting the
/// same pool row twice makes the second engine call `Err` (the in-closure
/// sentinel — the lane never degrades to a no-op measurement), and the refusal
/// commits NOTHING: the store generation does not move across it.
#[test]
fn the_delete_lane_refuses_a_missing_row() {
    let sizes = CrudSizes::of(Scale::Tiny);
    let dir = scratch("delete-refusal");
    let (db, conn) =
        super::corpus::load_stores(&dir, SEED, sizes, DurabilityLane::Durable).expect("load");
    let one = Protocol {
        warmups: 0,
        samples: 1,
    };
    let rows = ops::delete_rows(SEED, sizes, 1);
    lanes::delete_bumbledb(&db, one, &rows).expect("the first delete bears");
    let generation = db.generation().expect("generation");
    let err = lanes::delete_bumbledb(&db, one, &rows)
        .expect_err("the second delete of the same pool row must refuse");
    assert!(
        err.contains("Io("),
        "a refused delete is the Io sentinel (the message is not on the wire): {err}"
    );
    assert_eq!(
        db.generation().expect("generation"),
        generation,
        "a refused delete must leave the store untouched"
    );
    drop((db, conn));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_upsert_follows_its_stream_through_hits_and_misses() {
    let sizes = CrudSizes::of(Scale::Tiny);
    let proto = Protocol {
        warmups: 2,
        samples: 6,
    };
    let stream = ops::upsert_stream(SEED, sizes, 8, &mut ops::CounterModel::at_load(sizes));
    assert!(
        stream.iter().any(|op| op.prev.is_some()),
        "the stream must carry at least one hit: {stream:?}"
    );
    assert!(
        stream.iter().any(|op| op.prev.is_none()),
        "the stream must carry at least one miss: {stream:?}"
    );
    let dir = scratch("upsert-stream");
    let (db, conn) =
        super::corpus::load_stores(&dir, SEED, sizes, DurabilityLane::Durable).expect("load");
    lanes::upsert_bumbledb(&db, proto, &stream).expect("upsert engine");
    lanes::upsert_sqlite(&conn, proto, &stream).expect("upsert sqlite");
    assert_twins_identical(&db, &conn);
    drop((db, conn));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_read_query_translates_and_the_stream_generators_are_pure() {
    let translated = crate::translate::translate(&lanes::read_query(), super::schema(), &[])
        .expect("the read query translates");
    assert!(translated.sql.contains("SELECT"), "{}", translated.sql);
    let sizes = CrudSizes::of(Scale::Tiny);
    let mut a = ops::CounterModel::at_load(sizes);
    let mut b = ops::CounterModel::at_load(sizes);
    assert_eq!(
        ops::update_stream(SEED, sizes, 16, &mut a),
        ops::update_stream(SEED, sizes, 16, &mut b)
    );
    assert_eq!(
        ops::hot_update_stream(16, &mut a),
        ops::hot_update_stream(16, &mut b)
    );
    assert_eq!(
        ops::upsert_stream(SEED, sizes, 16, &mut a),
        ops::upsert_stream(SEED, sizes, 16, &mut b)
    );
    assert_eq!(
        ops::rmw_stream(SEED, sizes, 16, &mut a),
        ops::rmw_stream(SEED, sizes, 16, &mut b)
    );
    assert_eq!(a, b, "the models fold identically");
}

/// THE night-run regression, pinned (tiny corpus, no timing): seed 1's
/// update/hot streams touch keys the upsert stream also draws — asserted below,
/// so the collision is genuinely exercised — and before the shared
/// [`ops::CounterModel`] the upsert lane aborted with "the upsert drifted from
/// its stream: the stored value is not the stream's prev".
#[test]
fn a_colliding_seed_is_absorbed_by_the_counter_model() {
    let sizes = CrudSizes::of(Scale::Tiny);
    let proto = Protocol {
        warmups: 2,
        samples: 8,
    };
    let count = 10usize;
    let mut model = ops::CounterModel::at_load(sizes);
    let update = ops::update_stream(SEED, sizes, count, &mut model);
    let hot = ops::hot_update_stream(count, &mut model);
    let upsert = ops::upsert_stream(SEED, sizes, count, &mut model);
    let touched: std::collections::HashSet<u64> =
        update.iter().chain(hot.iter()).map(|op| op.key).collect();
    let collided: Vec<_> = upsert
        .iter()
        .filter(|op| touched.contains(&op.key))
        .collect();
    assert!(
        !collided.is_empty(),
        "seed {SEED} must exercise the collision this test pins"
    );
    assert!(
        collided.iter().any(|op| op.prev != Some(0)),
        "a collided key's prev must carry the earlier family's write, not the loaded 0: {collided:?}"
    );
    let dir = scratch("colliding-seed");
    let (db, conn) =
        super::corpus::load_stores(&dir, SEED, sizes, DurabilityLane::Durable).expect("load");
    lanes::update_bumbledb(&db, proto, &update).expect("update engine");
    lanes::update_sqlite(&conn, proto, &update).expect("update sqlite");
    lanes::update_bumbledb(&db, proto, &hot).expect("hot engine");
    lanes::update_sqlite(&conn, proto, &hot).expect("hot sqlite");
    lanes::upsert_bumbledb(&db, proto, &upsert).expect("upsert engine");
    lanes::upsert_sqlite(&conn, proto, &upsert).expect("upsert sqlite");
    assert_twins_identical(&db, &conn);
    drop((db, conn));
    let _ = std::fs::remove_dir_all(&dir);
}

const RUN_SEED: u64 = SEED;

fn load_poisoned(
    dir: &std::path::Path,
    lane: DurabilityLane,
    sizes: CrudSizes,
) -> Result<(bumbledb::Db<super::CrudWorld>, rusqlite::Connection), String> {
    let (db, conn) = super::corpus::load_stores(dir, RUN_SEED, sizes, lane)?;
    conn.execute(
        "INSERT INTO \"Doc\" VALUES (?1, ?2, ?3, ?4)",
        (
            999_999_999_i64,
            i64::try_from(u64::MAX / 2).expect("fits"),
            1_i64,
            vec![0u8; 32],
        ),
    )
    .map_err(|e| format!("poison: {e}"))?;
    Ok((db, conn))
}

#[test]
fn the_crud_gate_refuses_a_divergent_oracle() {
    let sizes = CrudSizes::of(Scale::Tiny);
    let dir = scratch("run-gate-divergent");
    let err = super::run::fold(
        &dir,
        RUN_SEED,
        sizes,
        Some(2),
        None,
        None,
        &|lane_dir, lane| load_poisoned(lane_dir, lane, sizes),
    )
    .expect_err("a poisoned mirror must not be timed");
    assert!(err.contains("ENGINES DISAGREE"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_full_crud_run_produces_both_lanes_and_parses() {
    let sizes = CrudSizes::of(Scale::Tiny);
    let dir = scratch("run-full");
    let (md, json_text) =
        super::run_with(&dir, RUN_SEED, sizes, Some(2), None, None).expect("the full crud run");
    assert!(md.contains("## lane durable"), "{md}");
    for family in super::families() {
        assert!(md.contains(family.name), "missing {} in\n{md}", family.name);
    }
    let parsed = crate::json::parse(&json_text).expect("the artifact parses");
    let lanes = parsed
        .get("lanes")
        .and_then(crate::json::Value::as_arr)
        .expect("lanes array");
    assert_eq!(lanes.len(), 1, "one durability lane (ENG-008)");
    for lane in lanes {
        let rows = lane
            .get("rows")
            .and_then(crate::json::Value::as_arr)
            .expect("rows array");
        assert_eq!(rows.len(), super::families().len(), "eleven rows per lane");
        assert!(
            lane.get("config")
                .and_then(crate::json::Value::as_str)
                .is_some_and(|config| config.contains("SQLite WAL")),
            "the lane carries its parity config prose"
        );
    }
    assert_eq!(
        parsed.get("poststate").and_then(crate::json::Value::as_str),
        Some("ok"),
        "the post-state field"
    );
    assert!(
        parsed
            .get("provenance")
            .is_some_and(|p| p.get("host").is_some()),
        "the provenance stamp rides the artifact (the one shared emitter)"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// An unknown `--only` name is refused before anything loads, and the refusal
/// lists the registry.
#[test]
fn an_unknown_only_name_is_refused() {
    let sizes = CrudSizes::of(Scale::Tiny);
    let dir = scratch("run-unknown-only");
    let err = super::run_with(
        &dir,
        RUN_SEED,
        sizes,
        Some(2),
        Some(&["nope".to_owned()]),
        None,
    )
    .expect_err("an unknown family name must refuse");
    assert!(err.contains("unknown family `nope`"), "{err}");
    assert!(err.contains("crud_read_point"), "{err}");
    assert!(err.contains("crud_mixed_90_10"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_filtered_run_still_gates_the_read_query() {
    let sizes = CrudSizes::of(Scale::Tiny);
    let dir = scratch("run-filtered-gate");
    let only = vec!["crud_insert".to_owned()];
    let err = super::run::fold(
        &dir,
        RUN_SEED,
        sizes,
        Some(2),
        Some(&only),
        None,
        &|lane_dir, lane| load_poisoned(lane_dir, lane, sizes),
    )
    .expect_err("the gate must run even when read_point is filtered out");
    assert!(err.contains("ENGINES DISAGREE"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(feature = "obs")]
#[test]
fn traced_crud_lands_the_pair_with_judgment_and_commit_spans() {
    let sizes = CrudSizes::of(Scale::Tiny);
    let dir = scratch("run-traced");
    let only = vec!["crud_insert".to_owned()];
    let (md, json_text) = super::run_with(&dir, RUN_SEED, sizes, Some(1), Some(&only), Some(&dir))
        .expect("the traced crud run (post-state fold included)");
    assert!(md.contains("Flame summaries"), "{md}");
    assert!(json_text.contains("\"flame\":"), "{json_text}");
    for lane in duralane::ALL {
        let lane_dir = dir.join("trace").join("crud").join(lane.label());
        let json_path = lane_dir.join("crud_insert.json");
        let text = std::fs::read_to_string(&json_path)
            .unwrap_or_else(|e| panic!("{}: {e}", json_path.display()));
        assert!(
            text.starts_with("[\n") && text.ends_with("\n]\n"),
            "{} parses as a Chrome array",
            json_path.display()
        );
        assert!(
            text.contains(bumbledb::obs::names::LMDB_COMMIT.label()),
            "{}: the LMDB commit span reaches the artifact",
            json_path.display()
        );
        assert!(
            text.contains("judgment"),
            "{}: the judgment spans reach the artifact",
            json_path.display()
        );
        let folded = std::fs::read_to_string(lane_dir.join("crud_insert.folded"))
            .expect("the folded twin lands beside the json");
        assert!(!folded.is_empty(), "a non-degenerate fold");
        for line in folded.lines() {
            let count = line.rsplit(' ').next().expect("a self-ns tail");
            assert!(count.parse::<u64>().is_ok(), "folded self-ns: {line}");
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// A one-row post-state divergence is loud: the error names the world and the
/// relation before rendering the multiset diff.
#[test]
fn poststate_divergence_is_loud() {
    let ours = vec![
        vec![Owned::U64(1), Owned::I64(10)],
        vec![Owned::U64(2), Owned::I64(20)],
    ];
    let theirs = vec![
        vec![Owned::U64(1), Owned::I64(10)],
        vec![Owned::U64(2), Owned::I64(21)],
    ];
    let err = poststate::assert_identical("crud", "Doc", ours, theirs)
        .expect_err("the post-states diverge");
    assert!(err.contains("crud/Doc"), "{err}");
    assert!(err.contains("POST-STATES DIVERGE"), "{err}");
}
