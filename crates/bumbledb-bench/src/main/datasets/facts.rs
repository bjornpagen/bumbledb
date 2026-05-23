use std::collections::BTreeSet;

use bumbledb_core::encoding::{DecimalRaw, TimestampMicros};
use bumbledb_lmdb::{Fact, Value};

pub(crate) fn sailors_facts(sailors: u64) -> Vec<Fact> {
    let mut facts = Vec::new();
    for sid in 1..=sailors {
        facts.push(Fact::new(
            "Sailor",
            [
                ("id", Value::Serial(sid)),
                ("name", Value::String(format!("sailor-{sid}"))),
                ("rating", Value::U64((sid % 10) + 1)),
                ("age", Value::I64(18 + (sid % 50) as i64)),
            ],
        ));
    }
    let boats = (sailors / 4).max(10);
    for bid in 1..=boats {
        facts.push(Fact::new(
            "Boat",
            [
                ("id", Value::Serial(bid)),
                ("name", Value::String(format!("boat-{bid}"))),
                ("color", Value::Enum(((bid % 3) + 1) as u8)),
            ],
        ));
    }
    let mut seen = BTreeSet::new();
    for sid in 1..=sailors {
        for offset in 0..5 {
            let bid = ((sid + offset * 7) % boats) + 1;
            let day = ((sid * 10 + offset) as i64) * 86_400;
            if seen.insert((sid, bid, day)) {
                facts.push(Fact::new(
                    "Reserve",
                    [
                        ("sailor", Value::Serial(sid)),
                        ("boat", Value::Serial(bid)),
                        ("day", Value::Timestamp(TimestampMicros(day))),
                    ],
                ));
            }
        }
    }
    facts
}

pub(crate) fn join_stress_facts(n: u64) -> Vec<Fact> {
    let mut facts = Vec::new();
    for id in 1..=n {
        facts.push(Fact::new(
            "A",
            [
                ("id", Value::Serial(id)),
                ("k", Value::Enum((id % 10) as u8)),
            ],
        ));
        facts.push(Fact::new(
            "B",
            [
                ("id", Value::Serial(id)),
                ("a", Value::Serial(((id - 1) % n) + 1)),
                ("k", Value::Enum((id % 10) as u8)),
            ],
        ));
        facts.push(Fact::new(
            "C",
            [
                ("id", Value::Serial(id)),
                ("b", Value::Serial(((id - 1) % n) + 1)),
                ("k", Value::Enum((id % 10) as u8)),
            ],
        ));
        facts.push(Fact::new(
            "D",
            [
                ("id", Value::Serial(id)),
                ("c", Value::Serial(((id - 1) % n) + 1)),
                ("k", Value::Enum((id % 10) as u8)),
            ],
        ));
    }
    let mut ab = BTreeSet::new();
    let mut ac = BTreeSet::new();
    let mut bc = BTreeSet::new();
    for a in 1..=n {
        for offset in 0..3 {
            let b = ((a + offset) % n) + 1;
            let c = ((a + offset * 2) % n) + 1;
            if ab.insert((a, b)) {
                facts.push(Fact::new(
                    "EdgeAB",
                    [("a", Value::Serial(a)), ("b", Value::Serial(b))],
                ));
            }
            if ac.insert((a, c)) {
                facts.push(Fact::new(
                    "EdgeAC",
                    [("a", Value::Serial(a)), ("c", Value::Serial(c))],
                ));
            }
            if bc.insert((b, c)) {
                facts.push(Fact::new(
                    "EdgeBC",
                    [("b", Value::Serial(b)), ("c", Value::Serial(c))],
                ));
            }
        }
    }
    facts
}

pub(crate) fn tpch_facts(n: u64) -> Vec<Fact> {
    let mut facts = Vec::new();
    for id in 1..=n {
        facts.push(Fact::new(
            "Customer",
            [
                ("id", Value::Serial(id)),
                ("nation", Value::U64((id % 5) + 1)),
            ],
        ));
        facts.push(Fact::new(
            "Supplier",
            [
                ("id", Value::Serial(id)),
                ("nation", Value::U64((id % 7) + 1)),
            ],
        ));
        facts.push(Fact::new(
            "Part",
            [
                ("id", Value::Serial(id)),
                ("brand", Value::U64((id % 11) + 1)),
            ],
        ));
        facts.push(Fact::new(
            "Orders",
            [
                ("id", Value::Serial(id)),
                ("customer", Value::Serial(((id - 1) % n) + 1)),
                (
                    "order_date",
                    Value::Timestamp(TimestampMicros(id as i64 * 10)),
                ),
            ],
        ));
    }
    let mut line = 1;
    for order in 1..=n {
        for offset in 0..4 {
            facts.push(Fact::new(
                "LineItem",
                [
                    ("id", Value::Serial(line)),
                    ("order", Value::Serial(order)),
                    ("part", Value::Serial(((order + offset) % n) + 1)),
                    ("supplier", Value::Serial(((order + offset * 3) % n) + 1)),
                    ("quantity", Value::I64((offset + 1) as i64)),
                    (
                        "extended_price",
                        Value::Decimal(DecimalRaw(line as i128 * 100)),
                    ),
                    (
                        "ship_date",
                        Value::Timestamp(TimestampMicros(line as i64 * 10)),
                    ),
                ],
            ));
            line += 1;
        }
    }
    facts
}
