# 70 — Embedding API

The host-facing surface. Guiding rule: the API is plain data in, plain data out —
builders/macros are host-side sugar, never the contract (`20-query-ir.md`). The one
exception with teeth is the `schema!` macro, whose grammar is normative here because
the schema is compiled into the binary (`10-data-model.md`).

## The `schema!` grammar (normative)

Two statement kinds inside the macro, in any order: **relation blocks** and
**dependency statements** (`30-dependencies.md` owns their semantics; this section
owns the surface).

```rust
bumbledb::schema! {
    relation Account {
        id: u64 as AccountId, serial,
        holder: u64 as HolderId,
        kind: enum Kind { Checking, Savings },
        active: interval<i64> as ActiveDuring,
    }

    Account(id) -> Account;                                  // redundant here (serial implies it) — and rejected as a duplicate
    Account(holder) <= Holder(id);
    Account(id | kind == Savings) == SavingsTerms(account);
}
```

- **Field syntax:** `name: type` with optional `as NewType` and optional `serial`.
  Types: `bool`, `u64`, `i64`, `str`, `bytes`, `enum Name { Variants }`,
  `interval<i64>`, `interval<u64>`. `as` is legal on u64, i64, and intervals (the
  newtype wraps the engine value; rustc polices domains — `10-data-model.md`).
  `serial` is legal on `u64` only and auto-materializes `R(field) -> R`.
  **There are no field-level constraint modifiers** — no `unique`, no `fk(...)`;
  those words do not parse.
- **Dependency statements:** `Rel(fields...) -> Rel;` (FD, key form only),
  `A(fields... | field == Literal, ...) <= B(fields...);` (containment),
  `==` for bidirectional. Projection lists are positional between the two sides;
  selections follow `|` as comma-separated `field == literal` pairs; literals are
  enum variant names, integer literals, `true`/`false`, string/byte literals, and
  `start..end` interval literals (half-open). The macro emits descriptors directly —
  relation/field names resolve to declaration-order ids at expansion time, so an
  unresolvable name is a compile error naming the relation and field — and
  performs no semantic validation beyond parse shape and name-to-id resolution:
  the schema validation boundary (`30-dependencies.md` roster) is the judge, and
  everything semantic beyond names is a normal typed error with the statement
  rendered back.
- The macro generates: relation descriptors, dependency statement descriptors, the
  host newtypes, per-relation fact structs (`Account { id, holder, kind, active }`),
  and the `schema()` constructor the `Db` functions take.

**Decision: the macro surface is the algebra, with no sugar keywords.** Owner ruling
(`30-dependencies.md` records the alternative and its loss). The macro
remains hand-rolled (no syn/quote — the dependency policy, `00-product.md`).

## Environment lifecycle

- `Db::open(path, &Schema)` — no tuning parameters: map size, max readers, and LMDB
  flags are internal (fsync durability per `00-product.md`); the schema argument is
  what gets fingerprint-verified. Open verifies format version, then schema
  fingerprint; each mismatch is a typed hard failure. `Db::create(path, &Schema)`
  initializes a fresh environment with the schema's fingerprint — and **refuses a
  directory that already holds any LMDB environment** (`AlreadyInitialized`): a
  bumbledb one (re-writing `_meta` counters over live data would be silent corruption,
  so create is exactly as non-destructive as open) or a foreign one (any other named
  database, or a non-empty unnamed root). The one exception is a half-created bumbledb
  store — a crash between directory creation and the meta commit leaves an empty root,
  and create recovers it.
- One process, one handle (`00-product.md`): every open holds an exclusive advisory
  lock on `<dir>/bumbledb.lock`; a second live handle on the same path — in this
  process or another — is `EnvironmentLocked` at open time. The handle is shareable
  across threads; drop closes and releases the lock.
- Dev-reset conveniences (delete + recreate) are host-side; production open never
  destroys data.

## Transactions

- `db.read(|snap| ...)` — one LMDB read snapshot; executes *prepared* queries
  (`db.prepare(&Query)` is the sole entry — pin-at-prepare, `40-execution.md`); sees
  a consistent generation (the snapshot-sourced tx id, `50-storage.md`). A prepared
  query executes only against snapshots of the database that prepared it — every
  execution entry checks the environment's process-unique instance id first and
  returns `ForeignPreparedQuery` on a foreign snapshot (plan, statistics, and view
  memo all belong to the preparing environment).
- `prepared.staleness(&snap)` — the plan-drift signal, the pin-at-prepare decision's
  compensating control (`20-query-ir.md`): per participating occurrence, the row
  count the plan was costed with against the snapshot's live `S` counter (one O(1)
  get each, ≤ 20 by the roster cap), each ratio
  `max(live, pinned) / max(1, min(live, pinned))` so shrink and growth both read as
  drift ≥ 1, plus the max. Pull-based and engine-policy-free: the engine never calls
  it and holds no thresholds — the host owns policy (`00-product.md`). Convention,
  not contract: re-prepare at max ratio ≥ 4× (the worst measured est/actual on a
  fresh plan is 3.3×, so 4× separates plan drift from estimation noise). Same
  foreign-snapshot guard as execution; it allocates — a diagnostic surface, never a
  warm-path call. Negated and chase-eliminated occurrences earn no statistics read
  at prepare and so carry no pin; guard probes pin nothing. The stats/EXPLAIN
  surface (`Snapshot::profile`) carries the same pin record per occurrence —
  "estimated from (pinned rows at prepare)" — so a drifted plan is visible in one
  read of the existing report.
