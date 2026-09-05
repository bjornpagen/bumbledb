use super::decode::{
    decode_i64, decode_padded_fixed_bytes, interval_i64_from_words, interval_u64_from_words,
};
use super::encode::{encode_interval_i64, encode_interval_u64};
use super::*;
use crate::encoding::FieldDecodeError;
use crate::error::CorruptionError;
use bumbledb_theory::schema::{FixedIntervalElement, IntervalElement};

fn encode_fixed_bytes(raw: &[u8], out: &mut Vec<u8>) {
    out.extend_from_slice(FixedBytesValue::new(raw).padded());
}

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

fn mixed_layout() -> FactLayout {
    FactLayout::new(&[
        ValueType::Bool,
        ValueType::Bool,
        ValueType::U64,
        ValueType::I64,
        ValueType::String,
        ValueType::FixedBytes { len: 12 },
        ValueType::Interval {
            element: IntervalElement::U64,
        },
        ValueType::Interval {
            element: IntervalElement::I64,
        },
    ])
}

#[test]
fn layout_offsets_are_cumulative_widths_with_no_padding() {
    let layout = mixed_layout();
    assert_eq!(layout.field_count(), 8);

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
        ValueRef::String(InternId::from_raw(7)),
        ValueRef::bytes(&[0xAA; 12]),
        ValueRef::IntervalU64(
            bumbledb_theory::Interval::<u64>::new(3, u64::MAX).expect("nonempty interval"),
        ),
        ValueRef::IntervalI64(
            bumbledb_theory::Interval::<i64>::new(i64::MIN, -5).expect("nonempty interval"),
        ),
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

    expected.extend_from_slice(&[0xAA; 12]);
    expected.extend_from_slice(&[0x00; 4]);
    expected.extend_from_slice(&encode_interval_u64(
        bumbledb_theory::Interval::<u64>::new(3, u64::MAX).expect("nonempty interval"),
    ));
    expected.extend_from_slice(&encode_interval_i64(
        bumbledb_theory::Interval::<i64>::new(i64::MIN, -5).expect("nonempty interval"),
    ));
    assert_eq!(fact, expected);
}

#[test]
fn field_bytes_slices_equal_independent_encodings() {
    let layout = mixed_layout();
    let mut fact = Vec::new();
    encode_fact(&mixed_values(), &layout, &mut fact);

    assert_eq!(field_bytes(layout.encoded(&fact), 0), &[0x01]);
    assert_eq!(field_bytes(layout.encoded(&fact), 1), &[0x00]);
    assert_eq!(field_bytes(layout.encoded(&fact), 2), encode_u64(u64::MAX));
    assert_eq!(field_bytes(layout.encoded(&fact), 3), encode_i64(i64::MIN));
    assert_eq!(field_bytes(layout.encoded(&fact), 4), encode_u64(7));
    let mut padded = Vec::new();
    encode_fixed_bytes(&[0xAA; 12], &mut padded);
    assert_eq!(field_bytes(layout.encoded(&fact), 5), padded);
    assert_eq!(
        field_bytes(layout.encoded(&fact), 6),
        encode_interval_u64(
            bumbledb_theory::Interval::<u64>::new(3, u64::MAX).expect("nonempty interval")
        )
    );
    assert_eq!(
        field_bytes(layout.encoded(&fact), 7),
        encode_interval_i64(
            bumbledb_theory::Interval::<i64>::new(i64::MIN, -5).expect("nonempty interval")
        )
    );
}

