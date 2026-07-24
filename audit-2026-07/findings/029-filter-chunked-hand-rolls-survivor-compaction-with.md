## filter_chunked hand-rolls survivor compaction with the exact branches the module's own kernels erased

category: inappropriate-branching | severity: medium | verdict: CONFIRMED | finder: engine:kernel
outcome: fixed 26f44a97

### Summary

The dense Allen filter path — `allen_filter_columns` / `allen_filter_columns_const`, the `FieldsAllen`/`FieldAllen` filtered-view kernels — ends in a hand-rolled scalar compaction loop (`filter_chunked`, crates/bumbledb/src/exec/kernel/allen.rs:195-199) that pays, per element, three costs the same module family's two existing survivor-compaction kernels were explicitly built (and documented) to erase. This is a third compaction representation where the codebase's doctrine (docs/design/representation-first.md; the kernel module docs themselves) demands one, and it sits at the end of the flag-free NEON Allen pipeline (allen.rs:20-31), reintroducing per-element flag µops the kernel's design keeps at zero — outside the reach of the `scripts/check-asm.sh` flag-free gate, which covers only the NEON core symbols.

### Evidence

The loop (crates/bumbledb/src/exec/kernel/allen.rs:195-199):

```rust
let mut write = start;
for (i, &keep) in keep[..len].iter().enumerate() {
    out[write] = u32::try_from(base + i).expect("positions fit u32");
    write += usize::from(keep != 0);
}
```

Each per-element cost is one a sibling kernel documents erasing:

- **Bounds-checked `out[write]` store.** compact.rs:14-20 documents for the identical `write <= read` cursor shape that the invariant "is invisible to LLVM, so the safe form carries an unelidable `items[write]` bounds check (`cmp`+`b.hs`, the same triad)" — which is why both `compact_u32_by_mask` (compact.rs:43-50) and `write_survivor_bits` (filter.rs:237-256, doctrine cite `m2max.codegen.bounds-checks-structural` at filter.rs:10-12) go unchecked under a hoisted invariant.
- **Per-item `u32::try_from(base + i).expect(...)`.** filter.rs:217-220 (`positions_fit_u32`): "the per-lane `u32::try_from` was a per-item branch … checked once." The panic cannot be hoisted by LLVM either — it must fire at the exact iteration, after earlier iterations' stores.
- **`keep != 0` instead of `keep & 1`.** compact.rs:9-12: "`mask[read] != 0` compiles to `cmp`+`cinc`, two µops confined to the 3-port flag triad (`m2max.core.flag-port-asymmetry`), where `& 1` is `and`+`add` on any of the 6 ALUs." The keep bytes on this path satisfy the 0/1 contract already: the NEON tbl table bytes are `((mask_bits >> code) & 1) as u8` (neon.rs:246-251) and the scalar fallback is `(mask.bits() >> code) & 1` (allen.rs:252-254) — so `keep & 1` is semantically identical, and LLVM cannot derive it itself because the 0/1 range is a contract, not a visible fact.

The path is live and its own docs promise the erased shape: image/view/apply.rs:594-621 routes `FieldsAllen`/`FieldAllen` through these kernels, with the comment at apply.rs:590-593 claiming "survivors via the branchless cursor-write"; `filter_chunked`'s own doc (allen.rs:179-180) claims "positions compact through the branchless cursor-write." The batch-path doc at allen.rs:117-119 ("survivors then feed the existing branchless cursor-write ([`super::compact_u32_by_mask`], 1.00 cy/item)") is true for `allen_filter_batch`'s callers (run_node.rs:360-368, probe_pass.rs:329-333) but the dense path never reaches that kernel.

The flag-free enforcement stops short of this loop: `scripts/check-asm.sh` gates only `allen_code_batch_neon`, `allen_code_batch_const_neon`, and `allen_filter_batch_neon` — the compaction tail of the dense scans is unpoliced, so the reintroduced `cmp`/`cinc`/`b.hs` µops are invisible to the gate the module doc (allen.rs:28-31) cites as the structural guarantee.

Cross-checked against docs/architecture/40-execution.md's sanctioned-kernel-shapes framing (cited at allen.rs:1-2): the dense filter's contract is codes → broadcast-mask membership → branchless cursor-write; the shipped tail diverges from the third stage.

### Bench impact

Per element of every dense Allen filter scan (per-atom Allen between two interval fields, or against a constant — the filtered-view shape), the compaction loop carries one unelidable bounds-check compare+branch, one `try_from` compare+branch, and one flag-triad `cmp`+`cinc`, where the family's measured shape is the 1.00 cy/item branchless cursor-write (`m2max.predict.branchless-flat`, filter.rs:226). The NEON classify and membership stages upstream are flag-free by construction and gate; this tail is where the pipeline's per-element flag traffic re-enters.

### Suggested fix

One representation for survivor compaction. Either:

1. Give `filter_chunked` the filter.rs shape: hoist the position guard once per call (`positions_fit_u32(n)` semantics — check `n - 1` fits u32 before the chunk walk), then compact with an unchecked cursor store and `write += usize::from(keep & 1)` (a keep-byte variant of `write_survivor_bits`, whose pre-size invariant `out.resize(start + len, 0)` already holds at allen.rs:194); or
2. Write the chunk's positions densely and reuse `compact_u32_by_mask` on the freshly appended range — making the "existing branchless cursor-write" claim in the module docs and at the apply.rs call site true on this path too.

Either way, extend `scripts/check-asm.sh` (or the microbench pins that own inlining discipline per the 2026-07-13 owner ruling recorded in the script) to cover the dense scans' compaction tail, so the flag-free law holds end to end.