# Storage correctness audit

Audit date: 2026-07-06. Auditor: storage-subsystem correctness pass, docs-first per the
mandate (paper → architecture docs → code, with concrete value traces for every
suspicion). All 254 unit tests plus the integration suites (incl. `tests/crash.rs`)
pass on this tree.

## Scope (files and docs read, with line counts)

Paper (`docs/free-join-paper/arXiv-2301.10841v2/`):

| File | Lines |
|---|---|
| `main.tex` | 163 |
| `tex/00-abstract.tex` | 15 |
| `tex/01-intro.tex` | 243 |
| `tex/02-background.tex` | 510 |
| `tex/03-free-join.tex` | 608 |
| `tex/04-optimizations.tex` | 478 |
| `tex/05-eval.tex` | 337 |
| `tex/06-discussion.tex` | 85 |

(`tex/025-tale.tex`, `07`, `08` are not `\input` by `main.tex`; 07/08 are empty.)

Architecture docs, in order: `README.md` (71), `00-product.md` (186), `10-data-model.md`
(227), `20-query-ir.md` (178), `30-execution.md` (295), `40-storage.md` (205),
`50-validation.md` (179), `60-api.md` (120).

Audited code (`crates/bumbledb/src/`):

| File | Lines | Role |
|---|---|---|
| `storage/env.rs` | 492 | environment lifecycle, `_meta`, txns, generation, `copy_compacted` |
| `storage/keys.rs` | 278 | `_data` keyspace codec |
| `storage/delta.rs` | 524 | write delta, interning staging, serial marks |
| `storage/commit.rs` | 1266 | six-phase commit |
| `storage/dict.rs` | 307 | interning dictionary |
| `storage/read.rs` | 418 | probes, fetch, scan, row count |
| `error.rs` | 715 | taxonomy (storage-relevant variants) |

Read as supporting evidence (invariant consumers / enforcement sites): `api/db.rs`
(writer mutex, `WriteTx`, `bulk_load`, `compact`), `lib.rs` (module visibility),
`encoding.rs` (order-preserving codecs, `field_bytes`, `fact_hash`, `FactLayout`),
`schema.rs` (constraint validation, `GuardKeyTooWide`, FK positional typing,
auto-uniques, `fk_targeted`), `schema/fingerprint.rs` (303), `image.rs` (the S-counter
cross-check at lines 232/268), `tests/crash.rs` (116).

## Verdict

The storage layer is sound. The two properties everything hangs on — (a) one LMDB write
transaction per commit containing every mutation (F/M/U/R, Q, S, `_dict`, dict next-id,
tx id) with fsync at the single commit point, and (b) the writer mutex in `Db::write`
held from snapshot-open through commit so the delta's read view provably equals the
commit-time base — both hold, and the second is what makes the delta's provisional
intern ids, serial marks, row-count deltas, and changed-reports exact rather than
racy. I traced every constraint-enforcement corner I could construct (delete+reinsert
of a unique key in both user orders, self-referencing FKs, cyclic same-delta inserts,
delete-target-with-no-op-reinsert-of-referrer, net-no-op dispositions against base)
and found no path by which a unique violation or dangling FK commits, and no path by
which counters drift from facts. No CRITICAL or HIGH findings. What remains are edge
fragilities in corruption *reporting* (panics where the contract promises typed
errors, on hand-corrupted stores only), one doc-code nuance in `create`'s refusal
check, and documented-but-sharp semantics worth recording (no-op-commit serial reuse,
delete-path interning).

## Findings

### [LOW] Corrupt short keys panic instead of returning typed Corruption errors

- `crates/bumbledb/src/storage/commit.rs:162` — `let tail = &surviving_key[surviving_key.len() - 12..];`
- `crates/bumbledb/src/storage/read.rs:126-129` — `raw_key[raw_key.len() - 8..]`

Documented invariant: "Corrupt data is a hard error, never a skip" (`40-storage.md`) —
the taxonomy in `error.rs` implements that as typed `Corruption(...)` values;
`commit.rs`'s own doc comment scopes its panics to "well-formed R keys this same commit
wrote."

