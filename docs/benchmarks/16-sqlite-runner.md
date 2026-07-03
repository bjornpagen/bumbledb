# PRD 16 — The SQLite runner and the fairness contract

Authority: `50-validation.md` (fully-indexed, prepared statements, ANALYZE,
synchronous=FULL), `00-product.md` (the claim is against SQLite's best self), the
WAL ruling.

## Purpose

SQLite measured under exactly the engine's protocol, with the fairness rules
encoded as assertions — a benchmark nobody can dismiss as a strawman.

## Technical direction

- `sqlite_run::PreparedFamily<'c> { stmt: rusqlite::Statement<'c>, param_order:
  Vec<ParamId>, result_types: Vec<ValueType> }` built once per family from PRD
  09's `Translated` — prepared exactly once, reused across every warmup and
  sample (mirroring `PreparedQuery`).
- `sqlite_run::sample(&mut PreparedFamily, params: &[Value]) -> Result<u64>`: bind
  via `sqlmap::to_sql_value`, iterate ALL rows with typed getters (drain — no
  lazy-cursor discounts), return the row count for the harness's black-box/work
  contract. Decoding into `compare::Owned` is verify's job, not the timed
  path's: the timed path does typed `get` calls and counts (both engines pay
  decode; bumbledb's timed path also materializes into ResultBuffer — comment
  the symmetry argument: ours materializes to a buffer, theirs does typed reads;
  both touch every value).
- Session setup for timing runs (`sqlite_run::open_for_bench(path)`):
  `journal_mode=WAL`, `synchronous=FULL`, `cache_size=-262144`,
  `mmap_size=1073741824`, `wal_autocheckpoint=0` during read runs (checkpoint
  once before measuring: `wal_checkpoint(TRUNCATE)`), then one warm-the-cache
  full scan? **No** — warmups are the warm-up, identically to ours. Every
  pragma's fairness rationale is a comment.
- The fairness contract as code — `sqlite_run::FairnessCheck` run before
  measuring: asserts journal mode, synchronous level, presence of every expected
  index (`PRAGMA index_list` per table vs the PRD 08 index registry), ANALYZE
  ran (`sqlite_stat1` non-empty), statement reuse (the runner owns the only
  construction site — assert by type: `PreparedFamily` has no `re-prepare`
  path).
- Write mirrors (PRD 15's shapes) live here too: `sqlite_run::commit_single` /
  `commit_batch` / `bulk` using prepared INSERTs and explicit transactions.

## Non-goals

Tuning SQLite beyond documented best practice (no carray, no unsafe pragmas —
`synchronous=FULL` is pinned by the doc). Testing SQLite itself.

## Passing criteria

- Unit tests: FairnessCheck passes on a PRD 08-loaded S corpus and fails when an
  index is dropped (test drops one, asserts the named failure);
  `sample` drains rows (count equals a COUNT(*) cross-check) and re-binding
  across param sets returns differing counts; prepared-once discipline —
  construct one PreparedFamily and run 100 samples without error; write mirrors
  execute at S with sane invariants (single < batch total time, etc. — direction
  only, no thresholds).
- `scripts/check.sh` green.
