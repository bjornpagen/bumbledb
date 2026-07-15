//! The extension-form differential: engine and model replay one fixed
//! delta stream over a window + order + ranked-order theory and must
//! agree on every verdict INCLUDING the violating statement — the
//! conformance face of the enforcement stage (verdict parity is the
//! typed identity, exactly the containment differential's law).

use bumbledb::schema::{
    FieldId, LiteralSet, RankChain, RankHop, RelationDescriptor, RelationId, SchemaDescriptor,
    Side, StatementDescriptor,
};
use bumbledb::{Db, Value};

use crate::differential::{Op, run};
use crate::fixture::{TempDir, field, side};
use crate::naive::{Delta, NaiveDb};

const HOLDER: RelationId = RelationId(0);
const ACCOUNT: RelationId = RelationId(1);
const ITEM: RelationId = RelationId(2);
const STEP: RelationId = RelationId(3);
const KIND_RANK: RelationId = RelationId(4);

/// The naive marks fixture's schema, verbatim (`naive/tests/judgment.rs`
/// § marks): one window, one plain order mark, one ranked order mark.
fn schema() -> SchemaDescriptor {
    let u64_field = |name: &str| field(name, bumbledb::schema::ValueType::U64);
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Holder".into(),
                fields: vec![u64_field("id"), u64_field("tag")],
            },
            RelationDescriptor {
                extension: None,
                name: "Account".into(),
                fields: vec![u64_field("holder"), u64_field("kind"), u64_field("num")],
            },
            RelationDescriptor {
                extension: None,
                name: "Item".into(),
                fields: vec![u64_field("doc"), u64_field("pos"), u64_field("note")],
            },
            RelationDescriptor {
                extension: None,
                name: "Step".into(),
                fields: vec![u64_field("flow"), u64_field("pos"), u64_field("kind")],
            },
            RelationDescriptor {
                extension: None,
                name: "KindRank".into(),
                fields: vec![u64_field("kind"), u64_field("rank")],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: HOLDER,
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Functionality {
                relation: KIND_RANK,
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Cardinality {
                source: Side {
                    relation: ACCOUNT,
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([(FieldId(1), LiteralSet::One(Value::U64(1)))]),
                },
                lo: 1,
                hi: Some(2),
                target: side(HOLDER, &[0], &[]),
            },
            StatementDescriptor::Order {
                relation: ITEM,
                position: FieldId(1),
                grouping: Box::new([FieldId(0)]),
                ranking: None,
            },
            StatementDescriptor::Order {
                relation: STEP,
                position: FieldId(1),
                grouping: Box::new([FieldId(0)]),
                ranking: Some(RankChain {
                    link: FieldId(2),
                    hops: Box::new([RankHop {
                        relation: KIND_RANK,
                        key: FieldId(0),
                        read: FieldId(1),
                    }]),
                }),
            },
        ],
    }
}

fn holder(id: u64) -> (RelationId, Vec<Value>) {
    (HOLDER, vec![Value::U64(id), Value::U64(0)])
}

fn account(holder: u64, kind: u64, num: u64) -> (RelationId, Vec<Value>) {
    (
        ACCOUNT,
        vec![Value::U64(holder), Value::U64(kind), Value::U64(num)],
    )
}

fn item(doc: u64, pos: u64, note: u64) -> (RelationId, Vec<Value>) {
    (
        ITEM,
        vec![Value::U64(doc), Value::U64(pos), Value::U64(note)],
    )
}

fn step(flow: u64, pos: u64, kind: u64) -> (RelationId, Vec<Value>) {
    (
        STEP,
        vec![Value::U64(flow), Value::U64(pos), Value::U64(kind)],
    )
}

fn kind_rank(kind: u64, rank: u64) -> (RelationId, Vec<Value>) {
    (KIND_RANK, vec![Value::U64(kind), Value::U64(rank)])
}

