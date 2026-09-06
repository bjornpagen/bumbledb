use super::*;
use crate::schema::{
    FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValidateDescriptor, ValueType,
};
use crate::work::{ExecutionPolicy, Resource};
use std::time::Duration;

fn schema() -> Schema {
    SchemaDescriptor {
        relations: vec![RelationDescriptor {
            name: "Number".into(),
            fields: vec![FieldDescriptor {
                name: "value".into(),
                value_type: ValueType::U64,
            }],
            extension: None,
        }],
        statements: vec![],
    }
    .validate()
    .unwrap()
}
fn work() -> WorkContext {
    ExecutionPolicy {
        input_bytes: 1_000_000,
        working_bytes: 1_000_000,
        scratch_bytes: 0,
        result_bytes: 0,
        rows: 10000,
        work_units: 1_000_000,
        timeout: Duration::from_secs(60),
    }
    .start()
    .unwrap()
}

fn permutations(indices: &mut [usize], at: usize, f: &mut impl FnMut(&[usize])) {
    if at == indices.len() {
        f(indices);
        return;
    }
    for other in at..indices.len() {
        indices.swap(at, other);
        permutations(indices, at + 1, f);
        indices.swap(at, other);
    }
}

#[test]
fn normalized_change_bytes_ignore_order_and_repetition_with_add_winning() {
    let schema = schema();
    let mut expected = None;
    // All 720 permutations, with repeated adds/removes and a removal-only row.
    let operations = [
        (true, 1),
        (false, 1),
        (true, 1),
        (false, 2),
        (true, 3),
        (false, 3),
    ];
    permutations(&mut [0, 1, 2, 3, 4, 5], 0, &mut |order| {
        let ctx = work();
        let mut builder = ChangeSet::builder(&schema, ctx.clone());
        for &i in order {
            let (add, value) = operations[i];
            if add {
                builder.insert(RelationId(0), &[Value::U64(value)]).unwrap();
            } else {
                builder.delete(RelationId(0), &[Value::U64(value)]).unwrap();
            }
        }
        let changes = builder.finish().unwrap();
        assert_eq!(changes.len(), 3);
        assert_eq!(ctx.used(Resource::Rows), 6);
        let bytes = changes.as_bytes();
        assert_eq!(&bytes[..10], b"BDBCSET\0\0\x01");
        assert_eq!(bytes[HEADER], 1);
        assert_eq!(bytes[HEADER + 24], 0);
        assert_eq!(bytes[HEADER + 48], 1);
        if let Some(expected) = &expected {
            assert_eq!(bytes, expected);
        } else {
            expected = Some(bytes.to_vec());
        }
        assert_eq!(
            ChangeSet::parse(&schema, bytes, &work())
                .unwrap()
                .as_bytes(),
            bytes
        );
        let retained = changes.clone();
        drop(changes);
        assert!(ctx.used(Resource::WorkingBytes) > 0);
        drop(retained);
        assert_eq!(ctx.used(Resource::WorkingBytes), 0);
    });
}

#[test]
fn strict_parser_rejects_truncation_duplicates_reordering_and_foreign_schema() {
    let schema = schema();
    let mut builder = ChangeSet::builder(&schema, work());
    builder.insert(RelationId(0), &[Value::U64(1)]).unwrap();
    builder.insert(RelationId(0), &[Value::U64(2)]).unwrap();
    let changes = builder.finish().unwrap();
    for end in 0..changes.as_bytes().len() {
        assert!(ChangeSet::parse(&schema, &changes.as_bytes()[..end], &work()).is_err());
    }
    let mut bytes = changes.as_bytes().to_vec();
    bytes[10] ^= 1;
    assert!(matches!(
        ChangeSet::parse(&schema, &bytes, &work()),
        Err(ChangeError::WrongSchema)
    ));
    let mut bytes = changes.as_bytes().to_vec();
    bytes[HEADER..].rotate_left(24);
    assert!(matches!(
        ChangeSet::parse(&schema, &bytes, &work()),
        Err(ChangeError::NonCanonicalOrder)
    ));
    let mut bytes = changes.as_bytes().to_vec();
    bytes.copy_within(HEADER..HEADER + 24, HEADER + 24);
    assert!(matches!(
        ChangeSet::parse(&schema, &bytes, &work()),
        Err(ChangeError::NonCanonicalOrder)
    ));
    let mut bytes = changes.as_bytes().to_vec();
    bytes.push(0);
    assert!(matches!(
        ChangeSet::parse(&schema, &bytes, &work()),
        Err(ChangeError::TrailingBytes)
    ));
}

