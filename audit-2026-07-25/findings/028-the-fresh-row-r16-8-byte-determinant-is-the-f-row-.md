## The fresh-row (R16) determinant-is-the-F-row-id probe is spelled three times in judgment.rs

unification | low | CONFIRMED | capacity-judge
outcome: fixed 2b1e87b0

### Summary

The R16 fresh-row move — "this key has no `U` tree; the 8-byte determinant IS the `F` row id, so probe `F` directly" — is hand-rolled at three sites in `crates/bumbledb/src/storage/commit/judgment.rs`, each repeating the identical width-check-with-corruption-message, then resolving the same fact key. Only the miss semantics legitimately differ per caller. The file's own module doc (judgment.rs:27-34) and `check_capacity`'s doc (judgment.rs:1091-1093) state the "one definition, never a sweeper copy" doctrine that this triplication violates.

### Evidence (verified)

The verbatim-triplicated fragment:

```rust
let word: [u8; 8] = <determinant>.try_into().map_err(|_| {
    Error::Corruption(CorruptionError::MalformedValue("fresh-row key width"))
})?;
```

- `judgment.rs:783-792` — `establishing_fact`: word parse, then `fact_by_row(data, txn.raw(), statement.relation, u64::from_be_bytes(word))`. `fact_by_row` (commit.rs:174-187) builds `keys::fact_key` + `data.get`, miss = `CorruptionError::MissingFact`.
- `judgment.rs:925-937` — `Checker::check_scalar`: word parse, inline `keys::fact_key(&mut self.key, probe.target_relation, ...)` + `self.data.get`, miss = `probe.unsatisfied()`, hit = `self.check_fact(probe, fact)`.
- `judgment.rs:1106-1118` — `Checker::check_capacity` (ScalarProbe arm): word parse, inline `keys::fact_key(&mut self.key, statement.target.relation, ...)` + `self.data.get`, miss = `return Ok(())` (no holder, nothing to judge, per `Capacity.lean: capacity_of_empty_parent`).

One correction to the original finding's evidence: site 1 does not inline `keys::fact_key + data.get` — it delegates to `fact_by_row`. So the exact-text triplication is the word parse + corruption message; the fact-key resolution is spelled inline twice and via the helper once. Substance unchanged: the same R16 probe exists in three spellings.

### Failure scenario / impact

No runtime failure today — all three sites are currently law-identical. The risk is drift: a future change to the fresh-row determinant shape (or to the corruption taxonomy for a malformed width) must be found and applied three times; missing one site produces inconsistent corruption classification — or a silently stale probe — for the same store state, across the commit path and `Db::verify_store`, the exact divergence the file's one-definition doctrine exists to close.

### Suggested fix

One helper — a free fn beside `fact_by_row` (or on `Checker`) taking `(data, txn, relation, determinant, key_buf)` and returning `Result<Option<&[u8]>>`: the width check + `keys::fact_key` + `F` get, `None` on miss. Callers map `None` to their own verdict: `establishing_fact` → its corruption (or keep routing through `fact_by_row` for the richer `MissingFact` payload), `check_scalar` → `probe.unsatisfied()`, `check_capacity` → `Ok(())`. Note the borrow shape: `check_scalar` passes `fact` on to `self.check_fact`, so a `&mut self` method returning a reference will fight the borrow checker — the free-fn form taking `&mut KeyBuf` explicitly avoids that.