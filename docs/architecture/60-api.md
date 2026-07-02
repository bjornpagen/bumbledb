# 60 — Embedding API

The host-facing surface. This doc exists so the API has an owner before it is fully
designed; decided fragments are normative, the rest is the OPEN list at the bottom.
Guiding rule: the API is plain data in, plain data out — builders/macros are host-side
sugar, never the contract (`20-query-ir.md`).

## Environment lifecycle

- `Db::open(path)` — path-only public surface; map size, max readers, and LMDB flags
  are internal (fsync durability per `00-product.md`). Open verifies format version,
  then schema fingerprint; each mismatch is a typed hard failure. `Db::create(path)`
  initializes a fresh environment with the compiled schema's fingerprint.
- One process (`00-product.md`); the handle is shareable across threads; drop closes.
- Dev-reset conveniences (delete + recreate) are host-side; production open never
  destroys data.

## Transactions

- `db.read(|snap| ...)` — one LMDB read snapshot; executes queries and prepared
  queries; sees a consistent generation (the snapshot-sourced tx id, `40-storage.md`).
- `db.write(|tx| ...)` — the single writer; commits on `Ok`, aborts on `Err`/panic.
  Write operations: `alloc(field) -> u64` (serial minting — insert new rows without
  reading a max, `10-data-model.md`), `insert(fact) -> bool` (changed-state report),
  `delete(fact) -> bool`, `bulk_load(...)` (the same delta mechanism at scale,
  `40-storage.md`).
- **The transaction is a delta** (`40-storage.md`): operations are in-memory set
  arithmetic; operation order is semantically irrelevant; nothing touches LMDB until
  commit, and an abort never wrote anything. `delete(old); insert(new)` in either
  order is the blessed mutation idiom — a host-side `replace()` helper is optional
  sugar, not an engine operation (closed decision).
- **Constraints are checked at commit against the final state**
  (`10-data-model.md`): `UniqueViolation`/`ForeignKeyViolation` errors surface from
  the commit, not from the offending call site, carrying the constraint name, the
  relation, and the offending fact. The whole transaction aborts.
- **Queries inside a write transaction are forbidden in v0** (decision): constraint
  checks are internal to the write path; application read-modify-write is a read
  transaction followed by a write transaction. **Reverses if:** real app flows can't
  live with the two-txn idiom.

## Facts and results

- The write-side fact representation is the schema-macro-generated struct per relation
  (`Account { id, holder, status }`), carrying host newtypes; the boundary encodes to
  canonical `fact_bytes`. A dynamic (untyped) fact form exists for ETL tooling.
- Query results: a `ResultSet` — column metadata (find terms, in `finds` order) plus
  rows of decoded values (String/Bytes decoded from intern ids at materialization,
  inside the caller's buffer). Results are **sets**: unordered; the host sorts.
  Zero-alloc path: caller-provided reusable buffer (`30-execution.md`); convenience
  path allocates a fresh buffer.
- Params are supplied positionally by `ParamId` at execution; count and structural
  types checked at bind time (`20-query-ir.md`).

## Errors (taxonomy skeleton)

- **Open errors:** `FormatMismatch`, `SchemaMismatch`, `Io`, `Lmdb`.
- **Validation errors** (IR boundary, `20-query-ir.md` roster): typed, enumerated,
  returned at prepare time.
- **Runtime query errors:** `Overflow` (aggregate range check), `Corruption` (hard
  error, never a skip — `40-storage.md`). They abort the query; the read transaction
  remains usable.
- **Write errors:** `UniqueViolation`, `ForeignKeyViolation` (both raised at commit,
  against the final state), `SerialExhausted`, `Corruption`, `Io`/`Lmdb`. Any error
  aborts the whole write transaction — and since the transaction is a delta, an
  aborted transaction never touched LMDB at all.
- Error payloads carry ids, not formatted strings, on hot paths (allocation contract).

## ETL / migration surface

Schema change = ETL into a new database (`10-data-model.md`). The **export surface is a
full-relation scan**: `snap.scan(relation) -> impl Iterator<Item = Fact>` over `F` in
row_id order (a storage iteration, not a query — results here are streams, not sets).
The old binary exports; the new binary `bulk_load`s, with explicit serial values
preserving identity (high-water advances past them). Backup = quiesced file copy
(`40-storage.md`).

## Host-side sugar (blessed patterns, never the contract)

- Newtype wrappers per schema type (`AccountId(u64)`, `Cents(i64)`) — the nominal
  safety layer (`10-data-model.md`).
- Query-fragment functions as the rule/view layer (`20-query-ir.md`).
- Future: a typed builder or `query!` macro emitting IR; a text frontend (OPEN, README).

## OPEN (this doc's honest list)

- Exact `ResultSet` memory layout and the caller-buffer trait.
- Dynamic-fact ETL form details and the bulk-import surface.
- Ordering/limit conveniences on results (host-side; shape undecided).
- EXPLAIN's output surface (`30-execution.md` owns the mechanism).
- Multi-process story (closed as out-of-envelope for v0; future item lives here).
