## colt gather_segment's get_unchecked invariant has no optimized-build or UB-detection referee left

category: bug | severity: medium | verdict: CONFIRMED | finder: r2:concurrency-unsafe-ffi

### Summary

The unchecked-gather interior of the COLT executor — `crates/bumbledb/src/exec/colt/gather.rs` — reads `words.get_unchecked(position as usize)` (line 197) and `bytes.get_unchecked(position as usize)` (line 205) under a cross-module invariant: every position in `segment` was minted as a row index of the exact image whose columns are being read. The invariant spans the executor's view builder, the force pass, and Colt's no-retarget rule; locally it is checked only by `debug_assert!` at lines 175 and 202, which compile out of release.

Every lane that could referee this invariant in optimized or UB-detecting form is gone:

- The fuzz corpus-replay lane (built optimized WITH debug-assertions) and the ASAN lane both died with the 2026-07-20 fuzzing hard-delete — recorded in `docs/architecture/60-validation.md` § "The fuzzing apparatus — deleted (the deletion record)", which explicitly names "CI's corpus-replay lane" and the ASAN lane "covering the FFI boundary Miri cannot reach" as deleted. `.github/workflows/ci.yml:35-37` confirms no fuzzing lanes remain; no `fuzz/` directory exists.
- The Miri lane cannot reach the site: `scripts/miri.sh:26-35` excludes `exec::colt` because "its test fixtures build real stores" — verified real, `exec/colt/tests.rs` opens `storage::env::Environment` over a `TempDir` and commits through LMDB.
- No release-with-debug-assertions build exists anywhere: no `[profile]` section in any workspace manifest, and the only release-mode test in CI is the allocation gate (`scripts/check.sh:32`, `cargo test --features alloc-counter --test alloc_gate --release`), which counts allocations, not answers or bounds.

What remains is debug-mode `cargo test --workspace` (check.sh:18), where the debug_asserts are active over the deterministic tests and seeded differentials — a referee only for inputs the fixed corpus already exercises, blind to optimized codegen and to UB.

The SAFETY comment at gather.rs:177-196 is candid about all of this — it records the referee's death and the Miri wall in its own text. This finding is the code agreeing with itself that its hottest unsafe island is unwatched.

### Evidence (all verified in the working tree)

- `crates/bumbledb/src/exec/colt/gather.rs:175` — `debug_assert!(segment.iter().all(|&p| (p as usize) < words.len()));`
- `crates/bumbledb/src/exec/colt/gather.rs:197` — `let word = unsafe { *words.get_unchecked(position as usize) };` (bytes twin at 202/205)
- `crates/bumbledb/src/exec/colt/gather.rs:186-196` — SAFETY comment: "the release-mode standing referee — the fuzz corpus replay, built optimized WITH debug-assertions — died with the fuzzing apparatus (the 2026-07-20 hard-delete ruling)… (The Miri lane cannot reach this site: colt's test fixtures open LMDB…)"
- `scripts/miri.sh:33-35` — "colt's probe logic is pure, but its test fixtures build real stores, so it is out with the rest"
- `docs/architecture/60-validation.md:912-1042` — the deletion record: `fuzz/` crate, `scripts/fuzz.sh`, CI's corpus-replay lane, and the ASAN lane all deleted 2026-07-20
- `crates/bumbledb/src/exec/colt/iter.rs:121,161` — the two callers: `gather_segment` sits on the batch-gather hot path of level iteration
- `crates/bumbledb/src/image/build.rs:361-379` — the pooled `TransientImage` slot (the finder's "build.rs:361-379"; correct path is `src/image/build.rs`): per-round refill under a "ping-pong discipline" that is comment-enforced for position lists. The `fill` body's `Arc::get_mut` precondition (build.rs:449-455) does representationally protect a live view's image bytes from in-place rewrite — but position lists are plain `u32` buffers with no tie to the image that minted them, so pairing a stale, longer position list with a refilled smaller image when building a new view is prevented by nothing but discipline.

The Free Join paper (docs/free-join-paper, COLT lazy-trie section) treats subtrie leaves as tuple indices into the underlying relation — exactly the positions-index-this-image invariant; the paper offers no bounds mechanism, so the code's referee gap is the whole story.

### Failure scenario

A future defect anywhere along the invariant's span — most plausibly in the fixpoint driver, where transient images are refilled per round while position buffers flow between rounds — pairs one view's survivor positions with a shorter column. In release: an out-of-bounds read with no assert, no Miri, no ASAN, no replay lane. The symptom is silent wrong join keys (garbage words folded into results), not a crash, so even the answer-checking lanes catch it only if their corpus happens to hit the broken path.

### Suggested fix

Any one restores a referee; (c) is the representational fix the project doctrine prefers:

(a) Split colt's pure probe/gather logic from its LMDB fixtures so a synthetic-image unit test runs under the Miri lane. This is genuinely feasible: `TransientImage::refill` (image/build.rs:404) already builds a full `RelationImage` from encoded word rows with zero storage involvement, so a store-free colt fixture requires no new machinery — miri.sh's own comment concedes "colt's probe logic is pure."

(b) Reinstate one release-with-debug-assertions replay binary over a small pinned corpus (a `[profile.release-checked]` inherit + one CI step).

(c) Make the invariant representational: have `View` mint a `Positions<'image>` witness tied to the image whose columns gather reads, so a stale-positions/new-image pairing stops compiling — this also hardens the fixpoint pool's comment-enforced half of the ping-pong discipline.
