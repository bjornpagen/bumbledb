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

fn agreed(db: &Db<SchemaDescriptor>, naive: &mut NaiveDb, delta: &Delta) -> Verdict {
    let engine = engine_write(db, delta);
    let model = match naive.apply(delta) {
        Ok(()) => Verdict::Committed,
        Err(violations) => Verdict::Aborted(violations),
    };
    assert_eq!(engine, model, "the twins must agree");
    engine
}

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
    let db = Db::create(dir.path(), descriptor.clone())
        .expect("create engine store")
        .expect("accepted");
    let mut naive = NaiveDb::new(&descriptor);

    let device =
        |a: u64, b: u64, id: u64| (DEVICE, vec![Value::U64(a), Value::U64(b), Value::U64(id)]);
    let delta = Delta {
        deletes: vec![],
        inserts: vec![
            (POOL, vec![Value::U64(1), Value::U64(9)]),
            (POOL, vec![Value::U64(2), Value::U64(3)]),
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
    let pool = |name: &str| (POOL, vec![Value::String(name.into())]);
    let device = |name: &str, id: u64| (DEVICE, vec![Value::String(name.into()), Value::U64(id)]);
    let expected = Verdict::Aborted(vec![Violation::Capacity {
        statement: StatementId(1),

        measure: 2,
    }]);

    let dir = TempDir::new("capacity-witness-interned");
    let db = Db::create(dir.path(), descriptor.clone())
        .expect("create engine store")
        .expect("accepted");
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

    let dir = TempDir::new("capacity-witness-pending");
    let db = Db::create(dir.path(), descriptor.clone())
        .expect("create engine store")
        .expect("accepted");
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
