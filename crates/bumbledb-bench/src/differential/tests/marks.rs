//! The extension-form differential: engine and model replay one fixed
//! delta stream over a window theory and must
//! agree on every verdict INCLUDING the violating statement — the
//! conformance face of the enforcement stage (verdict parity is the
//! typed identity, exactly the containment differential's law).

use bumbledb::schema::{
    FieldId, LiteralSet, RelationDescriptor, RelationId, SchemaDescriptor, Side,
    StatementDescriptor,
};
use bumbledb::{Db, Value};

use crate::differential::{Op, run};
use crate::fixture::{TempDir, field, side};
use crate::naive::{Delta, NaiveDb};

const HOLDER: RelationId = RelationId(0);
const ACCOUNT: RelationId = RelationId(1);

/// The naive marks fixture's schema, verbatim (`naive/tests/judgment.rs`
/// § marks): one selected window over Holder/Account.
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
                lo: 1,
                hi: Some(2),
                target: side(HOLDER, &[0], &[]),
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

fn write(deletes: Vec<(RelationId, Vec<Value>)>, inserts: Vec<(RelationId, Vec<Value>)>) -> Op {
    Op::Write(Delta { deletes, inserts })
}

/// The window-boundary schema, verbatim from the naive marks fixture
/// (`naive/tests/judgment.rs` § window exactness): the `{2}`
/// exactness window (statement 1) and the `{0}` exclusion window
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
                hi: Some(0),
                target: side(HOLDER, &[0], &[]),
            },
        ],
    }
}

/// A fixed stream over both oracles: green commits, the window's
/// violation families (floor, ceiling, and a key preemption over a
/// would-be window violation),
/// then the repairs — verdicts and complete citation sets compared
/// whole.
#[test]
fn window_verdicts_agree_with_the_model() {
    let dir = TempDir::new("differential-marks");
    let decl = schema();
    let db = Db::create(dir.path(), decl.clone()).expect("create marks store");
    let mut naive = NaiveDb::new(&decl);
    let ops = vec![
        // Green base: one holder with one selected child.
        write(vec![], vec![holder(7), account(7, 1, 0)]),
        // Floor: a childless parent.
        write(vec![], vec![holder(8)]),
        // Ceiling: a third selected child.
        write(vec![], vec![account(7, 1, 1), account(7, 1, 2)]),
        // Key preemption: two facts, one holder key — and childless too.
        write(
            vec![],
            vec![
                (HOLDER, vec![Value::U64(9), Value::U64(0)]),
                (HOLDER, vec![Value::U64(9), Value::U64(1)]),
            ],
        ),
        // Repairs commit: the second child (inside the ceiling), the
        // demolished group.
        write(vec![], vec![account(7, 1, 1)]),
        write(vec![holder(7), account(7, 1, 0), account(7, 1, 1)], vec![]),
    ];
    let summary = run(&db, &mut naive, &ops).unwrap_or_else(|divergence| {
        panic!("engine and model disagreed: {divergence:?}");
    });
    assert_eq!(
        (summary.commits, summary.aborts),
        (3, 3),
        "the stream exercises both verdicts"
    );
}

/// The empty-store pass for the window form (60-validation.md's
/// zero-fact duty, the marks family's share): every violating delta
/// here is judged against a store holding NOTHING — an abort applies
/// no facts, so each conviction lands on the same pristine store —
/// the window floor (childless parent);
/// then one green commit proves the stream non-vacuous.
#[test]
fn violating_deltas_against_a_zero_fact_store_agree_with_the_model() {
    let dir = TempDir::new("differential-marks-empty");
    let decl = schema();
    let db = Db::create(dir.path(), decl.clone()).expect("create empty marks store");
    let mut naive = NaiveDb::new(&decl);
    let ops = vec![
        // Window floor: a childless parent into the void.
        write(vec![], vec![holder(7)]),
        // The store is still empty; a green base commits.
        write(vec![], vec![holder(7), account(7, 1, 0)]),
    ];
    let summary = run(&db, &mut naive, &ops).unwrap_or_else(|divergence| {
        panic!("engine and model disagreed: {divergence:?}");
    });
    assert_eq!(
        (summary.commits, summary.aborts),
        (1, 1),
        "every conviction judged the zero-fact store; the green tail committed"
    );
}

/// The window-boundary subfamilies over both oracles: `{n}`
/// exactness (one under by deletion, one over by insertion), the `{0}`
/// exclusion (its first member convicts; out-of-σ children never
/// count), the window over an absent parent, and the
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
        // The {0} exclusion convicts its first member.
        write(vec![], vec![account(1, 9, 0)]),
        // The {0} exclusion admits everything outside sigma.
        write(vec![], vec![account(1, 5, 0), account(1, 6, 1)]),
        // A window over an absent parent constrains nothing.
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
        (4, 4),
        "the stream exercises both verdicts"
    );
}
