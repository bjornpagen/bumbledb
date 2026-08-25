//! Conformance lane 2 — multi-braid convergence, L9's executable
//! shadow: random interleavings of independent per-braid histories all
//! converge to one `catalog_digest` and one generation. The
//! order-quotient digest makes the gate sound for string-free and
//! string-carrying fixtures alike; this corpus keeps its rows
//! string-free so intern minting — store-state-relative by the aliasing
//! ruling — stays out of the instrument, the recorded scope note.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use bumbledb::schema::fingerprint::fingerprint as schema_fingerprint;
use bumbledb::schema::{
    Bound, FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, SchemaDescriptor,
    Side, StatementDescriptor, ValidateDescriptor as _, ValueType, Weight,
};
use bumbledb::{Db, Value};
use bumbledb_log::apply::{Applied, apply};
use bumbledb_log::braids::BraidId;
use bumbledb_log::codec::{BatchHeader, Codec, Op, OpKind};
use bumbledb_log::sidecar::Chain;

const VENUE: RelationId = RelationId(0);
const BOOKING: RelationId = RelationId(1);
const SEAT: RelationId = RelationId(2);
const NOTE: RelationId = RelationId(3);
const TAG: RelationId = RelationId(4);

const VENUE_COUNT: u64 = 4;
const SEAT_CEILING: u64 = 100;
const BASE_SEAT_UNITS: u64 = 10;

static DIR_SEQ: AtomicU64 = AtomicU64::new(0);

fn temp_dir(tag: &str) -> PathBuf {
    let seq = DIR_SEQ.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("f2_{tag}_{}_{seq}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("create test root");
    path
}

/// One braid carrying a key, a containment, and a weighted capacity —
/// venue (key + containment target + capacity parent), booking (slot
/// key + containment source), seat (weighted capacity child) — plus two
/// singleton braids for the interleaving gate. Every field is a `U64`:
/// the digest oracle must never hinge on intern mint order.
fn theory() -> SchemaDescriptor {
    let field = |name: &str| FieldDescriptor {
        name: name.into(),
        value_type: ValueType::U64,
        generation: Generation::None,
    };
    let side = |relation: RelationId, fields: &[u16]| Side {
        relation,
        projection: fields.iter().map(|f| FieldId(*f)).collect(),
        selection: Box::from([]),
    };
    SchemaDescriptor {
        relations: vec![
            RelationDescriptor {
                name: "venue".into(),
                fields: vec![field("id")],
                extension: None,
            },
            RelationDescriptor {
                name: "booking".into(),
                fields: vec![field("venue"), field("slot")],
                extension: None,
            },
            RelationDescriptor {
                name: "seat".into(),
                fields: vec![field("venue"), field("units")],
                extension: None,
            },
            RelationDescriptor {
                name: "note".into(),
                fields: vec![field("id"), field("body")],
                extension: None,
            },
            RelationDescriptor {
                name: "tag".into(),
                fields: vec![field("id"), field("label")],
                extension: None,
            },
        ],
        statements: vec![
            StatementDescriptor::Functionality {
                relation: VENUE,
                projection: Box::from([FieldId(0)]),
            },
            StatementDescriptor::Functionality {
                relation: BOOKING,
                projection: Box::from([FieldId(1)]),
            },
            StatementDescriptor::Containment {
                source: side(BOOKING, &[0]),
                target: side(VENUE, &[0]),
            },
            StatementDescriptor::Capacity {
                target: side(VENUE, &[0]),
                weight: Weight::Field(FieldId(1)),
                lo: 0,
                hi: Some(Bound::Lit(SEAT_CEILING)),
                source: side(SEAT, &[0]),
            },
        ],
    }
}

fn codec() -> Codec {
    let descriptor = theory();
    let schema = descriptor.clone().validate().expect("fixture validates");
    let fingerprint = schema_fingerprint(&schema).0;
    Codec::new(&descriptor, fingerprint)
}

fn op(kind: OpKind, relation: RelationId, row: Box<[Value]>) -> Op {
    Op {
        kind,
        relation,
        rows: vec![row],
    }
}

fn venue_row(id: u64) -> Box<[Value]> {
    Box::from([Value::U64(id)])
}

fn booking_row(venue: u64, slot: u64) -> Box<[Value]> {
    Box::from([Value::U64(venue), Value::U64(slot)])
}

fn seat_row(venue: u64, units: u64) -> Box<[Value]> {
    Box::from([Value::U64(venue), Value::U64(units)])
}

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    fn below(&mut self, bound: u64) -> u64 {
        self.next() % bound
    }
}