- `db.write(|tx| ...)` — the single writer; commits on `Ok`, aborts on `Err`/panic.
  Non-reentrant: a nested `write` from within a write closure on the same thread
  panics with a named message ("nested Db::write") rather than self-deadlocking on
  the writer mutex forever.
  Write operations: typed `alloc::<NewType>()` via the generated `Serial` newtypes
  (untyped: `alloc_at(SerialField) -> u64`, taking the witness
  `Schema::serial_field(relation, field)` resolves — see the ETL section) — serial
  minting, insert new rows
  without reading a max (`10-data-model.md`); `insert(&fact) -> bool` (changed-state
  report); `delete(&fact) -> bool`; `_dyn` forms of both for ETL tooling.
  `SerialExhausted` raises eagerly at the `alloc` call (the sequence state is knowable
  immediately), not at commit. Bulk import is `Db::bulk_load` — a `Db`-level method,
  not a write-closure operation (see the ETL section).
- **WriteTx point reads (decision):** `tx.contains(&fact) -> bool` (membership — the `insert`/`delete`
  return value's read-only sibling) and `tx.get::<F>(key) -> Option<F>` — lookup of
  the full fact through any key FD of its relation (typed via the key's newtype
  signature; `_dyn` form takes relation + statement id + encoded key). Both read
  **committed state overlaid with the pending delta** — the same final-state view
  the judgment checker judges (`50-storage.md`), so check-then-act is race-free by
  construction (single writer, one view). **The upsert idiom, blessed:**

  ```rust
  db.write(|tx| {
      match tx.get::<Account>(id)? {
          Some(old) => { tx.delete(&old)?; tx.insert(&Account { balance: old.balance + x, ..old })?; }
          None      => { tx.insert(&Account { id, balance: x, ..default })?; }
      }
      Ok(())
  })?
  ```

  **Full queries inside write transactions remain forbidden** — point reads are
  guard gets (allocation-free, no images, no plans); dragging the image cache and
  executor into the write path is the refused half. **Alternative:** keep the pure
  two-transaction idiom. **Why it lost:** the surveyed workloads' upserts and
  check-then-act guards are exactly the shape that needs a read of the state being
  written, and the two-txn idiom reintroduces the TOCTOU the single-writer design
  exists to kill (safe only under host-side serialization nobody polices).
  **Reverses if:** never — the guards are already read inside commit; this exposes
  the same gets one phase earlier.
- **The transaction is a delta** (`50-storage.md`): operations are in-memory set
  arithmetic; operation order is semantically irrelevant; nothing touches LMDB until
  commit, and an abort never wrote anything. `delete(old); insert(new)` in either
  order is the blessed mutation idiom — a host-side `replace()` helper is optional
  sugar, not an engine operation (closed decision).
- **Dependencies are judged at commit against the final state**
  (`30-dependencies.md`): `FunctionalityViolation`/`ContainmentViolation` errors
  surface from the commit, not from the offending call site, carrying the statement
  id (renderable back to the algebra through the schema), the judgment direction for
  `==` statements, and the offending fact's bytes. The whole transaction aborts.

## Facts and results

- The write-side fact representation is the schema-macro-generated struct per relation
  (`Account { id, holder, kind, active }`), carrying host newtypes; the boundary
  encodes to canonical `fact_bytes` (interval newtypes carry `Interval<i64>`/`
  Interval<u64>` values whose `start < end` invariant is enforced at construction —
  parse, don't validate, in the host too). A dynamic (untyped) fact form exists for
  ETL tooling.
- Query results: one concrete `ResultBuffer` (decided: columnar cells + a byte heap,
  no caller-buffer trait) — rows of decoded values (String/Bytes decoded from intern
  ids at materialization, into the buffer's byte heap; intervals as start/end word
  pairs), a `rows()` iterator, and column metadata via
  `PreparedQuery::column_types()` (the buffer itself stays typeless: stamping owned
  types per execution would allocate on the warm path). Contract on `Err`: the
  buffer's contents are unspecified — ignore `out` when `execute` errors; the
  snapshot stays usable. Results are **sets**: unordered; the host sorts. Zero-alloc
  path: caller-provided reusable buffer (`40-execution.md`); convenience path
  allocates a fresh buffer.
- Params are supplied positionally by `ParamId` at execution — scalars as values,
  **param sets as slices** (deduplicated at bind; the documented small-set planning
  assumption is `20-query-ir.md`'s); count and structural types checked at bind time.

## Errors (taxonomy skeleton)

- **Open errors:** `FormatMismatch`, `SchemaMismatch`, `Io`, `Lmdb`.
- **Schema errors** (declaration boundary, `30-dependencies.md` roster included):
  typed, enumerated, returned from schema validation before any environment exists.
- **Validation errors** (IR boundary, `20-query-ir.md` roster): typed, enumerated,
  returned at prepare time.
- **Runtime query errors:** `Overflow` (aggregate range check), `Corruption` (hard
  error, never a skip — `50-storage.md`). They abort the query; the read transaction
  remains usable.
- **Write errors:** `FunctionalityViolation`, `ContainmentViolation` (both raised at
  commit, against the final state, carrying statement ids), `SerialExhausted`,
  `Corruption`, `Io`/`Lmdb`. Any error aborts the whole write transaction — and
  since the transaction is a delta, an aborted transaction never touched LMDB at all.
- Error payloads carry ids, not formatted strings, on hot paths (allocation contract).

## ETL / migration surface

Schema change = ETL into a new database (`10-data-model.md`) — the only path from
any other format, stated. The **export
surface is a full-relation scan**: `snap.scan(relation)` yields *dynamic* facts
(`Result<Vec<Value>>` — per-item corruption is a hard error and the stream fuses)
over `F` in row_id order (a storage iteration, not a query — streams, not sets); the
typed sibling `snap.scan_facts::<F>()` decodes into the generated structs. The
dynamic form pairs with `Db::bulk_load(relation, facts)`: chunks of 4096 per
transaction, each chunk atomic, prior chunks committed on failure with the committed
count carried on `BulkLoadError`. The returned/carried count is **facts that changed
state** (idempotent re-inserts are consumed but not counted) — changed-not-consumed
semantics, stated. Mis-shaped dynamic facts (including out-of-range relation ids and
`start ≥ end` intervals) are typed `FactShape` errors (decided: ETL input is data,
not code — no panics on the import path). Explicit serial values preserve identity
(high-water advances past them). Untyped serial minting is resolve-once/mint-per-row:
`Schema::serial_field(relation, field) -> Result<SerialField, FactShapeError>`
validates the ids and the `Serial` generation once and returns a `Copy` witness
(private fields, one construction site — the type is the proof);
`tx.alloc_at(witness)` mints with no generation re-check anywhere on the path
(decided: a per-call typed error inside the mint was rejected — it validates on
every call and throws the proof away). **Import order under bidirectional statements is
the importer's obligation:** a `==` statement's cluster must land within one chunk's
transaction, so the documented import order is dependency-cluster order — parent and
arm facts interleaved — and a straddled cluster fails its chunk loudly
(`50-storage.md`). `Fact::encode_read`'s reader-side encode is host-reachable
surface — a stated decision: it reports "this fact cannot exist" for never-interned
values and is the membership-probe building block. `Db::compact` is safe concurrent
with a writer (LMDB's copy transaction reads one consistent snapshot; the copy simply
omits later commits). Backup = quiesced file copy (`50-storage.md`).

## Observability

Two feature-gated surfaces, both compiling to nothing under default features
(`00-product.md`: no always-on instrumentation in release paths): the `alloc-counter`
feature registers the counting allocator (events + bytes + live/peak, the gate's and
the benchmark's memory truth), and the `trace` feature enables `bumbledb::obs` —
explicit per-thread capture of nanosecond spans and point events over every prepare/
execute/commit phase, drained by tooling into Chrome-trace artifacts. Always
available: `snap.explain(..)` (rendered report) and the structured execution-stats
surface it is built from.

## Host-side sugar (blessed patterns, never the contract)

- Newtype wrappers per schema type (`AccountId(u64)`, `Cents(i64)`,
  `ActiveDuring(Interval<i64>)`) — the nominal safety layer (`10-data-model.md`).
- Query-fragment functions as the rule/view layer (`20-query-ir.md`).
- The outer-join merge: run the positive and the negated query, concatenate — the
  sanctioned decomposition (`20-query-ir.md`), a two-line host function.
- Zero-default aggregates: the host maps an absent aggregate row to 0 where the
  domain wants it (`20-query-ir.md` empty-set semantics).
- Future: a typed builder or `query!` macro emitting IR; a text frontend (OPEN,
  README) — either would lower to statements and IR, never around them.

## OPEN (this doc's honest list)

Resolved by ruling or implementation (recorded above): the `ResultBuffer` shape;
the dynamic-fact ETL form; EXPLAIN's surface (`snap.explain(&mut prepared, params)
-> (ResultBuffer, String)` — ANALYZE semantics, rendered-text report); WriteTx point
reads (decided).

Still open:

- Ordering/limit conveniences on results (host-side; shape undecided).
- The typed signature for multi-key `tx.get` disambiguation when a relation carries
  several key FDs over the same newtype (the `_dyn` form is unambiguous today;
  the typed sugar waits for real usage).
- Multi-process story (closed as out-of-envelope for v0; future item lives here).
