## Fold extent asserts — the documented safety guard for get_unchecked — wrap in release

category: bug | severity: low | verdict: CONFIRMED | finder: engine:kernel
outcome: fixed 26f44a97

### Summary

The three strided fold kernels in `crates/bumbledb/src/exec/kernel/fold.rs` guard their `unsafe { values.get_unchecked(..) }` bodies with a single extent assert:

```rust
assert!(stride > 0 && (count == 0 || (count - 1) * stride + offset < values.len()));
```

The multiplication is unchecked `usize` arithmetic. The workspace root `Cargo.toml` has no `[profile]` section, there is no `.cargo/config.toml`, and no `RUSTFLAGS` anywhere sets `overflow-checks`, so release builds use the default (off) and the multiply wraps silently. A wrapped product can satisfy the assert while the per-element indices `(i + lane) * stride + offset` — computed with the same wrapping arithmetic — index out of bounds into `get_unchecked`: undefined behavior behind the very assert the SAFETY comments cite as the whole invariant. The guard is not total over the functions' input type; it only happens to hold for the extents current callers pass.

### Evidence (all verified against the working tree)

- `crates/bumbledb/src/exec/kernel/fold.rs:22` — the assert in `fold_sum_biased_i64`, guarding the unsafe reads at fold.rs:35 and fold.rs:41 ("SAFETY: the extent assert above covers every index", fold.rs:34).
- `crates/bumbledb/src/exec/kernel/fold.rs:61` — the same assert in `fold_sum_u64`, guarding fold.rs:70 and fold.rs:76.
- `crates/bumbledb/src/exec/kernel/fold.rs:93` — the same assert in `fold_min_max_u64`, whose strided path `fold_min_max_u64_strided` (fold.rs:181–204) does unchecked reads at fold.rs:193 and fold.rs:200 with "SAFETY: the caller asserted the strided extent" (fold.rs:192).
- Root `Cargo.toml` contains only `[workspace]`, `[workspace.lints.rust]` (`unsafe_code = "deny"`, opted out per-block via `#[expect(unsafe_code, reason = "the localized unsafe operation has a documented safety invariant")]`, fold.rs:17–20), and `[workspace.lints.clippy]` — no `[profile.release]`, and `grep -rn "overflow-checks" **/Cargo.toml` returns nothing.
- Containment (why severity is low): `crates/bumbledb/src/lib.rs:101` is `pub(crate) mod exec`, so the kernels are crate-internal. Current callers pass real extents: `exec/sink/aggregate/sink.rs:100–121` always passes `stride = 1` (which takes the *checked* dense-slice path `&values[offset..offset + count]`, fold.rs:24/63/95, and never reaches the unsafe strided body); `exec/sink/aggregate/fold_batch.rs:164–206` passes `stride = arity`, `offset = first·arity + word`, `count = survivors.len()` with u32 row indices over an in-memory batch, so the product cannot overflow today.
- Tests cannot catch this: `exec/kernel/tests.rs` exercises strides 1,2,3,4,7 with valid extents, and in debug/test profiles `overflow-checks` defaults to *on*, so an overflowing extent would panic inside the assert rather than exhibit the release wrap. The divergence is release-only by construction.

The in-repo aggregate research doc (`docs/research/aggregate-comparisons.md`) covers comparison semantics, not this kernel guard, and does not bear on the verdict.

### Failure scenario

`fold_sum_u64(&[0u64; 1], /*stride*/ 1 << 63, /*offset*/ 0, /*count*/ 3)` in a release build: the guard computes `(3−1)·2^63 mod 2^64 = 0`, `0 + 0 < 1` passes; `stride ≠ 1` selects the strided path; the scalar tail loop at `i = 1` executes `values.get_unchecked(1 · 2^63)` — an out-of-bounds read, undefined behavior. Unreachable from today's call sites, but any future caller (or an extent derived from corrupted metadata) whose `count·stride` overflows `usize` slips through the same way, and the `# Panics` docs ("Only on a programmer-invariant violation: the strided extent exceeding `values`", fold.rs:12–15) promise a guarantee the code does not deliver.

### Suggested fix

Make the guard total with one cold checked computation per call (zero cost in the loop):

```rust
assert!(stride > 0
    && count.checked_sub(1).is_none_or(|c| c
        .checked_mul(stride)
        .and_then(|x| x.checked_add(offset))
        .is_some_and(|end| end < values.len())));
```

Or, per the repo's representation-first doctrine (docs/design/representation-first.md — parse, don't validate): take the extent as an already-parsed sub-slice / typed extent so the illegal extent is unrepresentable and the assert disappears entirely. Alternatively (belt-and-suspenders, weaker), set `overflow-checks = true` in `[profile.release]` — but that converts the UB into a panic rather than making the invariant hold, and taxes every release multiply.
