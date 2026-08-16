# Pre-publish audit index (0.13)

Lens: [REQUIRED-READING.md](REQUIRED-READING.md). Area reports:

| Area | File | Ship-blocker | Should-fix | Later |
| --- | --- | ---: | ---: | ---: |
| Engine API | [engine-api.md](engine-api.md) | 3 | 5 | 4 |
| Storage / delta / commit | [storage.md](storage.md) | 1 | 9 | 6 |
| Query / IR / exec | [query.md](query.md) | 0 | 7 | 11 |
| C / napi / TS | [bindings.md](bindings.md) | 1 | 6 | 5 |
| Lean / docs / benches / versions | [spec-docs.md](spec-docs.md) | 6 | 4 | 4 |

Raw counts overlap. Unique **do-not-publish** themes:

## Do not publish until these are true

1. **One write algebra in the published contract.** Architecture docs, cookbook recipe 28, and `scan_facts` rustdoc still teach `Db::bulk_load`, `alloc`, 4096 prefix-commit, and `Error::BulkLoad`. The engine, C ABI, TS SDK, Lean `Op`/`Event`, and benches already use collection `insert`/`delete` and `reserve` inside `write`. Same finding as API-3, STOR-01, SD-01–SD-04.

2. **One version identity: 0.13.0.** Engine crates are 0.12.0, TS/napi 0.12.2, `bumbledb-c` 0.1.0, while `bdb_abi_version()` is 2. Publishing this tree as 0.12.x ships a breaking write surface under a patch identity. `76-c-abi.md` still says ABI 1. BND-01, SD-05, SD-06.

3. **Empty `FreshRange` is not a minted id.** `[0, 0)` plus `start() -> T` fabricates `T::from_fresh(0)`. `Interval<T>` in the same crate already makes empty unrepresentable. API-1. FFI may still wire empty as `{0,0}` at the boundary.

4. **Poison is an error, not a string.** `applied: bool` × `poisoned: Option<String>` and `Error::TransactionPoisoned { message: String }` discard the original kind. API-2; STOR-02 is the same state machine.

## Should-fix on the 0.13 write seam (not query)

Representation work that belongs with this cutover, not a later cleanup:

| ID | Issue |
| --- | --- |
| API-4 / STOR empty range | Exclusive end typed as a minted `T`; `start()` vs `start_raw()` dual |
| STOR-03 | Four copied collection loops instead of one applicator |
| STOR-04 | Dyn parse then re-parse (`()` validator) |
| STOR-05 / API intern | `InternMode::Mint` dead arm |
| BND-02 | Napi returns only `changed`; SDK reconstructs `submitted` |
| BND-03 | SDK empty insert/delete/reserve skip native and miss poison |
| BND-04 | C applies the mutation then MISUSE on null `out_report` |
| BND-05 | `value_count == 0` / `chunks_exact(0)` |
| BND-06–07 | Leftover omit-to-mint comment; `76-c-abi.md` still says ABI 1 |
| SD-07–10 | Bench comments still name `bulk_load`; `scripts/lean.sh` untried |

## Later (do not block 0.13)

Query IR still admits illegal queries that `validate` then rejects (Q-01–Q-07). That is real representation debt. It is not the mutation cutover. `write_from` and prepared execute look correct after writes.

## Already right (do not “fix” back)

- No public `insertMany` / `InsertBatch` / `InsertFact` mint path.
- Insert/delete are collections; empty is lawful; singleton is `[fact]` / `[&fact]`.
- Lean `Op.insert`/`delete` take `List Fact`; `Event.reserve count`; fold theorems compile; `lake build` succeeded.
- C `bdb_tx_insert`/`delete`/`reserve`, rectangular rows, ABI 2.
- Napi `TxReq::{Insert, Delete, Reserve}`.
- Overlay `Absent` vs cancel is the right delta representation (storage.md finding 097).

## Suggested order of work

1. Docs + rustdoc + comments: one write surface (theme 1).
2. Lockstep 0.13.0 including unpublished `bumbledb-c` (theme 2).
3. `FreshRange` empty as a sum type, not `[0,0)` in Rust (theme 3).
4. `WritePhase` + `TransactionPoisoned` holding `Error` (theme 4).
5. Bindings: napi full report, empty collections still check poison, C out-param before apply.
6. Query IR (Q-*) after 0.13 ships, unless a should-fix is cheap and local.
