# Bindings audit — C ABI + TypeScript SDK + napi bridge

Pre-publish representation audit of the 0.13 cutover, under the SPOVs in `audit/REQUIRED-READING.md`. Scope: `crates/bumbledb-c/**`, `ts/src/**`, `ts/crate/**`, `ts/test/put.ts`, `ts/README.md`, `ts/COOKBOOK.md`. Working tree plus HEAD. Production code was not edited.

**Verdict:** the public write surface has moved onto collections, `MutationReport`, `reserve`, and `FreshRange [start, endExclusive)`. Several duals that the cutover claimed to retire still sit on the FFI seam or in SDK control-flow skips. **Do not publish 0.13 until versions lockstep** (BND-01). The rest of the should-fix list is representation work on that same seam, not leftover InsertFact.

## Counts

| Severity | Count |
| --- | --- |
| ship-blocker | 1 |
| should-fix-before-0.13 | 6 |
| later | 5 |
| **total** | **12** |

## What the cutover already got right

These are not findings. They are the representation the remaining defects are measured against.

- **No `InsertFact` / omit-to-mint type** in `ts/src`. `Fact<R>` requires every field. `ts/test/types.test.ts` pins omitted fresh cells as a type error. README and COOKBOOK fences mint with `tx.reserve` then `tx.insert(R, [{ … }])`.
- **No Batch names.** Napi `TxReq` is `{Insert, Delete, Reserve, Contains, Get, Commit, Abort}`. C is `bdb_tx_insert` / `bdb_tx_delete` with `row_count`, plus `bdb_tx_reserve`. No `bdb_tx_alloc`, no `bulk_load`, no `BDB_ERROR_KIND_BULK_LOAD`. `kind_of` matches `bumbledb::Error` exhaustively.
- **`MemberRelation` restored** and exported from `ts/src/db.ts` / `ts/src/index.ts`. `ts/test/c-sdk-2-probe.test.ts` uses it.
- **Public TS insert is collection-only:** `Iterable<Fact<R>>`. Singleton is `[fact]`. A bare fact object is not iterable, so scalar insert is not typable.
- **`MutationReport` is not flattened onto the fact.** `ts/test/db.test.ts` pins a field named `changed` as a legal cell beside `{ submitted, changed }` as a separate value. C `bdb_mutation_report` is `{ submitted, changed }` `u64`. Empty reserve is `[0, 0)`.
- **C inbound collections are rectangular:** `row_count × value_count` row-major. Jagged is unrepresentable on the C write path. `row_count == 0` is lawful and still calls `insert_dyn` (poison is checked).
- **C `bdb_abi_version()` is `2`** in both `crates/bumbledb-c/src/lib.rs` and the generated header.

---

### BND-01 (severity: ship-blocker)

**Location.** `ts/package.json` `0.12.2`; `ts/crate/Cargo.toml` `0.12.2`; `ts/npm/darwin-arm64/package.json` `0.12.2`; `crates/bumbledb-c/Cargo.toml` `0.1.0`; `crates/bumbledb/Cargo.toml` `0.12.0`. `bdb_version()` concatenates `CARGO_PKG_VERSION`. `engine_version()` bakes the napi crate version into the shipped `.node`.

**Illegal state.** A 0.13-shaped ABI (collection insert/delete, `reserve`, `MutationReport` counts, `abi_version == 2`) is still identified as the 0.12 line. Publishing the working tree as `0.12.2` would ship a breaking write surface under a patch identity. Publishing C hosts a `bdb_version()` of `bumbledb-c 0.1.0` next to `bdb_abi_version() == 2`.

**Representation critique.** Version is the identity of the layout. Two spellings of “what this binary is” (ABI generation vs crate/npm version) that disagree force every host to re-validate. The lockstep gate in `ts/scripts/build.ts` only equates main == platform == `ts/crate`; it cannot save a 0.13 cutover that was never bumped.

**Better representation.** One 0.13 identity: engine crates, `bumbledb-c`, npm main, platform package, and `ts/crate` all `0.13.0`. `bdb_abi_version() == 2` is the layout generation; `0.13.0` is the release that first ships it. Do not publish until that lockstep is the working tree.

