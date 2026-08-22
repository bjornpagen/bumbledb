mod common;

use bumbledb::{Error, Fact, Fresh, FreshRange, Value};

bumbledb::schema! {
    pub CutoverNamed;
    relation Label { name: str }
}

bumbledb::schema! {
    pub CutoverFresh;
    relation Cell {
        id: u64 as CellId, fresh,
        v: u64,
    }
}

#[test]
fn empty_fresh_range_cannot_yield_a_minted_id() {
    let empty: FreshRange<CellId> = FreshRange::Empty;
    assert!(empty.is_empty());
    assert_eq!(empty.len(), 0);
    assert!(empty.start().is_none());
    assert!(empty.get(0).is_none());
    assert!(empty.iter().next().is_none());
    assert!(empty.ids().is_none());
    assert!(empty.end_exclusive_raw().is_none());
    let collected: Vec<CellId> = FreshRange::<CellId>::Empty.into_iter().collect();
    assert!(collected.is_empty());

    let dir = common::TempDir::new("mutation-empty-fresh");
    let db = bumbledb::Db::create(dir.path(), CutoverFresh)
        .expect("create")
        .expect("accepted");
    let field = db
        .fresh_field(CellId::RELATION, CellId::FIELD)
        .expect("fresh field");
    db.write(|tx| {
        let range = tx.reserve::<CellId>(0)?;
        assert!(range.is_empty());
        assert!(range.start().is_none());
        assert!(range.get(0).is_none());
        assert!(range.iter().next().is_none());
        let raw = tx.reserve_at(field, 0)?;
        assert!(raw.start().is_none());
        assert!(matches!(raw, FreshRange::Empty));
        Ok(())
    })
    .expect("empty reserve")
    .unwrap();
}

#[test]
fn a_noop_insert_does_not_mark_applied_so_shape_fail_stays_clean() {
    let dir = common::TempDir::new("mutation-noop-not-applied");
    let db = bumbledb::Db::create(dir.path(), CutoverNamed)
        .expect("create")
        .expect("accepted");
    let row = [Value::String("keep".into())];
    db.write(|tx| tx.insert_dyn(Label::RELATION, [&row]).map(|_| ()))
        .expect("seed")
        .unwrap();
    db.write(|tx| {
        assert_eq!(
            tx.insert_dyn(Label::RELATION, [&row])?.changed(),
            0,
            "redundant"
        );
        let err = tx
            .insert_dyn(Label::RELATION, [Vec::<Value>::new()])
            .expect_err("shape");
        assert!(matches!(err, Error::FactShape(_)), "{err:?}");
        assert_eq!(
            tx.insert_dyn(Label::RELATION, [&[Value::String("next".into())]])?
                .changed(),
            1
        );
        Ok(())
    })
    .expect("shape fail after no-op did not poison")
    .unwrap();
}

#[test]
fn poison_preserves_the_original_error_and_empty_insert_is_no_engine_request() {
    let dir = common::TempDir::new("mutation-poison-kind");
    let db = bumbledb::Db::create(dir.path(), CutoverFresh)
        .expect("create")
        .expect("accepted");
    let outcome = db.write(|tx| {
        tx.insert_dyn(Cell::RELATION, [&[Value::U64(u64::MAX), Value::U64(0)]])?;
        let first = tx.reserve::<CellId>(1).expect_err("exhausted");
        assert!(
            matches!(first, Error::FreshExhausted { .. }),
            "first apply failure is the original: {first:?}"
        );
        assert_eq!(
            tx.insert_dyn(Cell::RELATION, Vec::<Vec<Value>>::new())
                .expect("empty is no engine request")
                .submitted(),
            0
        );
        Ok(())
    });
    match outcome {
        Err(Error::TransactionPoisoned { source }) => {
            assert!(matches!(source.as_ref(), Error::FreshExhausted { .. }));
        }
        other => panic!("Db::write aborts on Ok after poison: {other:?}"),
    }
}