fn write(deletes: Vec<(RelationId, Vec<Value>)>, inserts: Vec<(RelationId, Vec<Value>)>) -> Op {
    Op::Write(Delta { deletes, inserts })
}

/// The window-boundary schema, verbatim from the naive marks fixture
/// (`naive/tests/judgment.rs` § window exactness): the `2..2`
/// exactness window (statement 1) and the `0..*` vacuity window
/// (statement 2) over Holder/Account.
fn exact_schema() -> SchemaDescriptor {
    let u64_field = |name: &str| field(name, bumbledb::schema::ValueType::U64);
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Holder".into(),
                fields: vec![u64_field("id"), u64_field("tag")],
            },
            RelationDescriptor {
                extension: None,
                name: "Account".into(),
                fields: vec![u64_field("holder"), u64_field("kind"), u64_field("num")],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: HOLDER,
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Cardinality {
                source: Side {
                    relation: ACCOUNT,
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([(FieldId(1), LiteralSet::One(Value::U64(1)))]),
                },
                lo: 2,
                hi: Some(2),
                target: side(HOLDER, &[0], &[]),
            },
            StatementDescriptor::Cardinality {
                source: Side {
                    relation: ACCOUNT,
                    projection: Box::new([FieldId(0)]),
                    selection: Box::new([(FieldId(1), LiteralSet::One(Value::U64(9)))]),
                },
                lo: 0,
                hi: None,
                target: side(HOLDER, &[0], &[]),
            },
        ],
    }
}

/// A fixed stream over both oracles: green commits, every violation
/// family (floor, ceiling, gap, duplicate, ranked inversion, hop
/// rewrite, and a key preemption over a would-be window violation),
/// then the repairs — verdicts and complete citation sets compared
/// whole.
#[test]
fn window_and_order_verdicts_agree_with_the_model() {
    let dir = TempDir::new("differential-marks");
    let decl = schema();
    let db = Db::create(dir.path(), decl.clone()).expect("create marks store");
    let mut naive = NaiveDb::new(&decl);
    let ops = vec![
        // Green base: one holder with one selected child, ranked steps,
        // an ordered doc.
        write(
            vec![],
            vec![
                holder(7),
                account(7, 1, 0),
                kind_rank(10, 1),
                kind_rank(20, 2),
                step(1, 1, 10),
                step(1, 2, 20),
                item(1, 1, 10),
                item(1, 2, 11),
            ],
        ),
        // Floor: a childless parent.
        write(vec![], vec![holder(8)]),
        // Ceiling: a third selected child.
        write(vec![], vec![account(7, 1, 1), account(7, 1, 2)]),
        // The gap and the duplicate.
        write(vec![], vec![item(2, 1, 20), item(2, 3, 21)]),
        write(vec![], vec![item(1, 3, 12), item(1, 3, 13)]),
        // Ranked inversion in a fresh group.
        write(vec![], vec![step(2, 1, 20), step(2, 2, 10)]),
        // The hop rewrite: an untouched group inverts.
        write(vec![kind_rank(10, 1)], vec![kind_rank(10, 3)]),
        // Key preemption: two facts, one holder key — and childless too.
        write(
            vec![],
            vec![
                (HOLDER, vec![Value::U64(9), Value::U64(0)]),
                (HOLDER, vec![Value::U64(9), Value::U64(1)]),
            ],
        ),
        // Repairs commit: the second child (inside the ceiling), the
        // renumbered doc, the demolished group.
        write(vec![], vec![account(7, 1, 1)]),
        write(vec![item(1, 2, 11)], vec![item(1, 2, 12), item(1, 3, 11)]),
        write(vec![holder(7), account(7, 1, 0), account(7, 1, 1)], vec![]),
    ];
    let summary = run(&db, &mut naive, &ops).unwrap_or_else(|divergence| {
        panic!("engine and model disagreed: {divergence:?}");
    });
    assert_eq!(
        (summary.commits, summary.aborts),
        (4, 7),
        "the stream exercises both verdicts"
    );
}

