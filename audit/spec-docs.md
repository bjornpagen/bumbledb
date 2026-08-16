# Pre-publish audit: spec / docs / benches / version lockstep

Workspace: `/Users/bjorn/Documents/bumbledb`. Date: 2026-08-16.
Scope: Lean txn algebra, architecture docs, benches, crate/npm/C ABI versions.
`audit/REQUIRED-READING.md` was absent.

**Lean `lake build`:** succeeded (from `lean/`; Txn, Fresh, Bridge, conformance exe).
`scripts/lean.sh` (census + three-way cargo test) **untried**.

## Verdict

The 0.13 algebra is **already the engine, the C ABI, the TS SDK, and Lean `Op` / `Event`**. Publishing would still ship a lie: the normative docs still teach two bulk lanes, `alloc`, and 4096 prefix-commit, and the version spellings are not 0.13.0.

| Severity | Count |
|---|---|
| ship-blocker | 6 |
| should-fix-before-0.13 | 4 |
| later | 4 |

## What is already aligned (not findings)

- **Lean `Txn.Op`:** `insert` / `delete` take `List Fact`. Empty is a no-op; singleton is `[f]`. `insert_is_fold` / `delete_is_fold` exist and compiled. There is **no** third `WriteDelta` `Op` constructor. `Delta` remains the coalesced set pair (`WriteDelta`'s net content) — that is the judge input, not a second write surface.
- **Lean `Fresh.Event`:** `reserve (count : Nat)`. `reserve 0` is a no-op; `reserve 1` is the old scalar mint. `reserve_advances_by_count` exists and compiled. Bridge still names `WriteDelta::reserve` as the engine discharge — correct.
- **Engine:** `WriteTx::insert` / `delete` / `insert_dyn` / `delete_dyn` take collections and return `MutationReport`. `reserve` / `reserve_at` take a count and return `FreshRange`. `Error::BulkLoad` is gone. Crate-root rustdoc already says `scan` + `insert_dyn`.
- **C ABI:** `bdb_tx_insert` / `bdb_tx_delete` take `row_count`; `bdb_tx_reserve` takes `count`. `bdb_abi_version()` is **2**, documented in the header as collection insert/delete, `reserve`, and retirement of `alloc` / `bulk_load`.
- **TS SDK:** `Tx.insert` / `delete` take `Iterable<Fact>`; `Tx.reserve(relation, field, count)` returns `FreshRange`. No `insertMany` / `bulkLoad` in `ts/src`.
- **Benches (code):** `corpus.rs`, `writebench.rs`, cookbook `r28_migration_is_etl`, and `FuzzOp` already use `insert_dyn` inside `db.write`. `FuzzOp` is `Insert` / `Delete` / `Mixed`, not `InsertBatch`.
- **Cookbook tests:** `crates/bumbledb-query/tests/cookbook.rs` already uses `reserve(1)` and collection `insert` / `insert_dyn`. The markdown cookbook has not caught up.

---

### SD-01 (severity: ship-blocker)

**Location.** `docs/architecture/70-api.md` — Transactions, Conditional writes table, ETL / migration surface, dyn lane, freeze / OPEN ledger.

**Illegal state / lie.** The embedding-API chapter still teaches **two bulk lanes** (`Db::bulk_load` / `Db::bulk_load_dyn`), scalar `alloc::<NewType>()` / `alloc_at`, `insert(&fact) -> bool`, chunks of **4096** per transaction, `BulkLoadError` / `Error::BulkLoad { committed, error }` with prefix commits surviving, and a freeze that lists “the two bulk lanes” as v0 API. `tx.insert_all` is recorded as declined sugar *because* `Db::bulk_load` already exists. TS R11 still says `Tx.insert` returns `{ changed, ...fresh }` beside `insert(&fact) -> bool`.

**Representation critique.** Two APIs for one algebra (singleton insert vs bulk load) make a dual that the type cannot exist: empty, singleton, and many are one collection; a 4096 chunk with a committed prefix is a *sequence of ordinary commits*, not a third write verb. Architecture README rule 5: docs describe current reality, or the code change does not land.

**Better representation.** One write surface: `tx.insert(facts)` / `tx.delete(facts)` / `tx.reserve(count)` inside `db.write`. ETL is `snap.scan` then `tx.insert_dyn` under host-chosen transaction boundaries. Host that splits a load is a sequence of `commitOps` (already recorded in `Txn.lean` lines 109–114). Delete `BulkLoadError`. TS insert returns `MutationReport`; mint is `reserve`, not a side-channel on insert.

**Evidence.** Engine: `crates/bumbledb/src/api/db/insert.rs` (`insert` takes `IntoIterator`, empty → `MutationReport::EMPTY`, singleton is `[&fact]`). No `bulk_load` symbol in `crates/bumbledb/src`. No `BulkLoad` in `error.rs`. Doc still: 70-api.md ~413 (`snap.scan → bulk_load_dyn`), ~531–539 (`alloc`, `insert(&fact) -> bool`, `Db::bulk_load`), ~635 (unconditional class includes bulk_load), ~853–862 (two lanes, 4096, `BulkLoadError`), ~909–916 (dyn lane still names `bulk_load_dyn` and `alloc_at`), ~1149–1152 (R11), ~1179 (freeze “two bulk lanes”), ~1189–1193 (`insert_all` declined because bulk_load exists).

---

### SD-02 (severity: ship-blocker)

**Location.** `docs/architecture/10-data-model.md` § Fields: type + generation; closed-relation writes; ruling 2.

**Illegal state / lie.** The data-model chapter still shows `tx.alloc()?` then `tx.insert(&Account { ... })`, names `alloc` as “the only generator”, and treats `bulk_load` as the ETL verb that must preserve ids. Closed-relation writes still list `bulk_load` and `alloc` as the refused surface.

**Representation critique.** Generation is `reserve n` — the same collection as insert. Special-casing a scalar mint (`alloc`) and a bulk importer (`bulk_load`) is two extra constructors for one high-water mark. Lean already has `Event.reserve (count : Nat)` and `reserve 0` / `reserve 1` as the empty / singleton of that collection (`Txn/Fresh.lean`).

**Better representation.**

```rust
let ids = tx.reserve::<AccountId>(1)?;
tx.insert([&Account { id: ids.start(), holder, status }])?;
```

Closed writes: `insert` / `delete` / `reserve`, typed or dyn. ETL preserves identity by ordinary collection insert of complete facts (explicit fresh values), then `reserve` catches up — already the engine.

**Evidence.** Doc: 10-data-model.md ~291–305 (`tx.alloc()?`), ~353 (`ETL and bulk_load must preserve ids`), ~424 (`insert/delete, typed or dynamic, bulk_load, alloc`). Engine: `api/db/alloc.rs` is `reserve` / `reserve_at` only. Lean: `Fresh.Event.reserve`, `reserve_advances_by_count`.

---

### SD-03 (severity: ship-blocker)

**Location.** `docs/architecture/50-storage.md` ~497–511 (Bulk load paragraph).

**Illegal state / lie.** Storage still states bulk load as **engine** chunking: 4096 facts per transaction, failing chunk aborts, **prior chunks stay committed**, error carries committed count. It calls this the operationalization of Lean `scanLoad`, then admits they are not the same atomic judgment. The 4096 size is called “engine mechanism, not a Lean parameter.”

**Representation critique.** That paragraph is the illegal dual made official: one host call that secretly commits a prefix. Lean `scanLoad` judges one final state (`etl_lands_valid`). A host that splits writes is already modeled as a *sequence* of ordinary commits (`Txn.lean` narrowing, “that loop is not an API”). Encoding 4096 and prefix-commit as storage mechanism teaches a second write algebra the engine no longer has.

**Better representation.** Storage applies one `WriteDelta` per `db.write`. Chunking, if any, is host policy (recipe 28: load containment targets first; keep a `==` cluster inside one transaction). Delete the engine 4096 constant from this chapter.

**Evidence.** 50-storage.md ~497–511. Engine has no bulk_load chunker. Benches that still load a whole relation in one `db.write(|tx| tx.insert_dyn(...))` (`corpus.rs` 53–56, `writebench.rs` 160–176) already match the better representation; the storage doc does not.

---

### SD-04 (severity: ship-blocker)

**Location.** `docs/cookbook.md` recipe 28 (~1354–1364).

**Illegal state / lie.** The host-facing ETL recipe still says `bulk_load_dyn` imports and “the next `alloc` cannot collide.” The compiled pin `r28_migration_is_etl` already uses `insert_dyn` inside `write` and `tx.reserve(1)`.

**Representation critique.** Cookbook teaching a deleted verb makes the dual look like the blessed pattern. Architecture docs cite this recipe as the ETL identity’s instrument (`Txn.lean` Bridge: recipe 28).

**Better representation.** Match the test: `snap.scan` → transform → `db.write(|tx| tx.insert_dyn(rel, rows))` in dependency order; catch-up is `tx.reserve(1)`.

**Evidence.** cookbook.md ~1354, ~1362. `crates/bumbledb-query/tests/cookbook.rs` ~2131–2150 (`insert_dyn` + `reserve`). `tests/api.rs` `export_scan_bulk_loads_into_a_fresh_database` is the same loop under a stale name.

---

### SD-05 (severity: ship-blocker)

**Location.** Version spellings: workspace crates, TS package, C ABI crate, `ts/PUBLISHING.md`, `ts/scripts/build.ts` lockstep gate.

**Illegal state / lie.** 0.13 is not spelled anywhere. Workspace members are **0.12.0**; `@bjornpagen/bumbledb` and the darwin-arm64 platform package and `ts/crate` are **0.12.2**; `bumbledb-c` is **0.1.0**. `bdb_version()` bakes `CARGO_PKG_VERSION`, so a C host would print `bumbledb-c 0.1.0` next to ABI generation 2. `ts/PUBLISHING.md` still describes the 0.12.0 Query-sum release and says the three TS spellings are 0.12.0. The TS lockstep gate checks only main == platform == `ts/crate`; it **cannot see** `crates/bumbledb` or `crates/bumbledb-c`.

**Representation critique.** Four independent version numbers for one product is accidental complexity. A gate that passes while the engine and C ABI disagree is a lockstep hole that would publish a lie.

**Better representation.** One 0.13.0 spelling: workspace crates, `ts/package.json`, `ts/npm/*/package.json`, `ts/crate/Cargo.toml`, `crates/bumbledb-c/Cargo.toml`. Extend the lockstep gate (or a repo-level check) to engine + C. Rewrite `PUBLISHING.md` for 0.13 (collection writes, `reserve`, ABI 2, no bulk_load).

**Evidence.**

| spelling | value |
|---|---|
| `crates/bumbledb/Cargo.toml` (and bench, macros, query, theory) | 0.12.0 |
| `ts/package.json`, `ts/npm/darwin-arm64/package.json`, `ts/crate/Cargo.toml` | 0.12.2 |
| `crates/bumbledb-c/Cargo.toml` | 0.1.0 |
| `ts/PUBLISHING.md` | still 0.12.0 runbook |
| `ts/scripts/build.ts` `assertVersionLockstep` | TS three-way only |

---

### SD-06 (severity: ship-blocker)

**Location.** `docs/architecture/76-c-abi.md` ~54–56 vs `crates/bumbledb-c/src/lib.rs` ~101–110 and `include/bumbledb_c.h` ~665–668.

**Illegal state / lie.** The C ABI chapter says `bdb_abi_version()` is **`1`**. The generated header and the export say **`2`**, and 2 *is* the collection/`reserve`/retirement bump. Shipping 0.13 with the architecture leaf still on 1 tells C hosts the old ABI is current.

**Representation critique.** ABI generation is a single integer. Two spellings (doc 1, code 2) is the same class of lie as two bulk lanes: the type should have one value.

**Better representation.** Doc = 2, with the header’s sentence: collection-valued insert/delete, `reserve`, retirement of `alloc` / `bulk_load`. Pair with SD-05 so `bdb_version()` is not `0.1.0`.

**Evidence.** 76-c-abi.md: “`bdb_abi_version()` is `1` — bump on a layout-visible change.” `bdb_abi_version() -> u32 { 2 }` in `lib.rs`. Header comment at 665–668 agrees with code, not with 76-c-abi.md.

---

### SD-07 (severity: should-fix-before-0.13)

**Location.** `lean/Bumbledb/Txn.lean` (`insert_is_fold`, `delete_is_fold`); `lean/Bumbledb/Txn/Fresh.lean` (`reserve_advances_by_count`); `lean/Bumbledb/Bridge.lean` (`ledger` / `ledger_count = 98`).

**Illegal state / lie.** The 0.13 algebra’s representation theorems exist and build, but they are **not** Bridge rows. The census therefore cannot fail if they are renamed or deleted. Docs never cite `insert_is_fold` / `delete_is_fold`. Oracle only mentions `insert_is_fold` in a narrowing comment (`Oracle.lean` ~152–153).

**Representation critique.** Bridge is the machine-listable Lean↔Rust obligation ledger. An algebra change that is not a row is unofficial. `WriteDelta` stays the *mechanism* of `Delta` / `reserve`; it must not become a third `Op`.

**Better representation.** Add Bridge rows for `Txn.insert_is_fold`, `Txn.delete_is_fold`, and `Txn.Fresh.reserve_advances_by_count` (mechanisms: `WriteTx::insert` / `delete` / `reserve`; instruments: existing collection tests). Bump `ledger_count`. Cite the fold theorems from 70-api.md / 10-data-model.md once those chapters match the engine.

**Evidence.** Grep of `Bridge.lean`: no `insert_is_fold`, `delete_is_fold`, or `reserve_advances_by_count`. `ledger_count : ledger.length = 98`. Theorems present in Txn.lean ~206–236 and Fresh.lean ~189–192. `lake build` succeeded.

---

### SD-08 (severity: should-fix-before-0.13)

**Location.** Write-lane reporting: `crates/bumbledb-bench/src/lanes/writes.rs` `BULK_TX_CHUNK` / `bulk_append` row; `writebench.rs` `bulk_bumbledb`; `sqlite_run/bulk.rs` + `corpus.rs` `insert_rows`.

**Illegal state / lie.** `bulk_append` still reports `batch: 4096` and `commits_per_sec = facts_per_sec / 4096`. Bumbledb’s timed sample is **one** `db.write` of the whole posting stream (`writebench.rs` ~160–176). SQLite still commits in 4096-row transactions (`insert_rows` `take(4096)`). Dividing both throughputs by 4096 fabricates an engine commit rate the engine did not pay, and keeps the deleted bulk_load chunk as the published contract.

**Representation critique.** 4096 as a distinguished transaction size is accidental complexity left over from engine-owned chunking. Empty, 1, and N facts are the same `insert` inside one write. A fairness mirror that chunks SQLite at a constant the engine no longer uses is a special case pretending to be parity.

**Better representation.** Report bulk as facts/sec of one (or host-chosen) collection insert; if commits/sec is kept, divide by the **actual** commit count (1–2 on the engine side today). SQLite chunking becomes an explicit oracle choice, not “mirroring the engine’s bulk chunk.”

**Evidence.** `lanes/writes.rs` ~198–202, ~787–793. `writebench.rs` ~127–128 (comment still says `bulk_load`) vs ~163–176 (one `insert_dyn` per relation). `corpus.rs` ~98–123 (“mirroring the engine’s bulk chunk”, `take(4096)`).

---

### SD-09 (severity: should-fix-before-0.13)

**Location.** Bench corpus loaders and stress: `corpus.rs`, `calendar/corpus.rs`, `verify/run.rs`, `conformance.rs`, `stress.rs`, `scenarios/load.rs`.

**Illegal state / lie.** Code already uses `insert_dyn` inside `write`, but comments and constants still name `bulk_load` and the 4096 engine chunk. Calendar / verify still flush `==` clusters at `CHUNK = 4096` labeled “the engine’s bulk chunk.” Stress still documents `BulkLoad { committed: 65536, error: Lmdb(Io(EINVAL)) }` and “three chunk-commit boundaries” while the body is **one** insert of `2 * 4096 + 512` facts.

**Representation critique.** Host chunking of a bidirectional cluster is legal (one ordinary write per chunk). Pinning the size to the retired bulk_load constant, and keeping a stress test whose *name and comments* encode prefix-commit, keeps the illegal dual in the bench contract.

**Better representation.** Comments: “ordinary collection insert in one write.” `==` cluster: one write, or a host size chosen for memory — not “the engine’s bulk chunk.” Stress: one collection insert under contention; drop `BulkLoad` / 16-chunk arithmetic from the module docs, or actually issue N writes if multi-commit durability is the claim.

**Evidence.** `corpus.rs` ~41–47 vs ~53–56. `calendar/corpus.rs` ~40–45, `CHUNK = 4096`, prefix already one `insert_dyn` per relation (~69–72). `verify/run.rs` ~232, ~308. `stress.rs` ~1–6, ~38–41, ~99–122. `scenarios/load.rs` ~32 (`bulk_load:` in the error string).

---

### SD-10 (severity: should-fix-before-0.13)

**Location.** Public rustdoc: `crates/bumbledb/src/api/db/snapshot.rs` `scan_facts` (~350). Nearby production comments: `storage/delta/insert.rs` ~35, `storage/commit/applier.rs` ~294.

**Illegal state / lie.** `scan_facts` rustdoc still says the dyn scan “remains the ETL pairing for `Db::bulk_load`.” Intra-crate comments still say `Db::bulk_load` chunks route through insert/applier. `cargo doc` would publish a dangling method.

**Representation critique.** A rustdoc link to a deleted method is the same dual as 70-api.md, at the rustc surface.

**Better representation.** Pair `scan` / `scan_facts` with `WriteTx::insert_dyn` / `insert`. Delete bulk_load from storage comments (the path is just `delta.insert`).

**Evidence.** Grep of `crates/bumbledb/src/api`: only `snapshot.rs` still names `Db::bulk_load`. `insert.rs` / `applier.rs` comments as cited. Crate-root `lib.rs` ~24–25 already has the right pairing.

---

### SD-11 (severity: later)

**Location.** `lean/Bumbledb/Txn/Fresh.lean` — missing `reserve_is_fold`.

**Illegal state / lie.** None that ships. `reserve n` is definitionally `next + n`; the fold of `n` copies of `reserve 1` is the same mark and the same returned interval. The 0.13 lock asked for `insert_is_fold` / `delete_is_fold` (present), not this.

**Representation critique.** Empty and singleton of the mint collection should be the same theorem shape as insert: `reserve 0` / `reserve 1` / `reserve n` are one constructor. Without the fold theorem, the mint’s “count is not a special case” sentence is only prose.

**Better representation.** `theorem reserve_is_fold`: `Mint.run m (List.replicate n (.reserve 1)) = m.step (.reserve n)` (and the returned-id companion). Add a Bridge row next to `reserve_advances_by_count` (SD-07).

**Evidence.** Fresh.lean has `reserve_advances_by_count`, `step` for `.reserve n => ⟨m.next + n⟩`, no fold-of-ones theorem. Docs in Fresh.lean already say `reserve 0` is a no-op and `reserve 1` is the old scalar mint.

---

### SD-12 (severity: later)

**Location.** Internal test names and comments: `tests/api.rs` `export_scan_bulk_loads_into_a_fresh_database`; `tests/alloc_census.rs` ~1224; `tests/ramdisk_phase_r.rs` ~191–196 (`BULK_FACTS = 4096` as “Db::bulk_load’s 4096-fact commit, expressed through the same write path”); `tests/dyn_surface.rs` module docs still say `alloc_at`.

**Illegal state / lie.** Bodies already use `reserve` + collection `insert`. Names keep the retired vocabulary. Ramdisk’s 4096-vs-16 comparison is actually the *right* experiment under the new algebra (same write path, size differs); only the `bulk_load` citation is stale.

**Representation critique.** Renaming is hygiene, not a second algebra.

**Better representation.** Rename tests to `export_scan_inserts_into_a_fresh_database`; say “large collection insert” instead of bulk_load chunk.

**Evidence.** `tests/api.rs` ~354–401 (name vs `insert_dyn`). `ramdisk_phase_r.rs` ~191–196.

---

### SD-13 (severity: later)

**Location.** `crates/bumbledb-bench/src/corpus_gen/opgen.rs` module comments (~26–27, ~77–83).

**Illegal state / lie.** `FuzzOp` is already `Insert` / `Delete` / `Mixed`. Prose still says “insert batches” and “the step alphabet narrows to insert batches.”

**Representation critique.** “Batch” as a third verb is the dual the enum deleted. Staging facts into a pending delta until `Commit` is the transaction, not a bulk_load.

**Better representation.** “Collection insert/delete; `Commit` judges.”

**Evidence.** `opgen.rs` `enum FuzzOp` ~32–38 vs comments ~26–27, ~77–83.

---

### SD-14 (severity: later)

**Location.** `docs/architecture/70-api.md` OPEN ledger historical rows; `61-bench-lanes.md` `bulk_append` lane name.

**Illegal state / lie.** After SD-01 is fixed, leftover census rows (`tx.insert_all` declined) and the write-lane name `bulk_append` are history-shaped vocabulary. Architecture README forbids narrating how the design got here; a declined-sugar row that exists because bulk_load existed is that narration.

**Representation critique.** Lane *name* `bulk_append` can stay as a throughput row (facts/sec of a large collection insert) once SD-08 stops dividing by 4096.

**Better representation.** Drop or rewrite the OPEN `insert_all` row when 70-api is rewritten. Keep `bulk_append` as a report label or rename to `insert_stream` if the word “bulk” keeps teaching the dual.

**Evidence.** 70-api.md ~1189–1203. 61-bench-lanes.md ~162.

---

## Lean theorems vs 0.13 lock

| Locked item | Status |
|---|---|
| `Op.insert` / `delete` take `List Fact` | Present |
| `Event.reserve count` | Present |
| `insert_is_fold` / `delete_is_fold` | Present, compile; **not in Bridge** (SD-07) |
| Do not add a third `WriteDelta` `Op` | Honored (`Delta` is the net set pair only) |
| `reserve_is_fold` (empty/singleton mint) | Absent (SD-11, later) |

`Txn.lean` already records the honest ETL split: `scanLoad` is one judge; a host that chunks is a sequence of ordinary writes, not an API.

## Version / ABI lockstep holes (publish a lie)

- Engine workspace **0.12.0** vs TS **0.12.2** vs C **0.1.0** vs intended **0.13.0** (SD-05).
- C ABI doc **1** vs code/header **2** (SD-06).
- TS lockstep gate does not include engine or `bumbledb-c` (SD-05).
- `PUBLISHING.md` still the 0.12.0 runbook (SD-05).

## Ship-blockers vs later doc debt

**Must not publish 0.13 until:** SD-01–SD-06 (normative API/data-model/storage/cookbook + version + ABI integer). Those pages are what a host reads; they currently describe a deleted dual.

**Before tagging 0.13, should also:** SD-07–SD-10 (Bridge rows, write-lane 4096 rates, bench 4096 comments, public rustdoc). Otherwise benches and the obligation ledger still encode bulk_load.

**Can follow:** SD-11–SD-14 (reserve fold theorem, test names, fuzz prose, OPEN-ledger residue).
