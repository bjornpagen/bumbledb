use super::decode::{decode_fixed_bytes, decode_i64, decode_interval_i64, decode_interval_u64};
use super::encode::{encode_interval_i64, encode_interval_u64};
use super::*;
use crate::error::CorruptionError;
use crate::schema::IntervalElement;

/// A deterministic LCG so the property sweeps are reproducible.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0
    }
}

#[test]
fn bool_round_trip_and_strictness() {
    assert_eq!(encode_bool(false), 0x00);
    assert_eq!(encode_bool(true), 0x01);
    assert_eq!(decode_bool(0x00), Ok(false));
    assert_eq!(decode_bool(0x01), Ok(true));
    // Any other byte is corruption, never a distinct "true".
    for byte in [0x02, 0x7f, 0xff] {
        assert_eq!(decode_bool(byte), Err(CorruptionError::InvalidBool(byte)));
    }
}

#[test]
fn u64_round_trip_extremes() {
    for v in [0, 1, u64::MAX, u64::MAX - 1, 1 << 63, (1 << 63) - 1] {
        assert_eq!(decode_u64(encode_u64(v)), v);
    }
}

#[test]
fn i64_round_trip_extremes() {
    for v in [0, 1, -1, i64::MAX, i64::MIN, i64::MIN + 1, i64::MAX - 1] {
        assert_eq!(decode_i64(encode_i64(v)), v);
    }
}

#[test]
fn u64_order_preservation() {
    let samples = [
        0u64,
        1,
        2,
        255,
        256,
        65_535,
        1 << 32,
        (1 << 63) - 1,
        1 << 63,
        u64::MAX,
    ];
    for pair in samples.windows(2) {
        assert!(pair[0] < pair[1]);
        assert!(
            encode_u64(pair[0]) < encode_u64(pair[1]),
            "encode({}) must sort below encode({})",
            pair[0],
            pair[1]
        );
    }
}

#[test]
fn i64_order_preservation_across_sign_boundary() {
    let samples = [
        i64::MIN,
        i64::MIN + 1,
        -65_536,
        -256,
        -2,
        -1,
        0,
        1,
        2,
        256,
        65_536,
        i64::MAX - 1,
        i64::MAX,
    ];
    for pair in samples.windows(2) {
        assert!(pair[0] < pair[1]);
        assert!(
            encode_i64(pair[0]) < encode_i64(pair[1]),
            "encode({}) must sort below encode({})",
            pair[0],
            pair[1]
        );
    }
}

/// A mixed 1/8/16-byte layout: two Bools (adjacent 1-byte fields), U64,
/// I64, String, Bytes, and both Interval elements.
fn mixed_layout() -> FactLayout {
    FactLayout::new(&[
        TypeDesc::Bool,
        TypeDesc::Bool,
        TypeDesc::U64,
        TypeDesc::I64,
        TypeDesc::String,
        TypeDesc::FixedBytes { len: 12 },
        TypeDesc::Interval {
            element: IntervalElement::U64,
        },
        TypeDesc::Interval {
            element: IntervalElement::I64,
        },
    ])
}

#[test]
fn layout_offsets_are_cumulative_widths_with_no_padding() {
    let layout = mixed_layout();
    assert_eq!(layout.field_count(), 8);
    // 1 + 1 + 8 + 8 + 8 + 16 + 16 + 16 — 1-byte fields sit flush against
    // wider ones; the bytes<12> field is word-padded to 16.
    assert_eq!(layout.field_offset(0), 0);
    assert_eq!(layout.field_offset(1), 1);
    assert_eq!(layout.field_offset(2), 2);
    assert_eq!(layout.field_offset(3), 10);
    assert_eq!(layout.field_offset(4), 18);
    assert_eq!(layout.field_offset(5), 26);
    assert_eq!(layout.field_offset(6), 42);
    assert_eq!(layout.field_offset(7), 58);
    assert_eq!(layout.fact_width(), 74);
}

