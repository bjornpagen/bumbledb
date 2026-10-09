use bumbledb::Theory as _;
use bumbledb::schema::ValidateDescriptor as _;

use crate::harness::{self, Protocol};
use crate::worlds::corpus_gen::{GenConfig, Scale};

use super::{DispSizes, ForeignStream};

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
fn the_tiny_world_verifies_on_both_engines() {
    let dir = crate::fixture::TempDir::new("parity");
    let cfg = GenConfig {
        seed: 7,
        scale: Scale::Tiny,
    };
    let (db, conn) = super::load_stores(&dir, cfg).expect("load");
    for family in super::all() {
        super::verify_family(&db, &conn, family).expect(family.name);
    }
    drop(db);
}

#[test]
fn the_folds_produce_their_group_masses() {
    let dir = crate::fixture::TempDir::new("masses");
    let cfg = GenConfig {
        seed: 7,
        scale: Scale::Tiny,
    };
    let sizes = DispSizes::of(Scale::Tiny);
    let (db, _conn) = super::load_stores(&dir, cfg).expect("load");
    let mut buffer = bumbledb::Answers::new();
    let mut prepared = db
        .prepare(&super::probe_query(), crate::harness::bench_work())
        .expect("prepare");
    db.read(crate::harness::bench_work(), |snap| {
        snap.execute(&mut prepared, &[] as &[bumbledb::BindValue], &mut buffer)
    })
    .expect("execute");
    assert_eq!(buffer.len() as u64, sizes.tags, "one group per tag");
    let mut prepared = db
        .prepare(&super::stream_query(), crate::harness::bench_work())
        .expect("prepare");
    db.read(crate::harness::bench_work(), |snap| {
        snap.execute(&mut prepared, &[] as &[bumbledb::BindValue], &mut buffer)
    })
    .expect("execute");
    assert_eq!(buffer.len(), 1, "the ungrouped fold");
    drop(db);
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
