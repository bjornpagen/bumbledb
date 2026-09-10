use super::*;
use crate::schema::{
    FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValidateDescriptor, ValueType,
};
use crate::work::WorkContext;

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
    WorkContext::new()
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
        assert!(Arc::ptr_eq(&changes.0, &retained.0));
        drop(changes);
        drop(retained);
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
fn ordered_records_seal_identical_wire_and_share_the_payload() {
    let schema = schema();
    let mut builder = ChangeSet::builder(&schema, work());
    builder.insert(RelationId(0), &[Value::U64(9)]).unwrap();
    builder.delete(RelationId(0), &[Value::U64(3)]).unwrap();
    builder.insert(RelationId(0), &[Value::U64(5)]).unwrap();
    let expected = builder.finish().unwrap();
    let ctx = work();
    let sealed = ChangeSet::from_ordered_records(&schema, expected.records(), &ctx).unwrap();
    assert_eq!(sealed.as_bytes(), expected.as_bytes());
    assert_eq!(
        ChangeSet::parse(&schema, sealed.as_bytes(), &work())
            .unwrap()
            .as_bytes(),
        expected.as_bytes()
    );
    let retained = sealed.clone();
    assert!(Arc::ptr_eq(&sealed.0, &retained.0));
    drop(sealed);
    assert_eq!(retained.as_bytes(), expected.as_bytes());
    drop(retained);
}

#[test]
fn ordered_records_refuse_bad_rows_and_cancellation() {
    let schema = schema();
    let canonical = CanonicalRow::encode(
        schema.relation(RelationId(0)).fields(),
        &[Value::U64(1)],
        &work(),
    )
    .unwrap();
    let mut records = vec![ChangeRef {
        relation: RelationId(0),
        row: canonical.as_bytes(),
        kind: ChangeKind::Add,
    }];
    let cancelled = work();
    cancelled.cancel();
    assert!(matches!(
        ChangeSet::from_ordered_records(&schema, records.iter().copied(), &cancelled),
        Err(ChangeError::Work(WorkError::Cancelled))
    ));
    records.push(ChangeRef {
        relation: RelationId(0),
        row: b"malformed",
        kind: ChangeKind::Add,
    });
    let ctx = work();
    assert!(matches!(
        ChangeSet::from_ordered_records(&schema, records.iter().copied(), &ctx),
        Err(ChangeError::Row(_))
    ));
}

#[test]
fn failed_ingestion_spends_draft_and_releases_owned_memory() {
    let schema = schema();
    let ctx = work();
    let mut builder = ChangeSet::builder(&schema, ctx.clone());
    builder.insert(RelationId(0), &[Value::U64(1)]).unwrap();
    let failure = builder
        .insert(RelationId(0), &[Value::Bool(false)])
        .unwrap_err();
    assert_eq!(
        builder.insert(RelationId(0), &[Value::U64(2)]),
        Err(failure)
    );
    assert!(matches!(builder.finish(),Err(error) if error==failure));
}

#[test]
fn sorting_cancellation_returns_no_payload() {
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
}

fn command(schema: &Schema, actions: &[(bool, u64)]) -> ChangeSet {
    let mut draft = ChangeSet::builder(schema, work());
    for &(add, value) in actions {
        let row = [Value::U64(value)];
        if add {
            draft.insert(RelationId(0), &row).unwrap();
        } else {
            draft.delete(RelationId(0), &row).unwrap();
        }
    }
    draft.finish().unwrap()
}

