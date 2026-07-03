# PRD 04 — Write-path and image instrumentation

Authority: PRD 02's seam; `40-storage.md` commit phases.

## Purpose

Commit-phase spans with fact counts, the fsync isolated, bulk chunks visible — the
write-side half of "where did the time go".

## Technical direction

- `storage/commit.rs::commit`: outer span `commit` (Category::Commit, `a0` = net
  changed 1/0 via set_args); child spans mirroring the canonical order:
  `apply_deletes` (`a0` = facts deleted), `apply_inserts` (`a0` = facts inserted),
  `fk_forward` (`a0` = deduped probe count), `fk_restrict` (`a0` = scanned guard
  count), `counters_flush` (`a0` = pending interns flushed), `lmdb_commit` (the
  `txn.commit()` call alone — the fsync-bound number, isolated). The skip-empty
  early return emits event `commit_noop`.
  Counts flow from the existing `Applied` bookkeeping — do not add new tallies to
  the apply loops; read the sizes the code already has (delete/insert counts come
  from the delta's disposition partition sizes; compute once before the loops).
- `api/db.rs::bulk_load`: span `bulk_chunk` per chunk (`a0` = facts submitted,
  `a1` = facts changed).
- `api/db.rs::write`: span `write_txn` (Category::Commit) wrapping closure + commit
  (`a0` = 1 committed / 0 aborted-by-error, via set_args before the `?`).
- `image.rs::build` is already spanned from PRD 03 (`image_build`); add `a1` byte
  size once PRD 05 exposes it (leave `a1` = rows until then; PRD 05 flips it — note
  the handoff in both PRDs).
- Names into `obs::names` with arg docs.

## Non-goals

Per-operation delta events (insert/delete record calls are the application's hot
loop; the commit spans carry the aggregate counts).

## Passing criteria

- Unit tests (feature `trace`): capturing a `Db::write` that inserts 3 facts and
  deletes 1 asserts the six phase spans in exact order within `commit`, with
  `apply_deletes.a0 == 1`, `apply_inserts.a0 == 3`; an all-no-op write captures
  `commit_noop` and no phase spans; a `bulk_load` of 2.5 chunks captures three
  `bulk_chunk` spans with correct `a0` sums; an aborting write (constraint
  violation) captures `write_txn` with `a0 == 0` and no `lmdb_commit` span.
- Release allocation gate green; default-feature check green.
