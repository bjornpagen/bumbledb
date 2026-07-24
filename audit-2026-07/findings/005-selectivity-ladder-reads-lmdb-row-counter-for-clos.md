## Selectivity ladder reads the LMDB row counter for closed containment targets, which is always 0

category: bug | severity: high | verdict: CONFIRMED | finder: engine:plan-ir

### Summary

The distinct-count ladder's containment rung documents itself as bounding a field's distincts by "its target relation's row count (the containment domain), an enum by its variant list, a bool by 2" (`crates/bumbledb/src/plan/selectivity.rs:270-274`), and `docs/architecture/40-execution.md:647` promises the same ("schema bounds (containment domains, bool)"). The implementation reads the stored LMDB `S` counter — `read::row_count(txn, statement.target.relation)` (selectivity.rs:316) — but a **closed** relation's `S` counter is permanently absent, so the counter reads 0 and the rung computes distinct = `0.min(rows).max(1)` = **1**. The "enum by its variant list" case — the exact case closed relations exist for — is the one case the rung gets maximally wrong. The same raw counter read at `api/prepared/build.rs:725` pins any surviving closed EDB occurrence at `rows = 0`.

### Evidence (all verified in source)

**The rung reads the counter with no closedness branch** — `crates/bumbledb/src/plan/selectivity.rs:311-322`:

```rust
let mut containment_bound: Option<u64> = None;
for id in descriptor.outgoing() {
    let statement = schema.containment(*id);
    if statement.source.projection.as_ref() == [field] && statement.source.selection.is_empty()
    {
        let target_rows = read::row_count(txn, statement.target.relation)?;
        ...
    }
}
if let Some(bound) = containment_bound {
    return Ok(bound.min(rows).max(1));
}
```

**A missing counter is 0** — `crates/bumbledb/src/storage/read/row_count.rs:17-20` (`None => Ok(0)`).

**A closed relation's counter is never written:**
- Writes are refused: `crates/bumbledb/src/api/db.rs:437` returns `Error::ClosedRelationWrite` (declared `error.rs:1265`); alloc/insert/bulk-load document the same refusal (`api/db/alloc.rs:36`, `insert.rs:12`, `insert_dyn.rs:12`).
- `S` is written only from write deltas: `storage/commit/write.rs:334-348` (`flush_counters` folds `delta.row_count_deltas()`); `storage/env/create.rs` seeds no counters.
- Closed relations are storage-virtual: `schema/relation.rs:23-28` ("rows are ground axioms — frozen by the fingerprint, virtual in storage, write-refused"); their images are synthesized from the sealed extension, never LMDB (`image/cache/get_or_build.rs:57-61`, `image/cache/peek.rs:26-30`); `storage/keys.rs:216-229` debug-asserts no F/M/U/R entry may even name a closed relation.

**The rung does fire for closed targets:** `schema/validate.rs:151` pushes *every* containment into `relation_outgoing` regardless of target closedness, and the plain closed reference (`Alert(severity) <= Severity(id)`, `storage/commit/tests/closed.rs:36`) has an empty source selection, satisfying the rung's condition at selectivity.rs:314.

**Downstream arithmetic:** with distinct = 1, an Eq selection prices `estimate * 1 / 1 = rows` (selectivity.rs:169) — zero credit; the grounding fold's attached `Const::WordSet` membership prices `rows * |S| / 1`, clamped back to `rows` (selection_matches at :141-147, clamp at :258). Note the inversion: with **no** containment declared, the field would fall through to `DEFAULT_EQ_DISTINCT = 64` (selectivity.rs:324-327) and price `rows/64` — declaring the vocabulary containment makes the estimate strictly *worse* than declaring nothing.

**Second site, build.rs:** `api/prepared/build.rs:725` reads the same counter for every participating EDB occurrence. The grounding evaluator (`plan/ground/evaluate.rs`) deliberately keeps closed occurrences un-folded in four shapes — payload escaping to the head (:149), param-bearing filters (:146, `parse_resolvable` refuses params), a live join var with no membership home (:154-161), and the single-atom gate (:174-181). Each kept shape pins `rows = 0` and its `occurrence_stats` estimate clamps to 1 (true cardinality: up to the 256-row extension cap).

**Untested:** the ladder tests (`selectivity.rs:459-507`) and the tightest-bound test (:611-695) construct every containment target with `extension: None` (ordinary). The one closed-domain fixture, `cyclic_estimate_diagnosis_is_p3_not_a_domain_or_range_defect` (:884-935) — the "closed-domain rung is applied correctly (P1)" pin cited at `docs/architecture/40-execution.md:1050` — warms all three relation images with `get_or_build` (:893-897) before profiling, so the **image** rung serves the distincts and the cold closed-target containment path is never exercised anywhere in the test estate. The doc's P1 claim holds only for warm caches.

### Failure scenario

Cold prepare (no resident image for the source relation; the closed slot also unsynthesized or irrelevant — the rung consults the *source's* image only) of `Event(kind == <handle>, at: t), Posting(...)` under `Event.kind <= Kind.id` with `Kind` closed at 3 variants: `distinct_of(Event, kind)` = 1 instead of 3, so the kind filter earns zero selectivity credit and the DP orders the join as if the filtered Event scan kept every row (truth: ~rows/3, and even the no-schema floor would have said rows/64). Prepared plans pin their estimates and are never re-planned (selectivity.rs:72, "pinned at prepare, never re-planned"), so a plan prepared before the image warms keeps the garbage ordering for its lifetime. Direction of error is conservative (overestimate), so correctness is unaffected and WCOJ execution bounds the damage (40-execution.md's own framing) — the cost is join-order quality on exactly the vocabulary/calendar-family shapes the closed-relation feature targets, plus a pinned `rows = 0` honesty number on any kept closed occurrence's introspection surface.

### Suggested fix

Representation over control flow: a closed relation's row count *is* `extension().len()` — the option already encodes the kind ("The option **is** the kind — there is no relation-kind enum", schema/relation.rs:15-17). In the containment rung, take `schema.relation(statement.target.relation).extension().map_or_else(|| read::row_count(txn, ...), |rows| rows.len() as u64)`; identically at `api/prepared/build.rs:725` for the occurrence's own relation. Better still, wrap it once — a `relation_rows(txn, schema, rel)` that consults the sealed extension before the counter — so no future counter reader can repeat the divergence. Add a cold-cache ladder test with a closed target (the existing ladder test's fixture with `extension: Some(...)` on the target) pinning distinct = variant-list length.
