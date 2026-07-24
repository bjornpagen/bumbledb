## Fact::encode_read is dead machinery: the typed Snapshot::contains it exists for is missing

category: missing-free-feature | severity: medium | verdict: CONFIRMED | finder: engine:schema-api
outcome: fixed d890f2aa

### Summary

The point-operation matrix (typed/dyn × write/snapshot) has exactly one hole, and the machinery to fill it is already generated and paid for in every `schema!` expansion. `WriteTx` offers typed membership `contains(&fact)` and `contains_dyn`; `Snapshot` offers only `contains_dyn`. Meanwhile the `Fact` trait *requires* `encode_read` — the read-context encoder whose documented contract ("`Ok(false)` means a string or bytes value was never interned — the fact cannot exist in the database") is precisely the short-circuit a committed-state membership probe needs — and no engine code consumes it. Its only call sites in the repository are the macro's own tests. Worse, a host cannot consume it either: the bytes it produces feed `read::fact_row`, which is crate-private storage. A required trait method emitted into every generated fact impl, usable by nobody, whose one natural consumer was never shipped.

### Evidence (all verified in-repo)

- `crates/bumbledb/src/api/db.rs:147` — `fn encode_read(&self, snap: &Snapshot<'_, Self::Schema>, out: &mut Vec<u8>) -> Result<bool>;` required of every `Fact` impl, doc: "Encodes against a read context. `Ok(false)` means a string or bytes value was never interned — the fact cannot exist in the database".
- `crates/bumbledb-macros/src/lib.rs:2456-2458` — the macro emits `encode_read` for every fact struct, delegating to `::bumbledb::__private::encode_read_fact` (`crates/bumbledb/src/api/db/plumbing.rs:116`).
- Repo-wide grep for `encode_read(`: the only call sites are `crates/bumbledb/tests/schema_macro.rs:357,367,615`. Notably, `WriteTx::contains` (`crates/bumbledb/src/api/db/get.rs:149-156`) encodes through `encode_delete`, not `encode_read` — so the read-context encoder is consumed by literally nothing in the engine.
- `crates/bumbledb/src/api/db/snapshot.rs` (read in full): the complete `Snapshot` surface is `execute`/`execute_collect`/`execute_args`/`execute_collect_args`/`introspect`/`profile`/`scan` (lines 17-123), `contains_dyn` (line 141), `get_dyn` (line 172), `get` (line 232), `scan_facts` (line 261). No typed `contains` exists.
- `Snapshot::contains_dyn` (snapshot.rs:141-157) already contains the exact probe body a typed `contains` needs: encode → sealed-extension scan for closed relations → `read::fact_row`. A typed variant is `fact.encode_read(self, &mut buf)?` in front of the same tail.
- The bytes `encode_read` produces have no public consumer: `crate::storage::read::fact_row` is crate-internal, so even a host calling `encode_read` directly can do nothing with the output.
- Spec check (`docs/architecture/70-api.md`, the API contract doc this audit treats as normative): the WriteTx point-read surface is a recorded decision (line 504: `tx.contains(&fact)` / `tx.get(key)` / `get_dyn`); the dyn-lane roster (lines 820-846) records `snap.contains_dyn` / `snap.get_dyn`; the epistemic-class table (line 585) lists `WriteTx::{contains,get,get_dyn}`. Nowhere does the doc record a decision refusing a typed committed-state `contains` — the only nearby minimalism ruling ("no `Db`-level sugar: the freeze keeps `Db` minimal", line 513-515, echoed at snapshot.rs:214-216) is about the `Db` wrapper for `get`, not about the `Snapshot` surface. The hole is undocumented, not refused.

### Failure scenario

Not a runtime bug; an API-coherence hole with a real cost at call sites. A host holding a typed fact and wanting committed-state membership must either (a) open a write transaction and call `tx.contains` — `&mut self`, taking the single-writer mutex for a pure read — or (b) manually destructure the fact into a `Vec<Value>` (per-field boxing, `Box<str>` allocations for string fields) to call `snap.contains_dyn`, which then re-does the type checking the generated struct already proved at compile time and allocates two more Vecs internally (snapshot.rs:145,151). Both detours violate the repo's own doctrine: representation over control flow (the typed encoding already exists; the host re-derives it dynamically) and allocation control (per-field boxing for a probe whose allocation shape the trait already encodes). Meanwhile every `schema!` expansion carries and compiles a required method with zero consumers — "one meaning, one spelling" with zero spellings that reach it.

### Suggested fix

Either direction resolves the incoherence; the first is strictly small:

1. Add `Snapshot::contains<'f, F: Fact<'f, Schema = S>>(&self, fact: &F) -> Result<bool>` — `if !fact.encode_read(self, &mut buf)? { return Ok(false); }` followed by the existing sealed-extension / `read::fact_row` tail of `contains_dyn` (snapshot.rs:153-156). This completes the point-op matrix (typed/dyn × write/snapshot), gives `encode_read` its consumer, and mirrors the exact structure of `WriteTx::contains`. Update the 70-api.md point-read roster and the epistemic-class table to record it.
2. Or, if the committed-state typed probe is deliberately refused, record that refusal in 70-api.md and delete `encode_read` from the `Fact` trait, the macro emission, and `__private::encode_read_fact` — a required trait method with no consumer is the worst of both worlds.
