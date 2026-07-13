# PRD 02 — The nightly dividend: guards deleted by the standard library

**Depends on:** 01.
**Modules:** everywhere the target idioms appear — principally
`crates/bumbledb/src/encoding/`, `storage/keys.rs`, `storage/commit/`,
`image/decode.rs`, `interval/`, `exec/`, plus every test module using the
closure-error idiom; `crates/bumbledb/src/lib.rs` (`#![feature(…)]`
declarations).
**Authority:** README policy 9 (a feature is adopted only where it
deletes code or guards; refusals ledgered); the witness campaign's
census discipline (deletions counted and reconciled).
**Representation move:** dozens of `try_into().expect("fixed-width
slice")` guards exist because stable slices couldn't prove widths at the
type level. The standard library now can. A guard the type system
discharges is a guard deleted — the doctrine's smallest unit.

## Context (decided shape) — the adoption ledger

Each item: the feature, the idiom it kills, the criterion. Adopt ONLY
these four families in this PRD; everything else goes to the refusal
ledger (direction 4).

1. **Fixed-width slice extraction** (`split_first_chunk`,
   `split_last_chunk`, `as_chunks`, `first_chunk`/`last_chunk`, and the
   nightly `slice::as_array`/`get_disjoint_mut` where they fit): the
   `.try_into().expect("fixed-width slice")` and `expect("8-byte
   field")`/`("16-byte field")` family — the census counts ~30–40 sites
   across encoding/decode, keys parsing, judgment's interval-half
   slicing, guard-byte tails, image decode. Each becomes an
   array-pattern destructure or `as_chunks` walk whose width the TYPE
   carries. Where the slice width is a runtime fact (stored bytes), the
   checked variants return Option and the site's EXISTING corruption
   path consumes the None — the corruption semantics must not change
   (same typed errors, pinned by the existing tests).
2. **`let`-chains** (edition 2024): the nested `if let … { if … { if
   let … } } }` staircases — validate.rs, bind.rs, chase conditions,
   verify_store passes are the dense sites. Flatten every staircase the
   chain reaches; no logic change.
3. **`try` blocks** (nightly `try_blocks`): the
   `let x: Result<_> = (|| { … })();` closure-error idiom (sealed_checks
   tests, bench harness sites, anywhere `(||`-immediately-invoked
   appears). Each becomes `try { … }`. Grep-driven: `(|| {` with a `?`
   inside.
4. **Exhaustive-width byte walks** (`array_chunks`/`as_chunks` in
   iterator position) where encode/decode loops hand-index in 8-byte
   steps.

## Technical direction

1. Grep census FIRST, in the commit body: count the
   `fixed-width`/`8-byte`/`16-byte` expect family before and after —
   the delta is the PRD's headline number.
