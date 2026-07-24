## permuted_determinant_image pays an O(k²) inverse-permutation search per fact on the commit hot path and the verify_store sweep — the repo's own measured law already named the fix

category: perf | severity: medium | verdict: CONFIRMED | finder: engine:storage
outcome: fixed 4d19deb8

### Summary

Every R-edge key derivation calls `permuted_determinant_image`, which reconstructs the inverse of the schema's `key_permutation` by linear search *per determinant position, per fact*: `key_permutation.iter().position(|&p| usize::from(p) == determinant_pos)` inside a loop over all positions — O(k²) work per fact for a k-field projection. This is not a speculative micro-optimization: the repo's own measured-law gravestone on the sibling function quantified exactly this search at **1.23–1.25× per-fact overhead** (13 vs 17 ns/fact) against a pre-stated 1.09 bar, and recorded the reversal condition verbatim: *"Reverses if: the permuted arm ever precomputes its inverse (the search hoisted out of the per-fact loop)."* The finding is the repo cashing its own recorded IOU. The representation-level fix is one move at schema validation, where `key_permutation` is already minted.

### Evidence (all verified against the working tree)

- **The search itself** — `crates/bumbledb/src/storage/keys.rs:498-508`:
  ```rust
  for determinant_pos in 0..key_permutation.len() {
      let source_pos = key_permutation
          .iter()
          .position(|&p| usize::from(p) == determinant_pos)
          .expect("key permutation contains every determinant position");
      out.extend(field_bytes(fact_bytes, layout, usize::from(projection[source_pos].0)));
  }
  ```
  The inverse mapping is genuinely what the loop needs: fields have unequal encoded widths (16-byte general intervals, 8-byte fixed starts, padded `bytes<N>` — keys.rs:450-455), so the output must be laid down sequentially in determinant order, which requires determinant-pos → source-pos. The forward form stored today forces the per-fact search.

- **The measured law and its reversal condition** — `crates/bumbledb/src/storage/keys.rs:457-468` (mirrored in `docs/architecture/50-storage.md:126-138`, § key encoding, which I read): cleanup-0.5.0 ruling 8 measured the identity-permuted route 1.23–1.25× slower per fact than the direct arm (commit-shaped 3-field interval projection, warm DRAM, interleaved min-of-7 × 200k facts, two process runs), attributed the cost to "the permuted arm's O(n²) inverse search... real cost on the hot commit path," and pre-declared: "**Reverses if:** the permuted arm precomputes its inverse and re-measures within the house bar."

- **Hot-path call sites paying today** (each verified in context):
  - `crates/bumbledb/src/storage/commit/plan.rs:328-334` — commit plan derivation, per insert/delete fact × per σ-satisfied outgoing containment statement (loop at plan.rs:316).
  - `crates/bumbledb/src/storage/commit/judgment.rs:654-660` — `window_child_image`, per child fact of every window edge.
  - `crates/bumbledb/src/storage/commit/judgment.rs:725-731` — `closed_source_survivor`, inside a per-extension-row scan.
  - `crates/bumbledb/src/verify_store/facts.rs:289-295` and `389-395` — F-pass, per stored fact × per outgoing statement.
  - `crates/bumbledb/src/verify_store/reverse.rs:94-100` — R-pass, per stored reverse edge.

- **The forward form has no other consumer** — I grepped every `key_permutation` use. Outside validation and tests, all of them are `permuted_determinant_image` arguments (the six sites above). The permutation is minted once per statement at `crates/bumbledb/src/schema/validate.rs:1233-1243` — itself using the identical `position()` idiom, run once at seal time where its cost is irrelevant. Storing the inverse there instead (determinant position → projection index) is a pure representation swap: no runtime consumer changes meaning, only `schema/tests/valid.rs:249,321-323,361` assert concrete values and would flip trivially, and the doc contract at keys.rs:483-485 / plan/ground.rs:220 updates in place.

### Bench impact

Commit-lane benchmarks (commit/bulk-load plan derivation: every fact × every satisfied containment/window edge) and verify_store on stores with multi-field containment projections. Per the repo's own twin measurement the search costs ~4 ns/fact on a 3-field projection, i.e. the permuted arm becomes indistinguishable from the direct arm once the search is hoisted — which is precisely what the gravestone predicts, since `determinant_image` *is* the permuted encoder under the identity permutation. One refinement to the finding as filed: "quadratic in projection width" is per-fact O(k²) with k = projection width (typically 2–4), so the win is the measured constant times edge fanout, not an asymptotic cliff on realistic schemas — severity medium is right.

### Suggested fix

At `validate.rs:1233`, mint the *inverse* permutation (determinant position → projection index) instead of the forward form — the forward form has no other consumer, so replace rather than add. The loop body in `permuted_determinant_image` becomes a straight indexed gather:

```rust
for &source_pos in inverse_permutation {
    out.extend(field_bytes(fact_bytes, layout, usize::from(projection[usize::from(source_pos)].0)));
}
```

Then re-run the recorded twin measurement per the gravestone's own protocol (interleaved min-of-7 × 200k facts, two process runs, house bar 1.09) and retire or rewrite the measured-law text in both keys.rs and 50-storage.md — the law's reversal clause obligates the re-measure, not just the code change.
