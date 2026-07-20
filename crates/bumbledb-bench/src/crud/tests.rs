use bumbledb::FieldId;

use crate::compare::Owned;
use crate::corpus_gen::Scale;
use crate::duralane::{self, DurabilityLane};
use crate::harness::Protocol;
use crate::poststate;

use super::lanes::{self, FreshCursor};
use super::{CrudSizes, ids, ops};

fn scratch(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("bumbledb-crud-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// The declared world seals: two relations whose id constants match
/// descriptor order, and both scalar key statements — `Doc(key)` and
/// `Counter(key)` — present in the sealed statement roster (the upsert
/// lane's `ON CONFLICT` targets on the mirror).
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

/// Both durability lanes load value-identical twins at `Tiny`, judged
/// by the shared post-state comparator — the exact fold every write
/// lane will reuse.
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

/// A cross-matched twin is caught by the parity readback, naming the
/// pragma: a Durable-configured mirror judged as `Nosync` errs on
/// `synchronous`, and vice versa.
#[test]
fn the_lane_parity_assertion_catches_a_mismatched_synchronous() {
    let dir = scratch("parity-mismatch");
    let conn = rusqlite::Connection::open(dir.join("durable.sqlite")).expect("open");
    DurabilityLane::Durable.configure(&conn).expect("configure");
    let err = DurabilityLane::Nosync
        .assert_parity(&conn)
        .expect_err("a durable mirror is not a nosync twin");
    assert!(err.contains("synchronous"), "{err}");
    drop(conn);

    let conn = rusqlite::Connection::open(dir.join("nosync.sqlite")).expect("open");
    DurabilityLane::Nosync.configure(&conn).expect("configure");
    let err = DurabilityLane::Durable
        .assert_parity(&conn)
        .expect_err("a nosync mirror is not a durable twin");
    assert!(err.contains("synchronous"), "{err}");
    drop(conn);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The test fixture seed. The three seeded `Counter` streams (update,
/// upsert; hot is fixed at key 0) must draw keys that keep every
/// stream's `prev` accounting true when the families run back to back
/// on ONE twin pair — any collision aborts LOUDLY inside a write
/// closure (the refusal contract under test), so a colliding seed
/// fails deterministically here, never silently. (Seed 7, for the
/// record, collides: its update stream touches a key the upsert
/// stream's seeded-0 accounting still expects — and the abort said so.)
const SEED: u64 = 1;

/// The tiny per-family protocol: 1 warmup + 2 measured samples = 3
/// closure invocations (the delete pool at Tiny, 256, covers it).
const TINY_PROTO: Protocol = Protocol {
    warmups: 1,
    samples: 2,
};

/// Total closure invocations under [`TINY_PROTO`].
const COUNT: usize = 3;

/// The post-state judgment over both relations — the one fold every
/// write-family test ends on.
fn assert_twins_identical(db: &bumbledb::Db<super::CrudWorld>, conn: &rusqlite::Connection) {
    for rel in [ids::DOC, ids::COUNTER] {
        let name = super::schema().relation(rel).name();
        let ours = poststate::engine_rows(db, rel).expect("engine rows");
        let theirs =
            poststate::sqlite_rows(conn, super::schema().relation(rel)).expect("mirror rows");
        poststate::assert_identical("crud", name, ours, theirs).expect(name);
    }
}

/// The mixed lane's expected work under [`TINY_PROTO`]: the rotation
/// is deterministic (4 sets — 3 one-row hits, 1 miss — cycled across
/// 9 reads per invocation, warmups included), and work counts the
/// measured samples only: drained answers + 1 insert row per sample.
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

/// EVERY write family runs its engine runner then its `SQLite` runner
/// over the one shared op stream on a Durable twin pair, in registry
/// order, and the twins end value-identical on BOTH relations — the
/// representation verdict's proof: one stream, two engines, post-state
/// equality as a consequence. Each measurement's work is the family's
/// rows-per-sample × samples.
#[test]
fn every_crud_write_family_leaves_the_twins_value_identical() {
    let sizes = CrudSizes::of(Scale::Tiny);
    let dir = scratch("families-durable");
    let (db, conn) =
        super::corpus::load_stores(&dir, SEED, sizes, DurabilityLane::Durable).expect("load");
    // One cursor per engine pass: both mint the identical id sequence.
    let mut ours_cursor = FreshCursor::at_base(sizes);
    let mut theirs_cursor = FreshCursor::at_base(sizes);

    // The insert ladder (crud_insert, _10, _100, _1k).
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

    // crud_update.
    let stream = ops::update_stream(SEED, sizes, COUNT);
    let ours = lanes::update_bumbledb(&db, TINY_PROTO, &stream).expect("update engine");
    let theirs = lanes::update_sqlite(&conn, TINY_PROTO, &stream).expect("update sqlite");
    assert_eq!(ours.work, 2, "update: engine work");
    assert_eq!(theirs.work, 2, "update: mirror work");

    // crud_update_hot (the same runners over the hot stream).
    let stream = ops::hot_update_stream(COUNT);
    let ours = lanes::update_bumbledb(&db, TINY_PROTO, &stream).expect("hot engine");
    let theirs = lanes::update_sqlite(&conn, TINY_PROTO, &stream).expect("hot sqlite");
    assert_eq!(ours.work, 2, "hot: engine work");
    assert_eq!(theirs.work, 2, "hot: mirror work");

    // crud_upsert.
    let stream = ops::upsert_stream(SEED, sizes, COUNT);
    let ours = lanes::upsert_bumbledb(&db, TINY_PROTO, &stream).expect("upsert engine");
    let theirs = lanes::upsert_sqlite(&conn, TINY_PROTO, &stream).expect("upsert sqlite");
    assert_eq!(ours.work, 2, "upsert: engine work");
    assert_eq!(theirs.work, 2, "upsert: mirror work");

    // crud_rmw.
    let keys = ops::rmw_stream(SEED, sizes, COUNT);
    let ours = lanes::rmw_bumbledb(&db, TINY_PROTO, &keys).expect("rmw engine");
    let theirs = lanes::rmw_sqlite(&conn, TINY_PROTO, &keys).expect("rmw sqlite");
    assert_eq!(ours.work, 2, "rmw: engine work");
    assert_eq!(theirs.work, 2, "rmw: mirror work");

    // crud_delete.
    let ours = lanes::delete_bumbledb(&db, TINY_PROTO, SEED, sizes).expect("delete engine");
    let theirs = lanes::delete_sqlite(&conn, TINY_PROTO, sizes).expect("delete sqlite");
    assert_eq!(ours.work, 2, "delete: engine work");
    assert_eq!(theirs.work, 2, "delete: mirror work");

    // crud_mixed_90_10.
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

/// The Nosync lane runs the same families identically — the other
/// [`DurabilityLane`] constructor drives the identical runners over a
/// representative write subset and the twins still end value-identical.
#[test]
fn the_nosync_lane_runs_the_same_families_identically() {
    let sizes = CrudSizes::of(Scale::Tiny);
    let dir = scratch("families-nosync");
    let (db, conn) =
        super::corpus::load_stores(&dir, SEED, sizes, DurabilityLane::Nosync).expect("load");
    let mut ours_cursor = FreshCursor::at_base(sizes);
    let mut theirs_cursor = FreshCursor::at_base(sizes);

    lanes::insert_bumbledb(&db, TINY_PROTO, SEED, 1, &mut ours_cursor).expect("insert engine");
    lanes::insert_sqlite(&conn, TINY_PROTO, SEED, 1, &mut theirs_cursor).expect("insert sqlite");

    let stream = ops::upsert_stream(SEED, sizes, COUNT);
    lanes::upsert_bumbledb(&db, TINY_PROTO, &stream).expect("upsert engine");
    lanes::upsert_sqlite(&conn, TINY_PROTO, &stream).expect("upsert sqlite");

    let keys = ops::rmw_stream(SEED, sizes, COUNT);
    lanes::rmw_bumbledb(&db, TINY_PROTO, &keys).expect("rmw engine");
    lanes::rmw_sqlite(&conn, TINY_PROTO, &keys).expect("rmw sqlite");

    lanes::delete_bumbledb(&db, TINY_PROTO, SEED, sizes).expect("delete engine");
    lanes::delete_sqlite(&conn, TINY_PROTO, sizes).expect("delete sqlite");

    assert_twins_identical(&db, &conn);
    drop((db, conn));
    let _ = std::fs::remove_dir_all(&dir);
}

/// The delete lane's refusal contract, falsified from both sides:
/// deleting the same pool row twice makes the second engine call `Err`
/// (the in-closure sentinel — the lane never degrades to a no-op
/// measurement), and the refusal commits NOTHING: the store generation
/// does not move across it.
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
    lanes::delete_bumbledb(&db, one, SEED, sizes).expect("the first delete bears");
    let generation = db.generation().expect("generation");
    let err = lanes::delete_bumbledb(&db, one, SEED, sizes)
        .expect_err("the second delete of the same pool row must refuse");
    assert!(err.contains("delete-bearing"), "{err}");
    assert_eq!(
        db.generation().expect("generation"),
        generation,
        "a refused delete must leave the store untouched"
    );
    drop((db, conn));
    let _ = std::fs::remove_dir_all(&dir);
}

/// The upsert stream genuinely exercises both arms at Tiny — at least
/// one hit (`Some` prev) and one miss (`None` prev) — and the engine
/// runner follows it to a post-state value-identical with `SQLite`'s
/// native conflict-target upsert.
#[test]
fn the_upsert_follows_its_stream_through_hits_and_misses() {
    let sizes = CrudSizes::of(Scale::Tiny);
    let proto = Protocol {
        warmups: 2,
        samples: 6,
    };
    let stream = ops::upsert_stream(SEED, sizes, 8);
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

/// The read query translates to SQL (the canonical twin exists), and
/// the stream generators are pure: the same `(seed, sizes, count)`
/// yields the identical stream twice.
#[test]
fn the_read_query_translates_and_the_stream_generators_are_pure() {
    let translated = crate::translate::translate(&lanes::read_query(), super::schema(), &[])
        .expect("the read query translates");
    assert!(translated.sql.contains("SELECT"), "{}", translated.sql);
    let sizes = CrudSizes::of(Scale::Tiny);
    assert_eq!(
        ops::update_stream(SEED, sizes, 16),
        ops::update_stream(SEED, sizes, 16)
    );
    assert_eq!(
        ops::upsert_stream(SEED, sizes, 16),
        ops::upsert_stream(SEED, sizes, 16)
    );
    assert_eq!(
        ops::rmw_stream(SEED, sizes, 16),
        ops::rmw_stream(SEED, sizes, 16)
    );
}

/// The orchestration tests' seed. The full run replays every family
/// back to back under the REGISTRY warmups plus the 2-sample override
/// (10 invocations for the counter streams, not [`COUNT`]), so the
/// no-collision condition on the seeded streams is re-satisfied at
/// those lengths: seed 0's update stream never draws key 0 (the hot
/// lane's row) and its upsert stream never draws a key the update
/// stream touched. (Seed 1 — [`SEED`] — collides at length 10: the
/// upsert abort said so, loudly, exactly as the refusal contract
/// promises.)
const RUN_SEED: u64 = 0;

/// A loader that loads the real twin, then poisons the `SQLite` mirror
/// with one extra `Doc` row at the read rotation's guaranteed-miss key
/// (`u64::MAX / 2` — a key no insert lane can mint), so the gate's miss
/// set finds a row on one engine only. The fold under test is the SAME
/// fold [`super::run_with`] runs — only the store source differs.
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

/// The oracle gate refuses a divergent mirror: a poisoned `SQLite` twin
/// makes the fold `Err` naming the disagreement — nothing gets timed,
/// nothing gets rendered.
#[test]
fn the_crud_gate_refuses_a_divergent_oracle() {
    let sizes = CrudSizes::of(Scale::Tiny);
    let dir = scratch("run-gate-divergent");
    let err = super::run::fold(&dir, RUN_SEED, sizes, Some(2), None, &|lane_dir, lane| {
        load_poisoned(lane_dir, lane, sizes)
    })
    .expect_err("a poisoned mirror must not be timed");
    assert!(err.contains("ENGINES DISAGREE"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The full orchestration at `Tiny` with 2 samples per family: both
/// lane sections render with every family row, and the JSON artifact
/// parses back through our own parser with 2 lanes × 11 rows and the
/// post-state verdict. This is a correctness smoke test — no number it
/// produces is recorded anywhere.
#[test]
fn the_full_crud_run_produces_both_lanes_and_parses() {
    let sizes = CrudSizes::of(Scale::Tiny);
    let dir = scratch("run-full");
    let (md, json_text) =
        super::run_with(&dir, RUN_SEED, sizes, Some(2), None).expect("the full crud run");
    assert!(md.contains("## lane durable"), "{md}");
    assert!(md.contains("## lane nosync"), "{md}");
    for family in super::families() {
        assert!(md.contains(family.name), "missing {} in\n{md}", family.name);
    }
    let parsed = crate::json::parse(&json_text).expect("the artifact parses");
    let lanes = parsed
        .get("lanes")
        .and_then(crate::json::Value::as_arr)
        .expect("lanes array");
    assert_eq!(lanes.len(), 2, "two durability lanes");
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
    let _ = std::fs::remove_dir_all(&dir);
}

/// An unknown `--only` name is refused before anything loads, and the
/// refusal lists the registry.
#[test]
fn an_unknown_only_name_is_refused() {
    let sizes = CrudSizes::of(Scale::Tiny);
    let dir = scratch("run-unknown-only");
    let err = super::run_with(&dir, RUN_SEED, sizes, Some(2), Some(&["nope".to_owned()]))
        .expect_err("an unknown family name must refuse");
    assert!(err.contains("unknown family `nope`"), "{err}");
    assert!(err.contains("crud_read_point"), "{err}");
    assert!(err.contains("crud_mixed_90_10"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The gate is UNCONDITIONAL: filtering the run down to `crud_insert`
/// (the read query untimed as its own family) still gates the read
/// query, so a poisoned mirror still refuses the whole run.
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
        &|lane_dir, lane| load_poisoned(lane_dir, lane, sizes),
    )
    .expect_err("the gate must run even when read_point is filtered out");
    assert!(err.contains("ENGINES DISAGREE"), "{err}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A one-row post-state divergence is loud: the error names the world
/// and the relation before rendering the multiset diff.
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