**Evidence.** Context already named this; the tree confirms it. `ts/PUBLISHING.md` still narrates `0.12.0` as the current release. Header comment: “`2` is collection-valued insert/delete, `reserve`, and the retirement of `alloc` / `bulk_load`.”

---

### BND-02 (severity: should-fix-before-0.13)

**Location.** `ts/crate/src/lib.rs` `tx_report` / `tx_insert` / `tx_delete`; `ts/src/native.ts` `txInsert`/`txDelete`; `ts/src/db.ts` `insert`/`remove`.

**Illegal state.** The engine’s report is `{ submitted: u64, changed: u64 }`. C carries that struct. Napi replies `TxReply::Report(Result<(u64, u64), _>)` then **drops `submitted`** and returns only `changed` as `u64`. The SDK rebuilds `{ submitted: BigInt(rows.length), changed }`. For a length-1 collection the native return is the old boolean (`0n | 1n`) in bigint clothing. `ts/test/ffi.test.ts` asserts `native.txInsert(...) === 1n`.

**Representation critique.** Flattening the report onto a scalar reintroduces the boolean-`changed` dual one layer down. `submitted` becomes a host reconstruction, not a parsed engine value — validation that throws the proof away (King). C already has the right type. The SDK’s public `MutationReport` is theatre over a hole in the bridge.

**Better representation.** Napi `tx_insert`/`tx_delete` return `{ submitted, changed }` (bigint pair), the same report C writes. The SDK forwards it. Delete the `BigInt(rows.length)` reconstruction. Length-1 is not a different type.

**Evidence.**

```1178:1227:ts/crate/src/lib.rs
fn tx_report(
    tx: &External<TxHandle>,
    relation: u32,
    rows: &Array,
    req: fn(RelationId, Vec<Vec<Value>>) -> TxReq,
) -> napi::Result<u64> {
    // ...
    let (_submitted, changed) = reply!(
        worker.call(req(facts.0, facts.1))?,
        TxReply::Report,
        "transaction"
    )?;
    Ok(changed)
}
```

```1225:1228:ts/src/db.ts
			const changed = bridged("bumbledb tx insert", function record() {
				return native.txInsert(txHandle, entry.id, rows)
			})
			return Object.freeze({ submitted: BigInt(rows.length), changed })
```

---

### BND-03 (severity: should-fix-before-0.13)

**Location.** `ts/src/db.ts` `insert` / `remove` / `reserve` empty branches. Contrast: C `bdb_tx_insert` always calls `insert_dyn`; engine `WriteTx::insert_dyn` / `reserve_at` call `refuse_poisoned()` **before** the empty return.

**Illegal state.** Empty collection and `count === 0n` are special-cased in the SDK so native is never called. A poisoned transaction’s empty insert/delete/reserve therefore succeeds in TypeScript and fails in Rust/C with `TransactionPoisoned`. Empty insert also skips `resolveOrdinary` (closed / foreign membership). Empty reserve skips the fresh-field runtime check.

**Representation critique.** Empty is a lawful collection, not a second verb. Dijkstra’s `[0, 0)` already makes empty reserve a real range; `MutationReport::EMPTY` already makes empty insert a real report. The SDK reintroduced the special case as a control-flow skip (SPOV 3). Poison, membership, and “is this a fresh field” then have to be re-proved on every caller because the empty path threw that proof away.

**Better representation.** Always call native. Let the engine’s empty representation (`refuse_poisoned` then `EMPTY` / `[0, 0)`) be the one empty. Delete the three SDK early returns.

**Evidence.** Engine:

```28:32:crates/bumbledb/src/api/db/insert_dyn.rs
        self.refuse_poisoned()?;
        let rows: Vec<Row> = facts.into_iter().collect();
        if rows.is_empty() {
            return Ok(MutationReport::EMPTY);
        }
```

SDK:

