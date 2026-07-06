# Concurrency and crash-safety audit

Auditor scope: the seams — every place two subsystems, two threads, or a crash boundary
meet. Method: exact interleavings traced through the code; the one finding that could be
demonstrated end-to-end was demonstrated with a repro program against the public API
(scratchpad project, no repo modification).

## Scope (files and docs read, with line counts)

Paper (in full): `docs/free-join-paper/arXiv-2301.10841v2/main.tex` (162) and its inputs
`tex/00-abstract` (16), `tex/01-intro` (244), `tex/02-background` (511), `tex/03-free-join`
(609), `tex/04-optimizations` (479), `tex/05-eval` (337), `tex/06-discussion` (86).

Architecture docs (in full, in order): `README.md` (72), `00-product.md` (187),
`10-data-model.md` (228), `20-query-ir.md` (178), `30-execution.md` (296),
`40-storage.md` (206), `50-validation.md` (180), `60-api.md` (121).

Code: `storage/env.rs` (492), `image/cache.rs` (450), `api/db.rs` (903),
`storage/commit.rs` (1266), `api/prepared.rs` (2139), `image.rs` (695),
`image/view.rs` (576), `exec/colt.rs` (1200, structure + select/reset paths),
`storage/delta.rs` (524), `storage/dict.rs` (307), `exec/dispatch.rs` (637),
`exec/run.rs` (1451, skimmed for shared/interior-mutable state), `obs.rs` (434, trace
impl), `alloc_counter.rs` (254), `lib.rs` (123), `tests/crash.rs` (116), `tests/api.rs`
(879, concurrency/ETL/pinned families), `tests/alloc_gate.rs` (422),
`bumbledb-bench/src/corpus.rs` (256), `bumbledb-bench/src/driver.rs` (759, corpus-cache
path). Dependency verification: `heed-0.22.1/src/txn.rs` (RoTxn Send/Sync impls),
`heed-0.22.1/src/envs/env.rs` (Env Send+Sync, single-open registry).

## Verdict

The core concurrency and crash design is sound and is implemented as documented: the
generation clock is written atomically with the data it describes (one LMDB write
transaction covers all commit phases including the tx-id bump), readers source their
generation from inside their own snapshot, and the cache's invariant — `(relation, gen)`
maps only to an image built from a snapshot whose tx id is `gen` — holds under every
interleaving I could construct, because builders key by their own snapshot's id and
eviction is purely a memory policy. Crash points anywhere in the write path lose nothing
(abort-by-drop before `mdb_txn_commit`, LMDB atomicity across it, only a benign cache
leak after it). The one verified correctness hole is not thread-vs-thread but
handle-vs-handle: the prepared query's view memo keys on the bare `u64` generation with
no environment identity, so executing a `PreparedQuery` against a different `Db` (or a
recreated store) at a coinciding generation silently returns the *other database's data*
— confirmed by repro. Beyond that: a memory-lifetime contract violation in the memo's
parked placeholder slots, a self-deadlock expressible through nested `db.write`, and a
handful of notes.

## Findings

### [CRITICAL] Cross-environment PreparedQuery execution aliases the generation clock — wrong results, verified

- Files: `crates/bumbledb/src/api/prepared.rs:868-899` (`ViewMemo::bind` — key is
  `(Option<u64> generation, filters)` only), `crates/bumbledb/src/api/prepared.rs:921`
  and `:941-949` (`run_join` — a bind hit skips the image cache entirely),
  `crates/bumbledb/src/api/db.rs:363-385` (`Snapshot::execute` accepts *any*
  `&mut PreparedQuery`).
- Invariant at stake: derived state (view, COLT, memoized image `Arc`) built from
  generation G of environment E must only be consumed by snapshots of E at G. The memo
  key carries G but not E.
