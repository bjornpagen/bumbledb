//! Integration pins for `bumbledb_query::order`: sort keys as data
//! folded into one comparator for the language's own `sort_by`, run
//! against a real store — plus the totality pin over [`AnswerValue`]
//! variants constructed directly.

use bumbledb::{Answer, AnswerValue, Db, Interval};
use bumbledb_query::order::{self, SortKey};
use bumbledb_query::query;
use std::cmp::Ordering;

mod common;
use common::TempDir;

mod orders {
    bumbledb::schema! {
        pub Orders;

        relation Order {
            id: u64 as OrderId, fresh,
            amount: i64,
            label: str,
            window: interval<i64>,
        }
    }
}

use orders::{Order, OrderId, Orders};

fn span(start: i64, end: i64) -> Interval<i64> {
    Interval::<i64>::new(start, end).expect("nonempty half-open interval")
}

/// Five facts with ties on `amount` (two −5s, two 3s) and on `window`'s
/// start (three spans starting at 0, two of them identical), negative
/// and positive `amount` values.
fn seeded(tag: &str) -> (TempDir, Db<Orders>) {
    let dir = TempDir::new(tag);
    let db = Db::create(dir.path(), Orders).expect("create the Orders store");
    db.write(|tx| {
        for (id, amount, label, window) in [
            (1u64, -5i64, "banana", span(0, 10)),
            (2, 3, "apple", span(0, 5)),
            (3, -7, "cherry", span(2, 4)),
            (4, 3, "banana", span(0, 10)),
            (5, -5, "apple", span(5, 6)),
        ] {
            tx.insert(&Order {
                id: OrderId(id),
                amount,
                label,
                window,
            })?;
        }
        Ok(())
    })
    .expect("seed the five orders");
    (dir, db)
}

/// Executes the full projection — columns are the find terms in order:
/// 0 `id: u64`, 1 `amount: i64`, 2 `label: str`, 3 `window:
/// interval<i64>` — sorts the borrowed rows by `keys`, and hands the
/// sorted rows to `check`.
fn sorted_rows(tag: &str, keys: &[SortKey], check: impl Fn(&[Answer<'_>])) {
    let (_dir, db) = seeded(tag);
    let all = query!(Orders {
        (id, amount, label, window) | Order(id, amount, label, window);
    });
    let mut prepared = db.prepare(&all).expect("prepare the full projection");
    db.read(|snap| {
        let out = snap.execute_collect(&mut prepared, &[])?;
        let mut rows: Vec<Answer<'_>> = out.answers().collect();
        rows.sort_by(order::by(keys));
        check(&rows);
        Ok(())
    })
    .expect("execute and sort");
}

fn u64_at(row: &Answer<'_>, column: usize) -> u64 {
    let AnswerValue::U64(value) = row.get(column) else {
        panic!("column {column} finds u64");
    };
    value
}

fn i64_at(row: &Answer<'_>, column: usize) -> i64 {
    let AnswerValue::I64(value) = row.get(column) else {
        panic!("column {column} finds i64");
    };
    value
}

fn bounds_at(row: &Answer<'_>, column: usize) -> (i64, i64) {
    let AnswerValue::IntervalI64(value) = row.get(column) else {
        panic!("column {column} finds interval<i64>");
    };
    value.bounds()
}

#[test]
fn by_sorts_answers_ascending_by_one_column() {
    sorted_rows("order-asc-one", &[SortKey::Asc(0)], |rows| {
        let ids: Vec<u64> = rows.iter().map(|row| u64_at(row, 0)).collect();
        assert_eq!(ids, [1, 2, 3, 4, 5], "the full u64 column, ascending");
    });
}

#[test]
fn desc_reverses_and_later_keys_break_ties() {
    // Descending amount groups (3, 3), (−5, −5), (−7); the ascending id
    // key breaks each tie.
    sorted_rows(
        "order-desc-tiebreak",
        &[SortKey::Desc(1), SortKey::Asc(0)],
        |rows| {
            let seq: Vec<(i64, u64)> = rows
                .iter()
                .map(|row| (i64_at(row, 1), u64_at(row, 0)))
                .collect();
            assert_eq!(
                seq,
                [(3, 2), (3, 4), (-5, 1), (-5, 5), (-7, 3)],
                "amount descending, ids ascending within each tie"
            );
        },
    );
}

#[test]
fn i64_columns_order_numerically_across_sign() {
    // Negatives land before positives — pins that no word-order/sign
    // bug survives the materialization decode.
    sorted_rows("order-i64-sign", &[SortKey::Asc(1)], |rows| {
        let amounts: Vec<i64> = rows.iter().map(|row| i64_at(row, 1)).collect();
        assert_eq!(amounts, [-7, -5, -5, 3, 3], "numeric order across sign");
    });
}

#[test]
fn intervals_order_by_start_then_end() {
    // Three spans share start 0: [0,5) sorts before the two [0,10)s —
    // start first, end as the tiebreak.
    sorted_rows("order-interval", &[SortKey::Asc(3)], |rows| {
        let windows: Vec<(i64, i64)> = rows.iter().map(|row| bounds_at(row, 3)).collect();
        assert_eq!(
            windows,
            [(0, 5), (0, 10), (0, 10), (2, 4), (5, 6)],
            "intervals by start, then end"
        );
    });
}

#[test]
fn value_cmp_is_total_across_variants() {
    // Same-variant order, every variant.
    let less = [
        (AnswerValue::Bool(false), AnswerValue::Bool(true)),
        (AnswerValue::U64(1), AnswerValue::U64(2)),
        (AnswerValue::I64(-5), AnswerValue::I64(3)),
        (AnswerValue::String("apple"), AnswerValue::String("banana")),
        (
            AnswerValue::FixedBytes(&[1, 2]),
            AnswerValue::FixedBytes(&[1, 3]),
        ),
        (
            AnswerValue::IntervalU64(Interval::<u64>::new(1, 5).expect("nonempty")),
            AnswerValue::IntervalU64(Interval::<u64>::new(1, 7).expect("nonempty")),
        ),
        (
            AnswerValue::IntervalI64(span(-3, 0)),
            AnswerValue::IntervalI64(span(-2, 0)),
        ),
    ];
    for (small, big) in &less {
        assert_eq!(
            order::value_cmp(small, big),
            Ordering::Less,
            "{small:?} < {big:?}"
        );
        assert_eq!(
            order::value_cmp(big, small),
            Ordering::Greater,
            "{big:?} > {small:?}"
        );
        assert_eq!(
            order::value_cmp(small, small),
            Ordering::Equal,
            "{small:?} == itself"
        );
    }

    // The cross-variant rank chain, one representative per variant in
    // rank order: Bool < U64 < I64 < String < FixedBytes < IntervalU64
    // < IntervalI64 — total, and never Equal across variants.
    let chain = [
        AnswerValue::Bool(true),
        AnswerValue::U64(0),
        AnswerValue::I64(-1),
        AnswerValue::String(""),
        AnswerValue::FixedBytes(&[]),
        AnswerValue::IntervalU64(Interval::<u64>::new(0, 1).expect("nonempty")),
        AnswerValue::IntervalI64(span(0, 1)),
    ];
    for (i, low) in chain.iter().enumerate() {
        for high in &chain[i + 1..] {
            assert_eq!(
                order::value_cmp(low, high),
                Ordering::Less,
                "rank orders {low:?} before {high:?}"
            );
            assert_eq!(
                order::value_cmp(high, low),
                Ordering::Greater,
                "rank orders {high:?} after {low:?}"
            );
        }
    }
}
