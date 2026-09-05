use bumbledb::schema::ValidateDescriptor as _;
use bumbledb::{Db, Theory as _, Value};

use crate::differential::{self, Op};
use crate::naive::{Delta, NaiveDb};
use crate::writebench::write_protocol;

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
    let db = Db::create(&dir, world::WindowedWorld)
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

#[test]
fn the_window_rows_run_their_protocols() {
    let dir = scratch("rows");
    let windowed = Db::create(&dir.join("windowed"), world::WindowedWorld)
        .expect("create")
        .expect("accepted");
    super::load(&windowed, Mass::BENCH).expect("load windowed");
    let unwindowed = Db::create(&dir.join("baseline"), baseline::UnwindowedWorld)
        .expect("create baseline")
        .expect("accepted");
    super::load(&unwindowed, Mass::BENCH).expect("load baseline");

    let mut admission_mint = super::MINT_BASE;
    let mut baseline_mint = super::MINT_BASE;
    let mut exclusion_mint = super::MINT_BASE + (1 << 20);
    let admission = super::commit_window_admission(
        &windowed,
        write_protocol("commit_window_admission"),
        &mut admission_mint,
    )
    .expect("admission");
    assert_eq!(admission.work, 64, "one row per sample");
    assert!(admission.stats.min > 0);
    let baseline_row = super::commit_window_baseline(
        &unwindowed,
        write_protocol("commit_window_baseline"),
        &mut baseline_mint,
    )
    .expect("baseline");
    assert_eq!(baseline_row.work, 64);
    let exclusion = super::commit_window_exclusion(
        &windowed,
        write_protocol("commit_window_exclusion"),
        &mut exclusion_mint,
    )
    .expect("exclusion");
    assert_eq!(exclusion.work, 64);
    drop(windowed);
    drop(unwindowed);
    let _ = std::fs::remove_dir_all(&dir);
}

#[cfg(feature = "obs")]
#[test]
fn traced_windowed_lands_the_judgment_spans() {
    let dir = scratch("traced");
    let trace_dir = dir.join("trace");
    let mut flames = Vec::new();
    let rows = super::write_families(
        crate::corpus_gen::GenConfig {
            seed: 1,
            scale: crate::corpus_gen::Scale::Tiny,
        },
        &dir.join("scratch"),
        &|name| name == "commit_window_admission",
        crate::storemode::StoreMode::Durable,
        Some(&trace_dir),
        &mut flames,
    )
    .expect("the traced windowed lane");
    assert_eq!(rows.len(), 1, "one selected row");
    assert_eq!(flames.len(), 1, "one flame embed per traced family");
    let json_path = trace_dir.join("commit_window_admission.json");
    let text = std::fs::read_to_string(&json_path)
        .unwrap_or_else(|e| panic!("{}: {e}", json_path.display()));
    assert!(
        text.starts_with("[\n") && text.ends_with("\n]\n"),
        "{} parses as a Chrome array",
        json_path.display()
    );
    assert!(
        text.contains("judgment"),
        "the window judgment spans reach the artifact"
    );
    let folded = std::fs::read_to_string(trace_dir.join("commit_window_admission.folded"))
        .expect("the folded twin lands beside the json");
    assert!(!folded.is_empty(), "a non-degenerate fold");
    let _ = std::fs::remove_dir_all(&dir);
}
