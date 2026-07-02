# PRD 28 — Public API Assembly

Authority: `docs/architecture/60-api.md` (the whole doc), `40-storage.md` (commit →
eviction wiring), `00-product.md` (threading doctrine).

## Purpose

Assemble the pieces into the embedding surface: `Db`, transactions, the typed write
path, export, and the final error taxonomy.

## Technical direction

- `api` module. `Db::create(path, &Schema)` / `Db::open(path, &Schema)` (PRD 04 under
  the hood) → `Db` holding the environment, the image cache (PRD 11), and a writer
  mutex (single-writer serialization across app threads; the engine still owns zero
  threads). `Db: Send + Sync`.
- `db.read(|snap| ...)`: opens a `ReadTxn`, exposes `snap.execute(&mut prepared,
  params, &mut buf)`, `snap.explain(...)` (PRD 24), `snap.scan(relation)` — the export
  iterator yielding decoded facts (dynamic form: `Vec<Value>` per fact; strings
  resolved) in row_id order (a stream, not a set — per `60-api.md`).
- `db.write(|tx| ...)`: takes the writer mutex, builds a `WriteDelta` (PRD 06) over a
  read view; the closure gets typed ops — `tx.alloc::<AccountId-ish>()` via generated
  helpers (PRD 27) plus untyped `alloc(rel, field)`, `tx.insert(fact-struct)` /
  `insert_dyn(rel, &[Value])`, `delete` symmetric; `Ok` → commit (PRDs 07–08) →
  on `changed`, evict the cache (PRD 11) — the one wiring point; `Err`/panic → drop
  delta, LMDB untouched. Queries are not reachable from the write closure (the type
  simply offers none — forbidden by representation).
- `bulk_load`: the same delta mechanism with a documented chunking convenience
  (N facts per transaction); the fresh-database append-order path is a doc'd fast
  path, implemented only if trivial — otherwise a comment citing the decision and
  normal inserts (do not gold-plate).
- Final error pass: collapse the per-module error types into the `60-api.md` taxonomy
  (one public `Error`), payloads = ids + owned fact bytes where specified, no
  formatted strings constructed on hot paths; `impl Display` does the formatting
  lazily.
- `lib.rs` re-exports: the deliberate public surface only (`Db`, `Query` + IR types,
  `PreparedQuery`, `ResultBuffer`, `Error`, `Schema`, the macro). Everything else
  `pub(crate)`.

## Non-goals

Any e2e/oracle/benchmark harness (human-owned). Multi-process guards. Async anything.
`replace` sugar (host-side, not this crate — decision).

## Passing criteria

- Unit tests: the `60-api.md` usage shapes compile and run end-of-module (create →
  write{alloc+insert} → read{point lookup, join, aggregate} → mutate via
  delete+insert in either order → read again) — the doc's examples become tests
  verbatim; a panicking write closure leaves prior state intact and the writer mutex
  released; two threads reading concurrently while a third writes (module contract of
  the mutex + cache wiring, small and deterministic); export scan round-trips into a
  fresh database via bulk_load with explicit serials preserving identity and equal
  query results (the unit-level seed of the human ETL story); public surface reviewed
  against `60-api.md` (checklist in the PR description).
- Global commands green. The crate documents (rustdoc front page) the three-command
  contract and points at `docs/architecture/`.
