## run_node kept the memset scratch-sizing its line-parallel twin measured out of probe_pass

category: incoherence | severity: medium | verdict: CONFIRMED | finder: cross:branching
outcome: fixed 0f13feff

### Summary

Commit cfa480ce ("perf: the redundant zero-fills retired at the owned sites", 2026-07-17) replaced probe_pass's seven per-pass `clear(); resize(n, 0)` scratch resets with grow-only sizing (`grow_scratch`), because the profile showed the resets re-memset the full window every pass though every element of `[..n]` is written before it is read. That same commit's profile found the memset cost was **mostly run_node's** — 71 of 77 `_platform_memset` samples sat inlined at run_node's mirror sites — and its message records the port verbatim: *"The run_node mirror is the recorded followup — its loops are the extraction-refusal twins, so arm the same grow-only shape there consciously, never silently."* The followup was never done: run_node still carries the memset shape at all seven sites, no later commit touched it (`git log -S grow_scratch -- .../run_node.rs` is empty), and TODO.md does not track it. This breaks the file's own mirror discipline, declared at run_node.rs:166-173: "kept line-parallel — a change here needs its mirror there."

### Evidence

All verified against the working tree:

- **The seven memset sites in run_node** (`crates/bumbledb/src/exec/run/run_node.rs`):
  - 203-204: `scratch.hashes.clear(); scratch.hashes.resize(n, 0);` (sibling probe hash phase)
  - 250-251: mask reset (probe phase 2)
  - 282-283: mask reset (whole-value residuals)
  - 309-310: mask reset (word residuals)
  - 336-337: `scratch.allen_gather.clear(); scratch.allen_gather.resize(4 * n, 0);` (Allen gather)
  - 381-382: mask reset (measure/duration residuals)
  - 412-413: mask reset (point-membership probes)
- **The twin's grow-only shape** (`crates/bumbledb/src/exec/run/probe_pass.rs`): `grow_scratch(...)` at 82, 202, 245, 275, 306, 348, 384; definition with the measured rationale at 595-606 ("clear + resize(n, 0) re-memset the full window every pass (`_platform_memset`, 3.7% of `meets_chain`) though every element of `[..n]` is written before it is read").
- **Mirror rule**: run_node.rs:166-173 (extraction refused, twins kept line-parallel, "a change here needs its mirror there").
- **The recorded, undone followup**: commit cfa480ce's message — post-change profile 54/1498 = 3.6% of meets_chain, "every remaining sample at run_node's own resize sites"; the run_node mirror named as the followup. No subsequent commit ports it; TODO.md has no entry.
- **The write-before-read contract holds in run_node**, so the port is legal:
  - Every mask loop writes `mask[k]` for all `k in 0..n` before compaction, and `compact_u32_by_mask` slices the mask to `items.len()` internally (`crates/bumbledb/src/exec/kernel/compact.rs:29-32`: `assert!(mask.len() >= n); let mask = &mask[..n];`) — a stale grow-only tail is never read.
  - The only unwritten hashes belong to pinned siblings (run_node.rs:219 `if !pinned`), and a pinned probe never touches the hash: `probe_child_at`'s `Cursor::Row` arm (`crates/bumbledb/src/exec/colt/probe.rs:44-48`) resolves by `position_matches` field equality. The prefetch pass is also gated on `!pinned` (run_node.rs:234), so stale hashes are unread on that path too.
  - The duration ray-poison break (run_node.rs:391-403) exits via `break 'outer` **before** the compaction reads the mask — the exact exception probe_pass documents at 344-348, applicable verbatim.

Doctrine check: docs/design/representation-first.md's lens is satisfied by grow_scratch itself (the high-water mark is the representation; the per-pass memset was redundant control flow re-establishing an invariant the writes already establish) — run_node is the half of the twin pair where the doctrine was not applied.

### Bench impact

Not a correctness bug — a measured, still-open perf residue on the leaf pass. run_node runs once per parent entry at the last node and is the entire executor for single-node plans. Commit cfa480ce's post-fix profile prices the remaining cost precisely: 54/1498 samples (3.6%) of meets_chain in `_platform_memset`, all attributed to run_node's own resize sites. Leaf-heavy plans (single-node scans; multi-node plans whose last node has siblings or residuals) pay it per batch per pass.

### Suggested fix

Hoist `grow_scratch` from probe_pass.rs (595-606) to the shared parent module (`exec/run/mod.rs`) and use it at run_node's seven sites, carrying probe_pass's write-before-read comments (including the ray-poison note at probe_pass.rs:344-348 onto run_node's measure loop at 370-405). Two adjustments beyond the mechanical swap, both already modeled by the twin:

1. **allen_gather** — run_node.rs:338 calls `scratch.allen_gather.split_at_mut(n)` on the full vec; under grow-only sizing the vec may exceed `4*n`, so slice first: `scratch.allen_gather[..4 * n].split_at_mut(n)` (probe_pass.rs:307 already does this).
2. **Allen counter loop** — run_node.rs:365 iterates `&scratch.mask` whole (currently exact-length); slice to `&scratch.mask[..n]` (probe_pass.rs:330 already does this). `allen_filter_batch` resizes `keep` to `codes.len()` = n itself (kernel/allen.rs:120-123), so the mask length is n after that call regardless, but the sliced spelling keeps the `[..n]` contract explicit.

Per the commit's own instruction, land it as a conscious armed A/B, not silently — and re-run the meets_chain profile to confirm the remaining 3.6% memset share retires.
