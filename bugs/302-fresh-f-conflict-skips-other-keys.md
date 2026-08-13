# Fresh-row F conflict returns early and skips other key violations
- id: 302
- severity: medium
- confidence: confirmed
- area: correctness
- components: crates/bumbledb/src/storage/commit/applier.rs, crates/bumbledb/src/storage/commit/plan.rs
- status: open (do not fix)

## Summary

Phase 2 claims to be scan-complete: every violated **key statement** in the commit must appear in the sealed `CommitRejected` set. On a fresh-keyed relation, an occupied `F` slot records the auto-key functionality violation and then `return Ok(())`, skipping the rest of `insert_fact` — including the determinant loop for every additional `Functionality` key. The commit still rejects, but the rejection set is incomplete.

## Evidence

The function's own contract:

```105:110:crates/bumbledb/src/storage/commit/applier.rs
    // collector and the step continues (scan-complete: every determinant of
    // every fact is judged, so the rejection carries the complete set of
    // violated key statements; the transaction aborts after phase 2
    // either way, so the skipped put persists nothing — an `F`-conflicted
    // fact skips its remaining puts whole: its row id has no free slot to
    // land in, and the recorded conviction already names the statement).
```

The early return after recording only the fresh auto-key:

```121:142:crates/bumbledb/src/storage/commit/applier.rs
            Some(fresh) => {
                let f_len = keys::fact_key(&mut self.key, rel, fresh.row_id);
                if self.data.get(self.txn.raw(), &self.key[..f_len])?.is_some() {
                    // ...
                    self.violations.push(Violation::Functionality {
                        statement: fresh.statement,
                        fact: op.fact.into(),
                        incumbent: None,
                    });
                    return Ok(());
                }
                fresh.row_id
            }
```

The skipped loop starts at `applier.rs:171` (`for determinant in &op.determinants`). Those extra keys are real: the plan puts the fresh auto-key into `fresh_row` and **continues** so other keys still land in `determinants`:

```366:383:crates/bumbledb/src/storage/commit/plan.rs
        if statement.fresh_row {
            // ...
            fresh_row = Some(FreshRowOp { ... });
            continue;
        }
        determinants.push(DeterminantOp { ... });
```

`Violations::seal` dedupes by `(statement, direction)`, so a skipped second statement is gone for good.

## Why this is a bug

`rejection_is_complete` (statement arm) and the 30-dependencies “judged on final states” rule: the host gets the complete set of violated key statements so it can repair all of them. An insert that collides on both the fresh row id **and** a secondary unique key only cites the auto-key. Repairing that one collision and retrying then fails on the second — or a UI shows an incomplete diagnosis.

The “skipped put persists nothing” comment is true for durability (the write txn aborts). Completeness of the **rejection set** is a different invariant, and this return breaks it.

## How to trigger / repro sketch

1. Relation with a single-field fresh auto-key **and** a second scalar key, e.g. unique `email`.
2. Commit `{id: 1, email: "a"}` and `{id: 2, email: "b"}`.
3. Insert `{id: 1, email: "b"}` (explicit fresh resupply): `F` for row 1 is occupied (not the same fact hash, so not `DispositionDesync`), and the email `U` key collides with row 2.
4. Inspect `CommitRejected`: only the fresh auto-key statement is cited; the email key is missing.

## Related

- `lean/Bumbledb/Txn.lean: rejection_is_complete`
- `error.rs` `Violations::seal` (one citation per statement)
- Neighbor `U` conflicts in the same function correctly `continue` rather than return (`applier.rs:187-199`)