#[test]
fn append_field_matches_stored_fact_slices() {
    let layout = FactLayout::new(&[
        ValueType::Bool,
        ValueType::U64,
        ValueType::I64,
        ValueType::String,
        ValueType::FixedBytes { len: 12 },
        ValueType::Interval {
            element: IntervalElement::U64,
        },
        ValueType::Interval {
            element: IntervalElement::I64,
        },
        ValueType::FixedInterval {
            element: FixedIntervalElement::U64,
            width: 5,
        },
        ValueType::FixedInterval {
            element: FixedIntervalElement::I64,
            width: 3,
        },
    ]);
    let values = [
        ValueRef::Bool(true),
        ValueRef::U64(u64::MAX),
        ValueRef::I64(i64::MIN),
        ValueRef::String(InternId::from_raw(7)),
        ValueRef::bytes(&[0xAA; 12]),
        ValueRef::IntervalU64(
            bumbledb_theory::Interval::<u64>::new(3, u64::MAX).expect("nonempty interval"),
        ),
        ValueRef::IntervalI64(
            bumbledb_theory::Interval::<i64>::new(i64::MIN, -5).expect("nonempty interval"),
        ),
        ValueRef::IntervalU64(
            bumbledb_theory::Interval::<u64>::fixed(9, 5).expect("inside the Q2 bound"),
        ),
        ValueRef::IntervalI64(
            bumbledb_theory::Interval::<i64>::fixed(-2, 3).expect("inside the Q2 bound"),
        ),
    ];
    let mut fact = Vec::new();
    encode_fact(&values, &layout, &mut fact);
    assert_eq!(fact.len(), layout.fact_width());
    for (idx, &value) in values.iter().enumerate() {
        let mut appended = Vec::new();
        append_field(value, layout.field_type(idx), &mut appended);
        let sliced = field_bytes(layout.encoded(&fact), idx);
        assert_eq!(
            appended.as_slice(),
            sliced,
            "field {idx}: append_field diverges from the stored-fact slice"
        );
    }
}

#[test]
fn decode_field_round_trips_every_type() {
    let layout = mixed_layout();
    let values = mixed_values();
    let mut fact = Vec::new();
    encode_fact(&values, &layout, &mut fact);
    for (idx, expected) in values.iter().enumerate() {
        assert_eq!(decode_field(layout.encoded(&fact), idx), Ok(*expected));
    }
}

#[test]
fn decode_field_surfaces_corruption() {
    let layout = mixed_layout();
    let mut fact = Vec::new();
    encode_fact(&mixed_values(), &layout, &mut fact);
    fact[0] = 0x02;
    assert_eq!(
        decode_field(layout.encoded(&fact), 0),
        Err(FieldDecodeError::InvalidBool(0x02))
    );
    fact[0] = 0x01;
    fact[1] = 0x03;
    assert_eq!(
        decode_field(layout.encoded(&fact), 1),
        Err(FieldDecodeError::InvalidBool(0x03))
    );
    fact[1] = 0x00;

    fact[50..58].copy_from_slice(&encode_u64(0));

    let mut corrupt = [0u8; 16];
    let (corrupt_start, corrupt_end) = corrupt.split_at_mut(8);
    corrupt_start.copy_from_slice(&encode_u64(3));
    corrupt_end.copy_from_slice(&encode_u64(0));
    assert_eq!(
        decode_field(layout.encoded(&fact), 6),
        Err(FieldDecodeError::InvalidInterval(corrupt))
    );
    fact[50..58].copy_from_slice(&encode_u64(u64::MAX));

    fact[39] = 0x5A;

    let &tail = field_bytes(layout.encoded(&fact), 5)
        .last_chunk()
        .expect("bytes<12> spans two whole words");
    assert_eq!(
        decode_field(layout.encoded(&fact), 5),
        Err(FieldDecodeError::NonzeroFixedBytesPad(tail))
    );
    fact[39] = 0x00;
    assert_eq!(
        decode_field(layout.encoded(&fact), 5),
        Ok(ValueRef::bytes(&[0xAA; 12]))
    );
}

#[test]
fn typed_decode_reads_the_layout_arm() {
    let layout = mixed_layout();
    let mut fact = Vec::new();
    encode_fact(&mixed_values(), &layout, &mut fact);
    let view = layout.encoded(&fact);
    assert_eq!(decode_bool_at(view, 0), Ok(true));
    assert_eq!(decode_bool_at(view, 1), Ok(false));
    assert_eq!(decode_fixed_bytes(view, 5), Ok(&[0xAA; 12][..]));
    assert_eq!(
        decode_interval_u64(view, 6),
        Ok(bumbledb_theory::Interval::<u64>::new(3, u64::MAX).expect("nonempty interval"))
    );
    assert_eq!(
        decode_interval_i64(view, 7),
        Ok(bumbledb_theory::Interval::<i64>::new(i64::MIN, -5).expect("nonempty interval"))
    );
}

