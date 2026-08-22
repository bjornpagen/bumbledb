use bumbledb::Theory as _;
use bumbledb::schema::ValidateDescriptor as _;

use crate::corpus_gen::{GenConfig, Scale};
use crate::harness::{self, Modes, Protocol};
use crate::storemode::StoreMode;

use super::{
    DispSizes, FORCED_MAP_DISTINCT, FORCED_MAP_POSITIONS, ForeignStream, forced_spoke_map_bytes,
};

fn scratch(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("bumbledb-displaced-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

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

#[test]
fn the_bench_shape_exceeds_the_l2_by_layout_arithmetic() {
    let sizes = DispSizes::of(Scale::S);

    assert_eq!(FORCED_MAP_POSITIONS, sizes.spokes);
    let mut seen = vec![false; usize::try_from(sizes.hubs).expect("64-bit usize")];
    for i in 0..sizes.spokes {
        let m = crate::corpus_gen::mix(1, super::ids::SPOKE, i);
        seen[usize::try_from(m % sizes.hubs).expect("64-bit usize")] = true;
    }
    let distinct = u64::try_from(seen.iter().filter(|s| **s).count()).expect("fits u64");
    assert_eq!(distinct, FORCED_MAP_DISTINCT, "1 - e^-2 of 2^19, exactly");
    // The forced spoke map alone: 2^18 buckets → 2 MiB ctrl + 32 MiB

    let map = forced_spoke_map_bytes(FORCED_MAP_POSITIONS, FORCED_MAP_DISTINCT);
    assert_eq!(map, (1 << 18) * 8 + (1 << 18) * 16 * 8, "2^18 buckets");
    assert!(map >= 32 << 20, "the forced map is the >= 32 MiB claim");

    let touched = map + sizes.hub_image_bytes() + sizes.spokes * 8;
    assert!(touched >= 48 << 20, "≈ 50 MiB per steady-state probe pass");

    assert_eq!(sizes.spokes * 2 * 8, 16 << 20);

    assert_eq!(DispSizes::of(Scale::M), sizes);
    assert_eq!(DispSizes::of(Scale::L), sizes);
}

/// And each force is once per prepare: the second execute memo-hits, forcing
/// nothing and rebuilding no image, so every timed pass after warmup 1 is the
/// steady-state shape the module doc claims.
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

    let db = StoreMode::Durable
        .create(&dir.join("db"), super::DisplacedWorld)
        .expect("create");
    for rel in [super::ids::HUB, super::ids::SPOKE] {
        db.write(|tx| {
            tx.insert_dyn(rel, super::relation_rows(sizes, cfg.seed, rel))
                .map(bumbledb::MutationReport::changed)
        })
        .expect("load")
        .expect("accepted");
    }
    let mut prepared = db.prepare(&super::probe_query()).expect("prepare");
    let mut buffer = bumbledb::Answers::new();
    let mut traced_execute = || {
        obs::start_capture();
        db.read(|snap| snap.execute(&mut prepared, &[] as &[bumbledb::BindValue], &mut buffer))
            .expect("execute");
        obs::finish_capture()
    };

    let first = traced_execute();
    let forces: Vec<(u64, u64)> = first
        .iter()
        .filter(|e| e.point() == obs::names::COLT_FORCE)
        .map(|e| (e.a0(), e.a1()))
        .collect();
    assert_eq!(
        forces,
        vec![
            (sizes.hubs, sizes.tags),
            (FORCED_MAP_POSITIONS, FORCED_MAP_DISTINCT),
        ],
        "two forces: the hub tag prefix, then all spoke positions at the pinned distinct hub keys"
    );
    assert!(
        first.iter().any(|e| e.point() == obs::names::IMAGE_BUILD),
        "the first execute decodes the images"
    );

    let second = traced_execute();
    let count = |point: obs::TracePoint| second.iter().filter(|e| e.point() == point).count();
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
    db.read(|snap| snap.execute(&mut prepared, &[] as &[bumbledb::BindValue], &mut buffer))
        .expect("execute");
    assert_eq!(buffer.len() as u64, sizes.tags, "one group per tag");
    let mut prepared = db.prepare(&super::stream_query()).expect("prepare");
    db.read(|snap| snap.execute(&mut prepared, &[] as &[bumbledb::BindValue], &mut buffer))
        .expect("execute");
    assert_eq!(buffer.len(), 1, "the ungrouped fold");
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The interleave harness runs the between-pass closure before every warmup and
/// every timed sample, and the foreign stream touches the claimed mass through
/// the same code path at mass 0 (a no-op) and mass 1.
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
    resident.stream(); 
    let mut foreign = ForeignStream::new(1);
    assert_eq!(foreign.buf.len(), 1 << 20);
    foreign.stream();
    foreign.stream();
    assert_eq!(foreign.buf[0], 2, "each pass rewrites every line");
    assert_eq!(foreign.buf[64], 2);
    assert_eq!(foreign.buf[1], 0, "one byte per line dirties the line");
}
