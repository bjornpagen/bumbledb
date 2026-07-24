## Outbound strings use from_utf8_lossy — silent corruption where the inbound twin refuses typed

category: incoherence | severity: low | verdict: CONFIRMED | finder: ts:bridge
outcome: fixed b3e355eb

### Summary

The TS bridge's stated error taxonomy is that shape/marshaling problems throw typed. The inbound param lane honors it: `param_args` refuses non-UTF-8 string payloads with a typed `WireError` ("a corrupt payload is refused typed rather than unwrapped"). The outbound lane does the opposite: `ValueOut::from_value` renders `Value::String` with `String::from_utf8_lossy(...).into_owned()`, so non-UTF-8 store bytes cross back as silently mangled U+FFFD text instead of a thrown corruption error. Verification against the engine shows the corrupt-bytes path is genuinely reachable — the dyn read lanes feeding the bridge (`Snapshot::scan`, `get_dyn`) resolve intern ids **without** UTF-8 validation, bypassing the engine's own validating decode boundary (`resolve_string`, which returns the typed `CorruptionError::NonUtf8Intern`). The two directions of the same value lane disagree on the same invariant, and the outbound direction also contradicts the engine's corruption doctrine.

### Evidence (all verified against the working tree)

- `ts/crate/src/marshal.rs:948` — `Value::String(bytes) => Self::Text(String::from_utf8_lossy(bytes).into_owned())`.
- `ts/crate/src/lib.rs:582-585` — inbound twin: `Value::String(bytes) => BindValue::Str(std::str::from_utf8(bytes).map_err(|_| WireError("bumbledb: non-UTF-8 string param".into()))?)`, with the doctrine documented at `ts/crate/src/lib.rs:569-571`.
- Reachability of corrupt bytes to the lossy arm:
  - `ts/crate/src/lib.rs:743-753` (`snapshot_scan`) → `scan_rows` (lib.rs:666-676) → `Snapshot::scan`.
  - `crates/bumbledb/src/api/db/snapshot.rs:119-121` — scan decodes strings with `|id| Ok(Box::from(dict::resolve(&self.txn, id)?))`: raw dictionary bytes, no UTF-8 check.
  - `crates/bumbledb/src/api/db/snapshot.rs:201-203` — `get_dyn` (the `snapshot_get` path, lib.rs:775-794) uses the identical raw closure.
  - `crates/bumbledb/src/api/db/plumbing.rs:84-87` — the engine's validating decode boundary `resolve_string` exists and returns `Error::Corruption(CorruptionError::NonUtf8Intern(id))`; its doc says "UTF-8 is validated here, without a copy (parse, don't validate)". The scan/get_dyn closures bypass it.
- The query-execute lane is NOT exposed: answers are UTF-8-validated typed at materialization (`crates/bumbledb/src/api/prepared/resolve_memo.rs:50-51` → `NonUtf8Intern`; `answers.rs:53-61` documents "validated at materialization"). So every other decode lane convicts corruption typed; only scan/get through the bridge repairs it silently.
- Doctrine: `docs/architecture/10-data-model.md:163` — encoding checks "detect damage at rest rather than repairing host input"; `from_utf8_lossy` is precisely a repair. The `ValueOut` doc comment (`ts/crate/src/marshal.rs:926-931`) claims the conversion "stays a bijection on everything the engine can actually hand back" — false under corruption, since the engine CAN hand back non-UTF-8 bytes on these two lanes.

### Failure scenario

A bit-flipped LMDB dictionary page (or any at-rest damage to an intern payload) yields stored string bytes that are invalid UTF-8. `snapshotScan` or `snapshotGet` returns the fact to JS as text with U+FFFD substitutions — the application persists/propagates corrupted data with no error, and the reader cannot distinguish real replacement characters from damage. The same bytes fed inbound as a param would throw `WireError("bumbledb: non-UTF-8 string param")`, and the same intern id resolved through the query lane would surface `Corruption: intern id N: stored bytes are not UTF-8`.

### Suggested fix

Make the outbound String arm fallible, matching the inbound refusal: `from_value` returns `Result`, with the String arm doing `String::from_utf8(...)` whose `Err` throws the typed corruption error (mirroring `CorruptionError::NonUtf8Intern`'s message shape). Better still, fix it at the representation: have `Snapshot::scan`/`get_dyn` resolve strings through `resolve_string` (the validating boundary that already exists in plumbing.rs) so the bridge never receives invalid bytes and the `ValueOut` bijection claim becomes true by construction — parse, don't validate, at the one decode boundary, making illegal text unrepresentable on the wire.
