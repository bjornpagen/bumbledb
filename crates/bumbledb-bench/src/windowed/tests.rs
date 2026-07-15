use bumbledb::{Db, Theory as _, Value};

use crate::differential::{self, Op};
use crate::naive::{Delta, NaiveDb};

use super::{Mass, baseline, ids, parent_kind, relation_rows, world};

fn scratch(tag: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("bumbledb-windowed-{tag}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    dir
}

fn child(id: u64, parent: u64, flag: u64) -> (bumbledb::RelationId, Vec<Value>) {
    (
        ids::CHILD,
        vec![Value::U64(id), Value::U64(parent), Value::U64(flag)],
    )
}

/// The twin theories both validate, and only the windowed twin carries
/// the two window statements — the {0} exclusion among them
/// (`lo = hi = 0`).
#[test]
fn the_twin_theories_validate_and_differ_only_in_windows() {
    let windowed = world::WindowedWorld
        .descriptor()
        .validate()
        .expect("the windowed twin validates");
    let unwindowed = baseline::UnwindowedWorld
        .descriptor()
        .validate()
        .expect("the baseline twin validates");
    assert_eq!(windowed.windows().len(), 2, "the fan-cap and the exclusion");
    assert_eq!(unwindowed.windows().len(), 0, "the control carries none");
    assert_eq!(
        windowed.containments().len(),
        unwindowed.containments().len()
    );
    let exclusion = &windowed.windows()[1];
    assert_eq!(
        (exclusion.lo, exclusion.hi),
        (0, Some(0)),
        "the {{0}} window"
    );
}

/// Naive parity — the semantic oracle for the window judgment: the
/// unit mass loads on both oracles (the naive model is
/// O(parents × children) per judged delta — the unit-corpus
/// discipline), a legal sample commits, an over-cap burst and a
/// {0}-excluded child both abort with agreeing verdicts and citations
/// (the differential runner compares them by strict equality).
#[test]
fn the_window_verdicts_agree_with_the_naive_model() {
    let dir = scratch("naive");
    let mass = Mass::unit();
    let db = Db::create(&dir, world::WindowedWorld).expect("create");
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
    // A legal sample: one flag-0 child under parent 1 (kind 0).
    ops.push(Op::Write(Delta {
        deletes: vec![],
        inserts: vec![child(base, 1, 0)],
    }));
    // The over-cap burst: 64 more children under parent 2 blows the
    // 0..64 window (8 seeded + 1 + 64 > 64) — MUST abort on both.
    ops.push(Op::Write(Delta {
        deletes: vec![],
        inserts: (0..64).map(|k| child(base + 1 + k, 2, 0)).collect(),
    }));
    // The {0} exclusion: one flag-1 child under a kind-1 parent (parent
    // 0) — MUST abort on both; the same child under a kind-0 parent
    // commits (the exclusion selects parents by kind).
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

/// The three timed rows run their full protocols on seeded twins and
/// every measured commit is legal (the runners measure the judge, not
/// refusals).
#[test]
fn the_window_rows_run_their_protocols() {
    let dir = scratch("rows");
    let windowed = Db::create(&dir.join("windowed"), world::WindowedWorld).expect("create");
    super::load(&windowed, Mass::BENCH).expect("load windowed");
    let unwindowed =
        Db::create(&dir.join("baseline"), baseline::UnwindowedWorld).expect("create baseline");
    super::load(&unwindowed, Mass::BENCH).expect("load baseline");

    let admission = super::commit_window_admission(&windowed).expect("admission");
    assert_eq!(admission.work, 64, "one row per sample");
    assert!(admission.stats.min > 0);
    let baseline_row = super::commit_window_baseline(&unwindowed).expect("baseline");
    assert_eq!(baseline_row.work, 64);
    let exclusion = super::commit_window_exclusion(&windowed).expect("exclusion");
    assert_eq!(exclusion.work, 64);
    drop(windowed);
    drop(unwindowed);
    let _ = std::fs::remove_dir_all(&dir);
}