#[test]
fn append_field_writes_layout_bytes_width() {
    let mut out = Vec::new();
    append_field(
        ValueRef::bytes(&[0xAA; 16]),
        ValueType::FixedBytes { len: 8 },
        &mut out,
    );
    assert_eq!(out, vec![0xAA; 8]);
}

#[test]
fn fixed_bytes_round_trip_at_pad_boundaries() {
    for len in [1usize, 7, 8, 9, 63, 64] {
        let raw: Vec<u8> = (0..len)
            .map(|i| u8::try_from(i % 251).unwrap() + 1)
            .collect();
        let mut padded = Vec::new();
        encode_fixed_bytes(&raw, &mut padded);
        assert_eq!(padded.len(), len.div_ceil(8) * 8);
        assert_eq!(&padded[..len], &raw[..]);
        assert!(padded[len..].iter().all(|&b| b == 0));
        let decoded = decode_padded_fixed_bytes(&padded, u16::try_from(len).unwrap())
            .expect("zero pad decodes");
        assert_eq!(decoded.padded(), &padded[..]);
    }
}

#[test]
fn fixed_bytes_padded_order_is_byte_order() {
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

fn rand_interval_u64(rng: &mut Lcg) -> (u64, u64) {
    loop {
        let (a, b) = (rng.next(), rng.next());
        if a != b {
            return (a.min(b), a.max(b));
        }
    }
}

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
    for (start, end) in [
        (i64::MIN, i64::MAX),
        (i64::MIN, i64::MIN + 1),
        (i64::MAX - 1, i64::MAX),
        (0, 1),
        (-1, i64::MAX),
    ] {
        assert_eq!(
            interval_i64_from_words(encode_interval_i64(
                bumbledb_theory::Interval::<i64>::new(start, end).expect("nonempty interval")
            ))
            .map(bumbledb_theory::Interval::bounds),
            Ok((start, end))
        );
    }
    for (start, end) in [(0, u64::MAX), (0, 1), (u64::MAX - 1, u64::MAX)] {
        assert_eq!(
            interval_u64_from_words(encode_interval_u64(
                bumbledb_theory::Interval::<u64>::new(start, end).expect("nonempty interval")
            ))
            .map(bumbledb_theory::Interval::bounds),
            Ok((start, end))
        );
    }

    let mut rng = Lcg(0x0101);
    for _ in 0..1_000 {
        let (start, end) = rand_interval_u64(&mut rng);
        assert_eq!(
            interval_u64_from_words(encode_interval_u64(
                bumbledb_theory::Interval::<u64>::new(start, end).expect("nonempty interval")
            ))
            .map(bumbledb_theory::Interval::bounds),
            Ok((start, end))
        );
        let (start, end) = (
            start.cast_signed().min(end.cast_signed()),
            start.cast_signed().max(end.cast_signed()),
        );
        assert_eq!(
            interval_i64_from_words(encode_interval_i64(
                bumbledb_theory::Interval::<i64>::new(start, end).expect("nonempty interval")
            ))
            .map(bumbledb_theory::Interval::bounds),
            Ok((start, end))
        );
    }
}