Concrete failure scenario: the Restrict prefix scan surfaces *pre-existing* keys, not
only keys this commit wrote. Take a schema with an FK targeting a unique constraint
over a Bool or Enum field (guard width 1, legal): the restrict prefix
`R|rel(4)|cid(2)|guard(1)` is 8 bytes. Plant a corrupt 8-byte key equal to the bare
prefix (raw LMDB write, torn tooling, bit rot) and then delete the guard-holding fact:
`check_restrict` matches the key, computes `8 - 12`, and the slice index underflows —
a process panic mid-commit rather than a `Corruption` error. (With guard width ≥ 5 the
same corrupt shape misparses guard bytes as `source_rel`/`source_row` and typically
surfaces as `Corruption(MissingFact)` with garbage ids — survivable but misleading.)
Similarly, `read::scan`'s row-id slice panics on a corrupt F-namespace key of length
5–7. Same class: `delta.rs:107` / `dict.rs:84` `assert!` a `_meta` dict counter of
`u64::MAX` — a corrupt counter value panics rather than erroring.

Fix direction: length-check the scanned key against the expected fixed shape
(`prefix_len + 12` for R, `13` for F) and return
`Corruption(CorruptionError::MalformedValue(...))` on mismatch; turn the counter
asserts into the same typed error. The panic-vs-error distinction only matters on
already-corrupt stores, hence LOW.

### [LOW] `create` refuses only bumbledb environments, not "an environment"

- `crates/bumbledb/src/storage/env.rs:69-105` (`Environment::create`), surfaced at
  `api/db.rs:106`.

Documented invariant: `60-api.md` — create "**refuses a directory that already holds an
environment** (`AlreadyInitialized`) … create is exactly as non-destructive as open."

Concrete scenario: the refusal check is "does `_meta` exist as a named database." A
directory holding a *foreign* LMDB environment (another application's `data.mdb`, no
`_meta` DBI) passes the check: `create` opens that environment and creates
`_meta`/`_data`/`_dict` inside it, entangling two applications' data in one file. No
bytes of the foreign data are destroyed, and the half-created-bumbledb-env case (crash
between `create_dir_all` and the meta commit — LMDB atomicity leaves an env with no
`_meta`) *should* proceed, so the current check is right for recovery; but the doc's
wording promises refusal of any environment. Fix direction: either amend the doc
sentence to "already holds a *bumbledb* environment," or additionally refuse when the
unnamed root DB is non-empty / other named DBIs exist.

### [NOTE] `delete_fact` does not verify outgoing R entries existed

- `crates/bumbledb/src/storage/commit.rs:423` — `self.data.delete(...)` on the R key,
  return value ignored.

Documented invariant: "Deletes and inserts both check what they touch: a live `M` entry
whose `F` row or `U` guard is missing is the membership-desync corruption, a hard
error" (`40-storage.md`). The doc names F and U only, so this is not a contradiction —
but the asymmetry means a store corrupted by *losing* an R entry passes deletes
silently, and the corresponding Restrict protection for that referrer had already
silently lapsed. An F- or U-miss aborts with `MembershipDesync`; an R-miss is
undetectable on every path (the deferred offline M↔F↔U↔R sweeper is the stated
owner of this class). Fix direction: none required now; if the offline checker stays
unbuilt long-term, promoting the R delete to the same `MembershipDesync` check is one
line per call site.

### [NOTE] Serial values returned from a logically-no-op *successful* commit are re-issued

- `crates/bumbledb/src/storage/commit.rs:218-237` (empty-delta and
  `!applied.changed` paths drop the delta's `serial_next`, pending interns, and dict
  next-id), `delta.rs:217-219`.

Documented invariant: the generator is "never re-issuing any value observable in a
committed state; aborted transactions don't advance the committed sequence"
(`10-data-model.md`); `40-storage.md`/code: "pending allocations and interns of an
empty delta are deliberately dropped — none of them are observable."

