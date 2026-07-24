## run_node re-memsets scratch per batch — the pattern its own twin (probe_pass) documents as pure loss

category: incoherence | severity: low | verdict: CONFIRMED | finder: perf:olap-temporal

### Summary

`run_node` (the leaf pass — every plan terminates in it, per the entry assert at `crates/bumbledb/src/exec/run/run_node.rs:28-31`) zero-fills its per-batch scratch with `clear()` + `resize(n, 0)` at seven sites. Its declared line-parallel twin, `probe_pass`, replaced exactly this pattern with grow-only sizing and recorded the ruling in a doc comment: "`clear` + `resize(n, 0)` re-memset the full window every pass (`_platform_memset`, 3.7% of `meets_chain`) though every element of `[..n]` is written before it is read" (`probe_pass.rs:595-606`). The Allen kernel module independently records the same lesson for its `codes`/`keep` buffers ("the full per-batch refill was pure `_platform_memset` on the profile", `allen.rs:81-84`, `:117-123`). run_node diverging from a contract both twins' comments say must stay mirrored (`run_node.rs:165-173`, `probe_pass.rs:48-50`) is a coherence defect with a real, already-profiled perf cost on the pass where all leaf residuals — including the temporal lanes' Allen residuals — run.

### Evidence (all verified against the code)

The seven redundant zero-fill sites in `crates/bumbledb/src/exec/run/run_node.rs`:

- `:203-204` — `scratch.hashes.clear(); scratch.hashes.resize(n, 0);`
- `:250-251`, `:282-283`, `:309-310`, `:381-382`, `:412-413` — `scratch.mask.clear(); scratch.mask.resize(n, 0);`
- `:336-337` — `scratch.allen_gather.clear(); scratch.allen_gather.resize(4 * n, 0);` (a 32·n-byte memset immediately overwritten in full by the endpoint gather loop at `:341-352`)

The twin's grow-only contract and its adoption:

- `probe_pass.rs:602-606` — `grow_scratch` (`if v.len() < n { v.resize(n, T::default()) }`), doc at `:595-601` naming `_platform_memset` at 3.7% of `meets_chain`.
- Call sites replacing every corresponding pattern: `probe_pass.rs:82` (hashes), `:202`, `:245`, `:275`, `:348`, `:384` (mask), `:306` (allen_gather, sliced `[..4*n]` at `:307`).
- The mirror mandate: `run_node.rs:165-173` ("kept line-parallel — a change here needs its mirror there"); `probe_pass.rs:48-50` (same, pointing back).
- Same ruling in the kernel module: `allen.rs:81-84` (codes) and `:117-123` (keep) — resize-only, "the full per-batch refill was pure `_platform_memset` on the profile".

Write-before-read holds at every run_node site under grow-only, verified individually:

- Every mask loop writes `mask[k]` for all `k in 0..n` before `compact_u32_by_mask`, which asserts `mask.len() >= items.len()` and slices to `[..n]` internally (`compact.rs:30-32`), so an over-long grow-only buffer is legal at every compaction site.
- `allen_gather[..4n]` is fully written by the gather loop before `allen_code_batch` reads it.
- The measure-residual ray-poison break (`run_node.rs:391-395`) leaves a mask tail unwritten but `break 'outer`s at `:400-403` before any compaction reads it — the identical case `probe_pass.rs:344-348` documents as the reason grow-only stays legal there.
- The one asymmetry vs the twin: pinned siblings skip the hash write (`run_node.rs:219`) while `hashes[k]` is still read at `:265` — but `probe_child_at` ignores the hash entirely on `Cursor::Row` (field-equality via `position_matches`, `colt/probe.rs:46-48`), so the stale word is dead. Safe.
- `run_node.rs:365` iterates the full `scratch.mask` vec (unlike `probe_pass.rs:330`'s `[..n]` slice), but `allen_filter_batch` resizes `keep` to exactly `codes.len() == n` (`allen.rs:120-123`), so counter semantics survive a port; slicing `[..n]` anyway is the cleaner mirror.

Doc lens: this is the representation-first doctrine (`docs/design/representation-first.md`) applied at buffer-lifecycle level — the pooled high-water representation erases the per-batch memset that the clear+resize control flow re-pays; the repo already adopted that representation in two modules and left the third divergent.

### Bench impact

The finder reports an applied-and-reverted A/B: t2_overlap_join p50 165.7-166.2 ms baseline → 161.7-163.6 ms grow-only (~2% on the slowest temporal lane). Not re-run in this verification; however, the bottleneck is attested by the repo's own profile record (3.7% of `meets_chain` for the identical pattern, `probe_pass.rs:597-599`), and run_node is where every leaf batch — hence every Allen/measure residual of the t2/r2 lanes and every leaf sibling probe — pays it: one O(batch) memset per pass arm per batch. Both lanes exist in the bench suite (`bumbledb-bench/src/scenarios/temporal.rs`, `calendar/families.rs`).

### Suggested fix

Port `grow_scratch` to run_node's seven sites (hoist it from `probe_pass.rs:602` to the `run` module so both twins share the one function — it is behavior, not the refused pass extraction). For the Allen site, slice the split as `scratch.allen_gather[..4 * n]` exactly as `probe_pass.rs:307` does, and slice the keep iteration at `run_node.rs:365` to `[..n]` for line-parallelism. The write-before-read obligations are point-for-point the twin's, including the ray-poison early return; the only run_node-specific fact needed is that `Cursor::Row` probes ignore the hash word (`colt/probe.rs:46-48`), which makes the pinned arm's unwritten `hashes[k]` harmless — worth one comment at the hashes site.