fn mixed_values() -> Vec<ValueRef> {
    vec![
        ValueRef::Bool(true),
        ValueRef::Bool(false),
        ValueRef::U64(u64::MAX),
        ValueRef::I64(i64::MIN),
        ValueRef::String(7),
        ValueRef::fixed_bytes(&[0xAA; 12]),
        ValueRef::IntervalU64(3, u64::MAX),
        ValueRef::IntervalI64(i64::MIN, -5),
    ]
}

#[test]
fn encode_fact_matches_independent_field_encodings() {
    let layout = mixed_layout();
    let mut fact = Vec::new();
    encode_fact(&mixed_values(), &layout, &mut fact);
    assert_eq!(fact.len(), layout.fact_width());

    let mut expected = vec![0x01, 0x00];
    expected.extend_from_slice(&encode_u64(u64::MAX));
    expected.extend_from_slice(&encode_i64(i64::MIN));
    expected.extend_from_slice(&encode_u64(7));
    // bytes<12>: the 12 raw bytes zero-padded to the 16-byte word boundary.
    expected.extend_from_slice(&[0xAA; 12]);
    expected.extend_from_slice(&[0x00; 4]);
    expected.extend_from_slice(&encode_interval_u64(3, u64::MAX));
    expected.extend_from_slice(&encode_interval_i64(i64::MIN, -5));
    assert_eq!(fact, expected);
}

#[test]
fn field_bytes_slices_equal_independent_encodings() {
    let layout = mixed_layout();
    let mut fact = Vec::new();
    encode_fact(&mixed_values(), &layout, &mut fact);

    assert_eq!(field_bytes(&fact, &layout, 0), &[0x01]);
    assert_eq!(field_bytes(&fact, &layout, 1), &[0x00]);
    assert_eq!(field_bytes(&fact, &layout, 2), encode_u64(u64::MAX));
    assert_eq!(field_bytes(&fact, &layout, 3), encode_i64(i64::MIN));
    assert_eq!(field_bytes(&fact, &layout, 4), encode_u64(7));
    let mut padded = Vec::new();
    encode_fixed_bytes(&[0xAA; 12], &mut padded);
    assert_eq!(field_bytes(&fact, &layout, 5), padded);
    assert_eq!(
        field_bytes(&fact, &layout, 6),
        encode_interval_u64(3, u64::MAX)
    );
    assert_eq!(
        field_bytes(&fact, &layout, 7),
        encode_interval_i64(i64::MIN, -5)
    );
}

#[test]
fn decode_field_round_trips_every_type() {
    let layout = mixed_layout();
    let values = mixed_values();
    let mut fact = Vec::new();
    encode_fact(&values, &layout, &mut fact);
    for (idx, expected) in values.iter().enumerate() {
        assert_eq!(decode_field(&fact, &layout, idx), Ok(*expected));
    }
}

#[test]
fn decode_field_surfaces_corruption() {
    let layout = mixed_layout();
    let mut fact = Vec::new();
    encode_fact(&mixed_values(), &layout, &mut fact);
    fact[0] = 0x02; // corrupt the Bool
    assert_eq!(
        decode_field(&fact, &layout, 0),
        Err(CorruptionError::InvalidBool(0x02))
    );
    fact[0] = 0x01;
    fact[1] = 0x03; // corrupt the second Bool
    assert_eq!(
        decode_field(&fact, &layout, 1),
        Err(CorruptionError::InvalidBool(0x03))
    );
    fact[1] = 0x00;
    // Invert the IntervalU64 field (offset 42): end half below its start.
    fact[50..58].copy_from_slice(&encode_u64(0));
    let corrupt: [u8; 16] = fact[42..58].try_into().expect("16-byte field");
    assert_eq!(
        decode_field(&fact, &layout, 6),
        Err(CorruptionError::InvalidInterval(corrupt))
    );
    fact[50..58].copy_from_slice(&encode_u64(u64::MAX));
    // The pad-corruption fixture: a nonzero byte in the bytes<12> field's
    // trailing pad (offsets 26 + 12 .. 26 + 16) is typed corruption —
    // the pad is encoding, not data.
    fact[39] = 0x5A;
    let tail: [u8; 8] = fact[34..42].try_into().expect("trailing word");
    assert_eq!(
        decode_field(&fact, &layout, 5),
        Err(CorruptionError::NonzeroFixedBytesPad(tail))
    );
    fact[39] = 0x00;
    assert_eq!(
        decode_field(&fact, &layout, 5),
        Ok(ValueRef::fixed_bytes(&[0xAA; 12]))
    );
}