```1220:1254:ts/src/db.ts
			if (rows.length === 0) {
				return Object.freeze({ submitted: 0n, changed: 0n })
			}
			// ...
			if (count === 0n) {
				return freshRangeOf(0n, 0n)
			}
```

Napi `fact_rows` already admits empty and still sends `TxReq::Insert`. The skip is SDK-only.

---

### BND-04 (severity: should-fix-before-0.13)

**Location.** `crates/bumbledb-c/src/db.rs` `bdb_tx_insert` / `bdb_tx_delete` / `bdb_tx_reserve`. Test `insert_null_out_report_does_not_commit` in `crates/bumbledb-c/src/tests.rs`.

**Illegal state.** `out(out_report, …)` / `out(out_range, …)` runs **after** `insert_dyn` / `delete_dyn` / `reserve_at`. A null required out-param is `BDB_STATUS_MISUSE` (header: programming error, no error allocated) but the delta already moved. The test returns `Abort` after the misuse, so the commit path is unpinned. A callback that returns `Ok` after a null `out_report` **commits the insert**.

**Representation critique.** MISUSE is supposed to mean “this call did not happen.” Sequencing the side effect before the out-param parse makes MISUSE a post-condition on a live delta — a flag after the fact (SPOV 2). Null here is Hoare’s billion-dollar mistake on a required slot: the type admits null, so every call pays a check, and the check is in the wrong order.

**Better representation.** `require_out(out_report)?` (and `out_range`) before any engine mutation, same as `bdb_tx_get` already does for `out_row`. Then MISUSE cannot have written a fact.

**Evidence.**

```705:719:crates/bumbledb-c/src/db.rs
        let rows = rows_in(values, value_count, row_count)?;
        let report = tx_ref
            .transaction()?
            .insert_dyn(RelationId(relation), rows)
            .map_err(|error| fail_engine(error, None))?;
        out(
            out_report,
            bdb_mutation_report { submitted: report.submitted, changed: report.changed },
        )?;
```

`bdb_tx_get` is the correct order: `require_out(out_row)?` then the engine call.

---

### BND-05 (severity: should-fix-before-0.13)

**Location.** `crates/bumbledb-c/src/value.rs` `rows_in`.

**Illegal state.** `row_count > 0 && value_count == 0`: `total` is 0, `slice_in` yields `&[]`, then `chunks_exact(0)` **panics**. `catch_unwind` maps that to `BDB_ERROR_KIND_PANIC` and the store is poisoned. A 0-arity collection is a contract/shape question; it is not a panic.

**Representation critique.** Rectangular layout already makes jagged unrepresentable. Zero-width rows are a second axis that the chunker does not inhabit — a special case left in arithmetic instead of in the type (SPOV 3). Hostile C is this crate’s job; panic-as-poison is the wrong kind.

**Better representation.** If `value_count == 0`, return `Misuse` (or typed `FactShape`) before `chunks_exact`. Empty remains `row_count == 0` only.

**Evidence.**

```185:201:crates/bumbledb-c/src/value.rs
pub(crate) fn rows_in(
    values: *const bdb_value,
    value_count: usize,
    row_count: usize,
) -> BridgeResult<Vec<Vec<Value>>> {
    if row_count == 0 {
        return Ok(Vec::new());
    }
    let total = value_count
        .checked_mul(row_count)
        .ok_or(Fail::Misuse)?;
    let cells = slice_in(values, total)?;
    let mut rows = Vec::with_capacity(row_count);
    for chunk in cells.chunks_exact(value_count) {
```

Rust `slice::chunks_exact` panics on `chunk_size == 0`.

---

### BND-06 (severity: should-fix-before-0.13)

**Location.** `ts/src/marshal.ts` `rowOf` doc comment.

