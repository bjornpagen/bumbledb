//! Downstream tests for the actual core exports. References do not call the
//! implementation's canonicalizer or order-key routine to compute expectations.

use std::cmp::Ordering;
use std::collections::{BTreeSet, HashSet};
use std::num::FpCategory;

use bumbledb::{F64, F64ParseError, Id128, Id128ParseError};

fn reference_bits(bits: u64) -> u64 {
    match f64::from_bits(bits).classify() {
        FpCategory::Nan => 0x7ff8_0000_0000_0000,
        FpCategory::Zero => 0,
        _ => bits,
    }
}

fn reference_order(left: u64, right: u64) -> Ordering {
    let left = f64::from_bits(left);
    let right = f64::from_bits(right);
    match (left.is_nan(), right.is_nan()) {
        (true, true) => Ordering::Equal,
        (true, false) => Ordering::Greater,
        (false, true) => Ordering::Less,
        (false, false) => left.partial_cmp(&right).expect("neither operand is NaN"),
    }
}

/// Deterministic fixtures, not a production ID generator.
fn next_bits(state: &mut u64) -> u64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    *state
}

fn assert_float_image(bits: u64) {
    let expected = reference_bits(bits);
    let value = F64::from_bits(bits);
    assert_eq!(value.to_bits(), expected, "input={bits:016x}");
    assert_eq!(F64::from(f64::from_bits(bits)), value);
    assert_eq!(value.to_f64().to_bits(), expected);
    assert_eq!(f64::from(value).to_bits(), expected);
    assert_eq!(F64::from_bits(value.to_bits()), value);
    assert_eq!(value.is_nan(), f64::from_bits(bits).is_nan());
    assert_eq!(value.is_finite(), f64::from_bits(bits).is_finite());
    assert_eq!(value.is_infinite(), f64::from_bits(bits).is_infinite());
    assert_eq!(F64::from_canonical_bits(expected), Ok(value));
    assert_eq!(F64::try_from(value.to_be_bytes().as_slice()), Ok(value));
    assert_eq!(F64::from_order_key(value.to_order_key()), Ok(value));
    assert_eq!(F64::from_order_bytes(value.to_order_bytes()), Ok(value));
    if bits == expected {
        assert_eq!(F64::from_canonical_bits(bits), Ok(value));
    } else {
        assert_eq!(
            F64::from_canonical_bits(bits),
            Err(F64ParseError::NonCanonicalBits { bits })
        );
        assert_eq!(
            F64::try_from(bits.to_be_bytes().as_slice()),
            Err(F64ParseError::NonCanonicalBits { bits })
        );
    }
}

#[test]
fn f64_every_sign_exponent_and_fraction_boundary_class() {
    let fractions = [
        0,
        1,
        2,
        0x0007_ffff_ffff_ffff,
        0x0008_0000_0000_0000,
        0x000f_ffff_ffff_fffe,
        0x000f_ffff_ffff_ffff,
    ];
    for sign in [0, 1_u64 << 63] {
        for exponent in 0..2048_u64 {
            for fraction in fractions {
                assert_float_image(sign | (exponent << 52) | fraction);
            }
        }
    }
}

#[test]
fn f64_sampled_images_match_independent_ieee_classification() {
    let mut state = 0x382c_99ae_f0b1_a74d;
    for _ in 0..100_000 {
        assert_float_image(next_bits(&mut state));
    }
}

