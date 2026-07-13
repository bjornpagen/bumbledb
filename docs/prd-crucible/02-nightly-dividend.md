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
