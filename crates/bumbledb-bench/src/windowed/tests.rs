use bumbledb::schema::ValidateDescriptor as _;
use bumbledb::{Db, Theory as _, Value};

use crate::differential::{self, Op};
use crate::naive::{Delta, NaiveDb};

use super::{Mass, baseline, ids, parent_kind, relation_rows, world};

fn scratch(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("bumbledb-windowed-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

fn child(id: u64, parent: u64, flag: u64) -> (bumbledb::RelationId, Vec<Value>) {
    (
        ids::CHILD,
        vec![Value::U64(id), Value::U64(parent), Value::U64(flag)],
    )
}

#[test]
fn the_twin_theories_validate_and_differ_only_in_capacity_laws() {
    let windowed = world::WindowedWorld
        .descriptor()
        .validate()
        .expect("the windowed twin validates");
    let unwindowed = baseline::UnwindowedWorld
        .descriptor()
        .validate()
        .expect("the baseline twin validates");
    assert_eq!(
        windowed.capacities().len(),
        2,
        "the fan-cap and the exclusion"
    );
    assert_eq!(unwindowed.capacities().len(), 0, "the control carries none");
    assert_eq!(
        windowed.containments().len(),
        unwindowed.containments().len()
    );
    let exclusion = &windowed.capacities()[1];
    assert_eq!(
        exclusion.weight,
        bumbledb::schema::SealedWeight::Unit,
        "the count instance, explicitly"
    );
    assert_eq!(
        (exclusion.lo, exclusion.hi),
        (0, bumbledb::schema::SealedBound::Lit(0)),
        "the {{0}} window"
    );
}

#[test]
fn the_window_verdicts_agree_with_the_naive_model() {
    let dir = scratch("naive");
    let mass = Mass::unit();
    let db = Db::create(&dir, world::WindowedWorld, crate::harness::bench_work())
        .expect("create")
        .expect("accepted");
    let mut naive = NaiveDb::new(&world::WindowedWorld.descriptor());

    let mut ops = Vec::new();
    for rel in [ids::PARENT, ids::CHILD] {
        let mut delta = Delta::default();
        for row in relation_rows(mass, rel) {
            delta.inserts.push((rel, row));
            if delta.inserts.len() == 32 {
                ops.push(Op::Write(std::mem::take(&mut delta)));
            }
        }
        if !delta.inserts.is_empty() {
            ops.push(Op::Write(std::mem::take(&mut delta)));
        }
    }
    let base = mass.parents * mass.children_per_parent;

    ops.push(Op::Write(Delta {
        deletes: vec![],
        inserts: vec![child(base, 1, 0)],
    }));

    ops.push(Op::Write(Delta {
        deletes: vec![],
        inserts: (0..64).map(|k| child(base + 1 + k, 2, 0)).collect(),
    }));

    assert_eq!(parent_kind(0), 1);
    ops.push(Op::Write(Delta {
        deletes: vec![],
        inserts: vec![child(base + 100, 0, 1)],
    }));
    ops.push(Op::Write(Delta {
        deletes: vec![],
        inserts: vec![child(base + 101, 3, 1)],
    }));

    let summary = differential::run(&db, &mut naive, &ops).expect("verdict parity");
    assert_eq!(summary.aborts, 2, "the over-cap burst and the exclusion");
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}