/// The empty-store pass for the extension forms (60-validation.md's
/// zero-fact duty, the marks family's share): every violating delta
/// here is judged against a store holding NOTHING — an abort applies
/// no facts, so each conviction lands on the same pristine store —
/// window floor (childless parent), order gap, non-1-based lone
/// position, and ranked inversion with the rank hops in the SAME
/// delta; then one green commit proves the stream non-vacuous.
#[test]
fn violating_deltas_against_a_zero_fact_store_agree_with_the_model() {
    let dir = TempDir::new("differential-marks-empty");
    let decl = schema();
    let db = Db::create(dir.path(), decl.clone()).expect("create empty marks store");
    let mut naive = NaiveDb::new(&decl);
    let ops = vec![
        // Window floor: a childless parent into the void.
        write(vec![], vec![holder(7)]),
        // Order gap on a store with zero items.
        write(vec![], vec![item(1, 1, 10), item(1, 3, 12)]),
        // A lone position 2 is not 1-based.
        write(vec![], vec![item(1, 2, 10)]),
        // Ranked inversion, rank facts and steps in one delta.
        write(
            vec![],
            vec![
                kind_rank(10, 1),
                kind_rank(20, 2),
                step(1, 1, 20),
                step(1, 2, 10),
            ],
        ),
        // The store is still empty; a green base commits.
        write(vec![], vec![holder(7), account(7, 1, 0)]),
    ];
    let summary = run(&db, &mut naive, &ops).unwrap_or_else(|divergence| {
        panic!("engine and model disagreed: {divergence:?}");
    });
    assert_eq!(
        (summary.commits, summary.aborts),
        (1, 4),
        "every conviction judged the zero-fact store; the green tail committed"
    );
}

/// The window-boundary subfamilies over both oracles: `n..n`
/// exactness (one under by deletion, one over by insertion), `0..*`
/// vacuity (never gates), the window over an absent parent, and the
/// delete-then-reinsert seams — a net-nothing delta re-judges its
/// touched group (`lean/Bumbledb/Txn/DeltaRestriction.lean:
/// delta_restricted_commit_sound`) and a net-nothing reinsert beside a
/// real deletion still convicts. Verdicts and complete citation sets
/// compared whole.
#[test]
fn window_boundary_and_reinsert_verdicts_agree_with_the_model() {
    let dir = TempDir::new("differential-marks-exact");
    let decl = exact_schema();
    let db = Db::create(dir.path(), decl.clone()).expect("create exactness store");
    let mut naive = NaiveDb::new(&decl);
    let ops = vec![
        // Exactly n commits at n..n.
        write(vec![], vec![holder(1), account(1, 1, 0), account(1, 1, 1)]),
        // One under: a deletion hits the floor.
        write(vec![account(1, 1, 1)], vec![]),
        // One over: an insertion hits the ceiling.
        write(vec![], vec![account(1, 1, 2)]),
        // 0..* never gates.
        write(
            vec![],
            vec![
                account(1, 9, 0),
                account(1, 9, 1),
                account(1, 9, 2),
                account(1, 9, 3),
            ],
        ),
        // A window over an absent parent is vacuous.
        write(vec![], vec![account(3, 1, 0)]),
        // The net-nothing delete-reinsert: the touched group re-judged,
        // green.
        write(vec![account(1, 1, 1)], vec![account(1, 1, 1)]),
        // A net-nothing reinsert beside a real deletion still convicts.
        write(
            vec![account(1, 1, 0), account(1, 1, 1)],
            vec![account(1, 1, 0)],
        ),
    ];
    let summary = run(&db, &mut naive, &ops).unwrap_or_else(|divergence| {
        panic!("engine and model disagreed: {divergence:?}");
    });
    assert_eq!(
        (summary.commits, summary.aborts),
        (4, 3),
        "the stream exercises both verdicts"
    );
}
