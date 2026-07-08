# PRD 10 — WriteTx point reads

**Depends on:** 06, 07 (delta guard bookkeeping).
**Modules:** `crates/bumbledb/src/api/db/{write.rs,insert.rs,delete.rs}` + new `api/db/get.rs`, `crates/bumbledb/src/storage/delta.rs`.
**Authority:** `docs/architecture/70-api.md` (§ WriteTx point reads — decision block), `50-storage.md` (§ write path).

## Goal

`WriteTx` exposes `contains` and `get` reading **committed state overlaid with the
pending delta** — the same final-state view the judgment phase judges. Full queries
in write transactions remain unrepresentable (no snapshot, no prepared-query entry
point gains a WriteTx overload — do not add one).

## Technical direction

1. **Delta-side index:** the delta already tracks net dispositions by
   `(RelationId, fact_hash)`. Point reads by key need lookup by guard bytes, so
   `WriteDelta` gains a guard map maintained by insert/delete:
   `BTreeMap<(StatementId, Box<[u8]>), Disposition>` where the key statement's
   guard bytes are derived exactly as commit does (reuse PRD 06's slicing helper —
   one implementation, imported, never duplicated). Inserts record `Present(fact_bytes)`,
   deletes record `Absent`; last disposition wins, mirroring the fact map.
2. **`contains`:**
   ```rust
   pub fn contains<F: Fact>(&mut self, fact: &F) -> Result<bool>
   ```
   Encode through the *read* context (the existing `encode_read` — never minting;
   a never-interned string proves absence, short-circuit `false`), check the delta
   fact map, else probe `M` through the tx's committed-state view. This is the
   read-only sibling of `insert`/`delete`'s changed-report; share their encode
   scratch discipline.
3. **`get`:**
   - Dynamic: `pub fn get_dyn(&mut self, relation: RelationId, key: StatementId, key_values: &[Value]) -> Result<Option<Vec<Value>>>`
     — validate the statement is a `Functionality` on `relation`, encode
     `key_values` to guard bytes (type-checked against the projection; string/bytes
     via read-context intern lookup — miss ⇒ `Ok(None)`), then: delta guard map hit
     ⇒ decode its fact bytes (or `None` on `Absent`); miss ⇒ `U` get through the
     committed view ⇒ `F` fetch ⇒ decode.
   - Typed sugar for the dominant case only: `pub fn get<F: Fact>(&mut self, id: F::SerialKey) -> Result<Option<F>>`
     where the macro (PRD 05 already generates `Serial` newtypes) emits
     `F::SerialKey` for relations with exactly one serial field; relations with
     zero or several key FDs use `get_dyn` (the multi-key typed shape is an OPEN
     item in `70-api.md` — do not invent it).
4. Document (doc comment on both methods) the contract sentence from the
   architecture: "reads observe the final-state view the judgment phase will
   judge", and the upsert idiom from `70-api.md` as a doc example.
5. Allocation posture: these are write-path methods; the zero-alloc contract does
   not apply, but reuse the tx's scratch buffer for encoding as insert/delete do.

## Out of scope

Read-transaction changes; any query execution in write transactions.

## Passing criteria

- `[shape]` No prepared-query or snapshot type is reachable from `WriteTx`.
- `[shape]` Guard-byte derivation is a single shared function used by commit,
  delta index, and `get_dyn` (grep: exactly one definition site).
- `[test]` Read-your-writes: insert then `contains` = true, `get` = the fact;
  delete then `contains` = false, `get` = None; delete+reinsert(modified) then
  `get` = modified — all before commit, all equal to the post-commit read-txn
  answer (assert both).
- `[test]` Committed-state fallthrough: fact committed in a prior txn, untouched
  in this delta — `contains`/`get` find it.
- `[test]` The upsert idiom (get → delete+insert or insert) compiles as written in
  `70-api.md` and round-trips a counter increment across three write txns.
- `[test]` `get_dyn` with a never-interned string key value returns `Ok(None)`
  without growing the dictionary (assert dict next-id unchanged).
