//! Partial-image coverage: real reverse index, source visit bounds, rotating
//! selections, whole-image sharing, pinned versions, text generations and
//! cancellation. The heap and forced cursor paths are independent oracles.
use super::*;
use bumbledb_theory::schema::{Side, StatementDescriptor};

fn indexed_store() -> StoreFix {
    let mut desc = descriptor();
    desc.relations.push(RelationDescriptor {
        extension: None,
        name: "Account".into(),
        fields: vec![FieldDescriptor {
            name: "id".into(),
            value_type: ValueType::U64,
        }],
    });
    desc.statements.push(StatementDescriptor::Functionality {
        relation: RelationId(1),
        projection: Box::new([FieldId(0)]),
    });
    desc.statements.push(StatementDescriptor::Containment {
        source: Side {
            relation: POSTING,
            projection: Box::new([FieldId(1)]),
            selection: Box::new([]),
        },
        target: Side {
            relation: RelationId(1),
            projection: Box::new([FieldId(0)]),
            selection: Box::new([]),
        },
    });
    let fix = StoreFix::store("selected-image", desc);
    fix.insert_dyn(
        RelationId(1),
        &(0..8).map(|i| vec![Value::U64(i)]).collect::<Vec<_>>(),
    );
    fix.insert_dyn(
        POSTING,
        &posting_rows(
            &(0..256u64)
                .map(|i| (i, i % 8, "text", i.cast_signed()))
                .collect::<Vec<_>>(),
        ),
    );
    fix
}

#[test]
fn indexed_image_rotation_is_bucket_shaped_and_never_shared_as_full() {
    let fix = indexed_store();
    let mut prepared = fix.prepare(&by_account_query()).unwrap();
    let mut cursor = fix.prepare(&by_account_query()).unwrap();
    cursor.force_cursor_fallback(true);
    for account in [0, 1, 2, 99, 0, 99, 1, 0] {
        let params = [BindValue::U64(account), BindValue::I64(-1)];
        let actual = fix.execute(&mut prepared, &params).unwrap();
        assert_eq!(
            answers_of(&actual),
            answers_of(&fix.execute(&mut cursor, &params).unwrap())
        );
        assert!(
            matches!(prepared.last_visits, 0 | 32),
            "one bucket body walk, or a memo hit: {}",
            prepared.last_visits
        );
        assert_eq!(
            prepared.cache.image_count(),
            0,
            "partial images cannot impersonate whole relations"
        );
        let [PreparedRule::FreeJoin(rule)] = prepared.pipeline.main_rules() else {
            panic!("Free Join")
        };
        let Binding::Bound(bound) = &rule.memo.occs[0].active else {
            panic!("bound")
        };
        assert_eq!(
            bound.selections.as_deref(),
            Some([vec![account].into()].as_slice())
        );
    }
    // A separate broad query must see ALL rows, and its full image then
    // becomes reusable for any selection without more source visits.
    let mut broad = fix.prepare(&by_memo_query()).unwrap();
    assert_eq!(
        fix.execute(&mut broad, &memo_param("text")).unwrap().len(),
        256
    );
    let mut fresh = fix.prepare(&by_account_query()).unwrap();
    assert_eq!(
        fix.execute(&mut fresh, &[BindValue::U64(4), BindValue::I64(-1)])
            .unwrap()
            .len(),
        32
    );
    assert_eq!(fresh.last_visits, 0);
    let [PreparedRule::FreeJoin(rule)] = fresh.pipeline.main_rules() else {
        panic!("Free Join")
    };
    let Binding::Bound(bound) = &rule.memo.occs[0].active else {
        panic!("bound")
    };
    assert!(bound.selections.is_none());
}

