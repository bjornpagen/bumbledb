## Db::create never fsyncs the store directory's dirent chain — create-time power loss can lose the whole store

category: bug | severity: medium | verdict: CONFIRMED | finder: r2:crash-recovery-lifecycle

### Summary

`Environment::create` (the body of `Db::create`) makes the store directory with `create_dir_all`, lets LMDB create `data.mdb`, commits the `_meta` block — and returns without ever fsyncing a directory. LMDB fsyncs file *contents* per commit and never opens a directory at all (verified in the vendored `lmdb-master-sys-0.2.6` `mdb.c`: no `O_DIRECTORY`, no directory fsync anywhere), so the dirents for `data.mdb` and for the store directory itself are durable only when the filesystem's journal happens to flush them. This contradicts the durable kind's product law — "fsync per commit; a committed posting survives power loss" — and, more damningly, the project's *own* recorded standard: `Db::compact` implements and documents exactly the missing rule for its copy ("the file, its dirent in `dest`, then `dest`'s own dirent in the parent — the whole dirent chain a power loss would have to survive"). One mechanism, implemented in one of its two sites.

### Evidence (all verified against the working tree)

- `crates/bumbledb/src/storage/env/create.rs:29-34` — `create` is `create_dir_all(path)` → `acquire_lock` → `open_env` → `initialize`; `initialize` ends at line 92 with `wtxn.commit()?`. No `sync_all` on any directory anywhere in the path.
- `crates/bumbledb/src/api/db/open.rs:21-23` — `Db::create` is a thin wrapper over `Environment::create`; nothing above adds a sync.
- `grep -rn sync_all crates/bumbledb/src` — hits only `api/db/maintain.rs` (lines 59, 62). The create path has zero fsync sites of its own.
- `crates/bumbledb/src/api/db/maintain.rs:31-37, 56-64` — compact's doc and code are the project's recorded dirent-chain standard: `file.sync_all()` then `for dir in [dest, parent_dir(dest)] { ... dir.sync_all() }`, with the `COMPACT_DURABLE` obs event (obs.rs:205-208) and a trace test (`api/db/trace_tests.rs:300-323`) pinning that the directory syncs executed.
- `crates/bumbledb/src/storage/env.rs:70-72` and `docs/architecture/00-product.md:147-149` — the durable kind's claim: "fsync per commit on durable stores... A committed posting survives power loss — it's a ledger."
- LMDB itself does not close the gap: `~/.cargo/registry/src/.../lmdb-master-sys-0.2.6/lmdb/libraries/liblmdb/mdb.c` contains no directory open/fsync; its one `F_FULLFSYNC` site is the data-file sync define. `docs/architecture/50-storage.md` § durability discusses only the per-commit file fsync and the open-time checks; create-time dirent durability appears nowhere.
- `create.rs:58-63` shows the create-crash case was reasoned about only for *process kill* ("a half-created bumbledb store... has an empty root and still proceeds") — power-loss metadata durability was never addressed.

### Corrections to the original finding

- The `Db::ephemeral` fresh-init arm is **not** part of the bug: the ephemeral kind's on-disk contract explicitly renounces machine-crash durability (`env.rs:73-78`, 50-storage.md § the ephemeral store kind), so no dirent sync is owed there.
- Severity is medium, not high: on ext4/XFS/btrfs the initialize commit's own file fsync in practice drags the create transaction's dirents into the journal flush, so the store survives. The unguaranteed cases are strict-POSIX semantics and APFS (the primary dev platform), where fsync/`F_FULLFSYNC` of a file is not documented to persist its directory entry. The window is real but narrow: from `Db::create` until the filesystem's periodic metadata checkpoint (seconds to tens of seconds).

### Failure scenario

`Db::create("/data/ledger")` on a filesystem where file fsync does not imply dirent durability (APFS being the undocumented case); the host performs several `Db::write` commits, each returning Ok with its content F_FULLFSYNC-durable; power is lost before the filesystem checkpoints directory metadata. On reboot `/data/ledger` or `/data/ledger/data.mdb` is absent: every committed posting is gone even though every commit reported fsynced success. The durable lane's ledger claim fails silently — no error ever surfaced.

### Suggested fix

After `initialize`'s `wtxn.commit()`, sync the dirent chain exactly as compact already does: fsync `path` (data.mdb's directory entry) and `parent_dir(path)` (`path`'s own entry). Hoist `maintain.rs`'s `parent_dir` + dir-sync loop into one shared helper used by both compact and create — unification of two mechanisms that are secretly one, in the representation-first spirit (`docs/design/representation-first.md`); the helper can share (or sibling) the `COMPACT_DURABLE`-style obs event so the trace suite can pin the create-path syncs the same way `trace_tests.rs:323` pins compact's. Cost is two directory fsyncs once per store lifetime; steady-state commits are untouched.
