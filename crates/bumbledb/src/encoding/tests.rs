use super::decode::{decode_fixed_bytes, decode_i64, decode_interval_i64, decode_interval_u64};
use super::encode::{encode_interval_i64, encode_interval_u64};
use super::*;
use crate::error::CorruptionError;
use bumbledb_theory::schema::IntervalElement;

/// The byte-level fixtures' `bytes<N>` padder, spelled through the pad
/// law's one owner ([`FixedBytesValue::padded`]) — no second
/// implementation site survives outside it.
fn encode_fixed_bytes(raw: &[u8], out: &mut Vec<u8>) {
    out.extend_from_slice(FixedBytesValue::new(raw).padded());
}

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
            width: None,
        },
        TypeDesc::Interval {
            element: IntervalElement::I64,
            width: None,
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
    // bytes<12>: the 12 raw bytes zero-padded to the 16-byte word boundary.
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
        encode_interval_u64(
            bumbledb_theory::Interval::<u64>::new(3, u64::MAX).expect("nonempty interval")
        )
    );
    assert_eq!(
        field_bytes(&fact, &layout, 7),
        encode_interval_i64(
            bumbledb_theory::Interval::<i64>::new(i64::MIN, -5).expect("nonempty interval")
        )
    );
}

/// The parity law behind the typed-key determinant path (`Key`'s
/// `determinant_read`/`determinant_write`): for EVERY field type — Bool,
/// U64, I64, String, bytes<N>, both general intervals, both fixed-width
/// intervals — the bytes [`append_key_field`] produces for one value
/// equal the span `storage/keys::determinant_image` slices out of the
/// encoded fact at that field. One encoder, one slicer, byte-identical:
/// encoder drift between a key probe and stored determinants is
/// impossible.
#[test]
fn append_key_field_matches_determinant_image_slices() {
    use bumbledb_theory::schema::FieldId;
    let layout = FactLayout::new(&[
        TypeDesc::Bool,
        TypeDesc::U64,
        TypeDesc::I64,
        TypeDesc::String,
        TypeDesc::FixedBytes { len: 12 },
        TypeDesc::Interval {
            element: IntervalElement::U64,
            width: None,
        },
        TypeDesc::Interval {
            element: IntervalElement::I64,
            width: None,
        },
        TypeDesc::Interval {
            element: IntervalElement::U64,
            width: Some(5),
        },
        TypeDesc::Interval {
            element: IntervalElement::I64,
            width: Some(3),
        },
    ]);
    let values = [
        ValueRef::Bool(true),
        ValueRef::U64(u64::MAX),
        ValueRef::I64(i64::MIN),
        ValueRef::String(7),
        ValueRef::fixed_bytes(&[0xAA; 12]),
        ValueRef::IntervalU64(
            bumbledb_theory::Interval::<u64>::new(3, u64::MAX).expect("nonempty interval"),
        ),
        ValueRef::IntervalI64(
            bumbledb_theory::Interval::<i64>::new(i64::MIN, -5).expect("nonempty interval"),
        ),
        ValueRef::FixedIntervalU64(
            bumbledb_theory::Interval::<u64>::fixed(9, 5).expect("inside the Q2 bound"),
        ),
        ValueRef::FixedIntervalI64(
            bumbledb_theory::Interval::<i64>::fixed(-2, 3).expect("inside the Q2 bound"),
        ),
    ];
    let mut fact = Vec::new();
    encode_fact(&values, &layout, &mut fact);
    assert_eq!(fact.len(), layout.fact_width());
    for (idx, &value) in values.iter().enumerate() {
        let mut appended = Vec::new();
        append_key_field(value, &mut appended);
        let mut sliced = crate::storage::keys::DeterminantImage::scratch();
        crate::storage::keys::determinant_image(
            &layout,
            &[FieldId(u16::try_from(idx).expect("nine fields fit u16"))],
            &fact,
            &mut sliced,
        );
        assert_eq!(
            appended.as_slice(),
            sliced.as_bytes(),
            "field {idx}: append_key_field diverges from the stored-fact slice"
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
    // The expected error payload, rebuilt from the same primitives the
    // fixture used: the untouched start half ‖ the zeroed end half.
    let mut corrupt = [0u8; 16];
    let (corrupt_start, corrupt_end) = corrupt.split_at_mut(8);
    corrupt_start.copy_from_slice(&encode_u64(3));
    corrupt_end.copy_from_slice(&encode_u64(0));
    assert_eq!(
        decode_field(&fact, &layout, 6),
        Err(CorruptionError::InvalidInterval(corrupt))
    );
    fact[50..58].copy_from_slice(&encode_u64(u64::MAX));
    // The pad-corruption fixture: a nonzero byte in the bytes<12> field's
    // trailing pad (offsets 26 + 12 .. 26 + 16) is typed corruption —
    // the pad is encoding, not data.
    fact[39] = 0x5A;
    // The bytes<12> field's trailing word, sliced layout-first — the
    // error payload is the field's last whole word.
    let &tail = field_bytes(&fact, &layout, 5)
        .last_chunk()
        .expect("bytes<12> spans two whole words");
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
    // The determinant B-tree's need: memcmp order over the padded encodings of
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
            decode_interval_i64(encode_interval_i64(
                bumbledb_theory::Interval::<i64>::new(start, end).expect("nonempty interval")
            ))
            .map(bumbledb_theory::Interval::bounds),
            Ok((start, end))
        );
    }
    for (start, end) in [(0, u64::MAX), (0, 1), (u64::MAX - 1, u64::MAX)] {
        assert_eq!(
            decode_interval_u64(encode_interval_u64(
                bumbledb_theory::Interval::<u64>::new(start, end).expect("nonempty interval")
            ))
            .map(bumbledb_theory::Interval::bounds),
            Ok((start, end))
        );
    }
    // Random pairs, ordered into valid intervals, both element types.
    let mut rng = Lcg(0x0101);
    for _ in 0..1_000 {
        let (start, end) = rand_interval_u64(&mut rng);
        assert_eq!(
            decode_interval_u64(encode_interval_u64(
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
            decode_interval_i64(encode_interval_i64(
                bumbledb_theory::Interval::<i64>::new(start, end).expect("nonempty interval")
            ))
            .map(bumbledb_theory::Interval::bounds),
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
        // Sign-casting both halves of a valid u64 interval keeps start < end
        // exactly when both halves land on the same side of zero — skip the
        // pairs it inverts.
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

// ---------------------------------------------------------------------
// The exhaustive order-preservation suite (the crucible packet (git ecec1dc3)
// 15-exhaustive-miri.md, suite 3): for each of the six value types, the
// canonical encoding preserves the value order over an exhaustive small
// domain — every ordered pair checked (which pins injectivity too:
// `cmp` equality both ways). Each test carries its domain-size
// arithmetic; the domain is the claim.
// ---------------------------------------------------------------------

/// The i64 domain at byte granularity: the dense sign-boundary window
/// −260..=260 (crossing 0 and the ±255/±256 first-byte boundary), plus
/// every byte-boundary magnitude `v · 256^k` for `v ∈ {0x01, 0x7F,
/// 0x80, 0xFF}`, `k ∈ 0..8`, with both signs and ±1 neighbors (clamped
/// to the i64 range), plus the type extremes — so every encoded byte
/// position is exercised at its carry and sign edges.
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

/// Bool: the whole domain is {false, true} — all 2² = 4 ordered pairs.
/// (false, 0x00) < (true, 0x01) is the entire order claim.
#[test]
fn exhaustive_bool_encoding_preserves_order() {
    for x in [false, true] {
        for y in [false, true] {
            assert_eq!(encode_bool(x).cmp(&encode_bool(y)), x.cmp(&y));
        }
    }
}

/// I64 across the sign boundary at byte granularity: the sign-flipped
/// big-endian encoding preserves numeric order over the whole derived
/// domain ([`i64_byte_granularity_domain`] — 677 values, size asserted),
/// checked on all 677² = 458,329 ordered pairs. `cmp` equality in both
/// directions makes this order preservation AND injectivity.
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

/// U64 at byte granularity: the dense window 0..=520, every
/// byte-boundary magnitude with ±1 neighbors (as in the i64 domain,
/// unsigned), and the top extremes — 605 values (size asserted), all
/// 605² = 366,025 ordered pairs.
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

/// String: the fact encoding is the interned id's big-endian word — the
/// ONLY order it carries is id order (string-value order is refused by
/// design, `docs/architecture/10-data-model.md`: intern ids are
/// meaningless to order, and `Lt`-family operators on str are typed
/// validation errors). Domain: ids 0..=255 exhaustively, the word
/// boundaries 2⁸ᵏ ± 1, and the never-minted sentinel `u64::MAX` — 278
/// values (size asserted), all 278² = 77,284 ordered pairs.
#[test]
fn exhaustive_string_id_word_preserves_id_order_only() {
    let mut set = std::collections::BTreeSet::new();
    set.extend(0..=255u64);
    for k in 1..8u32 {
        let m = 1u64 << (8 * k);
        set.extend([m - 1, m, m + 1]);
    }
    set.extend([
        crate::storage::dict::SENTINEL_ID - 1,
        crate::storage::dict::SENTINEL_ID,
    ]);
    let domain: Vec<u64> = set.into_iter().collect();
    assert_eq!(domain.len(), 278, "the derived id domain");
    for &x in &domain {
        for &y in &domain {
            assert_eq!(encode_u64(x).cmp(&encode_u64(y)), x.cmp(&y));
        }
    }
}

/// bytes<N> prefix laws: ALL byte strings of length 1..=3 over the
/// NUL-free 4-symbol alphabet {0x01, 0x55, 0xAA, 0xFF} — 4 + 4² + 4³ =
/// 84 strings (count asserted), all 84² = 7,056 ordered pairs. Every
/// string pads to the same single 8-byte word, and because the 0x00 pad
/// byte sorts strictly below every alphabet symbol, padded memcmp order
/// equals raw lexicographic order INCLUDING the prefix law (a proper
/// prefix sorts strictly first) — and the encoding is injective over
/// the domain. The engine only ever compares equal declared widths
/// (`TypeDesc::FixedBytes { len }` is per-field), where the law holds
/// for arbitrary bytes; the cross-length half is the mathematical
/// boundary of the claim, and the final assert documents why it needs
/// the NUL-free alphabet.
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
    // The boundary of the claim: a NUL in the value collides with the
    // pad, so the cross-length law requires the NUL-free alphabet (the
    // engine never faces this — widths are fixed per field).
    let (mut with_nul, mut without) = (Vec::new(), Vec::new());
    encode_fixed_bytes(&[0x01, 0x00], &mut with_nul);
    encode_fixed_bytes(&[0x01], &mut without);
    assert_eq!(with_nul, without, "NUL and pad are indistinguishable");
}

/// Interval endpoint-pair ordering on a dense grid, both element types:
/// the 16-byte `start ‖ end` encoding sorts by the `(start, end)` tuple
/// under the element order.
///
/// Domain arithmetic — u64: endpoints {0..=20} ∪ {MAX−2, MAX−1, MAX}
/// (24 values, rays included: end == MAX), so C(24,2) = 276 nonempty
/// intervals and 276² = 76,176 ordered pairs. i64: endpoints {−10..=10}
/// ∪ {MIN, MIN+1, MAX−1, MAX} (25 values), so C(25,2) = 300 intervals
/// and 300² = 90,000 ordered pairs. Every pair checked.
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

// ---------------------------------------------------------------------
// The fixed-width interval family — interval<E, w>: one stored word (the
// start), the end derived from the TYPE's width, the Q2 bound
// (`start + w < MAX_END`) the corruption line
// (`lean/Bumbledb/Values.lean: FixedU64.not_ray`).
// ---------------------------------------------------------------------

/// A one-fixed-field layout per element domain, width `w`.
fn fixed_layout(element: IntervalElement, width: u64) -> FactLayout {
    FactLayout::new(&[
        TypeDesc::U64,
        TypeDesc::Interval {
            element,
            width: Some(width),
        },
    ])
}

#[test]
fn fixed_interval_round_trips_one_word() {
    // The encoding is the start word — 8 bytes, not 16 — and decode
    // re-derives the end from the layout's constant width.
    for (start, width) in [(0u64, 1u64), (3, 5), (1 << 40, 1 << 20), (u64::MAX - 3, 1)] {
        let layout = fixed_layout(IntervalElement::U64, width);
        assert_eq!(layout.fact_width(), 16, "8-byte scalar + 8-byte start");
        let interval =
            bumbledb_theory::Interval::<u64>::fixed(start, width).expect("in-domain fixed value");
        let mut fact = Vec::new();
        encode_fact(
            &[ValueRef::U64(9), ValueRef::FixedIntervalU64(interval)],
            &layout,
            &mut fact,
        );
        assert_eq!(field_bytes(&fact, &layout, 1), encode_u64(start));
        assert_eq!(
            decode_field(&fact, &layout, 1),
            Ok(ValueRef::FixedIntervalU64(interval))
        );
    }
    for (start, width) in [(i64::MIN, 7u64), (-1, 2), (0, 1), (i64::MAX - 3, 1)] {
        let layout = fixed_layout(IntervalElement::I64, width);
        let interval =
            bumbledb_theory::Interval::<i64>::fixed(start, width).expect("in-domain fixed value");
        let mut fact = Vec::new();
        encode_fact(
            &[ValueRef::U64(9), ValueRef::FixedIntervalI64(interval)],
            &layout,
            &mut fact,
        );
        assert_eq!(field_bytes(&fact, &layout, 1), encode_i64(start));
        assert_eq!(
            decode_field(&fact, &layout, 1),
            Ok(ValueRef::FixedIntervalI64(interval))
        );
    }
}

#[test]
fn fixed_interval_decode_rejects_a_start_at_the_q2_bound() {
    // Stored starts at or past `MAX_END − w` would derive a ceiling or
    // overflowed end — corruption, never a value; the boundary's inside
    // edge decodes. Both element ceilings encode to the same word, so
    // one word-domain sweep covers the u64 face...
    for width in [1u64, 5, 1 << 33] {
        let layout = fixed_layout(IntervalElement::U64, width);
        let bound = u64::MAX - width; // start + w == MAX_END: barred
        for start in [bound, bound + 1, u64::MAX] {
            let mut fact = Vec::new();
            encode_fact(
                &[ValueRef::U64(0), ValueRef::U64(start)],
                &layout,
                &mut fact,
            );
            assert_eq!(
                decode_field(&fact, &layout, 1),
                Err(CorruptionError::InvalidFixedIntervalStart(encode_u64(
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
            decode_field(&fact, &layout, 1),
            Ok(ValueRef::FixedIntervalU64(
                bumbledb_theory::Interval::<u64>::fixed(inside, width).expect("inside the bound")
            ))
        );
    }
    // ...and the i64 face rejects at ITS ceiling (`i64::MAX` encodes to
    // the same u64::MAX word — one bound, two domains).
    let layout = fixed_layout(IntervalElement::I64, 4);
    let mut fact = Vec::new();
    encode_fact(
        &[ValueRef::U64(0), ValueRef::I64(i64::MAX - 4)],
        &layout,
        &mut fact,
    );
    assert_eq!(
        decode_field(&fact, &layout, 1),
        Err(CorruptionError::InvalidFixedIntervalStart(encode_i64(
            i64::MAX - 4
        )))
    );
}

/// The fixed encoding is trivially the scalar embedding
/// (`lean/Bumbledb/Values.lean: encode_fixed_order_u64`): the one stored
/// word is `encode_u64`/`encode_i64` of the start, so the exhaustive
/// scalar suites above ARE this family's order proof. This arm pins the
/// residue those suites cannot see: across a dense start grid and width
/// extremes, the stored word ordering equals start ordering AND the
/// derived ends stay exact — dense grid, the Q2 boundary, w extremes.
#[test]
fn exhaustive_fixed_interval_start_word_preserves_start_order() {
    for width in [1u64, 2, 255, 1 << 32, u64::MAX - 2] {
        let layout = fixed_layout(IntervalElement::U64, width);
        let ceiling = u64::MAX - width; // the least barred start
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
                &[ValueRef::U64(0), ValueRef::FixedIntervalU64(interval)],
                &layout,
                &mut fact,
            );
            encoded.push(field_bytes(&fact, &layout, 1).to_vec());
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
    // The i64 face across the sign boundary: start order = word order.
    let layout = fixed_layout(IntervalElement::I64, 3);
    let starts: Vec<i64> = (-40..=40).collect();
    let encoded: Vec<Vec<u8>> = starts
        .iter()
        .map(|&start| {
            let interval =
                bumbledb_theory::Interval::<i64>::fixed(start, 3).expect("inside the Q2 bound");
            assert_eq!(interval.end(), start + 3, "the derived end is exact");
            let mut fact = Vec::new();
            encode_fact(
                &[ValueRef::U64(0), ValueRef::FixedIntervalI64(interval)],
                &layout,
                &mut fact,
            );
            field_bytes(&fact, &layout, 1).to_vec()
        })
        .collect();
    for (i, x) in starts.iter().enumerate() {
        for (j, y) in starts.iter().enumerate() {
            assert_eq!(encoded[i].cmp(&encoded[j]), x.cmp(y), "{x} vs {y}");
        }
    }
}

/// The keyed decode substitutes the caller's supplied values for the
/// projected fields WITHOUT invoking the string resolver — the keyed-get
/// hit path re-derives nothing the caller handed it (the `U` probe
/// already matched the determinant byte-for-byte). Non-projected fields
/// decode exactly as [`decode_values`] does.
#[test]
fn decode_values_keyed_never_resolves_a_projected_field() {
    use bumbledb_theory::Value;
    use bumbledb_theory::schema::FieldId;
    let layout = mixed_layout();
    let mut fact = Vec::new();
    encode_fact(&mixed_values(), &layout, &mut fact);
    // Projection (u64 field 2, str field 4): the resolver must never see
    // the projected string's id — a call is the failure.
    let supplied = [Value::U64(u64::MAX), Value::String(Box::from(*b"supplied"))];
    let decoded =
        super::decode_values_keyed(&fact, &layout, &[FieldId(2), FieldId(4)], &supplied, |id| {
            panic!("projected field resolved through the dictionary (id {id})")
        })
        .expect("decode");
    assert_eq!(decoded[2], supplied[0]);
    assert_eq!(decoded[4], supplied[1]);
    // The unkeyed decode of the same fact agrees everywhere else.
    let plain = super::decode_values(&fact, &layout, |id| {
        assert_eq!(id, 7);
        Ok(Box::from(*b"resolved"))
    })
    .expect("decode");
    for idx in [0, 1, 3, 5, 6, 7] {
        assert_eq!(decoded[idx], plain[idx]);
    }
}
