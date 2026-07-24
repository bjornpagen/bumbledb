## encode_determinant_with validates UTF-8 twice per string key value on the dynamic point-read path

category: inelegance | severity: low | verdict: CONFIRMED | finder: perf:points
outcome: fixed 7406bd74

### Summary

The dynamic point-read encoder type-checks each key value with `value_matches` — whose `String` arm fully scans the bytes for UTF-8 validity and then throws the result away, returning unit `Ok(())` — and then immediately re-scans the exact same bytes with `std::str::from_utf8(raw).expect("value_matches validated UTF-8 above")`. The expect message documents the duplication in the source. This is a textbook parse-don't-validate violation: the repo's own doctrine document, `docs/design/representation-first.md` ("Why precise types remove branches"), quotes King's rule verbatim — "a validator returns nothing and forces every downstream caller to re-check; a parser returns a refined type that carries the proof." `value_matches` is that validator, and `encode_determinant_with` is that forced re-check. The same validate-then-reparse pair is duplicated in the sibling dyn encoder.

### Evidence

- `crates/bumbledb/src/api/db/get.rs:72-84` — the loop calls `bumbledb_theory::schema::value_matches(value, &rel.field(field).value_type)` (line 74), then the `Value::String(raw)` arm runs `std::str::from_utf8(raw).expect("value_matches validated UTF-8 above")` (line 80) before probing `resolve_str(text)`.
- `crates/bumbledb-theory/src/schema.rs:205-211` — the `(Value::String(raw), ValueType::String)` arm is `if std::str::from_utf8(raw).is_ok() { Ok(()) } else { Err(ValueMismatch::Utf8) }`: the scan's product (the validated `&str`) is discarded, so the caller's `from_utf8` re-derives it byte-for-byte.
- Second copy of the same pattern: `crates/bumbledb/src/api/db/encode_dyn.rs:52` (`value_matches`) and `:79` (`from_utf8(raw).expect("value_matches validated UTF-8 above")`) — this is the encoder `contains_dyn` and `insert_dyn` actually go through (`get.rs:302-303` calls `self.encode_dyn`), so the finding's `contains_dyn` mention lands here, not in `get.rs`.
- Hot-path / bench relevance verified: `Snapshot::get_dyn` (`crates/bumbledb/src/api/db/snapshot.rs:172-180`) calls `super::get::encode_determinant_with`, and the `p5_keyed_get` scenario (`crates/bumbledb-bench/src/scenarios/points.rs:161-195, 318`) drives exactly that entry with 12-byte string keys (`format!("doc/{i:08x}")`) — the "0.5.0 flagship" surface per the p5 doc comment.
- Doctrine source checked: `docs/design/representation-first.md:111-117` names "Parse, Don't Validate" (King 2019) as a repo rule; the finding's doctrinal framing matches the document, not just its title.

### Bench impact

Every string-keyed `get_dyn` / `contains_dyn` / `insert_dyn` value is UTF-8-scanned twice. At p5's 12-byte keys the redundant scan is nanoseconds against the LMDB determinant probe and fact decode — severity low is correct. One sharpening of the finder's scenario: the 496-byte determinant cap (`crates/bumbledb/src/storage/keys.rs:201`, `MAX_DETERMINANT_WIDTH = 496`) does not bound raw string key length, because a string field contributes only its 8-byte intern id to the determinant — so a host passing long string keys pays the redundant scan proportionally to the full string, uncapped by determinant width. The cost is real but small; the finding's weight is doctrinal, on the surface the bench crowns.

### Suggested fix

At both call sites (`get.rs` and `encode_dyn.rs`), peel the `Value::String` arm before the shared kind check: match `(Value::String(raw), value_type)` first, run `from_utf8(raw)` exactly once, map `Err` to `ValueMismatch::Utf8` (preserving the typed-error contract) and use the parsed `&str` for the dictionary probe. Alternatively, add a `value_matches` variant in `bumbledb-theory` that returns a parsed view (e.g. `Result<Option<&str>, ValueMismatch>` or a small `Matched<'_>` enum) so the proof the scan produces travels with the `Ok` — the representation-first fix, and it also deletes the two `expect` calls whose messages currently apologize for the duplication.