#[test]
fn interval_encoding_orders_by_start_then_end() {
    let mut rng = Lcg(0x0202);
    for i in 0..1_000 {
        let x = rand_interval_u64(&mut rng);

        let y = if i % 2 == 0 {
            rand_interval_u64(&mut rng)
        } else {
            rand_interval_u64_from(&mut rng, x.0)
        };
        assert_eq!(
            encode_interval_u64(
                bumbledb_theory::Interval::<u64>::new(x.0, x.1).expect("nonempty interval")
            )
            .cmp(&encode_interval_u64(
                bumbledb_theory::Interval::<u64>::new(y.0, y.1).expect("nonempty interval")
            )),
            x.cmp(&y),
            "u64 encoding order diverges from tuple order for {x:?} vs {y:?}"
        );
        let (xi, yi) = (
            (x.0.cast_signed(), x.1.cast_signed()),
            (y.0.cast_signed(), y.1.cast_signed()),
        );

        if xi.0 < xi.1 && yi.0 < yi.1 {
            assert_eq!(
                encode_interval_i64(
                    bumbledb_theory::Interval::<i64>::new(xi.0, xi.1).expect("nonempty interval")
                )
                .cmp(&encode_interval_i64(
                    bumbledb_theory::Interval::<i64>::new(yi.0, yi.1).expect("nonempty interval")
                )),
                xi.cmp(&yi),
                "i64 encoding order diverges from tuple order for {xi:?} vs {yi:?}"
            );
        }
    }
}

