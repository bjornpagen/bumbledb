//! plumbing (`write_witnessed` → `dirty_relations` → `ImageCache::advance`):

use std::sync::Arc;

use super::*;
use crate::image::RelationImage;
use crate::ir::Value;
use crate::testutil::TempDir;
use bumbledb_theory::Interval;
use bumbledb_theory::schema::{
    FieldDescriptor, FieldId, Generation, IntervalElement, RelationDescriptor, SchemaDescriptor,
    ValueType,
};

fn wide_schema() -> SchemaDescriptor {
    let field = |name: &str, value_type: ValueType| FieldDescriptor {
        name: name.into(),
        value_type,
        generation: Generation::None,
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                extension: None,
                name: "W".into(),
                fields: vec![
                    field("u", ValueType::U64),
                    field("i", ValueType::I64),
                    field("s", ValueType::String),
                    field("b", ValueType::Bool),
                    field("small", ValueType::FixedBytes { len: 3 }),
                    field("large", ValueType::FixedBytes { len: 20 }),
                    field(
                        "during",
                        ValueType::Interval {
                            element: IntervalElement::I64,
                        },
                    ),
                    field(
                        "window",
                        ValueType::FixedInterval {
                            element: IntervalElement::U64,
                            width: 5,
                        },
                    ),
                ],
            },
            RelationDescriptor {
                extension: None,
                name: "Other".into(),
                fields: vec![field("v", ValueType::U64)],
            },
        ],
        statements: vec![],
    }
}

const W: RelationId = RelationId(0);
const OTHER: RelationId = RelationId(1);

fn wide_row(r: u64) -> Vec<Value> {
    let signed = i64::try_from(r).expect("small fixture index") - 5;
    let byte = u8::try_from(r % 251).expect("mod 251 fits");
    Vec::from([
        Value::U64(r),
        Value::I64(signed),
        Value::String(format!("row-{r}").into()),
        Value::Bool(r.is_multiple_of(2)),
        Value::FixedBytes(Box::from([byte, byte.wrapping_add(1), 0xA5])),
        Value::FixedBytes(vec![byte; 20].into_boxed_slice()),
        Value::IntervalI64(Interval::<i64>::new(signed - 7, signed + 3).expect("nonempty")),
        Value::IntervalU64(Interval::<u64>::new(r * 10, r * 10 + 5).expect("width 5")),
    ])
}

fn column_count(db: &Db<SchemaDescriptor>, rel: RelationId) -> usize {
    let types: Vec<bumbledb_theory::schema::ValueType> = db
        .schema
        .relation(rel)
        .fields()
        .iter()
        .map(|f| f.value_type)
        .collect();
    let spans = crate::image::column_spans(&types);
    spans
        .last()
        .map_or(0, |s| usize::from(s.first_column + s.width.column_count()))
}

fn assert_matches_rebuild(db: &Db<SchemaDescriptor>, rel: RelationId) -> Arc<RelationImage> {
    let txn = db.env.read_txn().expect("txn");
    let engine = db
        .cache
        .get_or_build(&txn, &db.schema, rel)
        .expect("engine image");
    let rebuilt =
        crate::image::build(&txn.catalog(), &db.schema, rel).expect("from-scratch rebuild");
    assert_eq!(engine.row_count(), rebuilt.row_count(), "row_count");
    let fields = db.schema.relation(rel).fields().len();
    for field in 0..fields {
        let field = FieldId(u16::try_from(field).expect("small fixture"));
        assert_eq!(engine.span(field), rebuilt.span(field), "span of {field:?}");
    }
    for column in 0..column_count(db, rel) {
        assert_eq!(
            engine.column(column),
            rebuilt.column(column),
            "column {column} slice"
        );
        assert_eq!(
            engine.distinct_count(column),
            rebuilt.distinct_count(column),
            "column {column} forced distinct"
        );
    }
    engine
}

#[test]
fn append_path_images_match_from_scratch_rebuilds_at_every_generation() {
    let dir = TempDir::new("db-append-differential");
    let db = Db::create(dir.path(), wide_schema())
        .expect("create")
        .expect("accepted");
    let mut next = 0u64;
    let mut insert_wide = |count: u64| {
        let from = next;
        next += count;
        db.write(|tx| {
            for r in from..from + count {
                tx.insert_dyn(W, [&wide_row(r)])?;
            }
            Ok(())
        })
        .expect("insert-only commit")
        .unwrap();
    };

    insert_wide(4);
    db.write(|tx| tx.insert_dyn(OTHER, [&[Value::U64(0)]]).map(drop))
        .expect("seed OTHER")
        .unwrap();
    assert_matches_rebuild(&db, W);
    assert_matches_rebuild(&db, OTHER);

    for round in 0..4u64 {
        insert_wide(2 + round % 3);
        let w = assert_matches_rebuild(&db, W);
        db.write(|tx| tx.insert_dyn(OTHER, [&[Value::U64(round + 1)]]).map(drop))
            .expect("touch OTHER only")
            .unwrap();
        let w_carried = assert_matches_rebuild(&db, W);
        assert!(
            Arc::ptr_eq(&w, &w_carried),
            "an untouched relation's image carries forward at zero copy"
        );
        assert_matches_rebuild(&db, OTHER);
    }

    insert_wide(1);
    insert_wide(3);
    insert_wide(2);
    assert_matches_rebuild(&db, W);

    let victim = wide_row(1);
    db.write(|tx| tx.delete_dyn(W, [&victim]).map(drop))
        .expect("delete from W")
        .unwrap();
    assert_matches_rebuild(&db, W);

    assert_matches_rebuild(&db, OTHER);
}

#[cfg(feature = "trace")]
#[test]
fn the_write_path_classifies_deletes_per_relation() {
    let dir = TempDir::new("db-append-pin");
    let db = Db::create(dir.path(), wide_schema())
        .expect("create")
        .expect("accepted");
    db.write(|tx| {
        for r in 0..3 {
            tx.insert_dyn(W, [&wide_row(r)])?;
        }
        tx.insert_dyn(OTHER, [&[Value::U64(0)]]).map(drop)
    })
    .expect("seed")
    .expect("accepted");
    let read = |rel: RelationId| {
        let txn = db.env.read_txn().expect("txn");
        db.cache.get_or_build(&txn, &db.schema, rel).expect("image");
    };
    read(W);
    read(OTHER);
    let seeded = db.cache_stats();
    assert_eq!(
        (seeded.builds, seeded.appends, seeded.carries),
        (2, 0, 0),
        "cold reads build from scratch"
    );

    let victim = wide_row(0);
    db.write(|tx| {
        tx.delete_dyn(W, [&victim])?;
        tx.insert_dyn(OTHER, [&[Value::U64(1)]]).map(drop)
    })
    .expect("mixed commit")
    .expect("accepted");
    read(W);
    let after_w = db.cache_stats();
    assert_eq!(
        (after_w.builds, after_w.appends, after_w.carries),
        (3, 0, 0),
        "a deleted-from relation rebuilds — never appends"
    );
    read(OTHER);
    let after_other = db.cache_stats();
    assert_eq!(
        (after_other.builds, after_other.appends, after_other.carries),
        (3, 1, 0),
        "the same commit's delete-free relation appends"
    );

    db.write(|tx| tx.insert_dyn(W, [&wide_row(9)]).map(drop))
        .expect("insert-only commit")
        .unwrap();
    read(W);
    read(OTHER);
    let end = db.cache_stats();
    assert_eq!(
        (end.builds, end.appends, end.carries),
        (3, 2, 1),
        "insert-only: the touched relation appends, the untouched one carries"
    );
}
