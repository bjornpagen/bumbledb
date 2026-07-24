## An ephemeral store that crossed a machine crash reopens as "verified" — the kind's own loss claim is undetectable on disk

category: incoherence | severity: low | verdict: CONFIRMED | finder: r2:crash-recovery-lifecycle
outcome: fixed ef5b9a42 + 70fb5f5d (R18)

### Summary

`Db::ephemeral`'s existing-store arm promises "the same version/kind/fingerprint checks as `open`", but those checks read only the `_meta` block. The ephemeral kind's own documentation — the `NO_SYNC` SAFETY comment, the `Db::ephemeral` rustdoc, and `50-storage.md` — states that a machine crash loses the store BY THE KIND'S OWN CLAIM. Yet nothing on disk records that a crash occurred: the `_meta` key roster has no cleanliness or epoch marker, so the reopen path cannot distinguish a cleanly handed-off scratch store from a claimed-lost one. It opens the latter "verified", and the loss surfaces later as read-time `Corruption` errors or, worse, as structurally valid stale pages served as current data. The state "this ephemeral store's contents are undefined" is real per the project's own claim but unrepresentable — the inverse of parse-don't-validate (`docs/design/representation-first.md` lens: a true condition of the store that no representation carries).

### Evidence (all verified against the code)

- `crates/bumbledb/src/storage/env/ephemeral.rs:14-24` — rustdoc: "an existing ephemeral store is opened (version, kind, fingerprint — the same checks as `Environment::open`)".
- `crates/bumbledb/src/storage/env/ephemeral.rs:58-68` — the reopen arm is `probe_ephemeral_kind` (meta-only reads, lines 100-134) followed by `verify_and_open`; no other signal is consulted.
- `crates/bumbledb/src/storage/env/open.rs:45-102` — `verify_and_open` checks format version, kind byte, `_data`/`_dict` presence, and fingerprint; it never walks data pages and consults no lifecycle state.
- `crates/bumbledb/src/storage/env.rs:195-212` — the COMPLETE `_meta` key roster: format version, fingerprint, tx id, dict next id, store kind, schema descriptor. No dirty/clean marker exists, so the distinction is structurally unrepresentable.
- `crates/bumbledb/src/storage/env/open_env.rs:58-68` — `EnvFlags::NO_SYNC` applied for the ephemeral kind; the SAFETY comment: "NO_SYNC trades machine-crash durability away... a machine crash loses the store". Same claim at `crates/bumbledb/src/api/db/open.rs:47-48` ("a machine crash loses the store BY THE KIND'S OWN CLAIM") and `docs/architecture/50-storage.md` § the ephemeral store kind ("a machine crash loses an ephemeral store by the store's own definition").
- The mechanism is real LMDB semantics, not speculation: under `MDB_NOSYNC` without `WRITEMAP` (the post-ruling-1 flag set), LMDB's documentation conditions crash integrity on the filesystem preserving write order — which APFS/ext4 page-cache writeback does not guarantee. A meta page can reach disk while freed-and-reused data pages it transitively references never do, yielding a fingerprint-valid meta over torn or generation-mixed trees.
- The project's banked crash evidence covers only process-kill, not power loss: `50-storage.md` § the ephemeral store kind explicitly notes "`NOSYNC` removes the fsync barrier, which only a power loss can exploit", and the retired deterministic sweep was a commit-pipeline crashpoint (process) sweep. No test or doc addresses reopen-after-machine-crash.
- Reopen is a blessed flow, not caller abuse: `docs/architecture/70-api.md` § environment lifecycle documents create-or-open, and `crates/bumbledb/tests/ephemeral.rs:88-110` pins that contents survive a clean process handoff — the on-disk state after a reboot is indistinguishable from that pinned clean case.

### Failure scenario

A staging ETL box runs the two-store staging pattern (`70-api.md`): an ephemeral store accumulates a day of judged facts under `NOSYNC`; the machine loses power; on reboot the orchestrator calls `Db::ephemeral(path)` to resume. The meta pages (flushed incidentally by background writeback) pass version/kind/fingerprint; the store opens "verified". Reads then either raise `Corruption` mid-ETL, or — if the torn pages are older CoW versions — silently serve a generation-mixed state that no committed transaction ever contained, and `snap.scan → bulk_load_dyn` ingests judged-looking facts into the durable store with no error at all.

### Suggested fix

Make the kind's own claim representable, either of:

1. **Wipe-and-reinit lineage marker (truest to scratch semantics):** persist one fsynced "opened" marker at each ephemeral open and clear it via a small synced commit at clean close (drop); at reopen, a present marker means the store may have crossed an unclean shutdown — re-initialize (the kind already promises nothing survives a machine crash, and `70-api.md` already frames a mistaken fresh store as destroying nothing durable). Reserve verified-reopen for a marker-proven clean lineage. Note this touches the "Db::ephemeral never destroys data" law in `70-api.md` § environment lifecycle, so it needs a ruling: the law was minted against typo'd paths and refusals, not against a store the kind itself declares lost.
2. **Detect-and-refuse:** the same marker, but a dirty reopen returns a typed refusal (e.g. `EphemeralUncleanShutdown`) and the host decides — keeps refusal-never-mutates intact at the cost of one more error variant.

Either way, "verified" stops vouching for pages the meta block cannot see.
