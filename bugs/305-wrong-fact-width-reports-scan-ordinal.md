# Image decode reports scan ordinal as LMDB row id in `WrongFactWidth`
- id: 305
- severity: low
- confidence: confirmed
- area: correctness
- components: crates/bumbledb/src/image/decode.rs
- status: open (do not fix)

## Summary

When a stored fact's byte length disagrees with the schema layout, image build/append raises `CorruptionError::WrongFactWidth { row_id, ... }`. The `row_id` field is filled with the **dense scan position** (`position as u64`), not the `F`-key row id. The real row id is discarded one frame up (`let (_row_id, fact_bytes)`). Operators and `verify_store`-adjacent diagnosis will look up the wrong `F` key.

## Evidence

`fill_columns` drops the storage row id:

```173:183:crates/bumbledb/src/image/decode.rs
    let mut position = from;
    for entry in scan {
        let (_row_id, fact_bytes) = entry?;
        if position >= row_count {
            return Err(Error::Corruption(CorruptionError::RowCountMismatch {
                relation: rel,
                stored: row_count as u64,
            }));
        }
        decode_fact(rel, plan, fact_width, fact_bytes, position, words, bytes)?;
        position += 1;
    }
```

`decode_fact` then labels the width error with that position:

```208:214:crates/bumbledb/src/image/decode.rs
    if fact_bytes.len() != fact_width {
        return Err(Error::Corruption(CorruptionError::WrongFactWidth {
            relation: rel,
            row_id: position as u64,
            expected: fact_width,
            actual: fact_bytes.len(),
        }));
    }
```

`scan` / `scan_from` (`storage/read/scan.rs`) yield `(row_id, bytes)` where `row_id` is parsed from the `F` key — the value this error type is documented to carry. After deletes, scan ordinals are not row ids (holes are skipped; ids are not dense).

## Why this is a bug

`WrongFactWidth.row_id` is part of the corruption diagnosis contract: it should name the `F | relation | row_id` key to inspect. Reporting the image position points at a different (or nonexistent) row after any delete, and at a shifted id whenever the high-water is not equal to the live count. The first-read-after-corruption path is exactly when this error fires.

## How to trigger / repro sketch

1. Manually (or via a test fixture) put an `F` fact whose value length ≠ layout width, with `row_id` well above the scan ordinal (e.g. only row id 1000 live).
2. Open and run any query that builds the relation image.
3. Read `WrongFactWidth { row_id }`: it is `0` (or the append tail index), not `1000`.

## Related

- `storage/read/scan.rs` `parse_facts` (correct `row_id`)
- `CorruptionError::WrongFactWidth` definition in `error.rs`
- Closed-relation synthesis also calls `decode_fact` with declaration index, which *is* the row id there — the lie is specific to the LMDB scan fill