- Interleaving (no threads needed; verified end-to-end through the public API — repro in
  the audit scratchpad, output `BUG CONFIRMED`):
  1. `Db::create(a)`, `Db::create(b)` with the same schema; commit one distinct fact to
     each → both stores at generation 1 with different contents.
  2. `let mut q = db_a.prepare(&query)` (any Free-Join-path query).
  3. `db_a.read(|s| s.execute_collect(&mut q, &[]))` → memo miss → builds views from
     A's snapshot, records generation 1 → returns A's row (correct).
  4. `db_b.read(|s| s.execute_collect(&mut q, &[]))` → `run_join` reads generation 1
     from B's snapshot → `memo.bind(occ, 1, filters)` **hits** the binding built from A
     → `continue` past `cache.get_or_build` → the executor joins over **A's image** →
     the result is A's data, silently, while the caller queried B.
- The same aliasing fires for a `PreparedQuery` that outlives its `Db` (its lifetime
  parameter borrows only the schema) when the host wipes and recreates a store
  (dev-reset): the fresh store's generations restart at 0 and climb back through the
  memoized numbers with different data. Single-handle usage is unaffected — within one
  environment the generation is monotonic and atomic with the data.
- Note the near-miss safety elsewhere: on a memo *miss* the image comes from
  `cache.get_or_build(txn, …)` with the executing snapshot's txn and the executing Db's
  cache, which is correct — only the hit path lacks environment identity. Guard-probe
  plans read purely through the snapshot and are immune.
- Fix direction: brand each `Db`/`ImageCache` with an environment epoch (e.g. a
  process-unique `u64` minted at `Db::open`/`create`, or the `Arc` address of the
  cache) and store it in the `PreparedQuery` at prepare time; `execute` checks it
  against the snapshot's Db and either re-keys the memo on `(epoch, generation,
  filters)` or returns a typed error on mismatch. Alternatively tie `PreparedQuery`'s
  lifetime/identity to the `Db` it came from (a `&'db`-shaped brand), making the misuse
  unrepresentable — the stronger, representation-first fix.

### [MEDIUM] Parked placeholder COLTs pin prepare-generation images for the prepared query's lifetime

