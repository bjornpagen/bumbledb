//! Deterministic fact fixtures.

use bumbledb_core::encoding::{DecimalRaw, TimestampMicros};
use bumbledb_lmdb::{Fact, Value};

/// Holder fact.
pub fn holder(id: u64, name: impl Into<String>) -> Fact {
    Fact::new(
        "Holder",
        [
            ("id", Value::Serial(id)),
            ("name", Value::String(name.into())),
        ],
    )
}

/// Account fact.
pub fn account(id: u64, holder: u64, currency: u8) -> Fact {
    Fact::new(
        "Account",
        [
            ("id", Value::Serial(id)),
            ("holder", Value::Serial(holder)),
            ("currency", Value::Enum(currency)),
        ],
    )
}

/// Posting fact.
pub fn posting(id: u64, account: u64, amount: i128, at: i64) -> Fact {
    Fact::new(
        "Posting",
        [
            ("id", Value::Serial(id)),
            ("account", Value::Serial(account)),
            ("amount", Value::Decimal(DecimalRaw(amount))),
            ("at", Value::Timestamp(TimestampMicros(at))),
        ],
    )
}

/// Account tag fact.
pub fn account_tag(account: u64, tag: u8) -> Fact {
    Fact::new(
        "AccountTag",
        [
            ("account", Value::Serial(account)),
            ("tag", Value::Enum(tag)),
        ],
    )
}

/// Number fact for overflow tests.
pub fn number(id: u64, n: i64, d: i128) -> Fact {
    Fact::new(
        "Number",
        [
            ("id", Value::Serial(id)),
            ("n", Value::I64(n)),
            ("d", Value::Decimal(DecimalRaw(d))),
        ],
    )
}

/// Seeded valid ledger facts.
pub fn seeded_ledger_facts() -> Vec<Fact> {
    vec![
        holder(1, "Alice"),
        holder(2, "Bob"),
        account(1, 1, 1),
        account(2, 1, 2),
        account(3, 2, 1),
        posting(1, 1, 100, 10),
        posting(2, 1, 200, 20),
        posting(3, 2, 300, 30),
        account_tag(1, 7),
        account_tag(2, 8),
    ]
}

/// Larger deterministic ledger facts.
pub fn generated_ledger_facts(scale: u64) -> Vec<Fact> {
    let scale = scale.max(1);
    let mut facts = Vec::new();
    for id in 1..=scale {
        facts.push(holder(id, format!("holder-{id}")));
        facts.push(account(id, id, 1));
    }
    let mut posting_id = 1;
    for account in 1..=scale {
        for offset in 0..3 {
            facts.push(posting(
                posting_id,
                account,
                posting_id as i128 * 100,
                posting_id as i64 * 10,
            ));
            facts.push(account_tag(account, offset + 1));
            posting_id += 1;
        }
    }
    facts
}
