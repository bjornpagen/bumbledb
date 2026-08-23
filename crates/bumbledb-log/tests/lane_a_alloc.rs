//! The alloc-discipline pin for the codec: encode and decode allocate
//! output construction only, so their allocation counts are
//! deterministic per input and bounded by stated budgets — measured on
//! the fixture below and pinned; a hunt may lower them, they must not
//! rise. One test function only: the counting allocator is
//! process-global and concurrent tests would pollute the window.

#[path = "lane_a_support/mod.rs"]
mod support;

use bumbledb::alloc_counter::{self, CountingAllocator};
use bumbledb::schema::RelationId;
use bumbledb::{Interval, Value};
use bumbledb_log::codec::{BatchHeader, Codec, Op, OpKind};

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

/// Measured exactly on this fixture: the wire buffer's growth alone.
const ENCODE_ALLOC_BUDGET: u64 = 6;

/// Measured exactly on this fixture: ops, rows, and boxed row storage
/// — decode's outputs and nothing else.
const DECODE_ALLOC_BUDGET: u64 = 9;

fn fixture_ops() -> Vec<Op> {
    let booking = |slot: u64, customer: u64, room: u64, qty: u64| -> Box<[Value]> {
        Box::from([
            Value::U64(slot),
            Value::U64(customer),
            Value::U64(room),
            Value::U64(qty),
            Value::IntervalU64(Interval::new(0, 5).expect("interval")),
        ])
    };
    vec![
        Op {
            kind: OpKind::Insert,
            relation: RelationId(1),
            rows: vec![
                booking(1, 7, 4, 2),
                booking(2, 7, 4, 3),
                booking(3, 8, 6, 1),
            ],
        },
        Op {
            kind: OpKind::Delete,
            relation: RelationId(1),
            rows: vec![booking(1, 7, 4, 2)],
        },
        Op {
            kind: OpKind::Insert,
            relation: RelationId(2),
            rows: vec![Box::from([Value::U64(4), Value::U64(9)]) as Box<[Value]>],
        },
    ]
}

fn window(work: impl FnOnce()) -> u64 {
    alloc_counter::reset();
    work();
    alloc_counter::count()
}

#[test]
fn codec_allocation_is_deterministic_and_budgeted() {
    let descriptor = support::schema("booking");
    let codec = Codec::new(&descriptor, support::corpus_fingerprint("booking"));
    let ops = fixture_ops();
    let header = BatchHeader {
        fingerprint: *codec.fingerprint(),
        braid: codec.braids().parse(0).expect("braid"),
        braid_gen: 1,
        prev: [0u8; 32],
        writer: 1,
        timestamp: 1_000,
    };

    // Warm every path once so lazy runtime setup stays out of the
    // window.
    let bytes = codec.encode(&header, &ops).expect("encode");
    codec.decode(&bytes).expect("decode");

    let first = window(|| {
        codec.encode(&header, &ops).expect("encode");
    });
    let second = window(|| {
        codec.encode(&header, &ops).expect("encode");
    });
    assert_eq!(first, second, "encode allocation is deterministic");
    assert!(
        first <= ENCODE_ALLOC_BUDGET,
        "encode window {first} within {ENCODE_ALLOC_BUDGET}"
    );

    let first = window(|| {
        codec.decode(&bytes).expect("decode");
    });
    let second = window(|| {
        codec.decode(&bytes).expect("decode");
    });
    assert_eq!(first, second, "decode allocation is deterministic");
    assert!(
        first <= DECODE_ALLOC_BUDGET,
        "decode window {first} within {DECODE_ALLOC_BUDGET}"
    );
}