Concrete scenario: `db.write(|tx| { let id = tx.alloc::<ItemId>()?; Ok(id) })` returns
`Ok(ItemId(5))` — a *successful* call, not an abort — and the next transaction's
`alloc` returns 5 again (likewise a `changed:false` commit whose ops all no-op against
base). The letter of the contract holds (5 was never observable in any committed
state), but the transaction reported success and handed the value to the host, so a
host persisting it out-of-band (log line, external system) sees a duplicate. Verified
behavior, matches the docs as worded; recorded so nobody later reads it as a bug or,
worse, "fixes" it by flushing counters on no-op commits (which would break the
tx-id-advances-iff-changed rule). No action.

### [NOTE] A no-op delete of a never-interned string permanently interns it

- `crates/bumbledb/src/api/db.rs:549-557` (delete encodes through the *write*
  context) → `delta.rs:91-114`.

Documented invariant: read paths promise "a dictionary miss means the literal cannot
match any fact: empty result, never an insert" (`10-data-model.md`); the write path
makes no such promise, and the code comment justifies write-context interning
(insert-then-delete of one fact must cancel byte-exactly, so the delete must see the
same provisional ids).

Concrete scenario: `tx.delete(&Holder { name: "ghost" })` where "ghost" was never
interned mints a provisional id; the delete itself no-ops (the fact bytes cannot be in
base — they embed a fresh id), but if the delta is otherwise non-empty the pending
intern flushes at commit and `_dict` grows by an entry no fact references. Consistent
with the accepted no-GC leak; semantically harmless (future lookups of "ghost" find an
id matching no facts). Recorded as designed behavior with a real trace. No action.

### [NOTE] Generation and row-id increments have no overflow guard (asymmetry with serial/dict counters)

- `crates/bumbledb/src/storage/commit.rs:285` — `txn.generation()? + 1`;
  `commit.rs:531-532` — `*next += 1` on the row-id high-water.

The serial generator (`delta.rs:190-195`) and dictionary minter (`delta.rs:107`) guard
their `u64` ceilings; the storage tx id and row-id high-water do not — after 2⁶⁴
state-changing commits or 2⁶⁴ inserts into one relation they would wrap (debug builds
panic, release wraps, and a wrapped row id would collide with live F keys). At the
design point (≤10⁷ facts, bursty commits) this is unreachable by ~12 orders of
magnitude; recorded only because the codebase guards the equally-unreachable serial
ceiling, so the asymmetry looks like an oversight rather than a choice. No action
needed.

## Checked and sound

Keyspace codec (`keys.rs`):

- Namespace tags are distinct first bytes (`F`=0x46, `M`=0x4D, `Q`=0x51, `R`=0x52,
  `S`=0x53, `U`=0x55): no key of one namespace can equal or prefix-match another
  namespace's scan. The `keys_sort_by_namespace_then_components` test pins byte order =
  (namespace, big-endian components) order.
