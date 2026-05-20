//! Deterministic row fixtures.

use bumbledb_core::encoding::{DecimalRaw, TimestampMicros};
use bumbledb_lmdb::{IdentityValue, Row, Value};

/// Holder row.
pub fn holder(id: u64, name: impl Into<String>) -> Row {
    Row::new(
        "Holder",
        [
            ("id", Value::Identity(IdentityValue::Serial(id))),
            ("name", Value::String(name.into())),
        ],
    )
}

/// Account row.
pub fn account(id: u64, holder: u64, currency: u64) -> Row {
    Row::new(
        "Account",
        [
            ("id", Value::Identity(IdentityValue::Serial(id))),
            ("holder", Value::Identity(IdentityValue::Serial(holder))),
            ("currency", Value::Enum(currency)),
        ],
    )
}

/// Posting row.
pub fn posting(id: u64, account: u64, amount: i128, at: i64) -> Row {
    Row::new(
        "Posting",
        [
            ("id", Value::Identity(IdentityValue::Serial(id))),
            ("account", Value::Identity(IdentityValue::Serial(account))),
            ("amount", Value::Decimal(DecimalRaw(amount))),
            ("at", Value::Timestamp(TimestampMicros(at))),
        ],
    )
}

/// Account tag row.
pub fn account_tag(account: u64, tag: u64) -> Row {
    Row::new(
        "AccountTag",
        [
            ("account", Value::Identity(IdentityValue::Serial(account))),
            ("tag", Value::Enum(tag)),
        ],
    )
}

/// Number row for overflow tests.
pub fn number(id: u64, n: i64, d: i128) -> Row {
    Row::new(
        "Number",
        [
            ("id", Value::Identity(IdentityValue::Serial(id))),
            ("n", Value::I64(n)),
            ("d", Value::Decimal(DecimalRaw(d))),
        ],
    )
}

/// Seeded valid ledger rows.
pub fn seeded_ledger_rows() -> Vec<Row> {
    vec![
        holder(1, "Alice"),
        holder(2, "Bob"),
        account(1, 1, 840),
        account(2, 1, 978),
        account(3, 2, 840),
        posting(1, 1, 100, 10),
        posting(2, 1, 200, 20),
        posting(3, 2, 300, 30),
        account_tag(1, 7),
        account_tag(2, 8),
    ]
}

/// Larger deterministic ledger rows.
pub fn generated_ledger_rows(scale: u64) -> Vec<Row> {
    let scale = scale.max(1);
    let mut rows = Vec::new();
    for id in 1..=scale {
        rows.push(holder(id, format!("holder-{id}")));
        rows.push(account(id, id, 840));
    }
    let mut posting_id = 1;
    for account in 1..=scale {
        for offset in 0..3 {
            rows.push(posting(
                posting_id,
                account,
                posting_id as i128 * 100,
                posting_id as i64 * 10,
            ));
            rows.push(account_tag(account, offset + 1));
            posting_id += 1;
        }
    }
    rows
}