#[test]
fn fixed_bytes_round_trip_at_pad_boundaries() {
    // Widths astride the word boundaries — 1/7/8/9/63/64 — round-trip
    // through the padded encoding, and the padded width is ⌈N/8⌉ × 8.
    for len in [1usize, 7, 8, 9, 63, 64] {
        let raw: Vec<u8> = (0..len)
            .map(|i| u8::try_from(i % 251).unwrap() + 1)
            .collect();
        let mut padded = Vec::new();
        encode_fixed_bytes(&raw, &mut padded);
        assert_eq!(padded.len(), len.div_ceil(8) * 8);
        assert_eq!(&padded[..len], &raw[..]);
        assert!(padded[len..].iter().all(|&b| b == 0));
        let decoded =
            decode_fixed_bytes(&padded, u16::try_from(len).unwrap()).expect("zero pad decodes");
        assert_eq!(decoded.as_bytes(), &raw[..]);
        assert_eq!(decoded.padded(), &padded[..]);
    }
}

#[test]
fn fixed_bytes_padded_order_is_byte_order() {
    // The guard B-tree's need: memcmp order over the padded encodings of
    // equal-width values equals byte order over the values (sortedness
    // is the index's need — order *operations* stay refused).
    let mut rng = Lcg(0x0303);
    for _ in 0..500 {
        let a: Vec<u8> = (0..9).map(|_| (rng.next() & 0xFF) as u8).collect();
        let b: Vec<u8> = (0..9).map(|_| (rng.next() & 0xFF) as u8).collect();
        let (mut pa, mut pb) = (Vec::new(), Vec::new());
        encode_fixed_bytes(&a, &mut pa);
        encode_fixed_bytes(&b, &mut pb);
        assert_eq!(pa.cmp(&pb), a.cmp(&b));
    }
}

/// A random valid U64 interval: two distinct draws, ordered.
fn rand_interval_u64(rng: &mut Lcg) -> (u64, u64) {
    loop {
        let (a, b) = (rng.next(), rng.next());
        if a != b {
            return (a.min(b), a.max(b));
        }
    }
}

/// A random valid U64 interval pinned to `start` (exercises the end
/// tiebreak, which random starts would never hit).
fn rand_interval_u64_from(rng: &mut Lcg, start: u64) -> (u64, u64) {
    loop {
        let end = rng.next();
        if end > start {
            return (start, end);
        }
    }
}

#[test]
fn interval_round_trip_edges_and_random_pairs() {
    // Edges: extreme starts, MAX_END ends, and minimal width (start + 1 == end).
    for (start, end) in [
        (i64::MIN, i64::MAX),
        (i64::MIN, i64::MIN + 1),
        (i64::MAX - 1, i64::MAX),
        (0, 1),
        (-1, i64::MAX),
    ] {
        assert_eq!(
            decode_interval_i64(encode_interval_i64(start, end)),
            Ok((start, end))
        );
    }
    for (start, end) in [(0, u64::MAX), (0, 1), (u64::MAX - 1, u64::MAX)] {
        assert_eq!(
            decode_interval_u64(encode_interval_u64(start, end)),
            Ok((start, end))
        );
    }
    // Random pairs, ordered into valid intervals, both element types.
    let mut rng = Lcg(0x0101);
    for _ in 0..1_000 {
        let (start, end) = rand_interval_u64(&mut rng);
        assert_eq!(
            decode_interval_u64(encode_interval_u64(start, end)),
            Ok((start, end))
        );
        let (start, end) = (
            start.cast_signed().min(end.cast_signed()),
            start.cast_signed().max(end.cast_signed()),
        );
        assert_eq!(
            decode_interval_i64(encode_interval_i64(start, end)),
            Ok((start, end))
        );
    }
}

