# Spec-fidelity review 01 — Values (PRD 02)

- **Subsection:** the value universe — `lean/Bumbledb/Values.lean` (all 411 lines) plus its
  Countermodels residents (`lean/Bumbledb/Countermodels.lean:150-201`), against
  `crates/bumbledb/src/value.rs`, `crates/bumbledb/src/interval.rs`,
  `crates/bumbledb/src/encoding.rs`, `crates/bumbledb/src/encoding/encode.rs`,
  `crates/bumbledb/src/encoding/decode.rs`, `crates/bumbledb/src/encoding/tests.rs`,
  and the measure evaluation sites.
- **Date:** 2026-07-14
- **Reviewer:** blind reviewer, pairing #1 (covenant PRD 15)
- **Ledger rows judged:** the nine PRD 02 rows, `lean/Bumbledb/Bridge.lean:87-130`.

## Per-theorem fidelity table

| Lean theorem / definition | Rust site | Premise discharge | Verdict |
|---|---|---|---|
| `interval_nonempty` (Values.lean:176) | `Interval::new` — interval.rs:34, 73 | `h : start < end` discharged by `(start < end).then_some` in both inherent impls; fields private, no `Default`, no struct-literal construction outside the module (grep-verified: `sweep.rs` works on raw word tuples, never mints `Interval`); `decode_field` re-proves via `decode_interval_*` then `new(..).expect` (decode.rs:161-172). Matches Bridge.lean:87-90. | FAITHFUL |
| `Interval.ext` (Values.lean:143) | derived `PartialEq/Eq` on the two data fields — interval.rs:20 | None (proof-irrelevance is free in Rust: no proof field exists). | FAITHFUL |
| `points_halfopen` (Values.lean:185) | `start`/`end` accessors interval.rs:54-62, 93-101; `allen::classify` allen.rs:255; `interval/sweep.rs` | Definitional in Lean (`Iff.rfl`); Rust consumers hold the contract by convention, watched by `adjacency_continues_and_the_minimal_gap_breaks` (sweep.rs:277) and the classify point-set oracle. Matches Bridge.lean:92-95. | FAITHFUL |
| `ray_is_unbounded_tail` (Values.lean:194) | `Interval::ray`/`is_ray` — interval.rs:42-50, 81-89; `MAX_END = u64::MAX`/`i64::MAX` (interval.rs:30, 69) = Lean `U64.maxEnd = 2^64−1`, `I64.maxEnd = 2^63−1` (Values.lean:83, 94) | Lean premise `x < maxEnd` is the Rust point-domain law ("points are `MIN ..= MAX_END − 1`", interval.rs:27-29); `ray(MAX_END)` is `None` exactly as `start < maxEnd` forces (tested interval.rs:163-164). Matches Bridge.lean:97-100. | FAITHFUL |
| `measure_ray_none` (Values.lean:203) | `measure()` — exec/sink.rs:151-153 (`(end != u64::MAX).then(..)`); poison raised as `Error::MeasureOfRay` at exec/run/execute.rs:400-402; kernel path image/view/apply.rs:360 | Ray test on the encoded end word: `u64::MAX` is the encoding of `MAX_END` for **both** elements (identity for u64; bias of `i64::MAX` for i64), so the one word test is exactly `isRay` in both domains. Matches Bridge.lean:102-105. | FAITHFUL |
| `measure_finite` (Values.lean:211) | same `measure()` word subtraction; `Term::Measure`/`FindTerm::Measure` — ir.rs:80, 164 | Lean `gap`: u64 `b − a`; i64 `(b − a).toNat` (Values.lean:109-117). Rust subtracts biased words — the bias cancels, exact for both elements up to `2^64 − 2` in the u64 result word; verified at the extreme by `duration_find_projects_the_measure_i64` (`[MIN, MAX−1) → u64::MAX − 1`, measure.rs:201-234). Matches Bridge.lean:107-110. | FAITHFUL |
| `encode_u64_order_embedding` (Values.lean:262) | `encode_u64` — encode.rs:14-16 (`to_be_bytes`) | Lean models the identity on ℕ; big-endian bytes realize it; the byte-level fact itself is sampled by `exhaustive_u64_encoding_preserves_order_at_byte_boundaries` (tests.rs:503, 605² ordered pairs incl. `u64::MAX`). | FAITHFUL |
| `encode_i64_order_embedding` (Values.lean:270) | `encode_i64` — encode.rs:21-23 (`cast_unsigned() ^ I64_SIGN_BIT`, encoding.rs:168) | Sign-bit XOR on two's complement computes exactly the bias `x + 2^63` of `encodeI64` (Values.lean:229); exhaustive sign-boundary suite (tests.rs:488, 677² pairs incl. `i64::MIN/MAX`) pins order **and** injectivity (`cmp` both ways). Matches Bridge.lean:117-120. | FAITHFUL |
| `encode_interval_order` + `_u64` (Values.lean:297, 306); `lexLt`, `encodeInterval*` (Values.lean:279-290) | `encode_interval_i64`/`_u64` — encode.rs:33-45; `concat_halves` encode.rs:47-52 | Each half order-preserving ⇒ 16-byte memcmp = `lexLt` on the encoded pair; `start < end` premise is the checked input type. Exhaustive grid (tests.rs:617: 276² u64 + 300² i64 pairs, rays and `MIN/MAX` edges included) plus the tiebreak-forcing random suite (tests.rs:364). No `Ord`/`PartialOrd` on `Interval` (interval.rs:20) — the deliberate non-order holds. Matches Bridge.lean:122-125. | FAITHFUL |
| `value_eq_iff_encode_eq` (Values.lean:385); `encodeAt`/`Value.encode` (Values.lean:367-377) | `encode_literal` — encode.rs:80-98; `encode_fact` — encode.rs:107-134 | Per-type injectivity: Bool strict 1 byte, u64/i64 bijective words, fixed-N padded bytes (pad constant per N), interval 16 bytes; str id via `encode_fact`'s `ValueRef::String` arm (encode.rs:120-122) with the per-database caveat carried on the Bridge row (Bridge.lean:127-130). Watched by `encode_fact_matches_independent_field_encodings` (tests.rs:151). Decode strictness (decode.rs:12-87) makes the canonical bytes the *only* accepted bytes. | FAITHFUL (see F1/F2) |
| `ValueType`/`Elem` (Values.lean:316-334) | `TypeDesc` — encoding.rs:37-60; `schema::ValueType` | Six structural types on both sides; no mask arm in either storable type. | FAITHFUL (see F1) |
| `StrId` no-order (Values.lean:339; Countermodels.lean:194-201) | `Error::OrderComparisonOnString` — error.rs:527; ids as bare `u64` (encoding.rs:147) | See F3 — surface behavior matches, mechanism class differs. | FAITHFUL w/ note |
| `FixedBytes n` (Values.lean:345); pad-invisibility narrowing (Values.lean:44-46) | `FixedBytesValue` — encoding.rs:86-132; `encode_fixed_bytes` — encode.rs:60-64 | Pad injectivity for fixed N holds (first N bytes decide); pad-order law and its NUL-alphabet boundary tested (tests.rs:567-605); nonzero pad = corruption (decode.rs:75-87). See F5 on width bounds. | FAITHFUL w/ note |
| `empty_interval_vacuous` countermodel (Countermodels.lean:179) | kept out-of-tree by `Interval::new` returning `Option` | The raw-bounds shape is unconstructible in Rust (private fields) and un-storable (decode rejects `start >= end`, decode.rs:40-64, tested tests.rs:408-430). | FAITHFUL |

