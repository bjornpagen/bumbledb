# Citation decode panics on a short cited `F` value
- id: 308
- severity: medium
- confidence: confirmed
- area: correctness
- components: crates/bumbledb/src/storage/commit.rs, crates/bumbledb/src/storage/commit/write.rs, crates/bumbledb/src/encoding/decode.rs
- status: fixed (2026-08-13)

## Summary

`fact_by_row` (the `F` get behind phase-2 incumbents and phase-3 containment/capacity witnesses) returns raw LMDB bytes with no width check. `commit` then runs those bytes through `decode_cited_facts` → `decode_values` → `decode_field` → `field_bytes`, which treats a length mismatch as a programmer invariant (`debug_assert` plus a slice). A `CommitRejected` whose cited or incumbent fact is a short `F` value therefore **panics** on the write path instead of returning `Corruption(WrongFactWidth)`, and the sealed violation set dies with the unwind. `fetch` / `scan` already call `check_width` with the real row id.

## Evidence

The apply/judgment `F` get has no width check:

```174:187:crates/bumbledb/src/storage/commit.rs
fn fact_by_row<'t>(
    data: Database<Bytes, Bytes>,
    txn: &'t RoTxn<'_, AnyTls>,
    relation: RelationId,
    row_id: u64,
) -> Result<&'t [u8]> {
    let mut key: KeyBuf = [0; MAX_KEY];
    let f_len = keys::fact_key(&mut key, relation, row_id);
    data.get(txn, &key[..f_len])?
        .ok_or(Error::Corruption(CorruptionError::MissingFact {
            relation,
            row_id,
        }))
}
```

Pointwise keys copy that slice into the citation (`applier.rs:344`); containment target-required does the same (`judgment.rs:715-723`). The one rejection exit then decodes with `?` (`write.rs:273-287`) through `decode_field`, whose contract is panic on wrong width (`encoding/decode.rs:153-156`). `field_bytes` (`decode.rs:122-125`) `debug_assert`s `fact_bytes.len() == layout.fact_width()` then indexes `offset..offset + desc.width()` — a 1-byte `F` value panics in debug and release.

Contrast: `storage/read/fetch.rs:35` and `scan.rs:144` call `check_width` and return `WrongFactWidth { row_id, ... }` with the `F`-key id.

## Why this is a bug

Corruption is a typed hard error (`50-storage.md`), never a panic. A store that still has a `U`/`R` entry pointing at a truncated `F` value is exactly the first-read-after-corruption class `fetch` already handles. On the commit path the same bytes abort the process during citation decoration, so the host loses both the sealed `CommitRejected` set (see 304) and a typed corruption diagnosis.

## How to trigger / repro sketch

1. Schema with a pointwise (interval) key, or a containment whose source probe names an `F` row.
2. Commit a well-formed fact; then overwrite that row's `F` value with a 1-byte payload, leaving `U`/`R` intact.
3. Insert a colliding fact (overlap on the pointwise key, or a stranded source) so phase 2/3 records a citation whose incumbent/witness is that row.
4. `commit` panics in `field_bytes` while decorating `CommitRejected`, instead of `Corruption(WrongFactWidth)`.

Scalar-key functionality citations store `incumbent: None` and the novel fact comes from the delta (correct width), so they do not hit this; the ReadersFull/`?` mask in 304 still applies to them.

## Related

- Finding 304 (same decoration pass; `?` kind-swap vs panic)
- `check_width` in `storage/read/check_width.rs`
- `CorruptionError::WrongFactWidth`

## Verification (2026-08-12)

Confirmed. `fact_by_row` (`storage/commit.rs:174-187`) is a bare `F` get: miss is `MissingFact`, success returns the value bytes with no `check_width`. Pointwise incumbents (`applier.rs:344`) and containment witnesses (`judgment.rs:715-723`) copy that slice into the citation. `decode_cited_facts` then runs `decode_values` (`write.rs:273-287`). `field_bytes` (`encoding/decode.rs:122-125`) `debug_assert`s layout width and slices `offset..offset+desc.width()`; a 1-byte `F` value panics in debug and release. `fetch`/`scan` already return `WrongFactWidth` with the `F`-key id (`check_width.rs:15-20`, `scan.rs:144`). Severity **medium**: requires prior corruption, but the write path panics instead of the typed error those read paths already use.

## Resolution (2026-08-13)

Commit-path `F` gets (`fact_by_row` and the fresh-row sibling) now `check_width` and return `Corruption(WrongFactWidth)` with the real row id; decoration also refuses to panic on a short cited fact and, with 304, will not erase a sealed rejection.
