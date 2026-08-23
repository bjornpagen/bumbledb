//! Conformance lane 2 — commutativity as a running oracle (the
//! executable shadow of L7/L8). Seeded random batch pairs on one braid
//! are filtered to strict footprint disjointness by the crate's own
//! `footprint` and `intersect`; each kept pair is applied A;B on one
//! store and B;A on another through the apply path. The set-level gate
//! (L8's statement): per-batch verdicts and row content agree across
//! orders. The representation gate: both orders land byte-identical
//! catalogs under `catalog_digest`. Then the braid corollary: random
//! interleavings of a multi-braid history all converge to one digest
//! (L8 composed with L9); the corollary's rows carry no strings, so
//! intern minting — store-state-relative by the aliasing ruling — never
//! enters the instrument.

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use bumbledb::schema::fingerprint::fingerprint as schema_fingerprint;
use bumbledb::schema::{
    Bound, FieldDescriptor, FieldId, Generation, RelationDescriptor, RelationId, SchemaDescriptor,
    Side, StatementDescriptor, ValidateDescriptor as _, ValueType, Weight,
};
use bumbledb::{Db, Value, Violations};
use bumbledb_log::apply::{Applied, apply};
use bumbledb_log::braids::BraidId;
use bumbledb_log::codec::{BatchHeader, Codec, Op, OpKind};
use bumbledb_log::footprint::{CapacityKey, capacity_profiles, footprint};
use bumbledb_log::intersect::{BaseMeasure, LoserDecision, intersect};
use bumbledb_log::sidecar::Chain;

const VENUE: RelationId = RelationId(0);
const BOOKING: RelationId = RelationId(1);
const SEAT: RelationId = RelationId(2);
const NOTE: RelationId = RelationId(3);
const TAG: RelationId = RelationId(4);

const VENUE_COUNT: u64 = 4;
const BASE_SLOTS: std::ops::Range<u64> = 100..108;
const SEAT_CEILING: u64 = 100;
const BASE_SEAT_UNITS: u64 = 10;
const PAIR_COUNT: u32 = 120;
const PAIR_SEED: u64 = 0xF2_2026_0822;

static DIR_SEQ: AtomicU64 = AtomicU64::new(0);