## Divergences

### F1 — `crate::value::Value` carries a seventh variant the value universe excludes — class (b), minor

Lean: the universe is six types and "The Allen mask is not a field type … it has no place in
the value universe" (`lean/Bumbledb/Values.lean:29-31`), and `Value` claims to be
"(`crate::value::Value`)" (`lean/Bumbledb/Values.lean:358-361`). Rust: `Value::AllenMask`
is a variant of that very enum (`crates/bumbledb/src/value.rs:52`), with the encoder
guarding by panic (`unreachable!`, `crates/bumbledb/src/encoding/encode.rs:96`). The
storable universe does match (no mask arm in `TypeDesc`, `crates/bumbledb/src/encoding.rs:37-60`;
extension rows fail `value_matches` before the encoder, `crates/bumbledb/src/schema/validate.rs:1084-1087`),
and the exclusion is a documented design fact — but the Lean `Value` mirrors the Rust
*storable* sum, not the literal enum it names. The spec does not determine what a mask
literal is; Rust answers with a panic-guarded extra variant. Imprecise citation, not a bug.

### F2 — the str carrier is split across two Rust types — class (b), minor

Lean `.str`'s carrier is `StrId` and `encodeAt .str = [s.id]`
(`lean/Bumbledb/Values.lean:352, 371`). In Rust, `Value::String` carries raw UTF-8
(`crates/bumbledb/src/value.rs:26`) and `encode_literal` panics on it
(`crates/bumbledb/src/encoding/encode.rs:92-94`); the modeled id encoding is implemented
by `encode_fact`'s `ValueRef::String(u64)` arm (`crates/bumbledb/src/encoding/encode.rs:120-122`).
The "callers peel first" premise is discharged at all three `encode_literal` call sites
(`crates/bumbledb/src/schema/validate.rs:542-546` — `CompiledCheck::Interned`;
`crates/bumbledb/src/schema/validate.rs:1084-1087`; `crates/bumbledb/src/schema/fingerprint.rs:180-183`),
each verified to route `String` elsewhere. The Bridge row honestly cites both mechanisms
(`lean/Bumbledb/Bridge.lean:127-130`), so the ledger is accurate; only the "mirrors
`crate::value::Value`" reading is loose — the mirror of Lean `Value` is really
`Value` ⊎ `ValueRef`.

### F3 — the str-order refusal is a validation rule in Rust, a typing fact in Lean — class (b), minor