#[test]
fn composition_matches_one_command_and_is_commutative_associative_idempotent() {
    let schema = schema();
    let actions = [
        (true, 1),
        (false, 1),
        (false, 2),
        (true, 3),
        (true, 1),
        (false, 3),
    ];
    let expected = command(&schema, &actions);
    assert_eq!(
        expected.counts(),
        ChangeCounts {
            added: 2,
            removed: 1
        }
    );
    let empty = command(&schema, &[]);
    permutations(&mut [0, 1, 2, 3, 4, 5], 0, &mut |order| {
        let a = command(&schema, &[actions[order[0]], actions[order[1]]]);
        let b = command(&schema, &[actions[order[2]], actions[order[3]]]);
        let c = command(&schema, &[actions[order[4]], actions[order[5]]]);
        let ab = a.compose(&b, &work()).unwrap();
        assert_eq!(ab.as_bytes(), b.compose(&a, &work()).unwrap().as_bytes());
        let left = ab.compose(&c, &work()).unwrap();
        let right = a
            .compose(&b.compose(&c, &work()).unwrap(), &work())
            .unwrap();
        assert_eq!(left.as_bytes(), right.as_bytes());
        assert_eq!(left.as_bytes(), expected.as_bytes());
        assert_eq!(left.counts(), expected.counts());
        let parsed = ChangeSet::parse(&schema, left.as_bytes(), &work()).unwrap();
        assert_eq!(parsed.counts(), left.counts());
        assert_eq!(
            left.compose(&parsed, &work()).unwrap().as_bytes(),
            left.as_bytes()
        );
        assert!(Arc::ptr_eq(
            &left.0,
            &left.compose(&left, &work()).unwrap().0
        ));
        assert!(Arc::ptr_eq(
            &left.0,
            &left.compose(&empty, &work()).unwrap().0
        ));
        assert!(Arc::ptr_eq(
            &left.0,
            &empty.compose(&left, &work()).unwrap().0
        ));
    });
}

#[test]
fn composition_refuses_foreign_schema_and_cancelled_identity_fast_paths() {
    let schema = schema();
    let a = command(&schema, &[(true, 1)]);
    let other = SchemaDescriptor {
        relations: vec![],
        statements: vec![],
    }
    .validate()
    .unwrap();
    let empty = command(&other, &[]);
    assert!(matches!(
        a.compose(&empty, &work()),
        Err(ChangeError::WrongSchema)
    ));
    let cancelled = work();
    cancelled.cancel();
    assert!(matches!(
        a.compose(&a, &cancelled),
        Err(ChangeError::Work(WorkError::Cancelled))
    ));
    assert!(matches!(
        a.compose(&command(&schema, &[]), &cancelled),
        Err(ChangeError::Work(WorkError::Cancelled))
    ));
}

#[test]
fn owned_record_cursor_shares_bytes_and_clones_an_independent_position() {
    let schema = schema();
    let actions: Vec<_> = (0..1025).map(|i| (i % 2 == 0, i)).collect();
    let changes = command(&schema, &actions);
    let expected: Vec<_> = changes
        .records()
        .map(|r| (r.kind, r.row.to_vec()))
        .collect();
    let mut cursor = changes.cursor();
    assert!(Arc::ptr_eq(&cursor.changes.0, &changes.0));
    drop(changes);
    for (index, (kind, row)) in expected.iter().enumerate() {
        let mut preview = cursor.clone();
        let next = preview.next_record().unwrap();
        assert_eq!((next.kind, next.row), (*kind, row.as_slice()));
        if index % 256 == 0 {
            let retry = cursor.next_record().unwrap();
            assert_eq!((retry.kind, retry.row), (*kind, row.as_slice()));
        }
        cursor = preview;
    }
    assert!(cursor.next_record().is_none());
    assert!(cursor.next_record().is_none());
}

#[test]
fn owned_parsing_reuses_allocation_and_has_identical_refusals() {
    let schema = schema();
    let changes = command(&schema, &[(true, 1), (false, 2)]);
    let bytes = changes.as_bytes().to_vec();
    let pointer = bytes.as_ptr();
    let parsed = ChangeSet::from_bytes(&schema, bytes, &work()).unwrap();
    assert_eq!(parsed.as_bytes().as_ptr(), pointer);
    assert_eq!(parsed.counts(), changes.counts());
    let mut corruptions: Vec<_> = (0..changes.as_bytes().len())
        .map(|end| changes.as_bytes()[..end].to_vec())
        .collect();
    for offset in [0, 9, 10, HEADER, HEADER + 1, HEADER + 24] {
        let mut bytes = changes.as_bytes().to_vec();
        bytes[offset] ^= 255;
        corruptions.push(bytes);
    }
    for bytes in corruptions {
        let copied = ChangeSet::parse(&schema, &bytes, &work()).unwrap_err();
        assert_eq!(
            ChangeSet::from_bytes(&schema, bytes, &work()).unwrap_err(),
            copied
        );
    }
    let stopped = work();
    stopped.cancel();
    assert!(matches!(
        ChangeSet::from_bytes(&schema, parsed.as_bytes().to_vec(), &stopped),
        Err(ChangeError::Work(WorkError::Cancelled))
    ));
}
