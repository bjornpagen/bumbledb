use bumbledb::Theory as _;

use crate::corpus_gen::{GenConfig, Scale};
use crate::harness::{self, Modes, Protocol};
use crate::storemode::StoreMode;

use super::{DispSizes, ForeignStream, forced_hub_map_bytes};

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

/// The ≥ 32 MiB claim, computed from the engine's own layout rules —
/// the lane's regime label is arithmetic, not hope. If the engine's
/// COLT sizing or image encoding changes, this pins the drift.
#[test]
fn the_bench_shape_exceeds_the_l2_by_layout_arithmetic() {
    let sizes = DispSizes::of(Scale::S);
    // The forced hub map alone: 2^18 buckets → 2 MiB ctrl + 32 MiB
    // bucket words = 34 MiB, past one P-cluster's 32 MiB L2.
    let map = forced_hub_map_bytes(sizes.hubs);
    assert_eq!(map, (1 << 18) * 8 + (1 << 18) * 16 * 8, "2^18 buckets");
    assert!(map >= 32 << 20, "the forced map is the >= 32 MiB claim");
    // The per-pass touched mass: map + both images.
    let touched = map + sizes.hub_image_bytes() + sizes.spoke_image_bytes();
    assert!(touched >= 64 << 20, "≈ 66 MiB per probe pass");
    // The stream shape's touched mass: two spoke columns = 16 MiB.
    assert_eq!(sizes.spokes * 2 * 8, 16 << 20);
    // Every timed scale shares the shape (the closure precedent).
    assert_eq!(DispSizes::of(Scale::M), sizes);
    assert_eq!(DispSizes::of(Scale::L), sizes);
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
