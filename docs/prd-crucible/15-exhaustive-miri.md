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

## Results (2026-07-13, executed at 6c8af0c + this change)

**Exhaustive suites — all landed green against current behavior with
zero engine edits (no trophies from enumeration).** The three suites
are 13 plain `#[test]`s totalling 0.46 s natively; nothing needed
`#[ignore]`. Every test carries its domain-size arithmetic; the counted
domains are asserted in the tests themselves.

1. *Allen* — `exec::kernel::tests::
   exhaustive_all_8192_masks_times_all_configuration_classes`: all
   2¹³ = 8,192 masks (loop bound counted and asserted) × 784
   configuration-class pairs (C(8,2)² over the endpoint set
   {0,1,2,3,4,MAX−2,MAX−1,MAX} — every 4-endpoint order type, rays and
   unsigned extremes included; all 13 basics asserted present) =
   6,422,528 cells: the vectorized code+filter kernel pipeline agrees
   with the scalar classifier on every one. `allen::tests::
   exhaustive_converse_involution_over_all_8192_masks` walks the full
   mask space (count asserted 8,192). `allen::tests::
   exhaustive_composition_table_spot_laws` enumerates the whole 13 × 13
   composition table from 46,656 interval triples on the 0..=8 grid
   (complete: a witness needs ≤ 6 distinct endpoints) and pins the
   identity row/column, the hand-provable singletons (b;b, a;a, m;m,
   d;d, s;d, f;d), o;o = {b,m,o}, b;bi = FULL, the converse
   anti-homomorphism over all 169 cells, and e ∈ r;r⁻¹.
2. *closed_member* — `schema::tests::closed_member::
   exhaustive_closed_member_matches_the_naive_bit_walk`: 834 patterns
   (257 prefixes covering empty/all-set/63-64/127-128/191-192, 257
   suffix complements, 256 singletons, 64 splitmix words) × 269 ids
   (all 256 in-range + 13 out-of-range probes) = 224,346 cells
   (count asserted) against a naive (word, bit)-coordinate walk.
3. *Encoding order preservation* — all ordered pairs per type (order
   AND injectivity): Bool 2² = 4; i64 677² = 458,329 (byte-granularity
   domain across the sign boundary, size asserted); u64 605² = 366,025;
   str intern-id word 278² = 77,284 (id order only — value order stays
   refused); bytes<N> 84² = 7,056 over all NUL-free strings of length
   ≤ 3 (prefix law included, with the NUL/pad-collision boundary pinned
   explicitly); interval u64 276² = 76,176 and i64 300² = 90,000 grid
   pairs (rays and element extremes included).

**Miri lane** — `scripts/miri.sh`, green on aarch64-apple-darwin AND
cross-interpreted x86_64-unknown-linux-gnu; **zero UB findings**. Every
exclusion is commented in the script with its reason: LMDB/heed test
fixtures are FFI (out), the hand-NEON Allen kernel is non-interpretable
intrinsics (skipped natively, RUN on the cross pass through the scalar
reference dispatch, so the whole Allen kernel surface is interpreted on
one target), the `exhaustive_` enumerations and five wordmap scale
contracts are budget skips whose logic runs through representative
subsets (the wordmap differential itself scales to 256 ops/round under
`cfg!(miri)`; natively unchanged at 2,000). Two infrastructure facts
recorded: (a) `cargo miri setup` builds the sysroot on first use;
(b) the cross pass needs `scripts/miri-cross-cc.sh` — lmdb-master-sys's
build script compiles LMDB's C for the requested target and this host
has no linux cross toolchain, so a host-arch stand-in compile satisfies
the build graph (under Miri the staticlib is never linked and the lane
never calls into LMDB by construction; the shim carries the rationale).

**ASAN lane** — `cargo fuzz run <target> -s address -- -runs=1000`,
strictly sequential, **zero AddressSanitizer reports, zero
suppressions** (corpus replay counts as runs, so targets with > 1k
seeds run their whole corpus):

| target | runs completed | outcome |
| --- | --- | --- |
| theory | 1,000 | clean (1 s) |
| ops | 3,381 (full 3,379-seed corpus) | clean (54 s, peak RSS 177 MB) |
| query | 4,738 (full 3,329-seed corpus) | clean at `-rss_limit_mb=4096` (693 s, peak RSS 3,339 MB) — see below |
| rewrites | 4,858 (full 4,835-seed corpus) | clean (37 s, peak RSS 619 MB) |
| crash | 1,000 | clean (64 s) |

*The one flag, dispositioned:* query's first session died at
libFuzzer's **default** `-rss_limit_mb=2048` (`out-of-memory (used:
2050Mb; limit: 2048Mb)`) during seed-corpus replay. Autopsy: live heap
at the kill was 41 MB; 231 MB sat in ASAN's quarantine across 2.56 M
chunks with 13.7 M cumulative chunk records; every top holder is a
libFuzzer-internal frame; the flagged input replays clean alone under
default limits (10.5 s). This is ASAN quarantine/metadata accounting
across the largest-corpus target (the only one carrying bundled
SQLite's uninstrumented-but-interposed C), not an engine leak. The
recorded remedy is a libFuzzer *resource* knob — `-rss_limit_mb=4096`
for this target under ASAN — no error was suppressed, no suppression
file exists, no engine code changed. PRD 16's orchestrator should carry
that flag for query's ASAN mode.

**Gates:** `cargo test -p bumbledb` green (796 lib tests + integration
suites, 0 failed); `cargo clippy --workspace --all-targets -- -D
warnings` green; `cargo fmt --all --check` green; `scripts/miri.sh`
green on both targets. Fingerprint surface untouched (tests, scripts,
and docs only; the one non-test source line is a `cfg!(miri)` op-count
scale inside an existing wordmap test module).

**Trophies: none.** All three enumerations pinned current behavior on
the first run; Miri found no UB in the pure modules on either target;
ASAN found no memory error in any target. The campaign's two
infrastructure walls (the linux cross-CC gap, the query ASAN RSS
accounting) are recorded above with their dispositions.