- Files: `crates/bumbledb/src/api/prepared.rs:456-469` (`build_view_memo` fills all
  `PARKED_SLOTS = 3` parked slots per occurrence with
  `Colt::new(View::All(Arc::clone(&image)), …)` at prepare time),
  `crates/bumbledb/src/api/prepared.rs:884-897` (`bind` — a stale active binding
  rebuilds in place and **never parks**, so for occurrences with zero residual filters
  the parked slots are never touched again), `docs/architecture/40-storage.md`
  ("Readers still pinned at older generations keep their `Arc`s alive **until their
  transactions end**"), `docs/architecture/30-execution.md` ("Memory bound: four COLT
  high-waters per occurrence per prepared query").
- Invariant at stake: the documented image-lifetime story — old-generation images die
  when the last pinned *reader transaction* ends. The memo extends image lifetime far
  past any transaction: (a) for zero-residual occurrences (the common ledger shape —
  selections and filter-free atoms), the three parked placeholders hold the
  prepare-time image `Arc` **forever**, across every subsequent commit and eviction;
  (b) for residual-bearing occurrences, parked bindings at dead generations survive
  until a future park happens to victimize them (stale-first selection exists at
  `prepared.rs:890`, but victimization only runs when the workload parks again).
- Crash point / interleaving: none — no wrong results (generation checks make stale
  parked bindings unhittable). The symptom is memory: N long-lived prepared queries ×
  occurrences × one full-relation image pinned at their prepare generations, plus up to
  three old-generation images per residual-rotating occurrence. At the 1 GB scale axiom
  a handful of prepared queries over large relations can hold hundreds of MB of dead
  slabs against the 2 GB working-set budget, invisible to `cache_resident()`.
- Fix direction: make parked slots `Option<ParkedView>` (empty at prepare — placeholder
  colts are pure ballast anyway), and drop or empty any parked binding whose generation
  is below the current one during `bind` (their unhittability is already proven there);
  the active slot self-heals on the next execution and needs nothing.

### [LOW] Nested `db.write` inside a write closure self-deadlocks on the writer mutex

- File: `crates/bumbledb/src/api/db.rs:160-187` (`Db::write` takes `self.writer.lock()`;
  the closure receives `&mut WriteTx` but the host still has `&Db` in scope).
- Invariant at stake: "the engine owns zero threads / single writer" assumes writers
  arrive one per thread; a re-entrant `db.write(|_| db.write(|_| …))` is expressible in
  safe code and `std::sync::Mutex` re-lock on the owning thread deadlocks (or panics,
  per its documentation — either way the writer thread is lost, and every other writer
  then queues behind it forever).
- For completeness on the sibling seams the task names: `db.read` inside a write
  closure **works** and sees pre-transaction committed state (a separate LMDB read
  snapshot; LMDB readers never block the writer) — the 60-api phrase "queries inside a
  write transaction are forbidden by representation" is true only of the `WriteTx`
  surface itself; and `db.write` inside `db.read` is safe (exercised by
  `tests/api.rs:600-620`).
- Fix direction: either document the re-entrancy hazard next to `Db::write`, or hold the
  writing thread's id in an atomic beside the mutex and panic with a typed message on
  re-entry (cheap, no hot-path cost — it's the write path).

### [NOTE] A panic between LMDB commit and cache eviction leaks (never corrupts) the cache

- File: `crates/bumbledb/src/api/db.rs:176-186` — `commit(delta, &self.env)?` makes the
  new generation durable; `self.cache.evict_older_than(...)` runs afterwards. A panic in
  between (only the trace spans and struct moves sit there) skips eviction; the writer
  mutex's poison is deliberately cleared on the next `write`
  (`db.rs:164`, `PoisonError::into_inner`).
- Traced outcome: stale-generation entries stay resident and `CacheInner.newest` stays
  low until the next state-changing commit evicts them. Because every cache key carries
  its generation and readers key by their snapshot-sourced id, no interleaving turns the
  leak into wrong data (a new reader at G+1 misses the stale (rel, G) entries by key).
  The poison-clearing itself is sound: every pre-commit panic path drops the delta and
  the LMDB `RwTxn` (abort) — verified through `commit.rs` (all five phases inside one
  `WriteTxn`; every `?`/panic drops it) and `WriteTx`'s unwind path.
- Asymmetry worth recording: the *cache* mutex does not get the same forgiveness — a
  panic while holding it (the critical sections are a HashMap probe/insert/retain and,
  under `trace`, an `obs::event` into a thread-local; effectively panic-free) would turn
  every subsequent query into `expect("cache mutex")` panics db-wide. Acceptable today;
  worth a comment if the critical section ever grows.

### [NOTE] Multi-process access is unguarded and corrupts engine invariants (documented as out of envelope)

- Files: `crates/bumbledb/src/storage/delta.rs:91-114` (provisional intern ids minted
  from the *snapshot's* committed counter under the assumption "single-writer discipline
  makes provisional = final"), `crates/bumbledb/src/api/db.rs:96` (the writer mutex is
  process-local), `docs/architecture/00-product.md` ("neither supported nor guarded").
- Interleaving (second process P2 on the same path — LMDB itself permits it): P1 opens
  its write view (dict next-id = N), P2 commits an intern at id N, P1 commits its own
  provisional id N → `_dict` reverse entry N is overwritten and P1's facts point at
  P2's bytes — silent dictionary corruption; serial marks regress the same way. This is
  the recorded owner decision, not a new bug; recorded here because the failure mode is
  *corruption*, not merely stale reads, and there is no lock-file tripwire making the
  misuse loud. A one-line advisory flock at open would convert it into an error.

### [NOTE] `alloc_gate`'s exact-zero window depends on the binary containing exactly one test

- File: `crates/bumbledb/tests/alloc_gate.rs:326-328` ("One test function: the gate
  binary is single-threaded by construction") vs `crates/bumbledb/src/alloc_counter.rs`
  (process-global atomics; the module honestly declares itself thread-naive, and its
  *unit* tests correctly serialize and use slack-tolerant assertions).
- The claim holds today. The fragility: adding a second `#[test]` to `alloc_gate.rs`
  makes cargo's default multi-threaded harness interleave allocations into the measured
  window and the exact-zero asserts become racy — silently, since nothing enforces the
  one-test invariant. The atomics themselves are sound under threads (the peak CAS loop
  at `alloc_counter.rs:47-56` is correct; `realloc`'s transient live-bytes overshoot by
  the old size is a documented-order artifact, harmless). Fix direction: a comment is
  already half the guard; `--test-threads=1` in the invoking script or a
  `#[cfg(test)] const _: () = ...` count assertion would finish it.

### [NOTE] No test executes prepared queries concurrently with a committing writer

- Files: `crates/bumbledb/tests/api.rs:286-350` (the concurrent reader/writer family
  reads via `snap.scan`, never via a `PreparedQuery`), `crates/bumbledb/src/api/prepared.rs`
  tests (generation-bump memo invalidation is exercised single-threaded only),
  `docs/architecture/50-validation.md` ("rapid write/read interleaving (a reader never
  sees a mismatched generation …)").
- The view-memo/eviction/build-race machinery — the newest and most intricate seam — is
  covered by single-threaded generation bumps plus the cache's own two-thread build
  race test (`image/cache.rs:373-400`), but no test drives `execute` on reader threads
  while a writer commits (the exact traffic pattern the doctrine advertises). My manual
  trace says it is sound (findings above are the only cracks), but this family is the
  cheap regression net for it.

## Checked and sound

- **Generation clock atomicity and sourcing.** The tx id is bumped inside the same
  `RwTxn` as phases 1–4 (`commit.rs:284-292`) and read by readers from `_meta` inside
  their own snapshot (`env.rs:253-259`); the per-txn `OnceCell` memoization is sound
  because the value is snapshot-constant. A no-op delta neither writes, advances the id,
  nor evicts (`commit.rs:218-237`, cache test `a_no_op_commit_does_not_invalidate_the_cache`).
- **The cache invariant under all rider/writer interleavings.** `(rel, G)` can only ever
  hold an image built from a snapshot whose tx id is G, because builders key by their own
  snapshot's id — eviction is pure memory policy. Traced safe: (a) reader-at-G inserts
  after writer commits G+1 but before eviction → transient entry, evicted next; (b)
  eviction first, insert second → the under-lock `generation < inner.newest` re-check
  (`cache.rs:177-184`) forces query-local, no resurrection; (c) two same-G builders →
  insert-if-absent adoption under the same lock (`cache.rs:185-199`, race test proves
  single shared `Arc`); (d) old-generation pinned reader → query-local build, map
  untouched (test `old_generation_miss_builds_without_populating_the_map`).
- **Crash boundaries of the commit.** All six documented phases (deletes, inserts,
  forward FK, Restrict, counter flush incl. dict + tx id, `mdb_txn_commit`) execute
  inside one `WriteTxn` opened at `apply` (`commit.rs:73-131, 214-299`); every error
  path returns before `txn.commit()` and the drop aborts. Kill-during-commit is
  exercised for real (`tests/crash.rs`), including M/Q/S consistency after reopen.
- **Write-path panic anatomy.** Panic in the closure, in `apply`, or anywhere before
  `mdb_txn_commit`: the delta (arena, provisional interns, serial marks) and the LMDB
  txn die in the unwind — nothing durable, nothing observable (`delta.rs` drop test,
  `dict.rs` abort test). The writer-mutex poison clear at `db.rs:164` is therefore
  justified; the only post-commit exposure is the cache-eviction leak (NOTE above).
- **Provisional interns and serial marks.** The write view is opened *under* the writer
  mutex after the previous commit fully completed (`db.rs:164-171`), so
  provisional-id = final-id holds in-process; aborted transactions re-issue both serial
  values and dict ids (tests `serials_allocated_in_an_aborted_txn_are_reissued`,
  `aborted_transaction_leaves_no_dictionary_entries`). `WriteDelta`/`commit` are
  `pub(crate)` — no public path bypasses the mutex.
- **Reader-while-writer.** MDB_NOTLS ties reader slots to txn objects (`env.rs:49-59`),
  so a thread pins an old snapshot while opening new ones; LMDB's reader table prevents
  free-page reuse under live readers (growth, never corruption). Pinned-at-T semantics
  across later commits are tested through scans and prepared queries
  (`tests/api.rs:587-630`), and an old-generation prepared execution correctly
  query-locals its images.
- **Send/Sync by type.** No `unsafe impl Send/Sync` anywhere in the crate. `Db` is auto
  `Send + Sync` (heed `Env` is; `Mutex`, `ImageCache` are) — statically asserted in
  `tests/api.rs:287`. `ReadTxn`/`Snapshot` are `Send` but **not** `Sync` (heed impls
  `Send` for `RoTxn<WithoutTls>` only, `txn.rs:237`), so the one-thread-per-LMDB-txn
  rule is type-enforced. `PreparedQuery` is `!Sync` (PhantomData, compile-fail doctest)
  and `Send`-safe: its state is owned pools, `Arc<RelationImage>` (immutable, no
  interior mutability), and no thread-affine handles. `ImageCache`'s trace counters are
  monotonic Relaxed statistics with no decision-making reader.
- **Images copy out of the mmap.** `image::build` decodes every column into owned slab
  `Vec`s inside the build txn (`image.rs:189-295`); no LMDB-borrowed byte survives the
  snapshot. Row count is cross-checked against `S` in both directions (over- and
  under-run are `Corruption`).
- **View-memo state-machine consistency.** `(colt, generation, filters)` swap as a unit
  in `bind` (active↔parked), so no interleaving of hits, parks, rebuild-in-place, and
  *mid-rebuild errors* (get_or_build failing after a park) leaves a colt paired with the
  wrong key; selections are deliberately outside the key and re-probed per execution,
  which is sound because dictionary content and view survivors are functions of the
  generation. Filter resolution (params, PendingIntern, Ne-miss sentinel) is
  deterministic per generation, so memo keys cannot alias *within* one environment.
- **Guard probes.** Read only `U`/`M`/`F` through the caller's snapshot
  (`dispatch.rs:224-275`); no image, cache, or memo state — immune to every seam above,
  including immediately-after-commit cold reads.
- **`Db::compact` concurrent with a writer.** `mdb_env_copy2(MDB_CP_COMPACT)` copies
  through an internal read-only transaction — a consistent snapshot; concurrent write
  txns cause file growth at worst. The destination file and its directory are both
  fsynced before return (`db.rs:216-234`).
- **Bulk-load crash semantics.** Chunked `Db::write` per 4096 facts; a process crash
  between chunks leaves prior chunks durable — exactly the documented partial-load
  contract; in-process failure carries the committed count (`BulkLoadError`).
- **Corpus-cache marker-last discipline (bench).** `ensure_corpus_with`
  (`driver.rs:57-72`) wipes any unmarked directory, loads (LMDB fsync-per-commit;
  compact syncs file + dir; SQLite `synchronous=FULL` + `fullfsync=ON` + truncating
  checkpoint), and writes `corpus.ok` strictly last — a crash anywhere loses only the
  marker and forces regeneration; a surviving marker implies durable content.
- **Lock ordering.** Exactly one nesting exists: bumbledb writer mutex → LMDB writer
  lock (inside `commit`). No path takes the LMDB write lock while holding anything else
  (compact uses a read txn; `Environment::open`'s registration txn predates the `Db`),
  so no inversion deadlock is constructible — only the re-entrancy self-deadlock in the
  LOW finding.