- Prefix-scan boundaries: every guard is fixed-width per constraint (all six types
  encode 1 or 8 bytes; width is a function of the constraint's field list), so a
  Restrict prefix scan `R|rel|cid|guard` matches exactly the referrer set — it cannot
  bleed into a different guard, a different constraint (cid differs at a fixed
  offset), a different relation, or the `S` namespace. `F|rel` prefix scans likewise
  cannot cross into `M`.
- `MAX_KEY = 511` equals LMDB's compiled default key ceiling; the widest possible key
  (Restrict with a maximal guard) is exactly 511 bytes (test-pinned).
  `MAX_GUARD_WIDTH = 511 − 19` is enforced at schema construction
  (`SchemaError::GuardKeyTooWide`, `schema.rs:460-470`), and FK guards are
  width-identical to their target unique guards by positional structural-type
  equality (`schema.rs:299-313`), so the U-side bound covers every R key. Key
  encoding never derives `Ord`; LMDB byte order is the only order, as documented.
- 40-storage's value-endianness rule holds: key components BE; M/U/Q/S/`_meta` values
  LE; dictionary ids BE in both forward values and reverse keys.

Crash safety:

- Everything a commit changes — F/M/U/R entries, Q serial marks, S row counts and
  row-id high-waters, `_dict` forward+reverse entries, the dict next-id, and the
  storage tx id — is written inside the single LMDB write transaction opened at
  `apply()` and committed once (fsync, LMDB defaults; `NOSYNC`/`WRITEMAP`/`MAPASYNC`
  unexpressible through `Environment`). Delta accumulation touches no LMDB data page;
  error/panic anywhere = drop = abort = nothing persisted (verified for every error
  return path in `apply`/`commit`; the `Applier`/`Applied` own the `RwTxn`, so `?`
  propagation drops it).
- Dictionary consistency across aborts: pending interns and the next-id flush in
  phase 4 of the same transaction; a commit that aborts after interning staged leaves
  `_dict` and the counter untouched (`dict-abort` and `commit8-pending-interns` tests;
  the provisional id is then legitimately re-minted).
- `Environment::create` is itself atomic (DBIs + all four `_meta` keys in one txn):
  a crash mid-create leaves an env that `open` rejects (`MetaMissing`) and a re-run of
  `create` completes.
- `Db::compact` copies via `mdb_env_copy2(MDB_CP_COMPACT)` (internally
  snapshot-consistent), refuses an existing destination, and fsyncs both the file and
  its directory entry before returning.
- The kill-during-commit family (`tests/crash.rs`) exercises this for real: SIGKILL
  mid-commit-loop at three delays, then reopen asserts an enumerable consistent state,
  M idempotence, serial continuation past every committed id, and (via image build)
  the S-vs-F-scan cross-check (`image.rs:232/268` constructs `RowCountMismatch`).

Single-writer discipline and delta exactness:

- `Db::write` (`api/db.rs:160-187`) holds the writer mutex from *before* the delta's
  read snapshot is opened until after commit and cache eviction; `storage` is
  `pub(crate)` (`lib.rs:64`) so no public path reaches `WriteDelta`/`commit` around
  the mutex; heed refuses a second in-process open of the same path, and multi-process
  is out of envelope by decision. Therefore the delta's snapshot equals the
  commit-time base — the premise under which provisional intern ids are final, serial
  marks are exact, and membership probes are truthful. Verified this premise is used
  nowhere it doesn't hold.
- Delta/apply accounting agreement, proven by case analysis and tests: for any
  insert/delete sequence on one fact, changed-ops strictly alternate, so the
  accumulated `row_count_delta` equals apply's actual effect in all four
  (base-present × final-disposition) cases; net-Insert of a base-present fact and
  net-Delete of a base-absent fact no-op at apply with zero counter drift and
  `changed:false` (the `delete_and_reinsert...` test); a `changed:false` commit has
  written nothing (both apply steps return before any put), so aborting its txn is
  exactly equivalent to committing it.
- The generation advances exactly once per state-changing commit, atomically with the
  data, and never on empty or all-no-op deltas (tested); readers obtain it via one
  `_meta` get inside their own snapshot (`ReadTxn::generation`, OnceCell-cached,
  `!Sync`) — never a process counter, closing the documented cache-poisoning race.

Constraint enforcement (no violation can commit):

- Canonical order (all deletes, then all inserts, over the deduplicated net delta)
  makes user operation order irrelevant: delete+insert of one unique key succeeds in
  either user order (tested), and a `U` conflict in phase 2 is a genuine final-state
  violation → typed abort with base intact (tested).
- Guard re-derivation by `field_bytes` slicing matches independently computed
  encodings (test), for both the serial auto-unique and FK guards.
- Forward FK: every *landed* insert contributes a probe; probes run against the final
  state (write txn reads its own writes), so same-delta targets, insertion order,
  cyclic pairs, and self-referencing facts all resolve correctly (tested for
  referrer-before-target). An insert that no-ops at apply (base already has the fact)
  safely skips its probe: its targets either survive (R entries + restrict protect
  them) or were deleted this txn, in which case its *surviving R entry* trips the
  Restrict scan — traced concretely (base `Target(5)`+`Source(1,5)`; delta =
  delete `Target(5)`, delete+reinsert `Source(1,5)` → net Insert no-op → phase 3b
  finds Source's R entry → `RemainingReference` abort). No hole.
- Restrict: scans exactly the FK-targeted guards deleted-and-not-reestablished;
  referrers deleted in the same txn removed their R entries in phase 1 (so
  delete-target-and-all-referrers commits, tested); a guard re-established by a
  different fact is subtracted (tested); `fk_targeted` is computed over the whole
  schema including self-references (`schema.rs` pass 2, sorted/deduped).
- Serial auto-uniques are ordinary constraints (materialized first, fingerprinted,
  guard-maintained), so duplicate serial ids are structurally impossible in any
  committed state.

Interning (`dict.rs`, `delta.rs`):

- Tag byte is *inside* the blake3 input (`forward_key`), so String/Bytes with equal
  bytes get distinct ids (tested); the collision axiom is implemented exactly as
  specified (forward hit trusted without byte verification; full 32-byte hash; same
  axiom for `M` keys — not truncated).
- Pending-intern staging: probe order is pending map → committed dict → mint from the
  snapshot's counter; ids are monotonic, never reused, append-only across commits and
  correctly re-issued after aborts (tested); one map per tag, keyed by raw bytes.
- The sentinel (`u64::MAX`) is asserted never-minted on both mint paths; `resolve`
  distinguishes dangling ids from tag mismatches from non-UTF-8 as three typed
  corruption variants.

Serial and row-id semantics:

- `Q` is read once per (relation, field) per transaction and advanced in memory;
  explicit values — below, above, or at `u64::MAX` (`saturating_add`) — advance the
  mark correctly; mixed explicit/generated tracks the running max (all tested);
  `alloc` refuses at `u64::MAX` with typed `SerialExhausted`, so the generator can
  never re-issue any committed value (every commit flushes marks for every touched
  field, and committed facts are always below the flushed mark).
- Row ids come from the S high-water, are assigned only when an insert actually lands,
  monotonically increase, and are never reused after deletes (holes are absent keys;
  scan skips them in row-id order — tested). Images use dense scan ordinals; nothing
  downstream assumes row-id density (the fixture with a deleted row exercises the
  hole).

Read path (`read.rs`):

- M/U probes and F fetch are same-snapshot consistent; a row id from M/U with no F row
  is typed `MissingFact`, wrong-width facts are typed `WrongFactWidth`, and the scan
  iterator structurally fuses after the first error ("never a skip" is not a caller
  obligation) — all tested, including a deliberately truncated F value.
- `row_count` (S stat 0) equals the F-scan count after mixed commits (tested), and the
  image build cross-checks it on every build.

Environment lifecycle (`env.rs`):

- Open-time check order is format version *before* fingerprint (tested with both
  corrupted); the fingerprint's canonical serialization is length-prefixed
  (aliasing-proof, tested), covers exactly the `10-data-model.md` input list (enum
  variant lists, generation flags, FK targets by name), and is golden-pinned.
- `create` refuses re-initialization over an existing bumbledb environment
  (`AlreadyInitialized`, tested — see the LOW nuance above for foreign environments);
  `MDB_NOTLS` (`read_txn_without_tls`) matches the designed long-lived-reader pattern;
  the map size (4 GB) comfortably exceeds the 1 GB scale axiom and map-full surfaces
  as a typed `Lmdb` error inside the atomic txn.

Error taxonomy (`error.rs`):

- Every storage failure mode named by the docs has a distinct typed variant with
  id-carrying payloads (no hot-path formatting); constraint errors carry
  relation/constraint ids plus offending fact bytes, and the Restrict arm names the
  surviving referrer by fact bytes, never by storage row id — exactly as
  `10-data-model.md` requires.
