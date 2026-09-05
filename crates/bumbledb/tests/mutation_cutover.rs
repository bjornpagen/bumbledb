mod common;

use bumbledb::{Error, Fact as _, Value};

bumbledb::schema! {
    pub CutoverNamed;
    relation Label { name: str }
}

bumbledb::schema! {
    pub CutoverIds;
    relation Cell {
        id: u64 as CellId,
        v: u64,
    }
}

// The old `empty_fresh_range_cannot_yield_a_minted_id` test retired with
// the fresh reservation machinery (E-NO-RESERVE): there is no FreshRange,
// no reserve, and the database issues no identity.

#[test]
fn a_noop_insert_does_not_mark_applied_so_shape_fail_stays_clean() {
    let dir = common::TempDir::new("mutation-noop-not-applied");
    let db = bumbledb::Db::create(dir.path(), CutoverNamed, common::work())
        .expect("create")
        .expect("accepted");
    let row = [Value::String("keep".into())];
    db.write(common::work(), |tx| tx.insert_dyn(Label::RELATION, [&row]).map(|_| ()))
        .expect("seed")
        .unwrap();
    db.write(common::work(), |tx| {
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
    let db = bumbledb::Db::create(dir.path(), CutoverIds, common::work())
        .expect("create")
        .expect("accepted");
    let outcome = db.write(common::work(), |tx| {
        // An empty collection is no engine request at any point.
        assert_eq!(
            tx.insert_dyn(Cell::RELATION, Vec::<Vec<Value>>::new())
                .expect("empty is no engine request")
                .submitted(),
            0
        );
        // Stage a real row, then fail an apply: the transaction poisons
        // and the FIRST failure is the original typed error.
        tx.insert_dyn(Cell::RELATION, [&[Value::U64(1), Value::U64(0)]])?;
        let first = tx
            .insert_dyn(Cell::RELATION, [Vec::<Value>::new()])
            .expect_err("shape");
        assert!(matches!(first, Error::FactShape(_)), "{first:?}");
        Ok(())
    });
    match outcome {
        Err(Error::TransactionPoisoned { source }) => {
            assert!(matches!(source.as_ref(), Error::FactShape(_)));
        }
        other => panic!("Db::write aborts on Ok after poison: {other:?}"),
    }
}
