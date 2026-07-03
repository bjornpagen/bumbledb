# PRD 15 — Write and cold benchmarks

Authority: `00-product.md` (bursty batched writes; cold reported not gated), the
suite ruling: writes and cold are `Kind::Report`.

## Purpose

The numbers that describe the write half and the rebuild spike honestly, without
gating on them: single-commit latency (fsync-bound), bulk throughput, and cold
first-execution.

## Technical direction

- `families::write_families()` registry, `Kind::Report`, measured by PRD 13's
  harness with write-appropriate protocols:
  1. **commit_single** — one sample = `db.write`: alloc a PostingId, insert one
     posting (fields seeded, referencing existing account/instrument/transfer),
     via the typed path. `Protocol { warmups: 8, samples: 64 }`. The SQLite
     mirror: one `INSERT` inside `BEGIN IMMEDIATE … COMMIT` with bound params on
     a reused prepared statement (synchronous=FULL — the same fsync bill).
  2. **commit_batch** — one sample = one `db.write` inserting 512 postings;
     SQLite mirror: 512 bound INSERT executions in one transaction.
     `{ warmups: 4, samples: 32 }`.
  3. **bulk** — one sample = `bulk_load` of a fresh 100_000-posting stream into
     a scratch copy of the corpus db (the sample must not mutate the shared
     corpus: each sample loads into relation rows with ids offset by sample
     index × 10⁶ and DELETEs them afterwards? No — deletion pollutes; instead
     each bulk sample runs in a **throwaway directory** seeded by
     `load_bumbledb` at S minus postings, timed on the posting load only).
     SQLite mirror identical shape. `{ warmups: 1, samples: 8 }`; report
     facts/sec.
  4. **cold_fk_walk** — PRD 13's `measure_cold` over the fk_walk family
     (rebuild spike: image build dominates). No SQLite mirror (SQLite has no
     comparable cache concept; report absolute, per the doc's "cold reported").
- Post-run integrity: commit_single/commit_batch leave the corpus db grown —
  runs against a scratch copy of the corpus directory (the runner copies or
  reloads; document that bench never mutates the verified corpus in place, so
  the verify stamp remains honest). Concretely: `bench` loads its own corpus per
  invocation (regeneration is cheap at S/M; at L the CLI's digest-keyed cache
  directory from PRD 19 provides a pristine copy).
- Serial-id collision discipline for inserted postings: ids minted via
  `tx.alloc` (never explicit), so samples cannot collide with corpus ids.

## Non-goals

Gating any of these ("every family must win" is the read set). Mixed
read-while-write scenarios (owner decision later; the concurrency tests own
correctness there).

## Passing criteria

- Unit tests at S: each write family runs one full protocol without error and
  reports plausible invariants (commit_single stats.min > 0; bulk facts/sec > 0;
  cold p50 ≥ warm fk_walk p50 on the same corpus — asserted with a 1× margin
  only, it is an inequality about rebuild cost); corpus-preservation test: after
  commit_single's protocol, the *scratch* db differs but the source corpus
  directory's digest-of-generation is untouched (assert by running against a
  copy and comparing the original's `Db::generation` before/after).
- `scripts/check.sh` green.
