## CountingAllocator does not forward alloc_zeroed, replacing calloc with malloc+memset under the obs build

category: bench-honesty | severity: low | verdict: CONFIRMED | finder: engine:interval-allen
outcome: fixed 537064d8

### Summary

The feature-gated counting allocator (`alloc-counter`, surfaced to the bench as `obs`) overrides only `alloc`, `dealloc`, and `realloc`. `GlobalAlloc::alloc_zeroed` therefore takes the trait's default implementation — `self.alloc(layout)` followed by `ptr::write_bytes(ptr, 0, size)` — instead of delegating to `System.alloc_zeroed`, which on unix uses `libc::calloc` and can hand back lazily-zeroed pages without touching them. Under measurement, every `vec![0; n]`-style allocation pays an explicit O(n) memset and immediately dirties every page. The event and byte counts stay correct (the fallback still routes through the counted `alloc`), but the module's SAFETY comment — "every method delegates directly to `System`" — is factually false, and the measured build's timing/resident-set behavior for zero-filled buffers diverges from the production allocator's beyond the counting the instrument sanctions.

Scope correction versus the original finding: the published night pins are out of the blast radius. The exposure is confined to obs-build diagnostic runs (`--alloc` windows) whose timed cold/post-commit executions allocate the image slabs.

### Evidence (all verified against the working tree and the local toolchain source)

- `crates/bumbledb/src/alloc_counter.rs:62-95` — the `unsafe impl GlobalAlloc for CountingAllocator` block contains exactly three methods: `alloc` (63), `dealloc` (72), `realloc` (81). No `alloc_zeroed`.
- `crates/bumbledb/src/alloc_counter.rs:59-60` — the SAFETY comment: "every method delegates directly to `System`". Untrue for `alloc_zeroed`, which never reaches `System.alloc_zeroed`.
- Toolchain source (nightly-2026-07-12): `core/src/alloc/global.rs:219-227` — default `alloc_zeroed` = `self.alloc` + `write_bytes(0)`; `std/src/sys/alloc/unix.rs:31-42` — `System.alloc_zeroed` calls `libc::calloc` when `align <= MIN_ALIGN && align <= size` (true for `vec![0u8; n]` and `vec![0u64; n]` on aarch64-apple-darwin, MIN_ALIGN 16). The calloc bypass is real.
- Zeroed-alloc sites in sanctioned windows: `crates/bumbledb/src/image/build.rs:86-87` (`let words = vec![0u64; word_len]; let bytes = vec![0u8; byte_len];` — the image-rebuild slabs, row-count-scaled, allocated on the first execution after a commit) and `crates/bumbledb/src/image/cardinality.rs:36` (`slots: vec![0; capacity]`). Smaller sites in planning/validation (`plan/planner/plan.rs:47,83`, `ir/validate/strata.rs:119,179,182`).
- Blast-radius bounds, checked:
  - `docs/architecture/61-bench-lanes.md` ("The night runbook", Build bullet): the main timed lanes build `cargo build --release -p bumbledb-bench` WITHOUT `obs`; the `obs` build goes "into its own target dir for `sweep-commit`" only. Published read/write/cold pins run under the production allocator — unaffected.
  - `crates/bumbledb-bench/src/sweep.rs` (module doc, lines ~50-55): `sweep-commit` measures the engine's `judgment_*` trace spans; I found no `vec![0;n]`/`from_elem` sites in the commit/judgment path, so the one obs timed lane in the night has no alloc_zeroed traffic inside its spans.
  - `crates/bumbledb/src/exec/colt/force.rs:30-32` and `grow.rs:17-18`: COLT pool cold growth uses `Vec::resize(.., 0)`, which reserves via `alloc`/`realloc` and writes zeros itself in every build — never `alloc_zeroed` — so the cold COLT window behaves identically with or without the counter.
  - `crates/bumbledb-bench/src/harness/measure.rs:80-110`: the alloc-window mode arms the counter and takes timing samples in the same measurement loop, so any `--alloc` run's `Measurement` pairs counter readings with wall times taken under the degraded allocator.
- Counts are unaffected: the default `alloc_zeroed` calls `self.alloc`, so events and bytes are billed exactly once, same as a forwarded implementation would. The gate tests (`tests/alloc_gate.rs`, `tests/alloc_census.rs`) are count-based and unharmed.

### Bench impact

An obs-build run that times a cold or post-commit execution — the windows `alloc_counter.rs:27-33` explicitly documents as sanctioned — allocates the image slabs at `image/build.rs:86-87` through malloc+memset instead of calloc: one extra full pass over the slab bytes plus immediate dirtying of every page, where the production build gets lazily-zeroed pages. For a bench-scale image (row_count × columns × 8 bytes, MBs) that is tens to hundreds of microseconds of divergence per rebuild and an inflated live-page footprint in exactly the windows the alloc-window diagnostic exists to characterize. The published night pins are not contaminated (non-obs builds), which is why severity is low rather than medium — but the instrument's contract, stated in its own module doc as "the benchmark's memory observability" (alloc_counter.rs:1-4) and in the SAFETY comment as pure delegation, is violated: the counter is supposed to add counting and nothing else.

### Suggested fix

Add the forwarding override to the impl at `alloc_counter.rs:62`:

```rust
unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
    ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
    let bytes = layout.size() as u64;
    ALLOC_BYTES.fetch_add(bytes, Ordering::Relaxed);
    LIVE_BYTES.fetch_add(bytes, Ordering::Relaxed);
    // SAFETY: forwarded contract.
    unsafe { System.alloc_zeroed(layout) }
}
```

This counts one allocation event plus bytes exactly like `alloc` (so the gate and census semantics are unchanged) and restores the SAFETY comment's claim to truth.