#[test]
fn selection_diversity_promotes_once_instead_of_thrashing_the_bounded_memo() {
    let fix = indexed_store();
    let mut prepared = fix.prepare(&by_account_query()).unwrap();
    for account in 0..4 {
        assert_eq!(
            fix.execute(
                &mut prepared,
                &[BindValue::U64(account), BindValue::I64(-1)]
            )
            .unwrap()
            .len(),
            32
        );
        assert_eq!(prepared.last_visits, 32);
        assert_eq!(prepared.cache.image_count(), 0);
    }
    assert_eq!(
        fix.execute(&mut prepared, &[BindValue::U64(4), BindValue::I64(-1)])
            .unwrap()
            .len(),
        32
    );
    assert_eq!(
        prepared.last_visits, 256,
        "fifth distinct selection promotes once"
    );
    assert_eq!(prepared.cache.image_count(), 1);
    for account in [0, 3, 4, 7, 99, 2, 1] {
        assert_eq!(
            fix.execute(
                &mut prepared,
                &[BindValue::U64(account), BindValue::I64(-1)]
            )
            .unwrap()
            .len(),
            if account < 8 { 32 } else { 0 }
        );
        assert_eq!(
            prepared.last_visits, 0,
            "promoted image covers every selection"
        );
    }
}

#[test]
fn indexed_image_coverage_survives_old_snapshots_refusal_and_generation_rotation() {
    let fix = indexed_store();
    let mut prepared = fix.prepare(&by_account_query()).unwrap();
    let params = [BindValue::U64(0), BindValue::I64(-1)];
    let pin = fix.db.owned_read().unwrap();
    let old_work = crate::api::db::test_operation();
    let old_source = super::super::source::QuerySource::store(pin.snapshot(), &old_work);
    let mut out = Answers::new();
    prepared
        .execute_source(&old_source, &params, &mut out)
        .unwrap();
    assert_eq!(out.len(), 32);
    fix.insert_dyn(POSTING, &posting_rows(&[(1000, 0, "new-text", 1000)]));
    assert_eq!(fix.execute(&mut prepared, &params).unwrap().len(), 33);
    prepared
        .execute_source(&old_source, &params, &mut out)
        .unwrap();
    assert_eq!(out.len(), 32, "old version cannot reuse new subset");

    // Clear the database cache while the old snapshot and query stay alive.
    fix.db.clear_cache();
    let expected = answers_of(&fix.execute(&mut prepared, &params).unwrap());
    assert_eq!(expected.len(), 33);
    assert!(expected.contains(&("new-text".into(), 1000)));
    prepared.release_memory();
    fix.db.clear_cache();
    let limited = crate::work::WorkContext::new();
    limited.cancel();
    let source = super::super::source::QuerySource::store(pin.snapshot(), &limited);
    assert!(prepared.execute_source(&source, &params, &mut out).is_err());
    assert!(
        out.is_empty(),
        "a failed bucket count never publishes partial answers"
    );
    assert_eq!(prepared.cache.image_count(), 0);
    assert_eq!(
        answers_of(&fix.execute(&mut prepared, &params).unwrap()),
        expected
    );

    // A fresh operation still reads the old snapshot after cache rotation.
    prepared.release_memory();
    fix.db.clear_cache();
    let fresh = crate::work::WorkContext::new();
    let source = super::super::source::QuerySource::store(pin.snapshot(), &fresh);
    prepared.execute_source(&source, &params, &mut out).unwrap();
    assert_eq!(out.len(), 32);
}

#[test]
fn cancelled_selected_image_publishes_no_cache_entry() {
    use crate::image::ImageBind;
    use crate::storage::store::StoreError;
    use crate::work::WorkError;

    let fix = indexed_store();
    let prepared = fix.prepare(&by_account_query()).unwrap();
    let work = crate::work::WorkContext::new();
    work.cancel();
    let pin = fix.db.owned_read().unwrap();
    let source = super::super::source::QuerySource::store(pin.snapshot(), &work);
    let images = crate::image::SourceImages::bind(&source, &prepared.cache);
    let [PreparedRule::FreeJoin(rule)] = prepared.pipeline.main_rules() else {
        panic!("Free Join")
    };
    let error = images
        .selection_image(
            &prepared.schema,
            POSTING,
            &rule.plan.occurrences()[0].selections,
            &[vec![0].into()],
        )
        .expect_err("cancelled build refuses before publishing");
    assert!(matches!(
        error,
        crate::Error::Store(error) if matches!(*error,
            StoreError::Work(WorkError::Cancelled))
    ));
    assert_eq!(prepared.cache.image_count(), 0);
}

