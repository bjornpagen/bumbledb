use super::*;

fn encode_fixed_bytes(raw: &[u8], out: &mut Vec<u8>) {
    out.extend_from_slice(FixedBytesValue::new(raw).padded());
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

fn i64_byte_granularity_domain() -> Vec<i64> {
    let mut set = std::collections::BTreeSet::new();
    set.extend(-260..=260i64);
    for k in 0..8u32 {
        for byte in [0x01i128, 0x7F, 0x80, 0xFF] {
            let m = byte << (8 * k);
            for candidate in [m - 1, m, m + 1, -m - 1, -m, -m + 1] {
                if let Ok(v) = i64::try_from(candidate) {
                    set.insert(v);
                }
            }
        }
    }
    set.extend([
        i64::MIN,
        i64::MIN + 1,
        i64::MIN + 2,
        i64::MAX - 2,
        i64::MAX - 1,
        i64::MAX,
    ]);
    set.into_iter().collect()
}

#[test]
fn exhaustive_bool_encoding_preserves_order() {
    for x in [false, true] {
        for y in [false, true] {
            assert_eq!(encode_bool(x).cmp(&encode_bool(y)), x.cmp(&y));
        }
    }
}

#[test]
fn exhaustive_i64_encoding_preserves_order_across_the_sign_boundary() {
    let domain = i64_byte_granularity_domain();
    assert_eq!(domain.len(), 677, "the derived byte-granularity domain");
    for &x in &domain {
        for &y in &domain {
            assert_eq!(encode_i64(x).cmp(&encode_i64(y)), x.cmp(&y), "{x} vs {y}");
        }
    }
}

#[test]
fn exhaustive_u64_encoding_preserves_order_at_byte_boundaries() {
    let mut set = std::collections::BTreeSet::new();
    set.extend(0..=520u64);
    for k in 0..8u32 {
        for byte in [0x01u128, 0x7F, 0x80, 0xFF] {
            let m = byte << (8 * k);
            for candidate in [m - 1, m, m + 1] {
                if let Ok(v) = u64::try_from(candidate) {
                    set.insert(v);
                }
            }
        }
    }
    set.extend([u64::MAX - 2, u64::MAX - 1, u64::MAX]);
    let domain: Vec<u64> = set.into_iter().collect();
    assert_eq!(domain.len(), 605, "the derived byte-granularity domain");
    for &x in &domain {
        for &y in &domain {
            assert_eq!(encode_u64(x).cmp(&encode_u64(y)), x.cmp(&y), "{x} vs {y}");
        }
    }
}

#[test]
fn exhaustive_string_id_word_preserves_id_order_only() {
    let mut set = std::collections::BTreeSet::new();
    set.extend(0..=255u64);
    for k in 1..8u32 {
        let m = 1u64 << (8 * k);
        set.extend([m - 1, m, m + 1]);
    }
    set.extend([InternId::SENTINEL.raw() - 1, InternId::SENTINEL.raw()]);
    let domain: Vec<u64> = set.into_iter().collect();
    assert_eq!(domain.len(), 278, "the derived id domain");
    for &x in &domain {
        for &y in &domain {
            assert_eq!(encode_u64(x).cmp(&encode_u64(y)), x.cmp(&y));
        }
    }
}

#[test]
fn exhaustive_fixed_bytes_prefix_laws_over_all_short_strings() {
    let alphabet = [0x01u8, 0x55, 0xAA, 0xFF];
    let mut strings: Vec<Vec<u8>> = Vec::new();
    for &a in &alphabet {
        strings.push(vec![a]);
        for &b in &alphabet {
            strings.push(vec![a, b]);
            for &c in &alphabet {
                strings.push(vec![a, b, c]);
            }
        }
    }
    assert_eq!(strings.len(), 84, "4 + 16 + 64 strings of length <= 3");
    let padded: Vec<Vec<u8>> = strings
        .iter()
        .map(|raw| {
            let mut out = Vec::new();
            encode_fixed_bytes(raw, &mut out);
            assert_eq!(out.len(), 8, "lengths <= 3 pad to one word");
            out
        })
        .collect();
    for (x, px) in strings.iter().zip(&padded) {
        for (y, py) in strings.iter().zip(&padded) {
            assert_eq!(
                px.cmp(py),
                x.cmp(y),
                "padded order diverges from raw order for {x:?} vs {y:?}"
            );
        }
    }

    let (mut with_nul, mut without) = (Vec::new(), Vec::new());
    encode_fixed_bytes(&[0x01, 0x00], &mut with_nul);
    encode_fixed_bytes(&[0x01], &mut without);
    assert_eq!(with_nul, without, "NUL and pad are indistinguishable");
}
