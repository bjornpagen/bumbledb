//! The copy-on-append differential referee, run through the REAL write
//! plumbing (`write_witnessed` → `dirty_relations` → `ImageCache::advance`):
//! an appended image must be indistinguishable from a from-scratch
//! rebuild at the column granularity — `row_count`, every field's span,
//! every column's full slice byte-for-byte, every forced distinct count —
//! across every field shape, over a chain of insert-only commits with
//! interleaved untouched relations, ending in the delete that forces the
//! rebuild fallback (the retired I1 copy-on-append packet (git history)).

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

/// W spans every field shape the image layer decodes — u64, i64, str,
/// bool, bytes<3> (one padded word), bytes<20> (three word columns),
/// interval<i64> (two stored words), interval<u64, 5> (one stored word,
/// derived end) — and OTHER is the interleaved untouched relation.
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
                            width: None,
                        },
                    ),
                    field(
                        "window",
                        ValueType::Interval {
                            element: IntervalElement::U64,
                            width: Some(5),
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

/// One deterministic W row per index — every shape carries the index so
/// a misplaced tail row diverges in every column.
fn wide_row(r: u64) -> Vec<Value> {
    let signed = i64::try_from(r).expect("small fixture index") - 5;
    let byte = u8::try_from(r % 251).expect("mod 251 fits");
    Vec::from([
        Value::U64(r),
        Value::I64(signed),
        Value::String(format!("row-{r}").into_bytes().into_boxed_slice()),
        Value::Bool(r.is_multiple_of(2)),
        Value::FixedBytes(Box::from([byte, byte.wrapping_add(1), 0xA5])),
        Value::FixedBytes(vec![byte; 20].into_boxed_slice()),
        Value::IntervalI64(Interval::<i64>::new(signed - 7, signed + 3).expect("nonempty")),
        Value::IntervalU64(Interval::<u64>::new(r * 10, r * 10 + 5).expect("width 5")),
    ])
}

/// Total image columns of `rel`, derived exactly as the build derives
/// them (the field→column map).
fn column_count(db: &Db<SchemaDescriptor>, rel: RelationId) -> usize {
    let types: Vec<bumbledb_theory::TypeDesc> = db
        .schema
        .relation(rel)
        .fields()
        .iter()
        .map(|f| f.value_type.type_desc())
        .collect();
    let spans = crate::image::column_spans(&types);
    spans
        .last()
        .map_or(0, |s| usize::from(s.first_column + s.width.column_count()))
}

/// The referee clause: the engine's image of `rel` at this snapshot
/// (whatever arm produced it) against a from-scratch [`crate::image::build`]
/// in the SAME read transaction — `row_count`, per-field spans, every
/// column slice byte-for-byte, every forced distinct. Returns the
/// engine's image so callers can also assert Arc identity.
fn assert_matches_rebuild(db: &Db<SchemaDescriptor>, rel: RelationId) -> Arc<RelationImage> {
    let txn = db.env.read_txn().expect("txn");
    let engine = db
        .cache
        .get_or_build(&txn, &db.schema, rel)
        .expect("engine image");
    let rebuilt = crate::image::build(&txn, &db.schema, rel).expect("from-scratch rebuild");
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
            engine.cardinality(column),
            rebuilt.cardinality(column),
            "column {column} forced distinct"
        );
    }
    engine
}

/// The differential referee over a generated commit chain: full build,
/// k insert-only appends with reads between, untouched-relation
/// carry-forward (Arc identity), chained commits without reads, and the
/// delete that forces the rebuild fallback — the appended image matches
/// a from-scratch rebuild at every generation.
#[test]
fn append_path_images_match_from_scratch_rebuilds_at_every_generation() {
    let dir = TempDir::new("db-append-differential");
    let db = Db::create(dir.path(), wide_schema()).expect("create");
    let mut next = 0u64;
    let mut insert_wide = |count: u64| {
        let from = next;
        next += count;
        db.write(|tx| {
            for r in from..from + count {
                tx.insert_dyn(W, &wide_row(r))?;
            }
            Ok(())
        })
        .expect("insert-only commit");
    };

    // Generation 1: the from-scratch build (no base exists yet).
    insert_wide(4);
    db.write(|tx| tx.insert_dyn(OTHER, &[Value::U64(0)]).map(drop))
        .expect("seed OTHER");
    assert_matches_rebuild(&db, W);
    assert_matches_rebuild(&db, OTHER);

    // The chain: insert-only commits into W with reads between, and an
    // untouched-W commit interleaved every other round — every
    // generation's image must match its rebuild (append arms), and the
    // untouched relation must carry the same Arc forward.
    for round in 0..4u64 {
        insert_wide(2 + round % 3);
        let w = assert_matches_rebuild(&db, W);
        db.write(|tx| tx.insert_dyn(OTHER, &[Value::U64(round + 1)]).map(drop))
            .expect("touch OTHER only");
        let w_carried = assert_matches_rebuild(&db, W);
        assert!(
            Arc::ptr_eq(&w, &w_carried),
            "an untouched relation's image carries forward at zero copy"
        );
        assert_matches_rebuild(&db, OTHER);
    }

    // Chained insert-only commits with NO reads between: the surviving
    // base absorbs the whole chain in one append.
    insert_wide(1);
    insert_wide(3);
    insert_wide(2);
    assert_matches_rebuild(&db, W);

    // The delete fork: a commit that deletes from W forces the rebuild
    // fallback for W — and the rebuilt image still matches the referee.
    let victim = wide_row(1);
    db.write(|tx| tx.delete_dyn(W, &victim).map(drop))
        .expect("delete from W");
    assert_matches_rebuild(&db, W);
    // OTHER was delete-free throughout: still carried, still identical.
    assert_matches_rebuild(&db, OTHER);
}

/// The delete-fallback pin through the REAL plumbing (feature `trace`):
/// `write_witnessed` classifies the delta per relation and `advance`
/// takes the right arm for each — a delta with one delete for W
/// increments `builds` (never `appends`) on W's next read; the same
/// commit's insert into OTHER leaves OTHER appendable; an insert-only
/// commit lands in `appends`/`carries`. An appended-across-a-delete bug
/// cannot exist silently.
#[cfg(feature = "trace")]
#[test]
fn the_write_path_classifies_deletes_per_relation() {
    let dir = TempDir::new("db-append-pin");
    let db = Db::create(dir.path(), wide_schema()).expect("create");
    db.write(|tx| {
        for r in 0..3 {
            tx.insert_dyn(W, &wide_row(r))?;
        }
        tx.insert_dyn(OTHER, &[Value::U64(0)]).map(drop)
    })
    .expect("seed");
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

    // One mixed delta: delete from W, insert into OTHER. Per-relation
    // arms: W rebuilds, OTHER appends.
    let victim = wide_row(0);
    db.write(|tx| {
        tx.delete_dyn(W, &victim)?;
        tx.insert_dyn(OTHER, &[Value::U64(1)]).map(drop)
    })
    .expect("mixed commit");
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

    // An insert-only commit into W: W appends, OTHER carries.
    db.write(|tx| tx.insert_dyn(W, &wide_row(9)).map(drop))
        .expect("insert-only commit");
    read(W);
    read(OTHER);
    let end = db.cache_stats();
    assert_eq!(
        (end.builds, end.appends, end.carries),
        (3, 2, 1),
        "insert-only: the touched relation appends, the untouched one carries"
    );
}
