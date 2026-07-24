## Every filter kernel pays a redundant full-width memset of the output before the cursor-write

category: perf | severity: medium | verdict: CONFIRMED | finder: engine:kernel

### Summary

All dense position-producing filter kernels pre-size their output with `out.resize(start + len, 0)` solely so the branchless cursor-write has indexable Vec length, then `truncate(write)` down to the survivor count. The zero-fill is a fully redundant O(len) store pass: every slot up to the final cursor is overwritten by the cursor-write, and every slot past it is truncated away. Worse than the codes/keep buffers this module already fixed, the trailing `truncate` shrinks `len` back to the survivor count, so the **full-width** zero-fill recurs on **every call** — pooled capacity does not amortize it, and the grow-only high-water trick (`probe_pass.rs::grow_scratch`) is structurally unavailable because `out.len()` is the meaningful survivor count. This is the exact `_platform_memset` disease this codebase has profiled and cured twice elsewhere.

### Evidence (all verified in source)

- `crates/bumbledb/src/exec/kernel/filter.rs:167-168` — `push_matching`: `let start = out.len(); out.resize(start + col.len(), 0);` … `out.truncate(write)` at 181. Serves `filter_eq_u64`, `filter_range_u64`, `filter_eq_u8`.
- `crates/bumbledb/src/exec/kernel/filter.rs:192-193` — `push_matching_pair`, same shape, truncate at 209. Serves `filter_point_in_u64`, `filter_any_point_in_u64`.
- `crates/bumbledb/src/exec/kernel/filter.rs:122-123` — `filter_duration_range_u64`, same shape, truncate at 152.
- `crates/bumbledb/src/exec/kernel/filter.rs:244-253` — `write_survivor_bits` stores `pos` at the cursor **on every lane** (`*out.get_unchecked_mut(write) = pos`), so every slot in `[start, write)` is overwritten before truncate; the zero-fill's only role is legitimizing the Vec length for the `&mut [u32]` slice.
- `crates/bumbledb/src/exec/kernel/allen.rs:193-200` — `filter_chunked` repeats resize(+len,0)/truncate per 256-element chunk, i.e. full-width zero-fill in aggregate for both Allen dense scans.
- `crates/bumbledb/src/image/view/apply.rs:385` — the `View::All` measure path passes a fresh `Vec::new()` into `filter_duration_range_u64`: full alloc plus a 4·len-byte zero pass before a single survivor is known.
- The disease is measured, not speculative, in this very codebase: `allen.rs:82-84` records that the codes buffer's full per-batch refill "was pure `_platform_memset` on the profile" (cured by capacity-retained resize with no clear), and `probe_pass.rs:595-601` records `clear` + `resize(n, 0)` re-memset at **3.7% of `meets_chain`** (cured by `grow_scratch`). Neither cure reaches the filter kernels because their `truncate` resets `len` every call.
- Fix precedent in-module: `crates/bumbledb/src/exec/kernel/compact.rs:43-49` — raw-pointer cursor writes + `set_len(write)` under the module's documented unsafe law (safe reference twin + bit-identity property test; the twin-test pattern is live in `exec/kernel/tests/filter_mask_twin.rs`).
- Doctrine: `docs/design/representation-first.md` lens — the zero value is a sentinel that a spare-capacity representation erases; the Vec length requirement is being satisfied by a store pass instead of by `set_len` over a proven-initialized prefix.

### Bench impact

Every dense predicate scan (`filter_eq`/`range`/`point_in`/`any_point_in`/`duration`, plus both Allen dense scans) issues two output-side store passes per visited element instead of one: the memset writes 4·len bytes, then the cursor-write stores once per lane. The relative cost is largest on highly selective scans over large columns (survivors ≪ len, so nearly all memset+cursor stores are to slots that end up truncated) and on the fresh-Vec `View::All` measure path where it is an up-front alloc+memset before any work. The codebase's own profiles priced this pattern at 3.7% of a bench lane (`meets_chain`) at a smaller surface.

### Suggested fix

Replace resize-then-truncate with `out.reserve(len)` + cursor writes through `out.as_mut_ptr()` into spare capacity + one `unsafe { out.set_len(write) }`, exactly the `compact.rs` shape: `write` starts at the initialized `start = out.len()`, advances at most once per visited position, and every slot in `[start, write)` is written before `set_len` exposes it (u32 carries no drop obligation; writing through a raw pointer into uninit spare capacity is sound for Copy words). `write_survivor_bits` takes a raw base pointer (or `spare_capacity_mut`) instead of `&mut [u32]`; the scalar tails write through the same pointer. Ship under the module's existing unsafe law: keep the safe reference twins as the differential oracle and add the bit-identity property test, per the `compact.rs`/`filter_mask_twin.rs` precedent.