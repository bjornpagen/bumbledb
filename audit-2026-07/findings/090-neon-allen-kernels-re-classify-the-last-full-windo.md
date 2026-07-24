## NEON Allen kernels re-classify the last full window when len is lane-aligned — always, on the chunked path

category: perf | severity: low | verdict: CONFIRMED | finder: engine:kernel
outcome: fixed 26f44a97

### Summary

Both NEON Allen kernels (and the broadcast-constant variant) run `n / lanes` full windows and then unconditionally run an overlapped tail window at `n - lanes`. When `n` is a multiple of the lane width, the main loop's last window (base `n - lanes`) and the tail window are the same window computed twice. The dense scan path chunks at `SCAN_CHUNK = 256`, so every non-final chunk is both 8- and 16-aligned: the code kernel does 33 windows of work for 32 windows of output (+3.1%) and the filter kernel 17 for 16 (+6.25%) on every full chunk. The kernel's own doc comment says the overlap re-classifies "up to 7 pairs" — the aligned case re-classifies 8 (a full window), so the code diverges from its own stated contract; this is unintended waste, not a measured trade.

### Evidence (verified in source)

- `crates/bumbledb/src/exec/kernel/neon.rs:152-168` — `allen_code_batch_neon`: `let mut left = n / 8;` countdown over full windows at bases 0, 8, ..., `8*(n/8 - 1)`.
- `crates/bumbledb/src/exec/kernel/neon.rs:170-182` — unconditional tail: `let tail = n - 8; allen_code_window(..., out.add(tail))`. When `n % 8 == 0`, `tail == ` the last full-window base: an exact duplicate.
- `crates/bumbledb/src/exec/kernel/neon.rs:207,220-227` — `allen_code_batch_const_neon`: identical shape.
- `crates/bumbledb/src/exec/kernel/neon.rs:269,286-290` — `allen_filter_batch_neon`: `let mut left = n / 16;` plus unconditional tail at `n - 16`; duplicate when `n % 16 == 0`. (The test-only spill arm at 321/330 shares the shape.)
- `crates/bumbledb/src/exec/kernel/neon.rs:126-128` — the doc comment: "re-classifying up to 7 pairs is free of both branches and a scalar tail". False in the aligned case (8 pairs / 16 codes — a whole window), which confirms the duplication was not the intended design.
- `crates/bumbledb/src/exec/kernel/allen.rs:176,189-190` — `const SCAN_CHUNK: usize = 256;` and `let len = SCAN_CHUNK.min(n - base);`: every non-final chunk of the dense filter-position scans (`allen_filter_columns`, `allen_filter_columns_const`) has `len == 256`, hitting the aligned case in both kernels on every full chunk.
- `crates/bumbledb/src/exec/kernel/allen.rs:216,232,248` plus the `debug_assert!(n >= 8/16)` at neon.rs:140/199/240 — the dispatch guarantees `n >= lanes`, so `(n - 1) / lanes` never underflows.

### Bench impact

On the 256-chunk dense scan path: a constant +3.1% of the code kernel's window arithmetic and +6.25% of the filter kernel's, on every full chunk, forever. The batch lanes (`probe_pass.rs:322`, `run_node.rs:353`) also pay the duplicate whenever their batch length happens to be lane-aligned. Low severity — a few percent of two already-fast kernels — but it is pure dead work erased by a one-character-class change.

### Suggested fix

Set the full-window count to `(n - 1) / 8` (resp. `(n - 1) / 16`). Coverage check: `k = (n-1)/lanes` full windows cover `0 .. k*lanes`, and `k*lanes >= n - lanes` for all `n >= lanes`, so the unconditional tail at `n - lanes` covers the remainder exactly once when aligned and overlaps (idempotently, as today) only when unaligned. The representation is preserved: no new branch, the countdown `sub`+`cbnz` back edge and the unconditional tail are unchanged, and `(n-1)/lanes` compiles to `sub`+`lsr` — no scalar flag writers — so the `scripts/check-asm.sh` flag-free gate (which greps the kernel symbols for `cmp|csel|adds|ccmp|bl`) still passes. Output bytes are bit-identical, so the bit-identity tests (`tests.rs:1071`, `tests.rs:1110`) are unaffected. Also correct the neon.rs:126-128 doc comment ("up to 7 pairs") to match, or let the fix make it true. Apply the same change to the test-only `allen_filter_batch_neon_spill_arm` (neon.rs:321) so the T7 A/B pin keeps comparing identical window walks.
