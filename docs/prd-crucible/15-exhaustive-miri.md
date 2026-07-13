# PRD 15 — Exhaustive enumeration, Miri, and ASAN: the small worlds proven whole

**Depends on:** 03 (if Q1 adopted, the portable reference bodies are the
Miri-interpretable lane), 11 (ASAN rides the fuzz targets). Independent
of 12–14.
**Modules:** `crates/bumbledb/tests/` (or the kernel modules'
`cfg(test)`) for the exhaustive suites; `scripts/` for the Miri and
ASAN invocations; no engine logic changes.
**Authority:** the fuzzing charter's complement: where a domain is
finite and small, random exploration is strictly worse than exhaustive
enumeration — enumerate it and close the question forever. Miri and
ASAN then cover the one axis neither differential nor fuzzer sees:
undefined behavior that happens to produce right answers today.
**Representation move:** none. This PRD converts three "surely fine"
beliefs into checked totalities.

## Context (decided shape)

**Exhaustive suites** (plain `#[test]`s, ignored-by-default where slow,
each with its domain-size arithmetic in a comment):

1. **All 8,192 Allen masks:** every 13-bit relation mask × the interval
   config classes → the vectorized configuration kernel agrees with the
   scalar classifier on every cell; plus the converse involution
   (converse(converse(m)) == m) and composition-table spot laws over
   the full mask space.
2. **`closed_member` boundaries:** every member index 0..=255 × the
   `[u64;4]` bitset → membership agrees with a naive bit walk;
   including the all-set, empty, and single-word-boundary (63/64,
   127/128, 191/192) patterns exhaustively.
3. **Encoding order-preservation:** for each of the six value types,
   the key encoding is order-preserving over an exhaustive small domain
   (i64 across the sign boundary at byte granularity; interval
   endpoint-pair ordering over a dense small grid; str/bytes prefix
   laws over all strings of length ≤3 from a small alphabet). Domain
   sizes chosen so the suite runs in seconds; the arithmetic comment
   proves exhaustiveness of the CLAIMED domain, not vibes.

**Miri lane** — scoped honestly: LMDB is FFI, so `cargo miri` cannot
cross `heed`. The lane is the PURE modules: encodings, kernels
(reference/portable bodies; NEON intrinsics are non-interpretable —
the scalar/portable twins run instead, which is exactly why PRD 03's Q1
matters), SWAR, condition folding, Allen algebra, bitset subsets. A
`scripts/miri.sh` runs the enumerated test list (`cargo miri
test -p bumbledb <filters>`) on aarch64 AND cross-interpreted
`--target x86_64-unknown-linux-gnu` (Miri interprets foreign targets —
this catches endianness/width assumptions in the scalar kernels for
free).

**ASAN lane** — `RUSTFLAGS=-Zsanitizer=address` over the fuzz targets
(cargo-fuzz's native mode: `cargo fuzz run <target> -s address`), which
covers the FFI boundary Miri cannot: LMDB map handling, the unsafe
key-slice reads at the heed seam. `scripts/fuzz.sh` (PRD 16) grows the
`-s address` flag; this PRD proves each target RUNS under ASAN (1k
iterations each) and documents any suppressions (expected: none;
any suppression is a conflict block, not a shrug).

## Technical direction

1. Write the three exhaustive suites against current behavior — these
   are pins in the policy-8 sense and also permanent tests; they land
   green with no engine edits (any failure is a trophy: stop, fix in
   its own commit, record).
2. `scripts/miri.sh` with the module filter list IN the script and a
   comment per exclusion naming the FFI reason — the exclusion list is
   the honest boundary, auditable.
3. Run the full Miri lane both targets; fix what it finds (expected
   surface: none in pure modules; the unsafe blocks from PRD 01's
   edition port are prime candidates if anything).
4. ASAN smoke per fuzz target; record iteration counts and outcomes in
   this file.

## Passing criteria

- `[test]` The three exhaustive suites green, each with its
  domain-arithmetic comment; total mask count 8,192 asserted in the
  Allen suite (the loop bound is the claim).
- `[shape]` `scripts/miri.sh` exists, green on aarch64-apple-darwin AND
  x86_64 cross-interpretation; every exclusion commented with its FFI
  reason.
- `[test]` Every fuzz target completes 1k iterations under ASAN with
  zero reports and zero suppressions.
- `[gate]` Workspace gates green.

## Doc amendments (rule 5)

The testing/measurement doc: the "small worlds" section (what is
enumerated exhaustively and therefore never fuzzed), the Miri lane's
honest scope, and the ASAN lane's coverage claim.
