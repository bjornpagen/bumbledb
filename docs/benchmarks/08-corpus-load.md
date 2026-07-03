# PRD 08 — Corpus loading: bumbledb and SQLite

Authority: `50-validation.md` (protocol: fully-indexed SQLite, synchronous=FULL,
ANALYZE), the benchmark discussion rulings (WAL; value mapping normative), PRDs
06–07.

## Purpose

One corpus, two stores, identical contents — the precondition for every verify and
every timing.

## Technical direction

- `corpus::load_bumbledb(db: &Db, cfg: &GenConfig) -> Result<LoadStats>`: per
  relation in FK-safe order (Currency, Holder, Instrument, Account, Transfer,
  Posting, Tag, AccountTag, TagNote) — `db.bulk_load(rel, gen::relation_rows(..))`.
  `LoadStats { facts: u64, wall: Duration, facts_per_sec: f64 }` (this number is
  also the bulk benchmark's cold source, PRD 15).
- `corpus::load_sqlite(conn: &rusqlite::Connection, cfg) -> Result<LoadStats>`:
  - DDL from the schema descriptors programmatically (`sqlmap::ddl(schema) ->
    Vec<String>`): table per relation, column types by the normative mapping —
    Bool→INTEGER, Enum→INTEGER, U64→INTEGER, I64→INTEGER, String→TEXT, Bytes→BLOB;
    `STRICT` tables; NOT NULL everywhere (no nulls exist).
  - **Fully indexed**: a UNIQUE index per unique constraint (incl. serial
    auto-uniques → `PRIMARY KEY` where it is the single serial field, else UNIQUE
    index), an index per FK field list, plus the family composites (declared in
    PRD 14 and created here from a shared `EXTRA_INDEXES` const): `posting(account,
    at)`, `posting(memo)`, `posting(instrument)`.
  - PRAGMAs before load: `journal_mode=WAL`, `synchronous=FULL`,
    `cache_size=-262144` (256 MB, documented), `temp_store=MEMORY`. Insert via one
    prepared statement per table inside transactions of 4096 rows (mirroring the
    engine's chunk). `ANALYZE` after load; `wal_checkpoint(TRUNCATE)` after ANALYZE.
  - Every PRAGMA and index choice carries a fairness comment naming why.
- `sqlmap::to_sql_value(&Value) -> rusqlite::types::Value` and back — the one
  mapping both loader and runner use; u64 conversion asserts `< 2^63`.
- Equality check helper `corpus::assert_loaded_equal(db, conn)` (test-support,
  `pub`): per relation, bumbledb `row_count` == `SELECT COUNT(*)`; then 100 seeded
  sample facts fetched from SQLite by serial id and byte-compared against
  `scan`-side values.

## Non-goals

Timing protocols (PRD 13). Reusing on-disk corpora across runs (regeneration is the
identity story; the CLI may cache directories keyed by digest — PRD 19's concern).

## Passing criteria

- Unit tests at S scale (S is a test-scale corpus by design): both loads complete;
  `assert_loaded_equal` passes; DDL golden test (pinned SQL for the nine tables +
  index list); PRAGMA verification test (query `journal_mode` back == "wal",
  `synchronous` == 2/FULL); value round-trip test for all six types through
  `to_sql_value` and back.
- `scripts/check.sh` green.
