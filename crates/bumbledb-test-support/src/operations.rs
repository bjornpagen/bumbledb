//! Randomized operation helpers for property tests.

use bumbledb_lmdb::{Fact, Value};
use proptest::prelude::*;

use crate::facts::{account, holder, posting};

/// Small valid fact batches with FK order preserved.
pub fn valid_ledger_facts_strategy() -> impl Strategy<Value = Vec<Fact>> {
    (1u64..8).prop_map(|count| {
        let mut facts = Vec::new();
        for id in 1..=count {
            facts.push(holder(id, format!("holder-{id}")));
        }
        for id in 1..=count {
            facts.push(account(id, id, 1));
        }
        for id in 1..=count {
            facts.push(posting(id, id, id as i128 * 100, id as i64 * 10));
        }
        facts
    })
}

/// Invalid duplicate fact batch.
pub fn duplicate_holder_facts() -> Vec<Fact> {
    vec![holder(1, "same"), holder(1, "other")]
}

/// Wrong-type fact for negative tests.
pub fn wrong_type_holder_fact() -> Fact {
    Fact::new(
        "Holder",
        [
            ("id", Value::String("bad".to_owned())),
            ("name", Value::String("x".to_owned())),
        ],
    )
}