#[test]
fn f64_payload_and_order_golden_vectors() {
    // The middle column is independently written wire bytes, not a call to
    // either implementation encoder. The final column is the frozen order key.
    let vectors = [
        (
            0xfff0_0000_0000_0000,
            [0xff, 0xf0, 0, 0, 0, 0, 0, 0],
            0x000f_ffff_ffff_ffff,
        ),
        (
            0xffef_ffff_ffff_ffff,
            [0xff, 0xef, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
            0x0010_0000_0000_0000,
        ),
        (
            0xbff0_0000_0000_0000,
            [0xbf, 0xf0, 0, 0, 0, 0, 0, 0],
            0x400f_ffff_ffff_ffff,
        ),
        (
            0x8000_0000_0000_0001,
            [0x80, 0, 0, 0, 0, 0, 0, 1],
            0x7fff_ffff_ffff_fffe,
        ),
        (0, [0; 8], 0x8000_0000_0000_0000),
        (1, [0, 0, 0, 0, 0, 0, 0, 1], 0x8000_0000_0000_0001),
        (
            0x000f_ffff_ffff_ffff,
            [0, 0x0f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
            0x800f_ffff_ffff_ffff,
        ),
        (
            0x0010_0000_0000_0000,
            [0, 0x10, 0, 0, 0, 0, 0, 0],
            0x8010_0000_0000_0000,
        ),
        (
            0x3ff0_0000_0000_0000,
            [0x3f, 0xf0, 0, 0, 0, 0, 0, 0],
            0xbff0_0000_0000_0000,
        ),
        (
            0x7fef_ffff_ffff_ffff,
            [0x7f, 0xef, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
            0xffef_ffff_ffff_ffff,
        ),
        (
            0x7ff0_0000_0000_0000,
            [0x7f, 0xf0, 0, 0, 0, 0, 0, 0],
            0xfff0_0000_0000_0000,
        ),
        (
            0x7ff8_0000_0000_0000,
            [0x7f, 0xf8, 0, 0, 0, 0, 0, 0],
            0xfff8_0000_0000_0000,
        ),
    ];
    for &(bits, payload, key) in &vectors {
        let value = F64::from_canonical_bits(bits).unwrap();
        assert_eq!(value.to_be_bytes(), payload);
        assert_eq!(F64::from_canonical_be_bytes(payload), Ok(value));
        assert_eq!(value.to_order_key(), key);
        assert_eq!(value.to_order_bytes(), key.to_be_bytes());
    }
    for pair in vectors.windows(2) {
        let left = F64::from_bits(pair[0].0);
        let right = F64::from_bits(pair[1].0);
        assert!(left < right);
        assert!(left.to_order_bytes() < right.to_order_bytes());
    }
    for &(left, _, _) in &vectors {
        for &(right, _, _) in &vectors {
            assert_eq!(
                F64::from_bits(left).cmp(&F64::from_bits(right)),
                reference_order(left, right)
            );
        }
    }
}

#[test]
fn f64_order_matches_ieee_numbers_with_nan_last() {
    let mut state = 0xd6a4_0fb1_c873_952e;
    let mut values = Vec::with_capacity(20_000);
    for _ in 0..20_000 {
        let left = next_bits(&mut state);
        let right = next_bits(&mut state);
        let a = F64::from_bits(left);
        let b = F64::from_bits(right);
        let expected = reference_order(left, right);
        assert_eq!(a.cmp(&b), expected);
        assert_eq!(a.partial_cmp(&b), Some(expected));
        assert_eq!(a.to_order_bytes().cmp(&b.to_order_bytes()), expected);
        assert_eq!(a == b, expected == Ordering::Equal);
        values.push(a);
    }
    values.sort_unstable();
    for pair in values.windows(2) {
        assert_ne!(
            reference_order(pair[0].to_bits(), pair[1].to_bits()),
            Ordering::Greater
        );
    }
}

#[test]
fn f64_set_identity_collapses_only_nan_payloads_and_zero_signs() {
    let values = [
        F64::ZERO,
        F64::from(-0.0),
        F64::NAN,
        F64::from_bits(0x7ff0_0000_0000_0001),
        F64::from_bits(0xffff_ffff_ffff_ffff),
        F64::from(1.0),
        F64::from(-1.0),
    ];
    let hashed: HashSet<_> = values.into_iter().collect();
    let ordered: BTreeSet<_> = values.into_iter().collect();
    assert_eq!(hashed.len(), 4);
    assert_eq!(ordered.len(), 4);
    assert_eq!(
        ordered.into_iter().map(F64::to_bits).collect::<Vec<_>>(),
        [
            0xbff0_0000_0000_0000,
            0,
            0x3ff0_0000_0000_0000,
            0x7ff8_0000_0000_0000
        ]
    );
    assert_eq!(F64::default(), F64::ZERO);
}

#[test]
fn f64_strict_decoder_refuses_wrong_width_and_noncanonical_order_holes() {
    let bytes = [0; 32];
    for len in 0..=32 {
        if len != 8 {
            assert_eq!(
                F64::try_from(&bytes[..len]),
                Err(F64ParseError::InvalidLength { actual: len })
            );
        }
    }
    // These fixed keys invert to -0, positive signaling NaN, and negative NaN.
    for (key, bits) in [
        (0x7fff_ffff_ffff_ffff, 0x8000_0000_0000_0000),
        (0xfff0_0000_0000_0001, 0x7ff0_0000_0000_0001),
        (0x0007_ffff_ffff_ffff, 0xfff8_0000_0000_0000),
    ] {
        assert_eq!(
            F64::from_order_key(key),
            Err(F64ParseError::NonCanonicalBits { bits })
        );
        assert_eq!(
            F64::from_order_bytes(key.to_be_bytes()),
            Err(F64ParseError::NonCanonicalBits { bits })
        );
    }
}

#[test]
fn scalar_values_have_exact_payload_width_and_one_canonical_home() {
    const NEGATIVE_ZERO: F64 = F64::from_bits(0x8000_0000_0000_0000);
    const ID: Id128 = Id128::from_bytes([0x3c; 16]);
    assert_eq!(std::mem::size_of::<F64>(), 8);
    assert_eq!(std::mem::size_of::<Id128>(), 16);
    assert_eq!(NEGATIVE_ZERO, F64::ZERO);
    assert_eq!(ID.to_bytes(), [0x3c; 16]);
    // Assignments must compile: public core values are re-exports, not copies.
    let theory_float: bumbledb_theory::F64 = NEGATIVE_ZERO;
    let theory_id: bumbledb_theory::Id128 = ID;
    assert_eq!(theory_float, F64::ZERO);
    assert_eq!(theory_id, ID);
}

#[test]
fn id128_canonical_hex_and_exact_bytes_golden() {
    let bytes = [
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88, 0x99, 0xaa, 0xbb, 0xcc, 0xdd, 0xee,
        0xff,
    ];
    let text = "00112233445566778899aabbccddeeff";
    let id = Id128::from_bytes(bytes);
    assert_eq!(id.to_string(), text);
    assert_eq!(format!("{id:?}"), format!("Id128({text})"));
    assert_eq!(Id128::from_hex(text), Ok(id));
    assert_eq!(text.parse::<Id128>(), Ok(id));
    assert_eq!(Id128::try_from(bytes.as_slice()), Ok(id));
    assert_eq!(Id128::from(bytes), id);
    assert_eq!(<[u8; 16]>::from(id), bytes);
    assert_eq!(id.as_bytes(), &bytes);
}

#[test]
fn id128_owns_bytes_and_has_no_reserved_patterns() {
    for value in 0..=u8::MAX {
        let mut bytes = [value; 16];
        let id = Id128::from_bytes(bytes);
        bytes.fill(value.wrapping_add(1));
        assert_eq!(id.to_bytes(), [value; 16]);
        assert_eq!(Id128::from_hex(&id.to_string()), Ok(id));
    }
    assert_eq!(
        Id128::from_bytes([0; 16]).to_string(),
        "00000000000000000000000000000000"
    );
    assert_eq!(
        Id128::from_bytes([0xff; 16]).to_string(),
        "ffffffffffffffffffffffffffffffff"
    );
}

#[test]
fn id128_strict_byte_and_text_widths() {
    let bytes = [0; 64];
    for len in 0..=64 {
        if len != 16 {
            assert_eq!(
                Id128::try_from(&bytes[..len]),
                Err(Id128ParseError::InvalidByteLength { actual: len })
            );
        }
        if len != 32 {
            assert_eq!(
                Id128::from_hex(&"0".repeat(len)),
                Err(Id128ParseError::InvalidHexLength { actual: len })
            );
        }
    }
    for text in [
        "00112233-4455-6677-8899-aabbccddeeff",
        "0x00112233445566778899aabbccddeeff",
        " 00112233445566778899aabbccddeeff",
        "00112233445566778899aabbccddeeff\n",
    ] {
        assert_eq!(
            Id128::from_hex(text),
            Err(Id128ParseError::InvalidHexLength { actual: text.len() })
        );
    }
}

#[test]
fn id128_rejects_every_noncanonical_digit_position() {
    for index in 0..32 {
        for invalid in [b'A', b'F', b'G', b'g', b'/', b':', b' ', b'-', 0] {
            let mut text = [b'0'; 32];
            text[index] = invalid;
            let text = std::str::from_utf8(&text).unwrap();
            assert_eq!(
                Id128::from_hex(text),
                Err(Id128ParseError::InvalidHexDigit { index })
            );
        }
    }
    // Correct byte count is insufficient: UTF-8 multibyte text is not hex.
    let unicode = format!("é{}", "0".repeat(30));
    assert_eq!(unicode.len(), 32);
    assert_eq!(
        Id128::from_hex(&unicode),
        Err(Id128ParseError::InvalidHexDigit { index: 0 })
    );
}

#[test]
fn id128_sampled_roundtrips_and_order_are_plain_byte_identity() {
    let mut state = 0x361f_290a_c948_b507;
    let mut previous = Id128::from_bytes([0; 16]);
    for _ in 0..10_000 {
        let mut bytes = [0; 16];
        bytes[..8].copy_from_slice(&next_bits(&mut state).to_le_bytes());
        bytes[8..].copy_from_slice(&next_bits(&mut state).to_le_bytes());
        let id = Id128::from_bytes(bytes);
        let expected: String = bytes
            .iter()
            .flat_map(|&byte| [byte >> 4, byte & 15])
            .map(|digit| char::from_digit(u32::from(digit), 16).unwrap())
            .collect();
        assert_eq!(id.to_string(), expected);
        assert_eq!(Id128::from_hex(&expected), Ok(id));
        assert_eq!(id.cmp(&previous), bytes.cmp(previous.as_bytes()));
        assert_eq!(id.cmp(&previous), expected.cmp(&previous.to_string()));
        let duplicate = Id128::try_from(bytes.as_slice()).unwrap();
        assert_eq!(HashSet::from([id, duplicate]).len(), 1);
        previous = id;
    }
}