#[test]
fn interval_encoding_orders_by_start_then_end() {
    // Byte-wise comparison of encodings must equal `(start, end)` tuple
    // comparison under the element order — the property the storage
    // layer's neighbor probes stand on (docs/architecture/50-storage.md).
    let mut rng = Lcg(0x0202);
    for i in 0..1_000 {
        let x = rand_interval_u64(&mut rng);
        // Every other pair shares a start so the end tiebreak is exercised.
        let y = if i % 2 == 0 {
            rand_interval_u64(&mut rng)
        } else {
            rand_interval_u64_from(&mut rng, x.0)
        };
        assert_eq!(
            encode_interval_u64(x.0, x.1).cmp(&encode_interval_u64(y.0, y.1)),
            x.cmp(&y),
            "u64 encoding order diverges from tuple order for {x:?} vs {y:?}"
        );
        let (xi, yi) = (
            (x.0.cast_signed(), x.1.cast_signed()),
            (y.0.cast_signed(), y.1.cast_signed()),
        );
        // Sign-casting both halves of a valid u64 interval keeps start < end
        // exactly when both halves land on the same side of zero — skip the
        // pairs it inverts.
        if xi.0 < xi.1 && yi.0 < yi.1 {
            assert_eq!(
                encode_interval_i64(xi.0, xi.1).cmp(&encode_interval_i64(yi.0, yi.1)),
                xi.cmp(&yi),
                "i64 encoding order diverges from tuple order for {xi:?} vs {yi:?}"
            );
        }
    }
}

#[test]
fn interval_decode_rejects_start_at_or_beyond_end() {
    // Equal and inverted bounds, both element types: corruption, never a
    // value — the encoding boundary enforces `start < end` exactly as it
    // enforces Bool's strict 0/1.
    for (start, end) in [(5u64, 5u64), (9, 3), (u64::MAX, 0)] {
        let mut bytes = [0; 16];
        bytes[..8].copy_from_slice(&encode_u64(start));
        bytes[8..].copy_from_slice(&encode_u64(end));
        assert_eq!(
            decode_interval_u64(bytes),
            Err(CorruptionError::InvalidInterval(bytes))
        );
    }
    for (start, end) in [(-2i64, -2i64), (4, -4), (i64::MAX, i64::MIN)] {
        let mut bytes = [0; 16];
        bytes[..8].copy_from_slice(&encode_i64(start));
        bytes[8..].copy_from_slice(&encode_i64(end));
        assert_eq!(
            decode_interval_i64(bytes),
            Err(CorruptionError::InvalidInterval(bytes))
        );
    }
}

#[test]
fn nullary_fact_layout_and_hash() {
    // Nullary relations are legal (10-data-model): the empty fact encodes
    // to zero bytes and still has a well-defined identity hash.
    let layout = FactLayout::new(&[]);
    assert_eq!(layout.fact_width(), 0);
    let mut fact = Vec::new();
    encode_fact(&[], &layout, &mut fact);
    assert!(fact.is_empty());
    assert_eq!(fact_hash(&fact), *blake3::hash(b"").as_bytes());
}

#[test]
fn fact_hash_is_full_32_byte_blake3() {
    let bytes = b"arbitrary fact bytes";
    let hash = fact_hash(bytes);
    assert_eq!(hash.len(), 32);
    assert_eq!(hash, *blake3::hash(bytes).as_bytes());
    assert_ne!(fact_hash(b"a"), fact_hash(b"b"));
}
