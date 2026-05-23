use bumbledb_core::encoding::{DecimalRaw, TimestampMicros};

use crate::{Fact, Value};

/// Generates deterministic benchmark facts.
pub fn benchmark_facts(scale: u64) -> Vec<Fact> {
    let mut facts = Vec::new();
    let scale = scale.max(1);

    for id in 1..=scale {
        facts.push(Fact::new(
            "Holder",
            [
                ("id", Value::Serial(id)),
                ("name", Value::String(format!("holder-{id}"))),
            ],
        ));
        facts.push(Fact::new(
            "Org",
            [
                ("id", Value::Serial(id)),
                ("name", Value::String(format!("org-{id}"))),
            ],
        ));
    }
    for id in 1..=3 {
        facts.push(Fact::new(
            "Instrument",
            [
                ("id", Value::Serial(id)),
                ("symbol", Value::String(format!("SYM{id}"))),
            ],
        ));
    }
    for id in 1..=scale {
        facts.push(Fact::new(
            "SourceDocument",
            [
                ("id", Value::Serial(id)),
                ("payload", Value::Bytes(format!("source-{id}").into_bytes())),
            ],
        ));
    }
    for id in 1..=scale {
        facts.push(Fact::new(
            "Account",
            [
                ("id", Value::Serial(id)),
                ("holder", Value::Serial(id)),
                ("currency", Value::Enum(1)),
            ],
        ));
    }
    for id in 1..=scale {
        facts.push(Fact::new(
            "JournalEntry",
            [
                ("id", Value::Serial(id)),
                ("source", Value::Serial(id)),
                (
                    "created_at",
                    Value::Timestamp(TimestampMicros(id as i64 * 10)),
                ),
            ],
        ));
    }
    let mut posting_id = 1;
    for account in 1..=scale {
        for offset in 0..3 {
            facts.push(Fact::new(
                "Posting",
                [
                    ("id", Value::Serial(posting_id)),
                    ("entry", Value::Serial(account)),
                    ("account", Value::Serial(account)),
                    ("instrument", Value::Serial((offset % 3) + 1)),
                    (
                        "amount",
                        Value::Decimal(DecimalRaw((posting_id as i128) * 100)),
                    ),
                    (
                        "at",
                        Value::Timestamp(TimestampMicros(posting_id as i64 * 10)),
                    ),
                ],
            ));
            facts.push(Fact::new(
                "PostingTag",
                [
                    ("posting", Value::Serial(posting_id)),
                    ("tag", Value::Enum((1 + offset) as u8)),
                ],
            ));
            posting_id += 1;
        }
    }
    for id in 2..=scale {
        facts.push(Fact::new(
            "OrgParent",
            [("child", Value::Serial(id)), ("parent", Value::Serial(1))],
        ));
        facts.push(Fact::new(
            "AuthorizationEdge",
            [
                ("subject", Value::Serial(id)),
                ("object", Value::Serial(1)),
                ("permission", Value::Enum(7)),
            ],
        ));
    }
    for id in 1..=3 {
        facts.push(Fact::new(
            "ExchangeRate",
            [
                ("id", Value::Serial(id)),
                ("base", Value::Serial(id)),
                ("quote", Value::Serial(1)),
                ("at", Value::Timestamp(TimestampMicros(id as i64 * 10))),
                ("rate", Value::Decimal(DecimalRaw(100_000_000))),
            ],
        ));
    }

    facts
}
