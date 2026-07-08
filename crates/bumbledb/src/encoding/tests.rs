use super::decode::decode_i64;
use super::*;
use crate::error::CorruptionError;

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
fn enum_ordinal_range_check() {
    assert_eq!(decode_enum(0, 3), Ok(0));
    assert_eq!(decode_enum(2, 3), Ok(2));
    assert_eq!(
        decode_enum(3, 3),
        Err(CorruptionError::EnumOrdinalOutOfRange {
            ordinal: 3,
            variant_count: 3
        })
    );
    // 256 variants: every u8 ordinal is valid.
    assert_eq!(decode_enum(255, 256), Ok(255));
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

/// A mixed 1/8-byte layout: Bool, Enum, U64, I64, String, Bytes.
fn mixed_layout() -> FactLayout {
    FactLayout::new(&[
        TypeDesc::Bool,
        TypeDesc::Enum { variant_count: 3 },
        TypeDesc::U64,
        TypeDesc::I64,
        TypeDesc::String,
        TypeDesc::Bytes,
    ])
}

#[test]
fn layout_offsets_are_cumulative_widths_with_no_padding() {
    let layout = mixed_layout();
    assert_eq!(layout.field_count(), 6);
    // 1 + 1 + 8 + 8 + 8 + 8 — 1-byte fields sit flush against 8-byte ones.
    assert_eq!(layout.field_offset(0), 0);
    assert_eq!(layout.field_offset(1), 1);
    assert_eq!(layout.field_offset(2), 2);
    assert_eq!(layout.field_offset(3), 10);
    assert_eq!(layout.field_offset(4), 18);
    assert_eq!(layout.field_offset(5), 26);
    assert_eq!(layout.fact_width(), 34);
}

fn mixed_values() -> Vec<ValueRef> {
    vec![
        ValueRef::Bool(true),
        ValueRef::Enum(2),
        ValueRef::U64(u64::MAX),
        ValueRef::I64(i64::MIN),
        ValueRef::String(7),
        ValueRef::Bytes(9),
    ]
}

#[test]
fn encode_fact_matches_independent_field_encodings() {
    let layout = mixed_layout();
    let mut fact = Vec::new();
    encode_fact(&mixed_values(), &layout, &mut fact);
    assert_eq!(fact.len(), layout.fact_width());

    let mut expected = vec![0x01, 0x02];
    expected.extend_from_slice(&encode_u64(u64::MAX));
    expected.extend_from_slice(&encode_i64(i64::MIN));
    expected.extend_from_slice(&encode_u64(7));
    expected.extend_from_slice(&encode_u64(9));
    assert_eq!(fact, expected);
}

#[test]
fn field_bytes_slices_equal_independent_encodings() {
    let layout = mixed_layout();
    let mut fact = Vec::new();
    encode_fact(&mixed_values(), &layout, &mut fact);

    assert_eq!(field_bytes(&fact, &layout, 0), &[0x01]);
    assert_eq!(field_bytes(&fact, &layout, 1), &[0x02]);
    assert_eq!(field_bytes(&fact, &layout, 2), encode_u64(u64::MAX));
    assert_eq!(field_bytes(&fact, &layout, 3), encode_i64(i64::MIN));
    assert_eq!(field_bytes(&fact, &layout, 4), encode_u64(7));
    assert_eq!(field_bytes(&fact, &layout, 5), encode_u64(9));
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
    fact[1] = 0x03; // corrupt the Enum ordinal (variant_count = 3)
    assert_eq!(
        decode_field(&fact, &layout, 1),
        Err(CorruptionError::EnumOrdinalOutOfRange {
            ordinal: 3,
            variant_count: 3
        })
    );
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
