//! Randomized operation helpers for property tests.

use bumbledb_lmdb::{Row, Value};
use proptest::prelude::*;

use crate::rows::{account, holder, posting};

/// Random test operation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Operation {
    /// Insert one row.
    Insert(Row),
    /// Replace one row.
    Replace(Row),
    /// Delete holder by ID.
    DeleteHolder(u64),
    /// Delete account by ID.
    DeleteAccount(u64),
}

/// Small valid row batches with FK order preserved.
pub fn valid_ledger_rows_strategy() -> impl Strategy<Value = Vec<Row>> {
    (1u64..8).prop_map(|count| {
        let mut rows = Vec::new();
        for id in 1..=count {
            rows.push(holder(id, format!("holder-{id}")));
        }
        for id in 1..=count {
            rows.push(account(id, id, 840));
        }
        for id in 1..=count {
            rows.push(posting(id, id, id as i128 * 100, id as i64 * 10));
        }
        rows
    })
}

/// Invalid duplicate row batch.
pub fn duplicate_holder_rows() -> Vec<Row> {
    vec![holder(1, "same"), holder(1, "other")]
}

/// Wrong-type row for negative tests.
pub fn wrong_type_holder_row() -> Row {
    Row::new(
        "Holder",
        [
            ("id", Value::String("bad".to_owned())),
            ("name", Value::String("x".to_owned())),
        ],
    )
}
