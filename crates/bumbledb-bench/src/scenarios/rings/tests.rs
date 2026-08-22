use bumbledb::schema::SchemaDescriptor;
use bumbledb::{AnswerValue, Answers, Db, Query, Value};

use crate::families::bind_values;

fn smoke_store(name: &str) -> (Db<SchemaDescriptor>, std::path::PathBuf) {
    let dir = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&dir);
    let db = Db::create(&dir, bumbledb::Theory::descriptor(super::Rings))
        .expect("create")
        .expect("accepted");
    for (rel, rows) in super::corpus::rows_smoke(7) {
        db.write(|tx| {
            tx.insert_dyn(rel, rows)
                .map(bumbledb::MutationReport::changed)
        })
        .expect("insert")
        .unwrap();
    }
    (db, dir)
}

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

#[test]
fn rings_smoke_gate_agrees_on_every_family() {
    let dir = std::env::temp_dir().join("bumbledb-rings-smoke-gate");
    let _ = std::fs::remove_dir_all(&dir);
    crate::scenarios::gate_scenario(&dir, &super::scenario_smoke(), 7)
        .expect("every rings family agrees with SQLite at smoke scale");
    let _ = std::fs::remove_dir_all(&dir);
}

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