#[test]
fn interval_decode_rejects_start_at_or_beyond_end() {
    for (start, end) in [(5u64, 5u64), (9, 3), (u64::MAX, 0)] {
        let mut bytes = [0; 16];
        bytes[..8].copy_from_slice(&encode_u64(start));
        bytes[8..].copy_from_slice(&encode_u64(end));
        assert_eq!(
            interval_u64_from_words(bytes),
            Err(FieldDecodeError::InvalidInterval(bytes))
        );
    }
    for (start, end) in [(-2i64, -2i64), (4, -4), (i64::MAX, i64::MIN)] {
        let mut bytes = [0; 16];
        bytes[..8].copy_from_slice(&encode_i64(start));
        bytes[8..].copy_from_slice(&encode_i64(end));
        assert_eq!(
            interval_i64_from_words(bytes),
            Err(FieldDecodeError::InvalidInterval(bytes))
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

#[test]
fn exhaustive_interval_encoding_orders_by_endpoint_pair_on_the_grid() {
    let mut u64_points: Vec<u64> = (0..=20).collect();
    u64_points.extend([u64::MAX - 2, u64::MAX - 1, u64::MAX]);
    let mut u64_intervals = Vec::new();
    for (i, &s) in u64_points.iter().enumerate() {
        for &e in &u64_points[i + 1..] {
            u64_intervals.push((s, e));
        }
    }
    assert_eq!(u64_intervals.len(), 276, "C(24,2) intervals");
    for &x in &u64_intervals {
        for &y in &u64_intervals {
            assert_eq!(
                encode_interval_u64(
                    bumbledb_theory::Interval::<u64>::new(x.0, x.1).expect("nonempty interval")
                )
                .cmp(&encode_interval_u64(
                    bumbledb_theory::Interval::<u64>::new(y.0, y.1).expect("nonempty interval")
                )),
                x.cmp(&y),
                "u64 {x:?} vs {y:?}"
            );
        }
    }

    let mut i64_points: Vec<i64> = (-10..=10).collect();
    i64_points.extend([i64::MIN, i64::MIN + 1, i64::MAX - 1, i64::MAX]);
    i64_points.sort_unstable();
    let mut i64_intervals = Vec::new();
    for (i, &s) in i64_points.iter().enumerate() {
        for &e in &i64_points[i + 1..] {
            i64_intervals.push((s, e));
        }
    }
    assert_eq!(i64_intervals.len(), 300, "C(25,2) intervals");
    for &x in &i64_intervals {
        for &y in &i64_intervals {
            assert_eq!(
                encode_interval_i64(
                    bumbledb_theory::Interval::<i64>::new(x.0, x.1).expect("nonempty interval")
                )
                .cmp(&encode_interval_i64(
                    bumbledb_theory::Interval::<i64>::new(y.0, y.1).expect("nonempty interval")
                )),
                x.cmp(&y),
                "i64 {x:?} vs {y:?}"
            );
        }
    }
}

#[test]
fn nullary_fact_layout_is_empty() {
    let layout = FactLayout::new(&[]);
    assert_eq!(layout.fact_width(), 0);
    let mut fact = Vec::new();
    encode_fact(&[], &layout, &mut fact);
    assert!(fact.is_empty());
}

// (`lean/Bumbledb/Values.lean: FixedU64.not_ray`).

fn fixed_layout(element: FixedIntervalElement, width: u64) -> FactLayout {
    FactLayout::new(&[ValueType::U64, ValueType::FixedInterval { element, width }])
}

#[test]
fn interval_words_reads_through_the_layout_width() {
    let general = ValueType::Interval {
        element: IntervalElement::U64,
    };
    assert_eq!(general.width(), 16);
    let encoded =
        encode_interval_u64(bumbledb_theory::Interval::<u64>::new(3, 9).expect("nonempty"));
    assert_eq!(interval_words(general, &encoded), Some((3, 9)));
    assert_eq!(interval_words(general, &encoded[..8]), None);

    let fixed = ValueType::FixedInterval {
        element: FixedIntervalElement::U64,
        width: 5,
    };
    assert_eq!(fixed.width(), 8);
    let start = encode_u64(10);
    assert_eq!(interval_words(fixed, &start), Some((10, 15)));
    assert_eq!(interval_words(fixed, &encoded), None);

    assert_eq!(interval_words(ValueType::U64, &encode_u64(1)), None);
}

#[test]
fn fixed_interval_round_trips_one_word() {
    for (start, width) in [(0u64, 1u64), (3, 5), (1 << 40, 1 << 20), (u64::MAX - 3, 1)] {
        let layout = fixed_layout(FixedIntervalElement::U64, width);
        assert_eq!(layout.fact_width(), 16, "8-byte scalar + 8-byte start");
        let interval =
            bumbledb_theory::Interval::<u64>::fixed(start, width).expect("in-domain fixed value");
        let mut fact = Vec::new();
        encode_fact(
            &[ValueRef::U64(9), ValueRef::IntervalU64(interval)],
            &layout,
            &mut fact,
        );
        assert_eq!(field_bytes(layout.encoded(&fact), 1), encode_u64(start));
        assert_eq!(
            decode_field(layout.encoded(&fact), 1),
            Ok(ValueRef::IntervalU64(interval))
        );
    }
    for (start, width) in [(i64::MIN, 7u64), (-1, 2), (0, 1), (i64::MAX - 3, 1)] {
        let layout = fixed_layout(FixedIntervalElement::I64, width);
        let interval =
            bumbledb_theory::Interval::<i64>::fixed(start, width).expect("in-domain fixed value");
        let mut fact = Vec::new();
        encode_fact(
            &[ValueRef::U64(9), ValueRef::IntervalI64(interval)],
            &layout,
            &mut fact,
        );
        assert_eq!(field_bytes(layout.encoded(&fact), 1), encode_i64(start));
        assert_eq!(
            decode_field(layout.encoded(&fact), 1),
            Ok(ValueRef::IntervalI64(interval))
        );
    }
}

#[test]
fn fixed_interval_decode_rejects_a_start_at_the_q2_bound() {
    for width in [1u64, 5, 1 << 33] {
        let layout = fixed_layout(FixedIntervalElement::U64, width);
        let bound = u64::MAX - width;
        for start in [bound, bound + 1, u64::MAX] {
            let mut fact = Vec::new();
            encode_fact(
                &[ValueRef::U64(0), ValueRef::U64(start)],
                &layout,
                &mut fact,
            );
            assert_eq!(
                decode_field(layout.encoded(&fact), 1),
                Err(FieldDecodeError::InvalidFixedIntervalStart(encode_u64(
                    start
                ))),
                "start {start} under width {width} sits at/past the Q2 bound"
            );
        }
        let inside = bound - 1;
        let mut fact = Vec::new();
        encode_fact(
            &[ValueRef::U64(0), ValueRef::U64(inside)],
            &layout,
            &mut fact,
        );
        assert_eq!(
            decode_field(layout.encoded(&fact), 1),
            Ok(ValueRef::IntervalU64(
                bumbledb_theory::Interval::<u64>::fixed(inside, width).expect("inside the bound")
            ))
        );
    }

    let layout = fixed_layout(FixedIntervalElement::I64, 4);
    let mut fact = Vec::new();
    encode_fact(
        &[ValueRef::U64(0), ValueRef::I64(i64::MAX - 4)],
        &layout,
        &mut fact,
    );
    assert_eq!(
        decode_field(layout.encoded(&fact), 1),
        Err(FieldDecodeError::InvalidFixedIntervalStart(encode_i64(
            i64::MAX - 4
        )))
    );
}

/// The fixed encoding is trivially the scalar embedding
/// (`lean/Bumbledb/Values.lean: encode_fixed_order_u64`): the one stored word
/// is `encode_u64`/`encode_i64` of the start, so the exhaustive scalar suites
/// above ARE this family's order proof.
#[test]
fn exhaustive_fixed_interval_start_word_preserves_start_order() {
    for width in [1u64, 2, 255, 1 << 32, u64::MAX - 2] {
        let layout = fixed_layout(FixedIntervalElement::U64, width);
        let ceiling = u64::MAX - width;
        let mut starts = std::collections::BTreeSet::new();
        starts.extend(0..=64u64);
        starts.extend((0..=8).map(|k| ceiling.saturating_sub(k + 1)));
        let starts: Vec<u64> = starts.into_iter().filter(|s| *s < ceiling).collect();
        let mut encoded = Vec::new();
        for &start in &starts {
            let interval =
                bumbledb_theory::Interval::<u64>::fixed(start, width).expect("inside the Q2 bound");
            assert_eq!(interval.end(), start + width, "the derived end is exact");
            let mut fact = Vec::new();
            encode_fact(
                &[ValueRef::U64(0), ValueRef::IntervalU64(interval)],
                &layout,
                &mut fact,
            );
            encoded.push(field_bytes(layout.encoded(&fact), 1).to_vec());
        }
        for (i, x) in starts.iter().enumerate() {
            for (j, y) in starts.iter().enumerate() {
                assert_eq!(
                    encoded[i].cmp(&encoded[j]),
                    x.cmp(y),
                    "width {width}: {x} vs {y}"
                );
            }
        }
    }

    let layout = fixed_layout(FixedIntervalElement::I64, 3);
    let starts: Vec<i64> = (-40..=40).collect();
    let encoded: Vec<Vec<u8>> = starts
        .iter()
        .map(|&start| {
            let interval =
                bumbledb_theory::Interval::<i64>::fixed(start, 3).expect("inside the Q2 bound");
            assert_eq!(interval.end(), start + 3, "the derived end is exact");
            let mut fact = Vec::new();
            encode_fact(
                &[ValueRef::U64(0), ValueRef::IntervalI64(interval)],
                &layout,
                &mut fact,
            );
            field_bytes(layout.encoded(&fact), 1).to_vec()
        })
        .collect();
    for (i, x) in starts.iter().enumerate() {
        for (j, y) in starts.iter().enumerate() {
            assert_eq!(encoded[i].cmp(&encoded[j]), x.cmp(y), "{x} vs {y}");
        }
    }
}

#[test]
fn decode_values_keyed_never_resolves_a_projected_field() {
    use bumbledb_theory::Value;
    use bumbledb_theory::schema::FieldId;
    let layout = mixed_layout();
    let mut fact = Vec::new();
    encode_fact(&mixed_values(), &layout, &mut fact);
    // Projection (u64 field 2, str field 4): the resolver must never see

    let supplied = [Value::U64(u64::MAX), Value::String(Box::from("supplied"))];
    let decoded = super::decode::decode_values_keyed(
        layout.encoded(&fact),
        &[FieldId(2), FieldId(4)],
        &supplied,
        |id| panic!("projected field resolved through the dictionary (id {id})"),
    )
    .expect("decode");
    assert_eq!(decoded[2], supplied[0]);
    assert_eq!(decoded[4], supplied[1]);

    let plain = super::decode_values(layout.encoded(&fact), |id| {
        assert_eq!(id, 7);
        Ok(Box::from("resolved"))
    })
    .expect("decode");
    for idx in [0, 1, 3, 5, 6, 7] {
        assert_eq!(decoded[idx], plain[idx]);
    }
}

/// Id128 physical words: the sixteen exact bytes, byte order = total
/// order; decode is total and returns the same identity (E-CODEC).
#[test]
fn id128_field_roundtrips_verbatim_and_orders_by_bytes() {
    use bumbledb_theory::Id128;
    let layout = FactLayout::new(&[ValueType::Id128, ValueType::U64]);
    assert_eq!(layout.fact_width(), 24);
    let id = Id128::from_bytes(*b"exact-sixteen-b!");
    let mut fact = Vec::new();
    encode_fact(&[ValueRef::Id128(id), ValueRef::U64(9)], &layout, &mut fact);
    assert_eq!(&fact[..16], &id.to_bytes()[..]);
    assert_eq!(decode_id128(layout.encoded(&fact), 0), id);
    assert_eq!(
        decode_field(layout.encoded(&fact), 0),
        Ok(ValueRef::Id128(id))
    );
    // Byte order is the one total order: no reinterpretation, no words.
    let smaller = Id128::from_bytes([0x00; 16]);
    let larger = Id128::from_bytes([0xff; 16]);
    assert!(encode_id128(smaller) < encode_id128(larger));
    assert_eq!(smaller.cmp(&larger), std::cmp::Ordering::Less);
}

/// Dense float interval physical words: two order keys, lexicographic by
/// `(start, end)`; corrupt words (noncanonical holes, NaN endpoints,
/// empty/inverted bounds) refuse instead of normalizing (F-INTERVAL).
#[test]
fn interval_f64_words_roundtrip_and_reject_corruption() {
    use bumbledb_theory::{F64, Interval};
    let ty = ValueType::Interval {
        element: IntervalElement::F64,
    };
    assert_eq!(ty.width(), 16);
    let layout = FactLayout::new(&[ty]);
    let span = Interval::<F64>::new(F64::NEG_INFINITY, F64::from(2.5)).expect("left ray");
    let mut fact = Vec::new();
    encode_fact(&[ValueRef::IntervalF64(span)], &layout, &mut fact);
    assert_eq!(decode_interval_f64(layout.encoded(&fact), 0), Ok(span));
    assert_eq!(
        decode_field(layout.encoded(&fact), 0),
        Ok(ValueRef::IntervalF64(span))
    );

    // The two halves sort lexicographically by (start, end).
    let low =
        encode_interval_f64(Interval::<F64>::new(F64::from(1.0), F64::from(2.0)).expect("checked"));
    let high =
        encode_interval_f64(Interval::<F64>::new(F64::from(1.0), F64::from(3.0)).expect("checked"));
    assert!(low < high);

    // Corrupt stored words refuse: an order-key hole (negative zero),
    // a NaN endpoint (canonical key but invalid bound), and an empty span.
    let order_key_of = |value: F64| value.to_order_key().to_be_bytes();
    let mut corrupt = Vec::new();
    corrupt.extend_from_slice(&(0x8000_0000_0000_0000u64 ^ u64::MAX).to_be_bytes());
    corrupt.extend_from_slice(&order_key_of(F64::from(1.0)));
    assert!(matches!(
        decode_interval_f64(layout.encoded(&corrupt), 0),
        Err(FieldDecodeError::NonCanonicalF64(_))
    ));
    let mut nan_end = Vec::new();
    nan_end.extend_from_slice(&order_key_of(F64::ZERO));
    nan_end.extend_from_slice(&order_key_of(F64::NAN));
    assert!(matches!(
        decode_interval_f64(layout.encoded(&nan_end), 0),
        Err(FieldDecodeError::InvalidInterval(_))
    ));
    let mut empty = Vec::new();
    empty.extend_from_slice(&order_key_of(F64::from(1.0)));
    empty.extend_from_slice(&order_key_of(F64::from(1.0)));
    assert!(matches!(
        decode_interval_f64(layout.encoded(&empty), 0),
        Err(FieldDecodeError::InvalidInterval(_))
    ));
}
