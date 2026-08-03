//! The capacity witness tie-break differential (C14's measure parity,
//! sharpened): when SEVERAL parents violate one capacity statement in
//! one delta, both twins must report the SAME parent's measure. The
//! engine walks touched parents in ascending ENCODED determinant-image
//! order — the target KEY's field order, `str` positions as intern
//! words — so a permuted key (key order ≠ statement projection order)
//! and a `str`-typed key (intern allocation order ≠ lexicographic
//! order) each pick a different winner than the naive decoded
//! projection-order compare used to.

use bumbledb::schema::{
    Bound, FieldId, RelationDescriptor, SchemaDescriptor, Side, StatementDescriptor, ValueType,
    Weight,
};
use bumbledb::{Db, RelationId, StatementId, Value};

use crate::differential::{Verdict, engine_write};
use crate::fixture::{TempDir, field};
use crate::naive::{Delta, NaiveDb, Violation};

const POOL: RelationId = RelationId(0);
const DEVICE: RelationId = RelationId(1);

fn side(relation: RelationId, projection: &[u16]) -> Side {
    Side {
        relation,
        projection: projection.iter().map(|f| FieldId(*f)).collect(),
        selection: Box::new([]),
    }
}

/// One write through both twins, verdicts compared whole; returns the
/// agreed verdict so the caller can pin the expected witness too (the
/// twins agreeing on the WRONG parent would still be a bug).
fn agreed(db: &Db<SchemaDescriptor>, naive: &mut NaiveDb, delta: &Delta) -> Verdict {
    let engine = engine_write(db, delta);
    let model = match naive.apply(delta) {
        Ok(()) => Verdict::Committed,
        Err(violations) => Verdict::Aborted(violations),
    };
    assert_eq!(engine, model, "the twins must agree");
    engine
}

/// Pool(a, b) keyed (b, a) — the PERMUTED key — with
/// `Pool(a, b) <=[1]{0..1} Device(a, b)`. Two parents violate with
/// different totals: statement-projection order (a, b) ranks
/// (1, 9) < (2, 3), determinant order (b, a) ranks (3, 2) < (9, 1) —
/// the engine walks determinant order, so the witnessed measure is
/// parent (2, 3)'s.
#[test]
fn the_witness_walks_the_permuted_key_order() {
    let descriptor = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Pool".into(),
                fields: vec![field("a", ValueType::U64), field("b", ValueType::U64)],
            },
            RelationDescriptor {
                extension: None,
                name: "Device".into(),
                fields: vec![
                    field("a", ValueType::U64),
                    field("b", ValueType::U64),
                    field("id", ValueType::U64),
                ],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: POOL,
                // The permutation under test: key order (b, a).
                projection: Box::new([FieldId(1), FieldId(0)]),
            },
            StatementDescriptor::Capacity {
                target: side(POOL, &[0, 1]),
                weight: Weight::Unit,
                lo: 0,
                hi: Some(Bound::Lit(1)),
                source: side(DEVICE, &[0, 1]),
            },
        ],
    };
    let dir = TempDir::new("capacity-witness-permuted");
    let db = Db::create(dir.path(), descriptor.clone()).expect("create engine store");
    let mut naive = NaiveDb::new(&descriptor);

    let device = |a: u64, b: u64, id: u64| {
        (
            DEVICE,
            vec![Value::U64(a), Value::U64(b), Value::U64(id)],
        )
    };
    let delta = Delta {
        deletes: vec![],
        inserts: vec![
            (POOL, vec![Value::U64(1), Value::U64(9)]),
            (POOL, vec![Value::U64(2), Value::U64(3)]),
            // Parent (1, 9): 2 devices; parent (2, 3): 3 devices.
            device(1, 9, 0),
            device(1, 9, 1),
            device(2, 3, 2),
            device(2, 3, 3),
            device(2, 3, 4),
        ],
    };
    let verdict = agreed(&db, &mut naive, &delta);
    assert_eq!(
        verdict,
        Verdict::Aborted(vec![Violation::Capacity {
            statement: StatementId(1),
            measure: 3,
        }]),
        "determinant order (b, a) reaches parent (2, 3) first"
    );
}

/// Pool(name str) keyed (name) with `Pool <=[1]{0..1} Device(pool)`.
/// "zebra" interns before "apple" (insert order), so the engine's
/// encoded walk reaches zebra's group first — lexicographic order says
/// apple. Runs the delta twice: once against committed intern ranks,
/// once with the mints pending inside the very delta the judgment
/// rejects.
#[test]
fn the_witness_walks_intern_order_not_lexicographic_order() {
    let descriptor = SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Pool".into(),
                fields: vec![field("name", ValueType::String)],
            },
            RelationDescriptor {
                extension: None,
                name: "Device".into(),
                fields: vec![
                    field("pool", ValueType::String),
                    field("id", ValueType::U64),
                ],
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: POOL,
                projection: Box::new([FieldId(0)]),
            },
            StatementDescriptor::Capacity {
                target: side(POOL, &[0]),
                weight: Weight::Unit,
                lo: 0,
                hi: Some(Bound::Lit(1)),
                source: side(DEVICE, &[0]),
            },
        ],
    };
    let pool = |name: &str| (POOL, vec![Value::String(name.as_bytes().into())]);
    let device = |name: &str, id: u64| {
        (
            DEVICE,
            vec![Value::String(name.as_bytes().into()), Value::U64(id)],
        )
    };
    let expected = Verdict::Aborted(vec![Violation::Capacity {
        statement: StatementId(1),
        // zebra's group (2 devices), NOT apple's (3) — intern order.
        measure: 2,
    }]);

    // Committed ranks: the pools land first (zebra minted before
    // apple), the violating device delta judges against the committed
    // dictionary.
    let dir = TempDir::new("capacity-witness-interned");
    let db = Db::create(dir.path(), descriptor.clone()).expect("create engine store");
    let mut naive = NaiveDb::new(&descriptor);
    let seed = Delta {
        deletes: vec![],
        inserts: vec![pool("zebra"), pool("apple")],
    };
    assert_eq!(agreed(&db, &mut naive, &seed), Verdict::Committed);
    let overflow = Delta {
        deletes: vec![],
        inserts: vec![
            device("zebra", 0),
            device("zebra", 1),
            device("apple", 2),
            device("apple", 3),
            device("apple", 4),
        ],
    };
    assert_eq!(agreed(&db, &mut naive, &overflow), expected);

    // Pending ranks: one rejected delta mints both names provisionally
    // (insert order: zebra first) — the judgment's ordering must see
    // those ranks even though the ids die with the abort.
    let dir = TempDir::new("capacity-witness-pending");
    let db = Db::create(dir.path(), descriptor.clone()).expect("create engine store");
    let mut naive = NaiveDb::new(&descriptor);
    let one_shot = Delta {
        deletes: vec![],
        inserts: vec![
            pool("zebra"),
            pool("apple"),
            device("zebra", 0),
            device("zebra", 1),
            device("apple", 2),
            device("apple", 3),
            device("apple", 4),
        ],
    };
    assert_eq!(agreed(&db, &mut naive, &one_shot), expected);
}
