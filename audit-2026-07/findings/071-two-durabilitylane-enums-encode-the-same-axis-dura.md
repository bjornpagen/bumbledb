## Two DurabilityLane enums encode the same axis — duralane.rs vs lanes/writes.rs

category: unification | severity: medium | verdict: CONFIRMED | finder: bench:honesty
outcome: fixed e5f35cb2 (R22)

### Summary

The bench crate defines the durability axis twice, as two independent closed sums with the same two points and the same intended pragma law:

- `crates/bumbledb-bench/src/duralane.rs:30-33` — `DurabilityLane { Durable, Nosync }`, with `store_mode()` (42-47), `label()` (51-56), `describe()` (60-76), `configure()` (94-115), and an `assert_parity()` pragma readback (127-164). Consumed by the crud lane (`crud/corpus.rs:11`, `crud/run.rs:24`) and the lawful lane (`lawful/load.rs:11`).
- `crates/bumbledb-bench/src/lanes/writes.rs:63-67` — a second `DurabilityLane { Durable, NoSync }`, with `store_mode()` (90-95), identical `"durable"`/`"nosync"` labels (72-77), `sqlite_sync_label()` (81-86), and `apply_sqlite()` (107-124) — and **no parity readback anywhere in the file**. Consumed by the writes lane itself and by the CLI `--lanes` parser (`cli.rs:11`, `cli/parse.rs:4,252-253`).

Both types are live simultaneously; nothing cross-checks them; and the lane that publishes the fsync-sensitive commits/sec ladders (writes) is exactly the one without the FairnessCheck-style readback that duralane carries.

### Evidence

All citations verified against the working tree.

1. **Two enums, one law, parallel appliers.** duralane's `Durable` arm delegates to `corpus::configure_sqlite` (`corpus.rs:87-89`: `synchronous=FULL`, `fullfsync=ON`, `checkpoint_fullfsync=ON`), and its `Nosync` arm sets the OFF trio (`duralane.rs:105-107`). The writes enum re-spells the identical trio by hand in `apply_sqlite` (`writes.rs:113-117` Durable, `writes.rs:118-122` NoSync). The mappings agree today only by parallel maintenance.

2. **The readback exists only on one side.** `assert_parity` (`duralane.rs:127-164`) is called at `crud/corpus.rs:95` and `lawful/load.rs:84`, and its cross-mismatch behavior is tested (`crud/tests.rs:77-87`). The writes lane calls `apply_sqlite` at `writes.rs:555` (bulk throwaway oracles) and `writes.rs:749` (`run_lane`) with no readback; a grep for `PRAGMA`/`query_row`/`assert_parity` over `writes.rs` finds nothing. Its tests (`writes.rs:956-975`, `the_durability_axis_has_exactly_two_points`) pin only labels and `store_mode`, never the applied pragmas.

3. **The code contradicts the recorded architecture decision.** `docs/architecture/60-validation.md:693-695`: "Durability parity is the `DurabilityLane` sum (`crates/bumbledb-bench/src/duralane.rs` — **the one constructor of both sides' config and the authority for every pragma**)", and :704-706 credits `assert_parity` with making a misconfigured twin "fail before flattering anyone." The writes-local twin enum with its own unaudited pragma applier is exactly what that clause rules out. The doc's "Reverses if: never" makes this a spec divergence, not a style nit.

4. **The fork is not hypothetical — the envelopes have already drifted.** `duralane::configure` sets `mmap_size=1GiB` and `wal_autocheckpoint=0` on every session, both arms (`duralane.rs:112-113`), and `describe()` advertises them in the artifacts. The writes lane's oracle sessions (`corpus::load_sqlite`/`configure_sqlite` + `apply_sqlite`) set **neither** — `corpus.rs` contains no `mmap_size` or `wal_autocheckpoint` pragma at all (those live in `sqlite_run::open_for_bench`, which the writes lane does not use). So the crud/lawful lanes and the writes lane already run their SQLite twins under different session envelopes while printing the same `"durable"`/`"nosync"` lane labels.

### Failure scenario

An edit to one applier — dropping `checkpoint_fullfsync` from one Durable arm, or adding a new pragma to `duralane::configure` only — forks the meaning of "durable"/"nosync" between the crud/lawful lanes and the writes lane with no compile error and no test failure, since no test relates the two types. Worse, because the writes lane has no readback, a pragma that fails to take effect there (SQLite silently ignores unknown/no-op pragma states; a swallowed `pragma_update` upstream) times the SQLite twin under the wrong sync level, and the published fsync-ladder numbers carry a durability label the session does not actually satisfy — precisely the "misconfigured twin flattering someone" that `assert_parity` exists to kill, per the doc clause.

### Suggested fix

Collapse to the one `duralane::DurabilityLane` the architecture doc already names as the authority: move `sqlite_sync_label()` onto it, have `lanes/writes.rs`, `cli.rs`, and `cli/parse.rs` import it, delete the writes-local enum, and call `assert_parity` after every pragma application site (`writes.rs:555` and `writes.rs:749` included). While unifying, decide the mmap/autocheckpoint envelope question once — either the writes lane's oracle sessions get the same `mmap_size`/`wal_autocheckpoint` settings or `describe()`/the report labels stop implying a single shared envelope — so the readback can pin the whole session config, not just the sync trio.
