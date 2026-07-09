# PRD 08 — Storage hardening: reopen-trust ceiling and the reader-slot cap

**Depends on:** nothing.
**Modules:** `crates/bumbledb/src/image/build.rs`,
`crates/bumbledb/src/storage/env/{open_env.rs,txn.rs}`,
`crates/bumbledb/src/storage/env.rs`, `crates/bumbledb/src/error.rs`.
**Authority:** `50-storage.md` (typed-corruption doctrine: reopen-trusted bytes
convert to typed errors, never crashes; the environment-constants rationale
style next to `MAP_SIZE`).

## Item A — corrupt-but-plausible `S` count drives an unbounded allocation

`slab_lengths` (`image/build.rs:19-32`) checks for *overflow* and returns typed
`Corruption` — good — but a merely-large corrupt count (2^40) passes checked math
and drives `vec![0u64; word_len]` into a multi-terabyte allocation: OOM/abort
instead of a typed error. Every other reopen-trusted decode path gets a typed
error; this one gets a crash.

**Direction:** before allocating, bound the claimed count by a witness the store
provides — a *ceiling*, not an equality. Either `row_count × fact_width ≤ LMDB
data-file size` or `row_count ≤ _data DBI entry count` (`mdb_stat`, O(1); the
DBI spans F/M/U/R so the entry count over-approximates — a ceiling is allowed
to; comment the over-approximation). Pick whichever reads cleaner at the call
site. Exceeding the ceiling returns a new
`CorruptionError::CounterDesync { relation, claimed, witness }` (follow the
existing corruption-variant payload style: ids and numbers, no strings).

## Item B — `max_readers` left at LMDB's default 126

`open_env` (`storage/env/open_env.rs:20`) configures `map_size` and `max_dbs(3)`
only; the reader table defaults to 126 slots. Inter-query parallelism is the
design's scaling axis (`00-product.md`), `MDB_NOTLS` binds slots to open
transaction objects (including the parked reader, `api/db.rs:159`), and
`storage/env/txn.rs:17` *names* reader-slot exhaustion as the failure mode — a
promise with no mechanism behind it. The 127th concurrent snapshot fails as a
generic `Lmdb` error.

**Direction:** set `max_readers(1024)` at open — a decision, not a knob. Measure
the reader-table cost of the raise (LMDB allocates the reader table in the lock
file; record bytes-per-slot and the total) and put the number in the constant's
doc comment, styled like `MAP_SIZE`'s "comfortably above the axiom" rationale
(`storage/env.rs:39`). Map `MDB_READERS_FULL` to a typed error naming the limit
(new variant or a named arm of the open/read error mapping — follow the existing
LMDB error-mapping site) instead of a raw passthrough.

## Passing criteria

- `[test]` Item A: a fixture store with a hand-corrupted large-but-plausible `S`
  value returns `Corruption(CounterDesync)` from the image build — process
  alive, payload names relation/claimed/witness.
- `[shape]` The ceiling check sits before any size-derived allocation in the
  build path; the over-approximation comment exists.
- `[test]` Item B: a stress test holding >126 concurrent read snapshots
  succeeds (thread spawn + barrier; keep it fast — open, hold, release). If the
  new limit is cheaply provokable, assert the typed error above it; otherwise
  the mapping gets a unit test at the error-conversion layer.
- `[shape]` `max_readers` is set at open with the measured-cost doc comment.
- `[gate]` Workspace gates green.

## Doc amendments (rule 5)

`50-storage.md`: the counter-trust paragraph gains the ceiling; the environment
constants get `max_readers` with its rationale sentence.