**Illegal state.** The write marshal still teaches omit-to-mint and `alloc`: “fresh minting happens BEFORE this point (the transaction fills omitted fresh cells via the engine's alloc lane).” That path is gone. `rowOf` throws if a field is `undefined`. The comment is a second, false specification sitting next to the code.

**Representation critique.** Docs are a representation of the API. A leftover omit-to-mint sentence is the dual of `InsertFact` surviving as prose. Callers (and later diffs) will re-derive the mint-on-insert branch from it.

**Better representation.** One sentence: every declared field is present; mint with `tx.reserve` first. Match `relation.ts` and `Tx.insert`.

**Evidence.**

```194:198:ts/src/marshal.ts
 * Marshals one complete fact object to its positional row, in field
 * declaration order (= ordinal ids). Every declared field must be present;
 * fresh minting happens BEFORE this point (the transaction fills omitted
 * fresh cells via the engine's alloc lane).
```

README/COOKBOOK fences are already on the reserve-then-insert spelling. This comment is the remaining teacher of the old dual.

---

### BND-07 (severity: should-fix-before-0.13)

**Location.** `docs/architecture/76-c-abi.md` line 55 vs `crates/bumbledb-c/include/bumbledb_c.h` / `crates/bumbledb-c/src/lib.rs`.

**Illegal state.** The architecture doc still says `` `bdb_abi_version()` is `1` ``. The header and the export return `2`. Two identities for one layout.

**Representation critique.** ABI generation is a single integer. A second spelling that lags is the same class of defect as BND-01, one file over. Hosts that read 76 instead of the header will generate against ABI 1 (`alloc` / `bulk_load`).

**Better representation.** 76 states `2` and names the layout-visible change (collections, `reserve`, retirement of `alloc`/`bulk_load`). The header comment is already that sentence.

**Evidence.** `docs/architecture/76-c-abi.md`: “`bdb_abi_version()` is `1` — bump on a layout-visible change.” Header: “C ABI generation. `2` is collection-valued insert/delete…”

---

### BND-08 (severity: later)

**Location.** C `bdb_tx_insert` rectangular buffer vs napi `fact_rows` (`Array` of `Array`).

**Illegal state.** Napi collections are jagged: each inner array can have a different length. Mixed arity is representable and then refused per row (`expected N values, got M`). C cannot spell mixed arity; arity is `value_count` for the whole collection.

**Representation critique.** Parse-don’t-validate: C parsed rectangularity into the type. Napi validates it. The SDK public surface (array of named facts) is the TS idiom and is fine; the cell-array FFI is the dual with C.

**Better representation.** If the native bridge ever grows a second host, give it the C layout (flat buffer + arity + row count). Not a 0.13 publish block: the Native object is not exported, and marshal fails before any row enters the delta.

**Evidence.** `ts/crate/src/marshal.rs` `fact_rows` loops `one_fact_row` per index. C `rows_in` is one `chunks_exact(value_count)` over a product.

---

### BND-09 (severity: later)

**Location.** `bdb_row_set_arity(rows, row)` in `crates/bumbledb-c/src/db.rs` and the header.

**Illegal state.** Outbound row sets are `Vec<Vec<Value>>` with **per-row** arity. Scan rows of one relation are rectangular by schema. A missing row and a 0-arity row both answer `0`. Jagged is representable on the way out after being unrepresentable on the way in.

**Representation critique.** Arity belongs to the relation (or the set), not the row index. Per-row arity is the jagged dual of BND-08, outbound.

**Better representation.** `bdb_row_set_arity(const bdb_row_set *)` — one width. Out-of-range row stays `bdb_row_set_get` MISUSE. ABI bump if it misses 0.13; fold into ABI 2 only if the header is still unshipped and you want one layout.

---

### BND-10 (severity: later)

**Location.** `ts/test/put.ts`. Used from `ts/test/db.test.ts`, cookbook tests, consumer-patterns, c-sdk-2-probe, etc.

**Illegal state.** The helper accepts `Record<string, unknown>` (omitted fresh cells), fills them via `reserve`, inserts `[fact]`, and `as Fact<R>`. That is omit-to-mint as a test dialect. The SDK surface does not export it (`files` is `dist`, `src`, `COOKBOOK.md`).

**Representation critique.** Tests are a representation of how the API is used. A widely imported `put()` re-teaches InsertFact. The `as Fact<R>` is the cast that InsertFact used to hide.

**Better representation.** Keep it test-only and unnamed in docs (current). Prefer explicit `reserve` in new tests that claim to document the write path. Do not export `put`.

**Evidence.** File header: “Test-only insert helper: completes omitted fresh cells with `reserve`, then inserts the singleton collection. The SDK does not mint on insert.”

---

### BND-11 (severity: later)

**Location.** `ts/src/marshal.ts` `KeyFact<R>`.

**Illegal state.** When `R` has no fresh field, `KeyFact<R> = Partial<Fact<R>>`. `{}` is typable as a primary-key get. Missing projection fields throw at `keyRowOf`. This is not InsertFact (insert still requires `Fact<R>`), but it is an incomplete-fact type on the read side.

**Representation critique.** Partial admits every subset; the legal subsets are the declared key projections. The type throws the proof away and the runtime re-validates (King). Declared-key `get(relation, statement, key)` already types the projection exactly via `DeclaredKeyFact`.

**Better representation.** Primary-key `get` without a fresh field should demand the primary projection the same way the three-arg form does, or stay `Partial` with the documented runtime check. Not a write-path 0.13 blocker.

**Evidence.**

```59:61:ts/src/marshal.ts
type KeyFact<R extends AnyRelation> = [FreshKeys<R>] extends [never]
	? Partial<Fact<R>>
	: { [K in FreshKeys<R>]: Fact<R>[K] }
```

---

### BND-12 (severity: later)

**Location.** `ts/test/ffi.test.ts` write tests; several native-level tests (`query.test.ts`, `psi-query-atoms.test.ts`, …).

**Illegal state.** Native tests still loop `native.txInsert(tx, REL, [oneRow])` per fact. The collection verb is exercised as a scalar-in-a-list. C tests do the same for seed rows; `collection_insert_and_shape_failure_persists_nothing` is the one rectangular collection pin.

**Representation critique.** A collection API whose tests never pass N>1 on the napi path leaves the length-1 boolean dual (BND-02) looking like the contract.

**Better representation.** One napi test that inserts N rows in one `txInsert` and asserts the report (once BND-02 returns it). Not a publish block.

**Evidence.** `ts/test/ffi.test.ts`: `for (const row of rows) { assert.equal(native.txInsert(tx, PERSON, [row]), 1n) }`.

---

## Dead kinds and leftover names (checked, not found)

| Probe | Result |
| --- | --- |
| `InsertFact` / `isInserted` / `Minted` in `ts/src` | absent |
| `InsertBatch` / `TxReq::InsertBatch` | absent |
| `bdb_tx_alloc` / `bdb_tx_bulk_load` / `BDB_ERROR_KIND_BULK_LOAD` | absent |
| Engine `Error` bulk-load variant | absent; C `kind_of` is exhaustive |
| Scalar `tx.insert(R, fact)` overload | absent; collection only |
| README/COOKBOOK omit-to-mint fences | absent; they `reserve` then insert collections |
| Boolean `changed` on the public TS report | absent; `bigint`. Boolean leftover is the napi scalar in BND-02 |
| `MemberRelation` dropped | restored and exported |

## Poison / report across FFI (summary)

| Layer | `changed` | Empty insert | Poison on empty |
| --- | --- | --- | --- |
| Engine | `u64` on `MutationReport` | `EMPTY` after `refuse_poisoned` | checked |
| C | `uint64_t` on `bdb_mutation_report` | calls `insert_dyn` | checked |
| Napi | **`u64` only** (BND-02) | worker still called | checked |
| TS SDK | `bigint` on reconstructed report | **skips native** (BND-03) | **not checked** |

## Header / ABI 2

The generated header matches the Rust structs for `bdb_mutation_report`, `bdb_fresh_range`, `bdb_tx_insert`/`delete`/`reserve`, and `bdb_abi_version == 2`. cbindgen `export.include` lists both new structs. Header sync is not a ship-blocker. Version identity (BND-01) and the stale ABI-1 sentence in 76 (BND-07) are.