#[test]
fn self_join_does_not_clone_a_differently_selected_partial_image() {
    let fix = indexed_store();
    let query = Query::single(Rule {
        finds: vec![FindTerm::Var(VarId(0)), FindTerm::Var(VarId(1))],
        atoms: (0..2)
            .map(|i| Atom {
                source: AtomSource::Edb(POSTING),
                bindings: vec![
                    (FieldId(1), Term::Param(crate::ir::ParamId(i))),
                    (FieldId(3), Term::Var(VarId(i))),
                ],
            })
            .collect(),
        negated: vec![],
        conditions: vec![],
    });
    let mut prepared = fix.prepare(&query).unwrap();
    let mut cursor = fix.prepare(&query).unwrap();
    cursor.force_cursor_fallback(true);
    for (a, b) in [(0, 1), (2, 0), (1, 0), (0, 0), (99, 1)] {
        let args = [BindValue::U64(a), BindValue::U64(b)];
        let canonical = |out: Answers| {
            let mut rows: Vec<_> = (0..out.len())
                .map(|i| {
                    let (AnswerValue::I64(a), AnswerValue::I64(b)) = (out.get(i, 0), out.get(i, 1))
                    else {
                        panic!("amounts")
                    };
                    (a, b)
                })
                .collect();
            rows.sort_unstable();
            rows
        };
        assert_eq!(
            canonical(fix.execute(&mut prepared, &args).unwrap()),
            canonical(fix.execute(&mut cursor, &args).unwrap())
        );
    }
}

#[test]
fn index_cardinality_is_exact_at_the_cutoff_and_observes_cancellation() {
    let fix = indexed_store();
    let pin = fix.db.owned_read().unwrap();
    let theory = fix.db.schema().compiled_theory().unwrap();
    for (relation, field, count) in [(POSTING, FieldId(1), 32), (RelationId(1), FieldId(0), 1)] {
        let compiled = theory
            .projections_of_relation(relation)
            .iter()
            .filter_map(|&id| theory.projection(id))
            .find(|p| p.projection.as_ref() == [field])
            .unwrap();
        let projection = pin.snapshot().projection(compiled.id).unwrap();
        let encoded = crate::storage::store::det_index::determinant_bytes(
            compiled,
            &[Value::U64(0)],
            &crate::api::db::test_operation(),
        )
        .unwrap();
        for limit in [0, count - 1, count, count + 1] {
            let work = crate::api::db::test_operation();
            assert_eq!(
                projection.count_bounded(&encoded, limit, &work).unwrap(),
                (count <= limit).then_some(count)
            );
        }
        let empty = crate::storage::store::det_index::determinant_bytes(
            compiled,
            &[Value::U64(99)],
            &crate::api::db::test_operation(),
        )
        .unwrap();
        let work = crate::api::db::test_operation();
        assert_eq!(projection.count_bounded(&empty, 0, &work).unwrap(), Some(0));
        work.cancel();
        assert!(
            projection.count_bounded(&empty, 0, &work).is_err(),
            "empty seeks still observe cancellation"
        );
    }
}

#[test]
fn dense_selection_uses_one_sequential_scan_without_prefetching_bucket_bodies() {
    let fix = indexed_store();
    fix.insert_dyn(
        POSTING,
        &posting_rows(
            &(256..512u64)
                .map(|i| (i, 0, "hot", i.cast_signed()))
                .collect::<Vec<_>>(),
        ),
    );
    let mut prepared = fix.prepare(&by_account_query()).unwrap();
    let actual = fix
        .execute(&mut prepared, &[BindValue::U64(0), BindValue::I64(-1)])
        .unwrap();
    assert_eq!(actual.len(), 288);
    assert_eq!(
        prepared.last_visits, 512,
        "counting index entries never fetches row bodies"
    );
    assert_eq!(prepared.cache.image_count(), 1);
}