#[test]
fn ordered_pending_seals_identical_wire_with_one_retained_payload_charge() {
    let schema = schema();
    let mut builder = ChangeSet::builder(&schema, work());
    builder.insert(RelationId(0), &[Value::U64(9)]).unwrap();
    builder.delete(RelationId(0), &[Value::U64(3)]).unwrap();
    builder.insert(RelationId(0), &[Value::U64(5)]).unwrap();
    let expected = builder.finish().unwrap();
    let pending: BTreeMap<_, _> = expected
        .records()
        .map(|record| ((record.relation, Box::from(record.row)), record.kind))
        .collect();
    let ctx = work();
    let sealed = ChangeSet::from_ordered_rows(&schema, &pending, &ctx).unwrap();
    assert_eq!(sealed.as_bytes(), expected.as_bytes());
    assert_eq!(
        ChangeSet::parse(&schema, sealed.as_bytes(), &work())
            .unwrap()
            .as_bytes(),
        expected.as_bytes()
    );
    assert_eq!(
        ctx.used(Resource::WorkingBytes),
        (sealed.as_bytes().len() + std::mem::size_of::<Payload>()) as u64
    );
    let retained = sealed.clone();
    drop(sealed);
    assert!(ctx.used(Resource::WorkingBytes) > 0);
    drop(retained);
    assert_eq!(ctx.used(Resource::WorkingBytes), 0);
}

#[test]
fn ordered_pending_refuses_bad_rows_and_exhaustion_without_retained_memory() {
    let schema = schema();
    let canonical = CanonicalRow::encode(
        schema.relation(RelationId(0)).fields(),
        &[Value::U64(1)],
        &work(),
    )
    .unwrap();
    let mut pending = BTreeMap::from([(
        (RelationId(0), Box::<[u8]>::from(canonical.as_bytes())),
        ChangeKind::Add,
    )]);
    let cancelled = work();
    cancelled.cancel();
    assert!(matches!(
        ChangeSet::from_ordered_rows(&schema, &pending, &cancelled),
        Err(ChangeError::Work(WorkError::Cancelled))
    ));
    let limited = work();
    let held = limited
        .reserve(ByteKind::Working, limited.limit(Resource::WorkingBytes))
        .unwrap();
    assert!(matches!(
        ChangeSet::from_ordered_rows(&schema, &pending, &limited),
        Err(ChangeError::Work(WorkError::Exhausted {
            resource: Resource::WorkingBytes,
            ..
        }))
    ));
    drop(held);
    assert_eq!(limited.used(Resource::WorkingBytes), 0);
    pending.insert(
        (RelationId(0), Box::from(&b"malformed"[..])),
        ChangeKind::Add,
    );
    let ctx = work();
    assert!(matches!(
        ChangeSet::from_ordered_rows(&schema, &pending, &ctx),
        Err(ChangeError::Row(_))
    ));
    assert_eq!(ctx.used(Resource::WorkingBytes), 0);
}

#[test]
fn failed_ingestion_spends_draft_and_releases_owned_memory() {
    let schema = schema();
    let ctx = work();
    let mut builder = ChangeSet::builder(&schema, ctx.clone());
    builder.insert(RelationId(0), &[Value::U64(1)]).unwrap();
    assert!(ctx.used(Resource::WorkingBytes) > 0);
    let failure = builder
        .insert(RelationId(0), &[Value::Bool(false)])
        .unwrap_err();
    assert_eq!(ctx.used(Resource::WorkingBytes), 0);
    assert_eq!(
        builder.insert(RelationId(0), &[Value::U64(2)]),
        Err(failure)
    );
    assert!(matches!(builder.finish(),Err(error) if error==failure));
}

#[test]
fn sorting_cancellation_returns_no_payload_or_live_reservation() {
    let schema = schema();
    let ctx = work();
    let mut builder = ChangeSet::builder(&schema, ctx.clone());
    for value in (0..100).rev() {
        builder.insert(RelationId(0), &[Value::U64(value)]).unwrap();
    }
    ctx.cancel();
    assert!(matches!(
        builder.finish(),
        Err(ChangeError::Work(WorkError::Cancelled))
    ));
    assert_eq!(ctx.used(Resource::WorkingBytes), 0);
}
