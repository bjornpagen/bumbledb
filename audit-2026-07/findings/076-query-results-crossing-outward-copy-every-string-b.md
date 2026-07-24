## Query results crossing outward copy every string/bytes payload twice — `rows_out` clones cells it already owns

category: perf | severity: medium | verdict: CONFIRMED | finder: r2:concurrency-unsafe-ffi

### Summary

Every read lane of the TS bridge — `preparedExecute`, `snapshotScan`, `snapshotGet`, `txGet`, `exhumeScan` — materializes owned rows on the worker (one fresh `Box<[u8]>` per string/bytes cell), then converts them to the outward `ValueOut` form by **borrowing** the cells it owns and cloning each payload a second time, immediately dropping the first copy. That is one extra heap allocation plus one extra payload memcpy per string/bytes cell of every row crossing the FFI, on the bridge's hottest paths, in a codebase whose stated reason for Rust is allocation control (docs/design/representation-first.md doctrine; the engine even ships `crates/bumbledb/src/alloc_counter.rs`). The code also contradicts its own documented contract: both conversion sites promise "one-copy" in their doc comments while performing two.

### Evidence

All citations verified against the working tree.

**First copy — worker-side owned rows** (`ts/crate/src/lib.rs:547-567`, `answers_rows`):

```rust
bumbledb::AnswerValue::String(v) => {
    Value::String(v.as_bytes().to_vec().into_boxed_slice())   // lib.rs:555-556
}
bumbledb::AnswerValue::FixedBytes(v) => {
    Value::FixedBytes(v.to_vec().into_boxed_slice())          // lib.rs:558-559
}
```

Its doc comment (lib.rs:544-546) reads: *"the one-copy crossing: decoded cells to owned values on the worker, natural JS values on the main thread."*

**Second copy — `rows_out` borrows what it owns** (`ts/crate/src/marshal.rs:998-1002`):

```rust
pub(crate) fn rows_out(rows: Vec<Vec<Value>>) -> Vec<Vec<ValueOut>> {
    rows.into_iter()
        .map(|row| row.iter().map(ValueOut::from_value).collect())
        .collect()
}
```

`row` is owned but `row.iter()` hands `from_value` a `&Value`, and `from_value` clones (`marshal.rs:948-949`):

```rust
Value::String(bytes) => Self::Text(String::from_utf8_lossy(bytes).into_owned()),
Value::FixedBytes(bytes) => Self::Bytes(bytes.to_vec()),
```

The owned `row` is dropped one line later — the first copy's buffers are freed right after being duplicated.

**All five lanes route through this:**
- `exhume_scan` → `marshal::rows_out` (lib.rs:474)
- `snapshot_scan` → `marshal::rows_out` (lib.rs:753); `scan_rows` feeds `answers_rows` (lib.rs:687)
- `prepared_execute` → `marshal::rows_out` (lib.rs:1293) — its doc comment (lib.rs:1271-1272) says *"One-copy owned rows out"*
- `snapshot_get` (lib.rs:794) and `tx_get` (lib.rs:1141): `found.map(|values| values.iter().map(ValueOut::from_value).collect())` — same borrow-then-clone on an owned `Vec<Value>`.

**The move is legal at every call site.** The only other `from_value` callers, `marshal.rs:1080` and `marshal.rs:1172`, iterate `for (name, value) in row.values` — the value is owned there too. Nothing needs the borrowing form.

**The copy is pure waste, not a deferral.** The final napi crossing copies the string into the JS heap regardless (`String::to_napi_value`, marshal.rs:977), and `Uint8Array::new(v)` (marshal.rs:978) takes the `Vec<u8>` as an external buffer — so the eliminated copy is not recovered anywhere downstream. Strings: 3 copies where 2 suffice; bytes: 2 copies where 1 suffices.

**`from_utf8_lossy` is dead conservatism.** `Value::String` is documented "Raw UTF-8 bytes" as the type's contract (`crates/bumbledb-theory/src/value.rs:24-26`), enforced at boundaries via `ValueMismatch::Utf8` (`crates/bumbledb-theory/src/schema.rs`); on this lane the bytes come from an engine `&str` (lib.rs:555-556). The inbound param lane already takes the strict stance — non-UTF-8 is *refused typed* (`lib.rs:582-585`: "a corrupt payload is refused typed rather than unwrapped") — so the outbound lossy conversion is inconsistent with the bridge's own boundary doctrine, and would silently mangle data in exactly the case the inbound lane refuses. `ValueOut`'s own doc comment (marshal.rs:926-931) claims the conversion "stays a bijection on everything the engine can actually hand back" — lossy replacement is not a bijection; a moving `String::from_utf8` is.

### Bench impact

A full-relation export or query result of N rows with a str/bytes column performs 2N payload allocations and 2N payload memcpys where N suffice, then N immediate frees of the duplicated buffers. `snapshotScan` and `preparedExecute` over string-bearing relations are exactly the bench lanes (docs/architecture/60-bench: the scan/execute lanes); allocation rate and large-scan wall time drop by the payload lane's size. This is not algorithmic, hence medium — but it sits on every row of every read crossing the bridge.

### Suggested fix

Mechanical, no representation change:

1. Add `ValueOut::from_owned(value: Value) -> Self` that moves the payloads: `Value::String(bytes)` → `String::from_utf8(bytes.into_vec())` reusing the allocation (the UTF-8 contract makes failure unreachable; on the impossible arm, refuse typed like `param_args` does rather than lossy-convert), `Value::FixedBytes(bytes)` → `bytes.into_vec()`.
2. `rows_out`: `row.into_iter().map(ValueOut::from_owned)`.
3. `snapshot_get` (lib.rs:794) and `tx_get` (lib.rs:1141): `found.map(|values| values.into_iter().map(ValueOut::from_owned).collect())`.
4. The two manifest sites (marshal.rs:1080, 1172) already own their values — switch them too and delete `from_value` entirely, which also retires the lossy arm and restores the documented bijection.

This makes the "one-copy crossing" comments at lib.rs:544-546 and lib.rs:1271-1272 true.
