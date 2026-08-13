# `S` row-count arithmetic overflow is always labeled “underflow”
- id: 307
- severity: info
- confidence: confirmed
- area: other
- components: crates/bumbledb/src/storage/commit/write.rs
- status: open (do not fix)

## Summary

Phase 4 folds `row_count_delta` into the stored `S` row count with `checked_add_signed`. Both overflow (count + large positive delta wrapping past `u64::MAX`) and underflow (count + negative delta below 0) take the same `CorruptionError::MalformedValue("S row count underflow")` arm. The commit still aborts; the diagnostic names the wrong direction when the failure is overflow.

## Evidence

```343:347:crates/bumbledb/src/storage/commit/write.rs
        let updated = current
            .checked_add_signed(count_delta)
            .ok_or(Error::Corruption(CorruptionError::MalformedValue(
                "S row count underflow",
            )))?;
```

`count_delta` comes from the delta’s per-relation insert/delete net (`storage/delta/insert.rs` `+= 1`, `delete.rs` `-= 1`). A huge insert-only transaction against a stored count near `u64::MAX` fails `checked_add_signed` because the sum does not fit in `u64`, not because the count went negative.

## Why this is a bug

Corruption messages are the offline diagnosis surface (`verify_store`, logs, support). Mislabeling overflow as underflow sends the operator to look for extra deletes / counter desync in the wrong direction. Functionally the commit still refuses to persist a wrapped count, so this is taxonomy-only.

## How to trigger / repro sketch

Craft (or mock) a stored `S` row count of `u64::MAX` and a positive `row_count_delta`. `flush_counters` returns `MalformedValue("S row count underflow")`. A genuine underflow (more net deletes than the stored count) produces the same string, which is the only accurate use of the word.

## Related

- `WriteDelta` `row_count_delta` map
- `CorruptionError::MalformedValue` (stringly-typed)

## Verification (2026-08-12)

Confirmed. `flush_counters` (`write.rs:343-347`) maps every `checked_add_signed` `None` to `MalformedValue("S row count underflow")`. `u64::checked_add_signed` fails for both a negative delta past 0 and a positive delta past `u64::MAX`. `count_delta` is the delta's net insert/delete (`insert.rs` `+= 1`, `delete.rs` `-= 1`). Overflow of an honest count is not a practical host path (near-2^64 live rows), but a stored `S` of `u64::MAX` plus one insert hits the overflow arm and is still labeled underflow. The commit aborts either way. Severity stays **info**.