2. Work file-by-file; every conversion is local and behavior-identical.
   The corruption-path conversions (direction 1's runtime-width cases)
   re-run their existing corruption tests untouched — if a test needs
   editing, the conversion changed semantics: revert and re-think.
3. `#![feature(…)]` declarations gain a one-line justification comment
   each, naming this PRD's ledger entry.
4. The refusal ledger (append to this PRD file at execution): evaluated
   and refused, one line + derivation each — expected entries:
   `allocator_api` (the arena is index-based by design — no pointers to
   type), `generic_const_exprs` (bytes<N> word counts — the emitted
   `⌈N/8⌉` arithmetic is three call sites; a still-unstable type-level
   feature buys nothing), `never_type` (no diverging-arm pain exists),
   `specialization` (banned outright — soundness holes), plus whatever
   else was considered. `portable_simd` is NOT refused here — it is
   PRD 03's whole subject.

## Passing criteria

- `[shape]` `grep -rn '"fixed-width slice"\|"8-byte field"\|"8-byte
  half"\|"16-byte field"\|"8-byte word"' crates` → ≤ 5 survivors, each
  with a comment naming why the type cannot carry it (target: 0; the
  allowance is for genuinely dynamic widths).
- `[shape]` `grep -rn "(|| {" crates --include='*.rs'` → zero
  immediately-invoked error closures; `try` blocks stand in their
  place.
- `[shape]` Every `#![feature]` has its justification line; the refusal
  ledger in this file is non-empty and each entry carries a derivation.
- `[test]` Zero test-assertion changes — the whole PRD is
  behavior-preserving; the workspace suite passes with identical counts
  (a corruption-path test edit is a defect, per direction 2).
- `[gate]` Workspace gates green at campaign close.

## Doc amendments (rule 5)

None — standard-library idiom adoption; the toolchain chapter note
landed in PRD 01.

## Refusal ledger (executed 2026-07-13)

Adopted: **`try_blocks` only** (named in `rust-toolchain.toml`). Every
other slice conversion landed on stable APIs (`split_first_chunk` /
`split_last_chunk` / `first_chunk` / `last_chunk` / `as_chunks`) plus
edition-2024 let-chains — no feature needed.

Refused, one line + derivation each:

- `allocator_api` — the arena is index-based by design; there are no
  pointers to type, so a typed allocator has nothing to hold.
- `generic_const_exprs` — `bytes<N>` word counts: the emitted `⌈N/8⌉`
  arithmetic is three call sites; a still-unstable type-level feature
  buys nothing over `div_ceil`.
- `never_type` — no diverging-arm pain exists anywhere in the sweep.
- `specialization` — banned outright: soundness holes.
- `slice_as_array` — considered for the exact-tail key parses;
  `<&[u8; N]>::try_from` carries the same width on stable, so the
  feature deletes nothing.
- `array_chunks` (iterator form) — every fixed-width walk fit the
  stable `as_chunks` view; an unstable iterator adapter deletes
  nothing.
- `split_array` — superseded by the stable `split_first_chunk` /
  `split_last_chunk` family this sweep standardizes on.
- `slice::get_disjoint_mut` — no site takes disjoint mutable
  fixed-width windows; nothing to adopt.

## Results (executed 2026-07-13)

**Fixed-width expect census.** Before: 22 sites matching the criterion
grep (`"fixed-width slice"` ×13, `"8-byte half"` ×4, `"8-byte word"`
×2, `"16-byte field"` ×2, `"8-byte field"` ×1) plus 22 more in the same
family under other messages (`"u64 field is 8 bytes"` ×4, `"ctrl
group"` ×3, test `"8"` ×3, `"8-byte field slice"` ×2, digest
`unwrap()` ×2, and one each of `"8-byte slice"`, `"8-byte trailing
word"`, `"16-byte field slice"`, `"fresh fields are 8 bytes"`,
`"interned fields are 8 bytes"`, `"length checked above"`, `"window
read"`, `"trailing word"`) — **44 total**, every one removed by this
sweep (the counts are the commit diff's own census). After:
**criterion grep = 0**;
**5 family survivors workspace-wide**, each with a comment naming why
the type cannot carry the width:

1. `encoding/decode.rs` `field_word_bytes` — the single funnel for
   every word-field consumer (validate, commit plan, delta insert,
   verify-store, image tests); width is a runtime layout fact.
2. `encoding/decode.rs` `decode_field`'s interval arm — the same
   layout-derived ruling, inline for the one 16-byte shape.
3. `storage/commit/judgment.rs` `check_coverage` — the guard scratch's
   interval tail is a `guard_key` construction invariant (parsed through
   `segment_words`, exactly like a stored guard key).
4. `exec/wordmap/probe.rs` — the mirror-tail window loads at arbitrary
   unaligned indices; the invariant is `ctrl.len() == capacity +
   WINDOW − 1`, not a type.
5. `encoding/tests.rs` — the corruption fixture slices its pad word
   layout-first off a `bytes<12>` field.

Ruling: the two `expect("u64")`s in `storage/commit/tests/commit.rs`
are stored-counter *value*-shape assertions (a test pinning what LMDB
holds), not slice-width guards — out of family, untouched. The
`"positions fit u32"` family is integer narrowing, not slice width —
out of this PRD's scope.

**Key parsing became split chains.** `keys.rs` gained
`parse_fact_key` / `parse_membership_key` / `parse_guard_key` /
`parse_stat_key`, and `parse_reverse_key` was rewritten — in each the
split chain IS the length check and the existing malformed/corruption
path consumes the `None`. The verify-store passes and the scan reader
now share the codec's parsers; `REVERSE_KEY_TAIL_LEN` died with the
manual arithmetic.

**`try` blocks.** Both immediately-invoked error closures became `try`
blocks (`storage/read/scan.rs`, `commit/tests/sealed_checks.rs`); the
third IIFE (`api/db/tests.rs`) existed only to route `?`'s `From`
conversion and became a plain `map_err` — no closure at all. Criterion
grep: zero immediately-invoked error closures remain. (Note: `?` inside
`try` does not yet infer the `From` conversion the closure form did —
the two conversion sites carry an explicit `map_err(Error::from)`.)

**let-chains.** One semantic collapse beyond PRD 01's clippy pass: the
chase evaluator's keyed-shape ladder became a single pattern-literal
`let &[(FieldId(0), k)]`; `decode_fixed_bytes`'s pad check became a
let-chain. The staircase audit found no further collapsible sites —
every remaining nest has a multi-statement or else-bearing inner block.

**`as_chunks` walks.** The image decoder's `FixedBytes` arm now zips
`starts` with the field's word chunks (the manual `8·i` stride died);
`fact_word`'s stride closure indexes `as_chunks` chunks; the colt SWAR
group reads index the ctrl slab as whole 8-byte groups (the regions are
8-aligned by construction — `Map::ctrl_start`'s documented invariant);
the chase evaluator's interval halves read from a two-chunk view. The
image decoder's single-word/interval reads became unsafe array-typed
pointer reads under the arm's existing SAFETY derivation (the width
moved into the type; the bounds argument is unchanged).

All conversions behavior-preserving: zero test-assertion-value changes,
corruption semantics pinned by the existing tests, the corpus digest
pin and the fingerprint pin `63e3b480…` unmoved.
