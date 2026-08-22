//! The capacity ray refusal's differential verdict (C10; the C17 slot

use bumbledb::schema::{
    Bound, FieldId, IntervalElement, RelationDescriptor, SchemaDescriptor, Side,
    StatementDescriptor, ValueType, Weight,
};
use bumbledb::{Db, RelationId, StatementId, Value};

use crate::differential::{Verdict, engine_write};
use crate::fixture::{TempDir, field};
use crate::naive::{Delta, NaiveDb, Violation};

const POOL: RelationId = RelationId(0);
const DEVICE: RelationId = RelationId(1);

fn span_type() -> ValueType {
    ValueType::Interval {
        element: IntervalElement::U64,
    }
}

fn interval(start: u64, end: u64) -> Value {
    Value::IntervalU64(bumbledb::Interval::<u64>::new(start, end).expect("nonempty"))
}

fn side(relation: RelationId, projection: &[u16]) -> Side {
    Side {
        relation,
        projection: projection.iter().map(|f| FieldId(*f)).collect(),
        selection: Box::new([]),
    }
}

fn descriptor() -> SchemaDescriptor {
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "Pool".into(),
                fields: vec![field("name", ValueType::U64), field("span", span_type())],
            },
            RelationDescriptor {
                extension: None,
                name: "Device".into(),
                fields: vec![
                    field("pool", ValueType::U64),
                    field("span", span_type()),
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
                weight: Weight::DurationOf(FieldId(1)),
                lo: 0,
                hi: Some(Bound::TargetDuration(FieldId(1))),
                source: side(DEVICE, &[0]),
            },
        ],
    }
}

const REFUSAL: StatementId = StatementId(1);

fn pool(name: u64, start: u64, end: u64) -> (RelationId, Vec<Value>) {
    (POOL, vec![Value::U64(name), interval(start, end)])
}

fn device(name: u64, start: u64, end: u64, id: u64) -> (RelationId, Vec<Value>) {
    (
        DEVICE,
        vec![Value::U64(name), interval(start, end), Value::U64(id)],
    )
}

/// One write through both twins, verdicts compared whole; returns the agreed
/// verdict so the caller pins the expected refusal identity.
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
fn a_ray_weight_refuses_with_one_agreed_verdict() {
    let descriptor = descriptor();
    let dir = TempDir::new("capacity-ray-weight");
    let db = Db::create(dir.path(), descriptor.clone())
        .expect("create engine store")
        .expect("accepted");
    let mut naive = NaiveDb::new(&descriptor);
    let refusal = Verdict::Aborted(vec![Violation::CapacityRayMeasure { statement: REFUSAL }]);

    assert_eq!(
        agreed(
            &db,
            &mut naive,
            &Delta {
                deletes: vec![],
                inserts: vec![pool(1, 0, 100), device(1, 3, u64::MAX, 0)],
            },
        ),
        refusal,
        "an inserted ray child refuses at the plan phase"
    );

    assert_eq!(
        agreed(
            &db,
            &mut naive,
            &Delta {
                deletes: vec![device(1, 3, u64::MAX, 7)],
                inserts: vec![],
            },
        ),
        Verdict::Committed,
        "the delete side never derives a weight slot"
    );

    // The plan-phase refusal preempts the judgment whole: the same

    assert_eq!(
        agreed(
            &db,
            &mut naive,
            &Delta {
                deletes: vec![],
                inserts: vec![
                    pool(1, 0, 100),
                    pool(1, 200, 300),
                    device(1, 3, u64::MAX, 0),
                ],
            },
        ),
        refusal,
        "the plan phase runs before any judgment collects"
    );

    assert_eq!(
        agreed(
            &db,
            &mut naive,
            &Delta {
                deletes: vec![],
                inserts: vec![pool(1, 0, 100), device(1, 0, 50, 0)],
            },
        ),
        Verdict::Committed
    );
}

#[test]
fn a_ray_ceiling_refuses_with_one_agreed_verdict() {
    let descriptor = descriptor();
    let dir = TempDir::new("capacity-ray-ceiling");
    let db = Db::create(dir.path(), descriptor.clone())
        .expect("create engine store")
        .expect("accepted");
    let mut naive = NaiveDb::new(&descriptor);

    assert_eq!(
        agreed(
            &db,
            &mut naive,
            &Delta {
                deletes: vec![],
                inserts: vec![pool(1, 5, u64::MAX), device(1, 0, 50, 0)],
            },
        ),
        Verdict::Aborted(vec![Violation::CapacityRayMeasure { statement: REFUSAL }]),
        "a ray-spanned parent refuses when the walk resolves its ceiling"
    );
}
