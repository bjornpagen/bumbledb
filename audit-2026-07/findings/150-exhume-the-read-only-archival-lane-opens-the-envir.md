## exhume — the spec'd read-only archival lane — opens the env read-write and demands a writable lock file, so it cannot read a store on read-only media

category: missing-free-feature | severity: low | verdict: CONFIRMED | finder: r2:crash-recovery-lifecycle
outcome: fixed ef5b9a42 (R17)

### Summary

`bumbledb::exhume` is specified as "the read-only, theory-less open" (docs/architecture/70-api.md:390 § Exhume; docs/architecture/50-storage.md:71), whose stated sighting is archival: "a run store whose creating schema has since evolved — the record outlives the schema, and exhume is how the record is read back" (the rebirth pattern). But nothing at the storage layer is read-only: the lock file is opened for writing with `create(true)`, the LMDB environment is opened without `MDB_RDONLY`, and dbi registration goes through a real write transaction. On exactly the media the archival story implies — a read-only bind mount, a restored snapshot, a mounted backup — exhume fails with a raw `Error::Io(EROFS/EACCES)` before any typed check runs. Read-only-ness is currently enforced only by API-surface omission on `Exhumed`; the storage layer's write path remains representable, against the doctrine in docs/design/representation-first.md (make illegal states unrepresentable).

### Evidence (all verified against the working tree)

- crates/bumbledb/src/storage/env/exhume.rs:53-59 — `exhume` calls `acquire_lock(path)` (line 54), then `open_env(path, StoreKind::Durable)` (line 55), then `let wtxn = env.write_txn()?;` (line 59) with the comment "it commits without writing anything — exhume never mutates a store, not even to adopt it."
- crates/bumbledb/src/storage/env/acquire_lock.rs:12-16 — the lock file opens with `.create(true).truncate(false).write(true)`; on a read-only filesystem this is the FIRST failure (EROFS/EACCES), before LMDB is ever touched.
- crates/bumbledb/src/storage/env/open_env.rs:28-74 — the one env-open chokepoint sets no `READ_ONLY` flag for any store kind; the only conditional flag is `NO_SYNC` for `StoreKind::Ephemeral` (line 68). LMDB's documented rule: an environment on a read-only filesystem must be opened with `MDB_RDONLY`, so the rw open would also fail even past the lock file.
- heed 0.22.1 (the locked dependency, Cargo.toml:63) exposes the flag: `src/mdb/lmdb_flags.rs:25: const READ_ONLY = ffi::MDB_RDONLY;` — the fix costs no new dependency.
- crates/bumbledb/src/api/db/exhume.rs:22-36 — `Exhumed` exposes descriptor/fingerprint/kind/read only; the doc comment says "No write surface exists on this type... never takes the writer path." The no-write claim is API-layer-only.
- docs/architecture/70-api.md § Exhume documents the exclusive advisory lock as deliberate ("one handle per path — the record being read stays still") but says nothing about read-only media; a grep for readonly/READ_ONLY/EROFS/EACCES/chmod/set_permissions across `src/storage/env`, `src/api/db/exhume`, and `tests/` returns nothing — no handling, no test.

### Failure scenario

`bumbledb::exhume("/mnt/backup-ro/runstore")` against a read-only mount: `acquire_lock` fails opening `bumbledb.lock` for write with `Error::Io(EROFS)` (or EACCES); were the lock somehow satisfied, the rw `mdb_env_open` fails next. The pure-read operation the type exists for is impossible on the media its own spec names, and the caller must first copy the store to writable disk.

### Suggested fix

Give exhume its own env-open arm carrying `EnvFlags::READ_ONLY` (a second arm in `open_env`, or a dedicated `open_env_readonly`, keeping the unsafe confined to that module per its unsafe-policy header). Register dbis through a read transaction — LMDB permits opening existing named databases in a read-only txn, which erases the write-txn-that-writes-nothing oddity at exhume.rs:56-59. For the advisory lock, either open `bumbledb.lock` read-only with a shared lock, or accept lockless open when the filesystem is read-only (an MDB_RDONLY env cannot corrupt anything; LMDB itself tolerates an unwritable lock file in this mode). The write path then becomes unrepresentable at the storage layer, matching the API layer's claim — the representation-first doctrine applied to the one lane whose whole identity is "never writes."