Lean machine-checks that no `LT`/`LE`/`Ord` instance exists on `StrId`
(`lean/Bumbledb/Countermodels.lean:194-201`) and the module doc says nominal safety lives
in "host Rust newtypes" (`lean/Bumbledb/Values.lean:8`). Rust has no intern-id newtype:
ids are bare `u64` (`crates/bumbledb/src/encoding.rs:147`), which is `Ord` — the B-tree
in fact orders them (`exhaustive_string_id_word_preserves_id_order_only`,
`crates/bumbledb/src/encoding/tests.rs:534-552`). The query-surface refusal is dynamic:
`Error::OrderComparisonOnString` (`crates/bumbledb/src/error.rs:527`). Observable behavior
matches the spec (no host can order strings through the query surface); the *mechanism*
is a validation error, not a type-level absence. Recorded because the Lean doc's
"typing fact" phrasing overstates the Rust side.

### F4 — the sentinel intern id is unmodeled — class (b), minor

Lean `StrId.id : Nat` is unbounded and every id is a value
(`lean/Bumbledb/Values.lean:339-341`); Rust reserves `SENTINEL_ID = u64::MAX` as
never-minted dictionary state (`crates/bumbledb/src/storage/dict.rs:80`). Unobservable so
long as the mint invariant holds (the order suite deliberately includes the sentinel,
`crates/bumbledb/src/encoding/tests.rs:541-544`); the spec simply doesn't speak here.

### F5 — `fixedBytes n` is total over ℕ in Lean, `1..=64` in Rust — class (b)/(c) flavor, minor

Lean admits `fixedBytes 0` (one-inhabitant carrier) and arbitrarily wide `n`
(`lean/Bumbledb/Values.lean:331, 345`); Rust makes widths outside `1..=MAX_FIXED_BYTES`
undeclarable (`crates/bumbledb/src/encoding.rs:27-31`, asserted at
`crates/bumbledb/src/encoding.rs:101-112`). The spec's extra generality is dead — types no
code implements — but `value_eq_iff_encode_eq` is proved uniformly in `n`, so nothing
false is claimed; the width ceiling is a narrowing the spec did not record (law 5 would
prefer it recorded).

### F6 — the decode/corruption boundary is spec-silent — class (b), minor

`Values.lean` models encode only; the decode side's strictness — `InvalidBool` on any
byte ≠ 0/1 (`crates/bumbledb/src/encoding/decode.rs:12-18`), `InvalidInterval` on
`start >= end` (`decode.rs:40-64`), `NonzeroFixedBytesPad` (`decode.rs:75-87`) — is
behavior the spec does not determine. It *supports* the canonical-bytes theorem (the
canonical encoding is the only accepted encoding), so no contradiction. Two adversarial
sub-notes: (i) the width checks guarding `field_bytes`/`decode_fixed_bytes` are
`debug_assert` only (`crates/bumbledb/src/encoding/decode.rs:76, 106`) — in release a
layout-mismatched caller would mis-slice silently; the premise is discharged structurally
(every caller derives both fact and slice from one `FactLayout`). (ii) the
`MeasureOfRay` payload carries *encoded* words (`crates/bumbledb/src/error.rs:1052-1058`),
so for i64 intervals the error's `start` is the biased word, not the logical value —
diagnostic surface only, no theorem touches it.

## No class (a) findings; no dead theorems

Every theorem in `Values.lean` has a located, live implementing site; every mechanism and
instrument string in the nine PRD 02 ledger rows (`lean/Bumbledb/Bridge.lean:87-130`) was
resolved on disk and the named test fns found (`interval.rs:134, 144, 151, 168`;
`interval/sweep.rs:277`; `api/prepared/tests/measure.rs:160, 201, 419`;
`encoding/tests.rs:47, 72, 151, 364, 488, 503`). The edge inventory — `MAX_END` both
elements, sign boundary at byte granularity, zero-width intervals (unrepresentable both
sides), minimal-width `[x, x+1)`, ray-at-ceiling `ray(MAX)` = `None`, pad/NUL collision
boundary, sentinel id — is exercised by the exhaustive suites with asserted domain sizes.

## GRADE: B

No behavior the spec forbids was found under adversarial reading: the constructors, the
sign-flip bias, the two-half lexicographic law, the ray/measure semantics (including the
`2^64 − 2` i64 extreme), and per-type encoding injectivity all compute exactly the
modeled functions under exactly the modeled premises, with every premise discharged at
the site the ledger claims and watched by genuinely exhaustive instruments. What keeps
this from an A is a cluster of six class-(b) impressions, all minor and half of them
self-documented in the spec: the Lean `Value` names `crate::value::Value` as its mirror
while that enum carries a panic-guarded seventh variant and a raw-bytes str carrier the
model does not admit (F1, F2), the str-order refusal is dynamically enforced where the
spec presents a typing fact (F3), and the sentinel id, width ceiling, and the whole
corruption-checking decode boundary are real engine behavior the spec leaves
undetermined (F4–F6). None is a plausible bug; all are precision debts at the
model-to-code naming seam.