/// One braid's scripted history: each batch encoded against the chain
/// its predecessor left, so the bytes are fixed however the braids
/// interleave downstream.
fn encode_history(
    codec: &Codec,
    braid: BraidId,
    script: &[Vec<Op>],
) -> Vec<(BraidId, u64, Vec<u8>)> {
    let mut head_hash = [0u8; 32];
    let mut head_ts = 0u64;
    let mut batches = Vec::with_capacity(script.len());
    for (index, ops) in script.iter().enumerate() {
        let slot = u64::try_from(index).expect("script fits u64") + 1;
        let ts = (2_000 + slot).max(head_ts);
        let header = BatchHeader {
            fingerprint: *codec.fingerprint(),
            braid,
            braid_gen: slot,
            prev: head_hash,
            writer: 7,
            timestamp: ts,
        };
        let bytes = codec.encode(&header, ops).expect("encode history batch");
        head_hash = *blake3::hash(&bytes).as_bytes();
        head_ts = ts;
        batches.push((braid, slot, bytes));
    }
    batches
}

fn apply_sequence(codec: &Codec, order: &[&(BraidId, u64, Vec<u8>)]) -> ([u8; 32], u64) {
    let root = temp_dir("interleave");
    let outcome = {
        let db = Db::create(&root.join("db"), theory())
            .expect("create")
            .expect("theory admits empty store");
        let mut chain = Chain::genesis(codec.braids());
        for (braid, slot, bytes) in order {
            let applied = apply(&db, &mut chain, codec, *braid, *slot, bytes).expect("apply io");
            assert!(
                matches!(applied, Applied::Advanced { .. }),
                "every history batch advances: braid {braid:?} slot {slot}: {applied:?}"
            );
        }
        let digest = db.catalog_digest().expect("catalog digest");
        let generation = db.generation().expect("generation").value();
        (digest, generation)
    };
    let _ = std::fs::remove_dir_all(&root);
    outcome
}

#[test]
fn multi_braid_interleavings_converge_to_one_digest() {
    let codec = codec();
    let world = codec.braids().braid_of(VENUE).expect("world braid");
    let notes = codec.braids().braid_of(NOTE).expect("note braid");
    let tags = codec.braids().braid_of(TAG).expect("tag braid");

    let world_script: Vec<Vec<Op>> = vec![
        vec![Op {
            kind: OpKind::Insert,
            relation: VENUE,
            rows: (1..=VENUE_COUNT).map(venue_row).collect(),
        }],
        vec![Op {
            kind: OpKind::Insert,
            relation: SEAT,
            rows: (1..=VENUE_COUNT)
                .map(|v| seat_row(v, BASE_SEAT_UNITS))
                .collect(),
        }],
        vec![op(OpKind::Insert, BOOKING, booking_row(1, 900))],
        vec![op(OpKind::Insert, BOOKING, booking_row(2, 901))],
        vec![
            op(OpKind::Insert, BOOKING, booking_row(3, 902)),
            op(OpKind::Insert, SEAT, seat_row(3, 5)),
        ],
        vec![op(OpKind::Delete, SEAT, seat_row(1, BASE_SEAT_UNITS))],
    ];
    let note_script: Vec<Vec<Op>> = (1..=5)
        .map(|i| {
            vec![op(
                OpKind::Insert,
                NOTE,
                Box::from([Value::U64(i), Value::U64(i * 11)]),
            )]
        })
        .collect();
    let tag_script: Vec<Vec<Op>> = (1..=5)
        .map(|i| {
            vec![op(
                OpKind::Insert,
                TAG,
                Box::from([Value::U64(i), Value::U64(i * 13)]),
            )]
        })
        .collect();

    let histories = [
        encode_history(&codec, world, &world_script),
        encode_history(&codec, notes, &note_script),
        encode_history(&codec, tags, &tag_script),
    ];
    let total: usize = histories.iter().map(Vec::len).sum();

    let canonical: Vec<&(BraidId, u64, Vec<u8>)> = histories.iter().flatten().collect();
    let (reference_digest, reference_generation) = apply_sequence(&codec, &canonical);
    assert_eq!(
        reference_generation,
        u64::try_from(total).expect("history fits u64")
    );

    let mut rng = Rng(0xF2_2026_0823);
    for _ in 0..24 {
        let mut cursors = [0usize; 3];
        let mut order: Vec<&(BraidId, u64, Vec<u8>)> = Vec::with_capacity(total);
        while order.len() < total {
            let remaining: u64 = histories
                .iter()
                .zip(cursors.iter())
                .map(|(history, cursor)| u64::try_from(history.len() - cursor).expect("fits u64"))
                .sum();
            let mut pick = rng.below(remaining);
            for (history, cursor) in histories.iter().zip(cursors.iter_mut()) {
                let left = u64::try_from(history.len() - *cursor).expect("fits u64");
                if pick < left {
                    order.push(&history[*cursor]);
                    *cursor += 1;
                    break;
                }
                pick -= left;
            }
        }
        let (digest, generation) = apply_sequence(&codec, &order);
        assert_eq!(
            digest, reference_digest,
            "every interleaving converges to the reference digest"
        );
        assert_eq!(generation, reference_generation);
    }
}
