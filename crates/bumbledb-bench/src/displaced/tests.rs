use bumbledb::Theory as _;

use crate::corpus_gen::{GenConfig, Scale};
use crate::harness::{self, Modes, Protocol};
use crate::storemode::StoreMode;

use super::{
    DispSizes, FORCED_MAP_DISTINCT, FORCED_MAP_POSITIONS, ForeignStream, forced_spoke_map_bytes,
};

fn scratch(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("bumbledb-displaced-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

/// The displaced schema validates and the registry is coherent: unique
/// names, each shape carrying a resident control (mass 0) and a
/// displaced ladder.
#[test]
fn the_schema_validates_and_the_registry_is_coherent() {
    let schema = super::DisplacedWorld
        .descriptor()
        .validate()
        .expect("the displaced schema validates");
    assert_eq!(schema.containments().len(), 1, "Spoke(hub) <= Hub(id)");
    let mut names = std::collections::BTreeSet::new();
    for family in super::all() {
        assert!(names.insert(family.name), "unique names");
        assert!(!family.about.is_empty());
    }
    for shape in ["disp_probe", "disp_stream"] {
        let masses: Vec<u64> = super::all()
            .iter()
            .filter(|f| f.name.starts_with(shape))
            .map(|f| f.displace_mib)
            .collect();
        assert_eq!(masses, vec![0, 24, 96], "{shape}: control + the ladder");
    }
}

/// The ≥ 32 MiB claim, computed from the engine's own layout rules over
/// the TRACED force shape (2^20 spoke positions, 453,241 distinct hub
/// keys — the obs test below pins those numbers against the engine
/// itself; this one pins the arithmetic they imply). If the engine's
/// COLT sizing or image encoding changes, this pins the drift.
#[test]
fn the_bench_shape_exceeds_the_l2_by_layout_arithmetic() {
    let sizes = DispSizes::of(Scale::S);
    // The pinned distinct count is the generator's arithmetic, not a
    // free constant: 2^20 uniform draws over the 2^19 hub key space.
    assert_eq!(FORCED_MAP_POSITIONS, sizes.spokes);
    let mut seen = vec![false; usize::try_from(sizes.hubs).expect("64-bit usize")];
    for i in 0..sizes.spokes {
        let m = crate::corpus_gen::mix(1, super::ids::SPOKE, i);
        seen[usize::try_from(m % sizes.hubs).expect("64-bit usize")] = true;
    }
    let distinct = u64::try_from(seen.iter().filter(|s| **s).count()).expect("fits u64");
    assert_eq!(distinct, FORCED_MAP_DISTINCT, "1 - e^-2 of 2^19, exactly");
    // The forced spoke map alone: 2^18 buckets → 2 MiB ctrl + 32 MiB
    // bucket words = 34 MiB, past one P-cluster's 32 MiB L2.
    let map = forced_spoke_map_bytes(FORCED_MAP_POSITIONS, FORCED_MAP_DISTINCT);
    assert_eq!(map, (1 << 18) * 8 + (1 << 18) * 16 * 8, "2^18 buckets");
    assert!(map >= 32 << 20, "the forced map is the >= 32 MiB claim");
    // The steady-state per-pass touched mass: the map, the iterated hub
    // image, and the gathered spoke val column.
    let touched = map + sizes.hub_image_bytes() + sizes.spokes * 8;
    assert!(touched >= 48 << 20, "≈ 50 MiB per steady-state probe pass");
    // The stream shape's touched mass: two spoke columns = 16 MiB.
    assert_eq!(sizes.spokes * 2 * 8, 16 << 20);
    // Every timed scale shares the shape (the closure precedent).
    assert_eq!(DispSizes::of(Scale::M), sizes);
    assert_eq!(DispSizes::of(Scale::L), sizes);
}

/// The regime label observed on the ENGINE, not derived beside it (the
/// arithmetic test above cannot catch plan drift — this one can): at
/// the real bench shape, the first probe execute forces exactly one
/// COLT map, ingesting all 2^20 SPOKE positions keyed by hub value with
/// the pinned distinct count — the executor iterates the hub side and
/// probes the ≈ 34 MiB spoke map. And the force is once per prepare:
/// the second execute memo-hits, forcing nothing and rebuilding no
/// image, so every timed pass after warmup 1 is the steady-state shape
/// the module doc claims.
#[cfg(feature = "obs")]
#[test]
fn the_engine_trace_pins_the_forced_map_and_its_memoization() {
    use bumbledb::obs;

    let dir = scratch("trace-pin");
    let cfg = GenConfig {
        seed: 1, // the bench default; distinct is seed-invariant below 2^20 anyway
        scale: Scale::S,
    };
    let sizes = DispSizes::of(cfg.scale);
    // Engine store only — the mirror is parity's business, not this pin's.
    let db = StoreMode::Durable
        .create(&dir.join("db"), super::DisplacedWorld)
        .expect("create");
    for rel in [super::ids::HUB, super::ids::SPOKE] {
        db.bulk_load_dyn(rel, super::relation_rows(sizes, cfg.seed, rel))
            .expect("load");
    }
    let mut prepared = db.prepare(&super::probe_query()).expect("prepare");
    let mut buffer = bumbledb::Answers::new();
    let mut traced_execute = || {
        obs::start_capture();
        db.read(|snap| snap.execute_args(&mut prepared, &[], &mut buffer))
            .expect("execute");
        obs::finish_capture()
    };

    let first = traced_execute();
    let forces: Vec<(u64, u64)> = first
        .iter()
        .filter(|e| e.name == obs::names::COLT_FORCE)
        .map(|e| (e.a0, e.a1))
        .collect();
    assert_eq!(
        forces,
        vec![(FORCED_MAP_POSITIONS, FORCED_MAP_DISTINCT)],
        "one force: all spoke positions, the pinned distinct hub keys"
    );
    assert!(
        first.iter().any(|e| e.name == obs::names::IMAGE_BUILD),
        "the first execute decodes the images"
    );

    let second = traced_execute();
    let count = |name: &str| second.iter().filter(|e| e.name == name).count();
    assert!(
        count(obs::names::VIEW_MEMO_HIT) > 0,
        "the second execute rides the view memo"
    );
    assert_eq!(
        count(obs::names::COLT_FORCE),
        0,
        "force is once per prepare"
    );
    assert_eq!(count(obs::names::IMAGE_BUILD), 0, "images are cached");

    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

/// `SQLite` parity at the shrunk scale (the windowed family's unit-mass
/// precedent): both shapes row-identical across engines on the `Tiny`
/// world, through the exact verify path the timed lane runs.
#[test]
fn the_tiny_world_verifies_on_both_engines() {
    let dir = scratch("parity");
    let cfg = GenConfig {
        seed: 7,
        scale: Scale::Tiny,
    };
    let (db, conn) = super::load_stores(&dir, cfg, StoreMode::Durable).expect("load");
    for family in super::all() {
        super::verify_family(&db, &conn, family).expect(family.name);
    }
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The probe fold groups by tag and the stream fold is a single row —
/// the work counts the timed lane black-boxes are the real answer
/// masses.
#[test]
fn the_folds_produce_their_group_masses() {
    let dir = scratch("masses");
    let cfg = GenConfig {
        seed: 7,
        scale: Scale::Tiny,
    };
    let sizes = DispSizes::of(Scale::Tiny);
    let (db, _conn) = super::load_stores(&dir, cfg, StoreMode::Durable).expect("load");
    let mut buffer = bumbledb::Answers::new();
    let mut prepared = db.prepare(&super::probe_query()).expect("prepare");
    db.read(|snap| snap.execute_args(&mut prepared, &[], &mut buffer))
        .expect("execute");
    assert_eq!(buffer.len() as u64, sizes.tags, "one group per tag");
    let mut prepared = db.prepare(&super::stream_query()).expect("prepare");
    db.read(|snap| snap.execute_args(&mut prepared, &[], &mut buffer))
        .expect("execute");
    assert_eq!(buffer.len(), 1, "the ungrouped fold");
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The interleave harness runs the between-pass closure before every
/// warmup and every timed sample, and the foreign stream touches the
/// claimed mass through the same code path at mass 0 (a no-op) and
/// mass 1.
#[test]
fn the_interleaved_harness_runs_between_every_pass() {
    let proto = Protocol {
        warmups: 2,
        samples: 3,
    };
    let mut between = 0u32;
    let mut passes = 0u64;
    let m = harness::measure_interleaved(
        proto,
        Modes::default(),
        1,
        || between += 1,
        || {
            passes += 1;
            Ok(1)
        },
    )
    .expect("measure");
    assert_eq!(between, proto.warmups + proto.samples);
    assert_eq!(passes, u64::from(proto.warmups + proto.samples));
    assert_eq!(m.work, u64::from(proto.samples));

    let mut resident = ForeignStream::new(0);
    resident.stream(); // the mass-0 control is a no-op, same path
    let mut foreign = ForeignStream::new(1);
    assert_eq!(foreign.buf.len(), 1 << 20);
    foreign.stream();
    foreign.stream();
    assert_eq!(foreign.buf[0], 2, "each pass rewrites every line");
    assert_eq!(foreign.buf[64], 2);
    assert_eq!(foreign.buf[1], 0, "one byte per line dirties the line");
}
