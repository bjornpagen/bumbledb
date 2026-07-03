# PRD 09 — Store compaction and size honesty

Authority: `docs/architecture/40-storage.md` (the `_data` keyspace, LMDB
geometry), suite README finding 5. Independent of PRDs 00–08.

## Purpose

The S corpus store is a 101 MB file holding ~64 MB of live pages: 39 % is
freelist churn from the 46 bulk-load commits, which LMDB never returns to the
filesystem. The other multipliers — ~5–6 `_data` entries per fact
(fact + membership hash + unique guard + one back-reference per FK) and 16 KB
Apple Silicon pages — are deliberate design rent for O(1) commit-time
constraint checks, and they stay. The churn does not: give the engine a
compaction door and make the bench's corpus cache walk through it, so the
digest-keyed store (and especially the L-scale one) is live-sized.

## Technical direction

- **`Db::compact`** in `api/db.rs`:

  ```rust
  /// Writes a compacted copy of the store to `dest` (a directory that
  /// must not exist): live pages only, freelist dropped, sequential
  /// layout. The source stays open and untouched — compaction is a
  /// copy, never in-place (crash-safe by construction: the source is
  /// the fallback until the caller swaps directories).
  pub fn compact(&self, dest: &Path) -> Result<()>
  ```

  Implemented over heed's environment copy with compaction
  (`Env::copy_to_file` + `CompactionOption::Enabled` in heed 0.22 — verify the
  exact API against the vendored version and note it; LMDB underneath is
  `mdb_env_copy2(MDB_CP_COMPACT)`). Create `dest`, copy into
  `dest/data.mdb`, fsync the file and the directory. No lock file is copied —
  LMDB recreates it on open.
- **The bench corpus cache compacts after load.** `driver::ensure_corpus`'s
  loader: load into `<root>/db-load/`, `compact` into `<root>/db/`, remove
  `db-load/`, then write the `corpus.ok` marker (marker-last ordering already
  makes partial states invisible). The verify/bench/trace paths open the
  compacted store with zero changes. Bulk-loaded corpora are exactly the
  churn-heavy case — 46 commits of CoW growth for a write-once store.
- **Size honesty in the report.** `report::StoreNumbers` gains
  `db_file_bytes` semantics clarification only — after compaction file ≈ live,
  so the single number becomes honest rather than gaining a sibling. The
  markdown line changes to `bumbledb file (compacted): N bytes`. Re-pin the
  report markdown golden.
- **What stays and why, written down:** amend `40-storage.md` with the
  measured anatomy — the `_data` entry multiplicity table (F/M/U/R per fact),
  the 16 KB page observation, the freelist behavior, and the compaction door —
  so the next person asking "why is the file big" reads a doc instead of
  running `mdb_stat`.

## Non-goals

Dropping or restructuring the `M` (membership) or `R` (back-reference)
keyspaces — that is a storage-format redesign with commit-path consequences,
not a size fix, and it would demand a migration (humans own those; explicitly
out). Auto-compaction on a live `Db` (a policy decision with write-availability
trade-offs; the tool-driven door is enough). Page-size tuning (OS-owned).
In-place compaction (never — copy and swap only).

## Passing criteria

- Unit test in the engine: create a store, bulk-load a multi-chunk synthetic
  corpus (enough commits to grow a real freelist), `compact` to a fresh
  directory, and assert: (a) the compacted `data.mdb` is **≤ 0.8 ×** the
  source file size (the S corpus measures 0.61×; 0.8 leaves margin across page
  sizes), (b) the compacted store opens, (c) a full scan of every relation
  yields byte-identical fact streams to the source (fold both through
  `bumbledb::digest` and compare), (d) `Db::generation()` matches, and (e) a
  write to the compacted store commits and reads back (it is a first-class
  store, not a snapshot).
- `compact` to an existing directory fails with a typed error (never
  clobbers).
- Driver test: `ensure_corpus` at S produces a `<root>/db/` with no
  `db-load/` residue, the marker present, and `verify` green against it (the
  existing e2e-shaped driver test covers the latter — it must pass unchanged).
- The report markdown golden re-pinned with the new store line.
- `40-storage.md` amendment landed. `scripts/check.sh` green.