fn temp_dir(tag: &str) -> PathBuf {
    let seq = DIR_SEQ.fetch_add(1, Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!("f2_{tag}_{}_{seq}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("create test root");
    path
}

/// One braid carrying every footprint class — venue (key + containment
/// target + capacity parent), booking (slot key + containment source),
/// seat (weighted capacity child) — plus two singleton braids for the
/// interleaving corollary. Every field is a `U64`: the digest oracle
/// must never hinge on intern mint order.
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
    Codec::new(&descriptor, fingerprint).expect("fixture vocabulary")
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

/// The shared base every pair is judged against: four venues, one
/// ten-unit seat per venue, eight bookings on slots 100..108.
fn base_ops() -> Vec<Op> {
    let venues = Op {
        kind: OpKind::Insert,
        relation: VENUE,
        rows: (1..=VENUE_COUNT).map(venue_row).collect(),
    };
    let seats = Op {
        kind: OpKind::Insert,
        relation: SEAT,
        rows: (1..=VENUE_COUNT)
            .map(|v| seat_row(v, BASE_SEAT_UNITS))
            .collect(),
    };
    let bookings = Op {
        kind: OpKind::Insert,
        relation: BOOKING,
        rows: BASE_SLOTS.map(|s| booking_row(base_venue(s), s)).collect(),
    };
    vec![venues, seats, bookings]
}

fn base_venue(slot: u64) -> u64 {
    slot % VENUE_COUNT + 1
}

fn other_venue(venue: u64) -> u64 {
    venue % VENUE_COUNT + 1
}

/// The base measures the interval test prices shared venue groups
/// against — only groups that exist at the base carry a measure, so a
/// pair meeting at a not-yet-established group is conservatively
/// conflicted by `CapacityMeasureMissing` instead of guessed at.
fn base_measures(codec: &Codec) -> BTreeMap<CapacityKey, BaseMeasure> {
    let mut measures = BTreeMap::new();
    for venue in 1..=VENUE_COUNT {
        let probe = [op(OpKind::Insert, SEAT, seat_row(venue, 1))];
        let profiles = capacity_profiles(codec.vocabulary(), &probe).expect("probe profiles");
        for key in profiles.keys() {
            measures.insert(
                *key,
                BaseMeasure {
                    measure: BASE_SEAT_UNITS,
                    floor: 0,
                    ceiling: Some(SEAT_CEILING),
                },
            );
        }
    }
    measures
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

/// One extra op from the adversarial menu: base-key collisions (verdict
/// fodder for the equal-verdicts gate), base deletes, small and
/// oversized seat moves, new and doomed venues, more fresh bookings.
fn extra_op(rng: &mut Rng, fresh: &mut u64) -> Op {
    match rng.below(8) {
        0 => {
            let slot = 100 + rng.below(BASE_SLOTS.end - BASE_SLOTS.start);
            op(
                OpKind::Insert,
                BOOKING,
                booking_row(other_venue(base_venue(slot)), slot),
            )
        }
        1 => {
            let slot = 100 + rng.below(BASE_SLOTS.end - BASE_SLOTS.start);
            op(OpKind::Delete, BOOKING, booking_row(base_venue(slot), slot))
        }
        2 => op(
            OpKind::Insert,
            SEAT,
            seat_row(1 + rng.below(VENUE_COUNT), 1 + rng.below(5)),
        ),
        3 => op(
            OpKind::Delete,
            SEAT,
            seat_row(1 + rng.below(VENUE_COUNT), BASE_SEAT_UNITS),
        ),
        4 => op(
            OpKind::Insert,
            VENUE,
            venue_row(VENUE_COUNT + 1 + rng.below(5)),
        ),
        5 => op(OpKind::Delete, VENUE, venue_row(1 + rng.below(VENUE_COUNT))),
        6 => op(
            OpKind::Insert,
            SEAT,
            seat_row(1 + rng.below(VENUE_COUNT), 5 * SEAT_CEILING),
        ),
        _ => op(
            OpKind::Insert,
            BOOKING,
            booking_row(1 + rng.below(VENUE_COUNT), take_fresh(fresh)),
        ),
    }
}

fn take_fresh(fresh: &mut u64) -> u64 {
    *fresh += 1;
    *fresh
}

/// Every generated batch opens with a fresh-slot booking insert, so an
/// accepted batch is state-changing by construction and the apply
/// path's publish-law instrument stays out of the oracle's way.
fn gen_batch(rng: &mut Rng, fresh: &mut u64) -> Vec<Op> {
    let mut ops = vec![op(
        OpKind::Insert,
        BOOKING,
        booking_row(1 + rng.below(VENUE_COUNT), take_fresh(fresh)),
    )];
    for _ in 0..rng.below(3) {
        ops.push(extra_op(rng, fresh));
    }
    ops
}

struct PairStats {
    filtered: u32,
    shared_capacity: u32,
}

/// One kept pair: two batches the algebra ruled strictly disjoint.
struct Pair {
    a: Vec<Op>,
    b: Vec<Op>,
}

/// Seeded proposals filtered to strict disjointness by the crate's own
/// `footprint` + `intersect` — the generator's filter is the algebra
/// under test, and its strict-disjointness verdict must be symmetric.
fn disjoint_pairs(codec: &Codec) -> (Vec<Pair>, PairStats) {
    let vocabulary = codec.vocabulary();
    let measures = base_measures(codec);
    let mut rng = Rng(PAIR_SEED);
    let mut fresh = 500_000u64;
    let mut pairs = Vec::new();
    let mut stats = PairStats {
        filtered: 0,
        shared_capacity: 0,
    };
    let mut proposals = 0u32;

    while pairs.len() < usize::try_from(PAIR_COUNT).expect("count fits usize") {
        proposals += 1;
        assert!(
            proposals < 5_000,
            "generator starved: {} disjoint pairs after {proposals} proposals",
            pairs.len()
        );
        let a = gen_batch(&mut rng, &mut fresh);
        let b = gen_batch(&mut rng, &mut fresh);
        let footprint_a = footprint(vocabulary, &a).expect("footprint of a");
        let footprint_b = footprint(vocabulary, &b).expect("footprint of b");
        let a_vs_b =
            intersect(vocabulary, &footprint_a, &a, &b, &measures).expect("intersect a vs b");
        let b_vs_a =
            intersect(vocabulary, &footprint_b, &b, &a, &measures).expect("intersect b vs a");
        assert_eq!(
            matches!(a_vs_b, LoserDecision::Disjoint),
            matches!(b_vs_a, LoserDecision::Disjoint),
            "strict disjointness is symmetric: {a_vs_b:?} vs {b_vs_a:?}\nA: {a:?}\nB: {b:?}"
        );
        if !matches!(
            (&a_vs_b, &b_vs_a),
            (LoserDecision::Disjoint, LoserDecision::Disjoint)
        ) {
            stats.filtered += 1;
            continue;
        }
        let profiles_a = capacity_profiles(vocabulary, &a).expect("profiles of a");
        let profiles_b = capacity_profiles(vocabulary, &b).expect("profiles of b");
        if profiles_a.keys().any(|key| profiles_b.contains_key(key)) {
            stats.shared_capacity += 1;
        }
        pairs.push(Pair { a, b });
    }
    (pairs, stats)
}

/// The engine's verdict for one batch, stripped of the slot number it
/// landed at — the value L7 says both orders must agree on.
#[derive(Debug, Clone, PartialEq)]
enum Verdict {
    Accepted,
    Rejected(Violations),
}

fn verdict(applied: Applied) -> Verdict {
    match applied {
        Applied::Advanced { .. } => Verdict::Accepted,
        Applied::Rejected(violations) => Verdict::Rejected(violations),
        other => panic!("a generated batch must judge, not absorb or refuse: {other:?}"),
    }
}

/// Encodes `ops` at the braid's next slot (chain-addressed header) and
/// applies it; the chain advances exactly when the engine committed.
fn apply_next(
    db: &Db<SchemaDescriptor>,
    chain: &mut Chain,
    codec: &Codec,
    braid: BraidId,
    ops: &[Op],
    ts: u64,
) -> Applied {
    let position = chain.position(braid);
    let header = BatchHeader {
        fingerprint: *codec.fingerprint(),
        braid,
        braid_gen: position.g + 1,
        prev: position.prev,
        writer: 7,
        timestamp: ts.max(position.ts),
    };
    let bytes = codec.encode(&header, ops).expect("encode generated batch");
    apply(db, chain, codec, braid, position.g + 1, &bytes, 0).expect("apply io")
}

/// Every row either order could have touched: the base rows plus both
/// batches' rows. Two stores holding the same judged content agree on
/// exactly this universe — a row outside it exists in neither.
fn probe_rows(first: &[Op], second: &[Op]) -> Vec<(RelationId, Box<[Value]>)> {
    let mut probes = Vec::new();
    for source in [&base_ops()[..], first, second] {
        for one in source {
            for row in &one.rows {
                probes.push((one.relation, row.clone()));
            }
        }
    }
    probes
}

struct OrderRun {
    first: Verdict,
    second: Verdict,
    digest: [u8; 32],
    generation: u64,
    presence: Vec<bool>,
}

/// Applies base, then `first`, then `second` on a fresh store; returns
/// both verdicts with the final digest, generation, and the presence
/// bit of every probed row.
fn run_order(
    codec: &Codec,
    braid: BraidId,
    first: &[Op],
    second: &[Op],
    probes: &[(RelationId, Box<[Value]>)],
) -> OrderRun {
    let root = temp_dir("pair");
    let outcome = {
        let db = Db::create(&root.join("db"), theory())
            .expect("create")
            .expect("theory admits empty store");
        let mut chain = Chain::genesis(codec.braids());
        let base = apply_next(&db, &mut chain, codec, braid, &base_ops(), 1_000);
        assert!(
            matches!(base, Applied::Advanced { .. }),
            "the shared base applies clean: {base:?}"
        );
        let first_verdict = verdict(apply_next(&db, &mut chain, codec, braid, first, 1_001));
        let second_verdict = verdict(apply_next(&db, &mut chain, codec, braid, second, 1_002));
        let presence = db
            .read(|instance| {
                probes
                    .iter()
                    .map(|(relation, row)| instance.contains_dyn(*relation, row))
                    .collect::<bumbledb::Result<Vec<bool>>>()
            })
            .expect("probe rows");
        OrderRun {
            first: first_verdict,
            second: second_verdict,
            digest: db.catalog_digest().expect("catalog digest"),
            generation: db.generation().expect("generation").value(),
            presence,
        }
    };
    let _ = std::fs::remove_dir_all(&root);
    outcome
}

/// L8's own statement as a running oracle: under strict disjointness,
/// each batch's verdict and the judged row content are identical in
/// either apply order.
#[test]
fn disjoint_pairs_commute_in_verdicts_and_row_content() {
    let codec = codec();
    let braid = codec.braids().braid_of(VENUE).expect("world braid");
    let (pairs, stats) = disjoint_pairs(&codec);
    let mut rejected_kept = 0u32;

    for Pair { a, b } in &pairs {
        let probes = probe_rows(a, b);
        let forward = run_order(&codec, braid, a, b, &probes);
        let reverse = run_order(&codec, braid, b, a, &probes);

        assert_eq!(
            forward.first, reverse.second,
            "A's verdict is order-independent under disjointness\nA: {a:?}\nB: {b:?}"
        );
        assert_eq!(
            forward.second, reverse.first,
            "B's verdict is order-independent under disjointness\nA: {a:?}\nB: {b:?}"
        );
        assert_eq!(
            forward.presence, reverse.presence,
            "both orders judge one row content\nA: {a:?}\nB: {b:?}"
        );
        assert_eq!(
            forward.generation, reverse.generation,
            "both orders advance the generation identically\nA: {a:?}\nB: {b:?}"
        );
        if forward.first != Verdict::Accepted || forward.second != Verdict::Accepted {
            rejected_kept += 1;
        }
    }

    assert!(
        pairs.len() >= 100,
        "the gate demands at least 100 pairs: {}",
        pairs.len()
    );
    assert!(
        stats.filtered > 0,
        "the filter must have teeth: every proposal passed"
    );
    assert!(
        stats.shared_capacity > 0,
        "at least one kept pair shares a venue group under a passing interval test"
    );
    assert!(
        rejected_kept > 0,
        "at least one kept pair carries a rejection, so verdict equality is not vacuous"
    );
}

/// The representation half of the gate: both orders land one catalog
/// byte-for-byte under `catalog_digest` — the pin behind L8's
/// representations-equal claim.
#[test]
fn disjoint_pairs_land_byte_identical_catalogs() {
    let codec = codec();
    let braid = codec.braids().braid_of(VENUE).expect("world braid");
    let (pairs, _) = disjoint_pairs(&codec);

    for Pair { a, b } in &pairs {
        let probes = probe_rows(a, b);
        let forward = run_order(&codec, braid, a, b, &probes);
        let reverse = run_order(&codec, braid, b, a, &probes);
        assert_eq!(
            forward.digest, reverse.digest,
            "both orders land one catalog byte-for-byte\nA: {a:?}\nB: {b:?}"
        );
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
            let applied = apply(&db, &mut chain, codec, *braid, *slot, bytes, 0).expect("apply io");
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
