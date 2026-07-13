# PRD 02 — CheckedInterval: the encoder becomes total

**Depends on:** nothing in this set. The flagship P0.
**Modules:** `crates/bumbledb/src/value.rs` (`Value::IntervalU64/I64`),
`interval.rs` (the host `Interval<T>`), `encoding/encode.rs` (the
debug_asserts at :36 and :45), every `Value::Interval*` construction
site (api ingress, bench naive/translate/corpus_gen, tests), and the
consumers that destructure the raw pairs.
**Authority:** audit P0-1, verified current: the encoder gates
`start < end` with `debug_assert!` — a release-mode no-op — and
`Value` carries raw bounds by documented decision ("dumb data").
Soundness today is recovered downstream by the always-on decode
re-check; the Lean vacuity countermodel says a malformed interval that
reaches dependency checking makes coverage vacuous. The invariant must
move from the decode net to the construction boundary.
**Representation move:** a malformed interval becomes UNCONSTRUCTIBLE
in any value that can reach an encoder, so the decode re-check demotes
from last-line-of-defense to what it should be: a corruption detector
for bytes that were damaged at rest.

## Context (decided shape)

```rust
/// Non-empty half-open interval over an order-preserving element
/// domain. Construction is the only validation site; encoders take
/// this and never raw bounds. The ray is `end == E::MAX_END` — the
/// existing storage sentinel, kept codec-identical; `MAX_END` as a
/// finite endpoint is unrepresentable because `new` requires
/// `start < end` and `ray(MAX_END)` is refused, exactly as the host
/// `Interval<T>` already behaves.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct CheckedInterval<E: IntervalElement> { start: E, end: E }
```

- `CheckedInterval` UNIFIES with the existing host `Interval<T>` — do
  not create a parallel type. The decided move: `Interval<T>` (already
  private-field, Option-constructed, non-Ord, ray-aware) IS the checked
  interval; the work is making `Value` carry it:
  `Value::IntervalU64(Interval<u64>)`, `Value::IntervalI64(Interval<i64>)`
  replacing the raw `(u64, u64)` / `(i64, i64)` payloads.
- `encode_interval_u64/i64` change signature to take `Interval<T>` (or
  its two accessor words) and the `debug_assert!`s at encode.rs:36,45
  are DELETED — there is nothing left to assert. Any call site that
  today passes raw words must obtain an `Interval` first; a site that
  cannot (corruption-path re-encode, if any exists) is a policy-5 stop.
- The spec's `UpperBound::{Finite, Unbounded}` enum is REFUSED as a
  stored representation (codec compatibility + the repo's recorded
  stance "∞ is a value of the representation, not a sentinel",
  interval.rs:28). Instead the ray query surface stays `is_ray()`. This
  refusal is recorded here deliberately — the spec's goal (no magic
  finite/infinite conflation) is already met by `MAX_END`-as-
  unconstructible-finite-endpoint.
- `value_matches`' boundary rejections (`FactShapeError::EmptyInterval`
  etc.) remain — they now guard DESERIALIZED/host-built descriptors,
  and several become structurally unreachable from `Value` itself;
  each arm that becomes unreachable is deleted, not kept defensively.
- `encode_fact`'s debug-only per-field `TypeDesc` asserts
  (encode.rs:108-145) are in scope: decide each — either the call path
  proves the match (document the witness that proves it) or the assert
  is promoted to a typed error. No release-mode-only checks survive on
  the encode path.

## Technical direction

1. Pin first: a lock test that `Interval::new(s, e)` with `s >= e` and
   `ray(MAX_END)` are `None` exists (interval.rs:134-140) — extend with
   the release-relevant lock: no safe public API can produce encoded
   bytes for a malformed interval (construct-through-Value is now
   impossible by type; the test asserts the `Value` variants only
   accept `Interval`).
2. Change the `Value` variants; chase the compiler through every
   construction and destructuring site (engine, bench naive model,
   translate, corpus_gen arms, macros if they construct `Value`).
   Bench generators draw `(start, end)` pairs — they now draw through
   `Interval::new` and re-draw on `None` (the entropy seam's bounded
   draw makes this deterministic; assert the digest pin is unchanged —
   if the pin moves, the generator changed behavior and the PRD is
   mis-executed: stop).
3. Rewire the encoders; delete the debug_asserts; sweep
   encode_fact's debug-only checks per the decision rule above.
4. The decode re-checks (decode.rs:43-63, image/decode.rs:253-255) are
   UNTOUCHED — they are the corruption boundary and stay always-on.

## Passing criteria

- `[shape]` `grep -n "debug_assert" crates/bumbledb/src/encoding/encode.rs`
  → zero hits.
- `[shape]` `Value::IntervalU64`/`IntervalI64` carry `Interval<_>`;
  `grep -rn "IntervalU64(" crates | grep -v "Interval::"` finds no raw
  two-word construction (mechanical: the tuple form no longer compiles).
- `[test]` The corpus digest pin test green, byte-unchanged (the
  generator's draw stream must not move).
- `[test]` Full workspace suite green including bench lib (no skips)
  and `cargo test` in fuzz/; a bounded fuzz smoke per policy 7.
- `[shape]` Every deleted defensive arm listed in the commit body with
  the structural reason it became unreachable.
- `[gate]` Fingerprint pin test byte-untouched and green; clippy
  workspace `-D warnings`; fmt.

## Doc amendments (rule 6)

`10-data-model.md` § intervals: the constructor-boundary sentence
("malformed intervals are unconstructible in any encodable value; the
decode check is a corruption detector"). `60-validation.md`: the
theorem↔evidence row for interval vacuity updates its Rust-evidence
cell to this PRD's shape.
