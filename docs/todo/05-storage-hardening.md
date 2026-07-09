# 05 — Storage hardening: reopen-trust ceiling and the reader-slot cap

**Kind:** hardening — two small items with the same theme: state read back at open
time is trusted further than it should be, and one advertised property has no
configuration behind it. Both decided; no options remain.

## Item A — corrupt-but-plausible `S` count drives an unbounded allocation

### Current behavior

Image build sizes its slabs from the `S` row counter
(`crates/bumbledb/src/image/build.rs:47-78`). `slab_lengths` (`build.rs:19-32`)
uses checked arithmetic and returns typed `Corruption` on **overflow** before
allocating — good. But a merely-large, non-overflowing corrupt count (say 2^40)
passes the checked math and drives `vec![0u64; word_len]` into a multi-terabyte
allocation: abort/OOM instead of a clean `Corruption` error. `S` is engine-written
and fsync-guarded, so the risk is low — but it is untrusted-at-reopen bytes, and
every other decode path (fact width checks, dict tag checks, interval word checks)
converts corruption into typed errors rather than crashes.

### The work

Before allocating, bound the claimed row count by a witness the store itself
provides — a *ceiling*, not an equality: either `row_count × fact_width ≤ LMDB
data file size` or `row_count ≤ _data DBI entry count` (`mdb_stat`, O(1);
entries span F/M/U/R so the count over-approximates, which a ceiling is allowed
to do — pick whichever reads cleaner at the call site and comment the
over-approximation). Exceeding the ceiling returns `Corruption(CounterDesync)`
(new variant) naming relation, claimed count, and the witness. One test:
hand-corrupt `S` in a fixture store, assert the typed error, not an abort.

## Item B — `max_readers` left at LMDB's default 126

### Current behavior

`open_env` configures `map_size` and `max_dbs(3)` only
(`crates/bumbledb/src/storage/env/open_env.rs:20`); LMDB's reader-table default is
126 slots. The design's scaling axis is **inter-query parallelism**
(`00-product.md`), and with `MDB_NOTLS`, slots bind to open transaction objects
(including the parked reader, `api/db.rs:159`). The 127th concurrent snapshot
fails with `MDB_READERS_FULL` surfacing as a generic `Lmdb` error;
`storage/env/txn.rs:17` even names reader-slot exhaustion as the failure mode —
a promise with no mechanism behind it, the inverse of README rule 3.

### The work

Set `max_readers = 1024` explicitly at open — a decision, not a knob. Measure and
record the reader-table cost of the raise (bytes per slot region) in the constant's
doc comment, documented next to `MAP_SIZE`'s "comfortably above the axiom"
rationale (`storage/env.rs:39`). Map `MDB_READERS_FULL` to a typed error naming
the limit rather than a raw `Lmdb` passthrough.

## Acceptance

- Corrupt-`S` fixture test returns `Corruption`, process alive.
- A stress test opening >126 concurrent snapshots succeeds (and the typed, named
  error is asserted above the new limit if provokable cheaply).

## Doc amendments (rule 5)

`50-storage.md`: the counter-trust paragraph gains the ceiling; the environment
constants table gains `max_readers` with its rationale sentence.
