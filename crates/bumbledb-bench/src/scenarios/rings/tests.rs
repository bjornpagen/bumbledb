//! The rings smoke tests: tiny corpora, zero timing. The gate test runs
//! the full uncapped multiset oracle for every family; the bomb test
//! asserts the corpus construction theorem's exact answer; the wash-ring
//! test pins the planted ring's hit and miss.

use bumbledb::schema::SchemaDescriptor;
use bumbledb::{AnswerValue, Answers, Db, Query, Value};

use crate::families::bind_values;

/// A scratch store loaded with the SMOKE corpus.
fn smoke_store(name: &str) -> (Db<SchemaDescriptor>, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let db = Db::create(&dir, bumbledb::Theory::descriptor(super::Rings)).expect("create");
    for (rel, rows) in super::corpus::rows_smoke(7) {
        db.bulk_load_dyn(rel, rows).expect("bulk load");
    }
    (db, dir)
}

/// Executes one global-Count query and returns its single answer, or
/// `None` for the empty answer set (a global Count over an empty
/// binding set yields NO rows in this engine — the HAVING COUNT(*)>0
/// collapse — so the miss is an empty set, never a zero row).
fn run_count(db: &Db<SchemaDescriptor>, query: &Query, params: &[Value]) -> Option<u64> {
    let mut prepared = db.prepare(query).expect("prepare");
    let mut buffer = Answers::new();
    db.read(|snap| snap.execute(&mut prepared, &bind_values(params), &mut buffer))
        .expect("execute");
    match buffer.len() {
        0 => None,
        1 => match buffer.get(0, 0) {
            AnswerValue::U64(count) => Some(count),
            other => panic!("a Count answer is U64, got {other:?}"),
        },
        n => panic!("a global Count yields at most one row, got {n}"),
    }
}

/// The tier-0 oracle gate: every rings family × param set at SMOKE
/// scale must produce value-identical multisets on both engines — the
/// FULL uncapped gate (`gate_scenario`), correctness only, no timing.
#[test]
fn rings_smoke_gate_agrees_on_every_family() {
    let dir = std::env::temp_dir().join("bumbledb-rings-smoke-gate");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("scratch dir");
    crate::scenarios::gate_scenario(&dir, &super::scenario_smoke(), 7)
        .expect("every rings family agrees with SQLite at smoke scale");
    let _ = std::fs::remove_dir_all(&dir);
}

/// The mechanical tuning law: the hand-tuned r2 twin actually removed
/// the inflation — the pinned constant contains no Allen OR-chain — and
/// it is still the counted fold.
#[test]
fn r2_tuned_twin_has_no_or_chain() {
    assert!(
        !super::HAND_R2.contains(" OR "),
        "the tuned rendering must carry no Allen OR-chain"
    );
    assert!(
        super::HAND_R2.contains("COUNT"),
        "the tuned rendering is still the counted fold"
    );
}

/// The tuned lane's placeholder row mirrors the canonical translation's
/// exactly — same slots, same order — so both lanes bind identically.
#[test]
fn r2_tuned_param_slots_match_canonical() {
    let canonical = crate::translate::translate(&super::temporal_ring(), super::schema(), &[])
        .expect("r2 translates");
    assert_eq!(
        super::r2_tuned().params,
        canonical.params,
        "the tuned param slots mirror the canonical translation"
    );
}

/// The construction theorem, asserted: each bomb's bipartite part is
/// triangle-free (a 3-cycle would alternate sides and need an A→A or
/// B→B edge the generator cannot emit), so the triangle count is
/// EXACTLY the 3 rotations of the planted cycle — the analytic oracle.
#[test]
fn bomb_answer_is_the_planted_triangle() {
    let (db, dir) = smoke_store("bumbledb-rings-bomb-answer");
    assert_eq!(
        run_count(&db, &super::bomb_t1(), &[]),
        Some(3),
        "tier 1: exactly the planted triangle's rotations"
    );
    assert_eq!(
        run_count(&db, &super::bomb_t2(), &[]),
        Some(3),
        "tier 2: exactly the planted triangle's rotations"
    );
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The planted wash ring is found at smoke scale: r1 and r2 both count
/// it at the 1000 threshold (planted amount `9_999`; identical planted
/// spans make EQUALS ∈ INTERSECTS), and the `1_000_000` miss yields the
/// empty answer set (amounts top out below `10_000`).
#[test]
fn planted_wash_ring_is_nonempty_at_smoke() {
    let (db, dir) = smoke_store("bumbledb-rings-wash-ring");
    let hit = run_count(&db, &super::wash_ring(), &[Value::I64(1000)])
        .expect("the planted ring clears the 1000 bar");
    assert!(hit >= 1, "r1 counts the planted ring");
    let temporal = run_count(&db, &super::temporal_ring(), &[Value::I64(1000)])
        .expect("identical planted spans intersect pairwise");
    assert!(temporal >= 1, "r2 counts the planted ring");
    assert_eq!(
        run_count(&db, &super::wash_ring(), &[Value::I64(1_000_000)]),
        None,
        "the miss: an empty binding set is the empty answer set, not a zero row"
    );
    drop(db);
    let _ = std::fs::remove_dir_all(&dir);
}
