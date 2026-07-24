# 50 — Storage

LMDB is the only durable backend (decision recorded in `00-product.md`), accessed
through `heed`. Durability (fsync per commit on durable stores; the durable
constructors cannot express `NOSYNC`/`WRITEMAP`/`MAPASYNC` — the ephemeral store
KIND below is a different store, never a mode), write
atomicity, and reader snapshot isolation come from real LMDB transactions. Single
writer, many reader threads, one process (`00-product.md`).
**Decision: `heed`.** **Alternative:** `lmdb-rkv`/raw FFI. **Why it lost:** heed is the
maintained thin binding; raw FFI buys nothing at this layer. **Reverses if:** heed
becomes a correctness or maintenance liability.

**The lock law is a writer law** (ruled 2026-07-23, R17). One handle per path
governs the write surface: the exclusive advisory lock
(`storage/env/acquire_lock.rs`) belongs to the writing constructors — `Db`
handles, durable or ephemeral — and to nothing else. A read-only open takes no
lock and opens the environment `MDB_RDONLY` (its own arm of the one raw-open
chokepoint, `storage/env/open_env.rs`), registering dbis through a read
transaction: a read-only environment can corrupt nothing, so there is nothing
for a lock to protect. `exhume` (`storage/env/exhume.rs`) is thereby genuinely
read-only down to the storage layer — the archival lane works on read-only
media, restored snapshots, and mounted backups, with no carve-outs — and its
read-only-ness is representation, not API-surface omission: from a read-only
environment the write path is unrepresentable.

Environment constants are decisions, not knobs: `map_size` is fixed at 32 GiB
(comfortably above the 1 GB scale axiom — the map is the hard capacity ceiling,
headroom, never the design point), and
`max_readers` at 1024 — inter-query parallelism is the scaling axis and `MDB_NOTLS`
binds reader slots to open transaction objects, so LMDB's default 126 would cap
concurrent snapshots, not threads; the raise costs a measured 64 bytes of lock file
per slot (~64 KiB total), and the snapshot past the table is the typed `ReadersFull`
error naming the limit. The map is an **address-space reservation, never an
allocation**: no open path truncates or preallocates `data.mdb` to the map (LMDB's
full-map ftruncate lives only under `MDB_WRITEMAP` — `mdb_env_map`, mdb.c — and no
store kind carries that flag), so a store's data file holds exactly the pages ever
committed, on every filesystem. (Retraction, cleanup-0.5.0 ruling 1: the size was
4 GiB, priced when the ephemeral kind eagerly materialized its full map at every
open; dropping `WRITEMAP` and the eager capacity contract — § the ephemeral store
kind — made the raise free. The retracted text also claimed every open ftruncates
the map: true only of `WRITEMAP` opens, i.e. never of durable stores; verdict read
off mdb.c and pinned by the refusal tests' `< 1 GiB` fixture bounds.)

## Design inputs (why this layout)

The first governing observation: an encoded fact is already a fixed-width row
sliceable into columns for free — **one sequential scan of a row-major table yields
every column.** A layout that instead stores field values severally across many
namespaces turns megabytes of column data into hundreds of thousands of LMDB
point-gets; this layout exists to make that shape unwritable.

The second: **derived index namespaces are
derived accelerators for the dependency judgments (`30-dependencies.md`), not the
judgments' definitions.** `U` exists so the functionality check is O(log n) per
touched fact; `R` exists so the containment check's target side is O(log n) per
touched key. A namespace is the *plan* an accepted statement promised at the
acceptance gate — which is why statements without such a plan are rejected at
declaration, never discovered here.

## Key layout (one `_data` database, first-byte namespaces)

```
F | relation_id | row_id            -> fact_bytes     row-major facts   (reader: image builds, point-lookup fetch, export scan)
M | relation_id | fact_hash         -> row_id         membership        (reader: insert/delete idempotence, point lookups, WriteTx point reads)
U | relation_id | statement | key   -> row_id         FD determinants         (reader: functionality checks — put-conflict and neighbor probes —
                                                      key-probe lookups, WriteTx key reads, coverage walks)
R | statement | key | source_rel | source_row -> ()   statement-scoped edges  (readers: target-side containment checks on delete/shrink;
                                                      the window judgment's child-group count walk)
Q | relation_id | field_id          -> next_u64       fresh sequences  (readers: alloc, and commit's row-id
                                                      assignment on a fresh-keyed relation — the one id
                                                      allocator, ruled 2026-07-23, R16, § below; the ratchet law:
                                                      an EXPLICIT fresh-field value advances the
                                                      high-water past itself — at op time in memory,
                                                      flushed at commit — so a copied id can never
                                                      collide with a later mint; a correctness law of
                                                      fresh semantics, not an import special case)
S | relation_id | stat              -> u64            counters: stat 0 = row count (readers: the planner,
                                                      and image build's cross-check against the F scan);
                                                      stat 1 = row_id high-water (reader: commit's row-id assignment —
                                                      fresh-less relations only: a fresh-keyed relation's row id
                                                      IS its fresh id, minted from Q; ruled 2026-07-23, R16)
```

Plus `_meta` (format version, store kind, schema fingerprint, storage tx id, the
dictionary next-id counter — the delta's pending-intern design mints provisional ids
against it from read snapshots — and the **schema descriptor**: the canonical
schema-encoding bytes the fingerprint hashes, persisted whole so the store is
**self-describing**; readers: `exhume` — the read-only, theory-less open,
`70-api.md` § exhume — and `Db::verify_store`, whose descriptor pass convicts a
store whose descriptor does not hash to the stored fingerprint. Written at
creation; **back-filled by any successful fingerprint-matching open**: the
verified fingerprint proves the caller's theory is the creating theory, so open
writes the theory's canonical bytes in its own committed transaction — the
adoption path for every pre-descriptor store, one open under the true schema and
the store is self-describing forever. The storage tx id does not advance (the
descriptor is not query-visible state). Descriptor presence is ADDITIVE — no
format-version bump: the version-bump law targets keys open *decodes*, where
absence would be a silent default; the ordinary open path never reads this key,
and `exhume` refuses its absence with the typed `DescriptorMissing` naming the
remedy — absence is a stated condition, never a default) and `_dict` (**str-only** — `bytes<N>` values are inline in
facts, never interned, so the key hash carries no type tag: forward
`blake3(bytes) → id`, reverse `id → bytes`; collision axiom in
`10-data-model.md`). Key components are big-endian
(order-sensitive); stored values are not order-sensitive and are little-endian
(dictionary ids big-endian) — pinned here for the offline checker this doc defers.

- Every namespace names its reader above (README rule 3); a namespace with no
  reader is deleted. Declared opt-in accelerator namespaces exist only with a
  benchmark that demands them (OPEN, README).
- **Closed relations appear in no namespace.** Their extension is sealed in the
  theory (§ Virtual relations below) — the store contains zero vocabulary bytes —
  so no `F`/`M`/`U`/`R`/`Q`/`S` entry may name a closed `RelationId`: the write
  surface refuses (`ClosedRelationWrite`), the commit plan debug-asserts at every
  fact-op derivation (`keys::debug_assert_ordinary`), and `Db::verify_store`
  convicts any stored entry (`ClosedRelationEntry` — the entry's existence is the
  finding).
- **One id allocator** (ruled 2026-07-23, R16). A fresh id and a row id are the
  same monotone u64, minted once. On a **fresh-keyed relation** — one whose
  statements include the fresh auto-key the schema mints per `fresh` field —
  the first fresh field's value IS the `F` row id: **scan order is fresh
  order**, and that field's auto-key maintains no `U` tree, because the entry
  it would write is a pure transcription (`be(fresh) → le(row_id)`, one
  monotone counter re-keying another). The auto-key's functionality judgment is
  the `F` put-conflict itself — the identical conflict mechanism the `U` phase
  uses — so a fresh-keyed point read (WriteTx, snapshot, the query key probe)
  pays one B-tree descent, not two, and every insert drops the transcription's
  put and its conflict get. Later fresh fields and every secondary key
  statement keep their `U` entries. `Q` is the one mint for such relations; the
  `S` row-id high-water exists only where no fresh field does. Two consumers
  re-derive under the merged mint: the image append base's prefix boundary —
  explicit fresh re-supply (the documented correction idiom, `10-data-model.md`)
  can land an `F` key BELOW a retained base, so a non-tail insert evicts the
  base on the writer's eviction path (§ the columnar image cache) — and
  `Db::verify_store`'s counter pass, which judges the one mark where it judged
  two. The unification changes stored bytes, so the version-bump law below
  applies.
- `fact_bytes` = the canonical encoding owned by `10-data-model.md`; identity =
  bytes — the encoding is injective, so byte equality IS value equality
  (`lean/Bumbledb/Values.lean: value_eq_iff_encode_eq`).
- `fact_hash` = full 32-byte blake3 of `fact_bytes`; an `M` hit is trusted without
  verification (collision axiom, recorded in `10-data-model.md`).
- **`U` determinant index keys** are the FD statement's projected fields' canonical
  encodings, concatenated in statement order — order embeddings, so key order is
  value order (`lean/Bumbledb/Values.lean: encode_u64_order_embedding`,
  `encode_i64_order_embedding`). An interval field (always last —
  `30-dependencies.md` gate) contributes its 16 bytes — and a FIXED-WIDTH
  interval field (`interval<E, w>`, `10-data-model.md` § the admission rule)
  contributes its ONE 8-byte start word, the width halving: the end is the
  type's, re-derived wherever the tail is read (the neighbor probe, the
  coverage walk, the sweeper's disjointness pass all parse the tail through
  the key's interval-tail shape). Either way, within one scalar-prefix
  group the determinant B-tree is **ordered by interval start**
  (`lean/Bumbledb/Values.lean: encode_interval_order`; the fixed family's
  one word is trivially the scalar embedding, `encode_fixed_order_u64`):
  the property the pointwise check and the coverage walk stand on. A
  stored fixed start at or past the Q2 bound (`start + w < MAX_END`) is
  corruption, exactly as an inverted general interval is. A `bytes<N>` field contributes
  its ⌈N/8⌉ padded words — memcmp order over the uniform-width padded encodings is
  value-byte order, which is all the determinant needs (order *operations* on `bytes<N>`
  stay refused at the query surface; sortedness is the index's need, not a
  semantics). `MAX_DETERMINANT_WIDTH` admits the 16-byte interval contribution and the
  widest `bytes<64>` one; width overflow is a declaration-time error.
  The determinant is re-derived per fact by slicing projected fields out of
  `fact_bytes` — two encoders, `determinant_image` (statement projection
  order, the `U` key's segment) and `permuted_determinant_image` (target-key
  order for `R` keys). **The direct arm is measured law** (cleanup-0.5.0
  ruling 8, the Measure phase, 2026-07-19, `bench-out/measure-twins/`):
  `determinant_image` is the permuted encoder under the identity
  permutation, and the identity-permuted route measured **1.23–1.25×
  slower per fact** (13 vs 17 ns/fact, commit-shaped 3-field interval
  projection, warm DRAM, interleaved min-of-7 × 200k facts, two process
  runs; pre-stated bar 1.09) — the permuted arm's per-fact inverse search
  is real cost on the hot commit path, so the pair stays split.
  **Reverses if:** the permuted arm precomputes its inverse and re-measures
  within the house bar.
- **`R` keys are statement-scoped**, not relation-scoped: `statement` is the
  schema-global materialized statement id (`10-data-model.md` fingerprint), and
  `key` is the *target-side* projection value the source fact requires. One source
  fact contributes one `R` entry per containment statement whose selection it
  satisfies — conditional containments write reverse edges only for facts inside
  their σ, so the arm-validity and totality directions of a `==` each get exactly
  the edges they need. Bidirectional statements are two statement ids, symmetric
  entries. The extension form writes the same machinery under its own
  statement id: a **cardinality window** writes one edge per φ-satisfying
  child fact, `key` = the child's projection in target-key determinant order —
  the window judgment's child-group count is one prefix walk of that bucket
  (reader: `check_windows`; the window's edges exist for closed TARGETS too,
  where they are the count's only index — unlike a closed-target containment,
  whose member test needs none).
- The `statement` component of every `U` and `R` key is always the
  fingerprint-pinned `StatementId`. Validation-minted `KeyId` and
  `ContainmentId` witnesses exist only in the sealed in-memory schema and never
  enter storage bytes; the commit plan derives the persisted id from the typed
  statement at the key-construction boundary.
- Key-component widths: `relation_id` u32, `field_id` u16, statement id u16, `row_id`
  u64 — all big-endian; ids assigned by declaration/materialized order and pinned by
  the fingerprint.
- Open-time checks, in order: storage format version, then store kind, then schema
  fingerprint — each
  mismatch is a hard failure. No other format version opens and no migration path
  exists (ETL is the story, `70-api.md`; compatibility is never a design input,
  `00-product.md`). **Every on-disk encoding change bumps the version** — the
  untagged-dictionary cutover (version 2) initially shipped without a bump and a
  stale cached store silently mis-decoded until the two-oracle run caught it; a
  version bump turns that whole class into a loud open-time refusal. Version 3 is
  the dependency-vocabulary extension: the canonical schema encoding moved
  (literal-set selections, the window and order-mark statement forms), so every
  v2 fingerprint was computed under a retired encoding. Version 4 is the order
  purge: the statement spine sum shrank when the order-mark form and its
  `R`-edge namespace left the vocabulary
  (`docs/architecture/30-dependencies.md` § refused: order marks), so every v3
  fingerprint was computed under a retired encoding — nothing deployed carries
  an order statement. Version 5 is the store-kind marker: every store carries a
  `_meta` kind byte (§ the ephemeral store kind) that open consults — a new meta
  key read at open is an encoding change, so it bumps (nothing deployed carries a
  v4 store; a v4 store would otherwise open with the kind key absent, which is
  exactly the silent-default class the bump law exists to refuse).
- **A half-created store is not corruption** (ruled 2026-07-23, R18). Before
  those checks can run, open classifies the meta block itself — one
  classification, shared by every constructor, never the same branch
  hand-written three ways. No `_meta` over an empty root is the half-created
  store (the crash window between environment creation and the meta commit): a
  never-born store holding zero data. `Db::create` proceeds — creation heals
  it; the ephemeral open treats it as fresh; `Db::open` refuses it with a typed
  not-initialized error naming `Db::create` as the remedy — never `Corruption`.
  No `_meta` over a non-empty root is the foreign-environment refusal,
  `AlreadyInitialized`. `MetaMissing` convicts only a genuinely absent key
  inside an initialized store.
- **Malformed and missing are distinct meta states** (ruled 2026-07-23, R18).
  One decode discipline for every `_meta` value — the split the store-kind
  reader pins: an absent key is `MetaMissing`; a present value that fails to
  decode (wrong width, unknown byte) is the malformed-value corruption naming
  the key, never `MetaMissing`. The two states point at opposite remedies
  (initialize vs. investigate a torn write), so one error value never encodes
  both.

**Decision: one `_data` database with first-byte namespaces.** **Alternative:** one
LMDB database per namespace (enables per-namespace append mode and integer-key layouts).
**Why it lost:** a fixed tiny DBI set is simpler and the access patterns are prefix
scans and point gets either way. Stated consequence: LMDB append mode is only usable
for a **fresh-database bulk load written in global key order**; incremental writes never
append (an `M`/`Q`/`S` key always exceeds every `F` key). **Reverses if:** bulk-load
profiling shows append mode mattering for incremental use.

**Decision: `M` indirection (hash → row_id) rather than keying facts by their bytes.**
**Alternative:** `F | relation | fact_bytes -> ()` directly. **Why it lost:** fact-bytes
keys make `F` keys wide and unbounded-ish, while dense-ordinal row storage wants a
compact monotonic key; and images need scan ordinals anyway. **Reverses if:** never
likely; revisit only with the layout.

## Write path: the transaction is a delta

A write transaction is an **in-memory delta** — a net insert-set and delete-set of
canonical fact bytes, arena-backed, recording **net dispositions against committed
state**. During the closure, `insert`/`delete` are pure set arithmetic: encode the
fact, probe `M` (a read-only get) plus the delta to compute the `changed: bool`
return value, and record the *net* effect — a redundant op (insert of a committed
fact, delete of an absent one) records nothing, and an op whose net effect is
nothing *cancels* the pending opposite entry (delete + re-insert of a committed
fact, or insert + delete of an absent one, leaves no entry). The op-time probe is
authoritative because the single-writer mutex holds committed state stable for the
delta's lifetime; last-disposition-wins is a consequence of these rules, not a rule
of its own. **The invariant this buys:** the insert set contains exactly the facts
commit will add and the delete set exactly the facts it will remove — every entry
applies at commit (base state disagreeing with a proved disposition is the
`DispositionDesync` corruption, never a skip), the empty delta is the only no-op
commit shape, and judging a no-op insert is unrepresentable (the judgment-direction
divergence this closes is pinned in `60-validation.md`). `alloc` reads `Q` once at
first use per (relation, field) and increments
in memory (a transaction sees its own allocations); explicit-value inserts advance the
in-memory mark past the supplied value; mixed explicit/generated allocation tracks the
running maximum. **WriteTx point reads** (`70-api.md`: existence of a fact, lookup
through an FD key) are the same committed-state gets overlaid with the delta — they
observe exactly the final-state view the judgment checker will judge, which is what
makes read-modify-write idioms (upsert, check-then-act conditions) sound without exposing
query machinery to the write path. **Nothing touches an LMDB data page until commit**
— an abort (error or panic) drops the arena and LMDB was never written, making
"failed writes leave nothing" true by construction, not by rollback.

**Commit applies the delta in one canonical order** — this is what makes dependency
enforcement commit-time final-state judgment (`30-dependencies.md`) with plain eager
mechanics:

The derivation and judgment phases consume validation's witnesses directly:
relation key walks resolve `KeyId` to key projection and pointwise flag, while
outgoing and dependent walks resolve `ContainmentId` to sides, compiled checks, and
the enforcement sum. Persisted keys and typed errors still take each witness's
embedded `StatementId`; no commit path reconstructs or asserts descriptor/enforcement
variant agreement.

1. **Deletes**: per deleted fact — `M` get → row_id → delete `F`, `M`, its `U` entries
   (determinant keys re-derived by slicing projected fields out of `fact_bytes` — never a
   scan), and its outgoing `R` entries. Deleted `U` keys are recorded per statement
   (the target-side check set for step 3).
2. **Inserts**: per inserted fact — `F` put (row_id from the one id allocator,
   § key layout: the first fresh field's value on a fresh-keyed relation — where
   the `F` put-conflict IS that auto-key's functionality violation, recorded
   into the same collector — and the in-memory high-water otherwise),
   `M` put, `U` puts, `R` puts (per containment statement whose selection the fact
   satisfies). Because every delete has already landed and the insert-set is
   deduplicated, a scalar `U` put conflict here **is** a functionality violation —
   recorded into the commit's violation collector (the conflicting put is skipped;
   the incumbent keeps the determinant) and the phase finishes scan-complete, so the
   rejection after step 2 carries the COMPLETE set of violated key statements
   (`30-dependencies.md` § judged on final states) and preempts step 3; the whole
   transaction aborts. For a **pointwise FD** (interval-carrying determinant), the put
   cannot conflict on exact bytes alone; the insert additionally runs the
   **ordered-neighbor probe** — cursor-seek to (scalar prefix, start):
   predecessor in the same prefix group with `end > start`, or successor with
   `start < end`, is the violation, recorded identically. Two probes, O(log n),
   same B-tree. Deletes and inserts both check what they touch: a live `M` entry
   whose `F` row or `U` determinant is missing is the membership-desync corruption, a
   hard error — never silently scrubbed.
3. **Judgment phase** (final-state probes; LMDB write txns read their own writes) —
   one checker, statement-driven, restricted to delta-touched bindings:
   - **Containment, source side:** every inserted fact satisfying a statement's
     source selection probes the target's key determinant for its projected tuple; the
     found target fact is checked against the target selection (one `F` get when a
     selection exists). For interval positions, the probe is the **coverage walk**:
     from the determinant entry at or before the source interval's start, walk start-
     ordered entries of the prefix group, requiring no gap before the source's end
     — O(log n + segments). The sealed `IntervalCoverage` enforcement carries the
     validator-minted `DisjointDeterminantProof` that the target's pointwise key keeps
     its prefix group disjoint and start-ordered; `check_coverage` requires that
     token, so the soundness premise is represented rather than assumed
     (`30-dependencies.md`) — and under exactly that premise the one-pass verdict
     equals the point-subset denotation
     (`lean/Bumbledb/Exec/Sweep.lean: sweep_covered_sound_complete`; a source ray
     demands a target ray, `ray_needs_ray`). The frontier loop is the shared
     segment sweep (`interval/sweep.rs`, one walk for the checker's gap verdict and
     `Pack`'s coalescing fold); the commit site owns only entry-segment location
     and the key-shape trust checks.
   - **Containment, target side:** every target key tuple deleted in step 1 and not
     re-established in step 2 probes its statements' `R` prefixes for surviving
     source entries; for interval positions the deleted-or-shrunk window's `R`
     range is walked and each surviving source is re-checked for coverage against
     the final target state. Survivors *inserted this commit* are skipped — the two
     sides partition the final state's sources (`30-dependencies.md` § judged on
     final states): an inserted source's own probe already judged the same tuple
     source-side. A surviving pre-existing requirer → violation recorded, naming
     the *source* fact by its bytes. **Re-establishment is per statement, ψ-qualified:** for a
     dependent statement with a nonempty target selection, a re-landed determinant tuple
     counts as re-established only if the establishing fact satisfies that
     statement's ψ (one `F` get per re-established tuple per ψ-carrying dependent;
     empty-ψ dependents use the plain set difference). Without the qualification,
     delete + re-insert of identical key bytes with a changed selection field
     strands sources in a committed state — the unqualified difference is unsound
     under selections.
   - Bidirectional statements run both bullets with the sides swapped — the same
     two code paths, no third.
   - **Cardinality windows:** every TOUCHED parent key tuple — derived by the
     plan from the delta's child facts (both dispositions, φ-blind) and its
     ψ-selected parent facts (`lean/Bumbledb/Txn/DeltaRestriction.lean:
     touchedParents`) — probes the target key's `U` determinant for its
     ψ-selected holder in the final state (one get, plus one `F` get where ψ
     is nonempty; a closed parent answers from the compiled member set), then
     counts the child group by one ordered walk of the window statement's `R`
     bucket, stopped as soon as the verdict is decided
     (`lean/Bumbledb/Oracle.lean: cardinality_plan_decides`,
     `window_plan_consultations`). A closed CHILD set stored no edges: the
     φ-selected axioms are counted by an honest ≤256-row extension scan. A
     count under the floor or over the ceiling records a violation carrying
     the statement id, the parent fact's bytes, and the observed count.
   Any failure → typed error carrying the statement id, abort. The probe primitive
   ("does any fact match / does no fact match") is shared with the query executor's
   anti-probe (`40-execution.md`) — one mechanism, two callers.
4. **Counters flush**: row_id high-waters, row counts, `Q` sequences, the pending
   dictionary entries and next-id, storage tx id.
5. **LMDB commit** (fsync). The durability boundary parses its errno once (the
   trust-boundary rule, applied to the OS): a raw OS errno out of `mdb_txn_commit`
   — the commit's write/sync syscalls; on macOS the data-page `pwrite`s and
   `fcntl(F_FULLFSYNC)`, whose errno `mdb.c` surfaces raw with no fallback sync —
   is the typed `CommitSync` error naming phase and syscall class, never a bare
   `Lmdb(Io(...))`. The transient form (`F_FULLFSYNC` observed failing under I/O
   pressure) gets a **bounded, observable retry**: the failed commit aborted its
   transaction (nothing persisted), so the whole transaction is rebuilt from the
   immutable plan and re-committed — each retry an obs event
   (`commit_sync_retry`), the escaping error carrying the count. The contract is
   untouched: a retry re-runs the full write-and-sync, so every commit that
   reports success fsynced — no sync mode exists on a durable store, and none
   may be born (the ephemeral store KIND is not a mode: § the ephemeral store
   kind).

User operation order inside the closure is therefore semantically irrelevant
(`lean/Bumbledb/Txn.lean: final_state_judgment_order_free`); the
delete-before-insert trap and reference-insertion-ordering are unrepresentable. Crash
consistency is LMDB atomicity — *tested* (the kill-during-commit crash/reopen family,
`60-validation.md`; the crashpoint table below was additionally exercised
adversarially until the fuzzing apparatus was deleted, `60-validation.md` § the
deletion record). Dictionary entries are never removed (accepted leak; the delete
path never *adds* one either — a never-interned value proves its fact absent).

**Crashpoints: the named atomicity structure.** Under the `crashpoint` feature (off
by default; the hook macro expands to nothing without it, and the compiled hooks are
inert unless `BUMBLEDB_CRASHPOINT` is set) every phase boundary above is NAMED, and a
process whose environment names one aborts there — a real unclean death, no unwinding
cleanup. The table (`storage/commit.rs`)
IS the claimed atomicity structure, reviewable in one grep of the hook macro's call
sites. The recovery claim it makes was proven per point by the `crash` fuzz target
and its deterministic sweep before the fuzzing apparatus was deleted
(`60-validation.md` § the deletion record — the sweeps ran green, every point): the
store
reopens, `verify_store` is green, full contents equal the pre-victim state at every
point before `mdb_txn_commit` and the post-commit state after it (all-or-nothing —
there is no third observable outcome), and re-running the torn commit lands its post
state. We do not fault-inject the filesystem — LMDB owns that layer; we kill
ourselves between logical phases.

| crashpoint | where | recovery |
| --- | --- | --- |
| `after-staging` | staging over, before plan derivation | prefix |
| `mid-write-m` | phase 2, after a fact's `M` put | prefix |
| `mid-write-f` | phase 2, after a fact's `F` put | prefix |
| `mid-write-u` | phase 2, after a `U` determinant put | prefix |
| `mid-write-r` | phase 2, after an `R` edge put | prefix |
| `before-judgment` | phases 1–2 applied, before phase 3 | prefix |
| `mid-write-s` | phase 4, after an `S` row-count put | prefix |
| `after-judgment` | phases 3–4 done, before `mdb_txn_commit` | prefix |
| `after-commit` | `mdb_txn_commit` returned, before the memo update | post |
| `after-memo-update` | after the image-cache eviction and commit-seq bump | post |

The counters-only no-op commit is deliberately outside the table: it never changes
query-visible state, and its crash story is the existing kill test. The one
recovery-side nuance, recorded: a prefix-side death during the very *first* commit
recovers to the empty store, which the offline sweeper flags for unsatisfied domain
quantifications by design (`30-dependencies.md` — closed-source statements are
violated until their backings land); the crash oracle compares that case's findings
against a fresh store's, exactly.

Two write-side asymmetries, recorded as decisions rather than left as surprises:
**R-delete verification** — deleting a fact deletes its `R` entries without
verifying they existed (unlike `F`/`M`/`U`, whose absence is the
`MembershipDesync` hard error); a missing `R` entry is not independently
detectable at delete time without re-deriving every statement's edges, and the
class is covered by the offline sweeper, `Db::verify_store` — the same
compensating control that re-verifies the rest of M↔F↔U↔R consistency.
**The online path maintains reverse edges; the offline pass proves them** —
including the delete-asymmetry fixture in the verifier suite
(`crates/bumbledb/src/verify_store/tests.rs`, the deterministic
corruption fixtures). **Counter overflow
checks** — the fresh ceiling is checked
(`FreshExhausted` at `u64::MAX`, because hosts can supply explicit fresh values),
while the storage tx id and the fresh-less row-id high-waters are not: they advance
by at most one per commit/insert, so wrapping needs ~2⁶⁴ commits — twelve orders
beyond the scale axiom, and no host input can jump them. A fresh-keyed relation's
row-id mint IS its fresh mark (the one id allocator, § key layout; ruled
2026-07-23, R16) — host-jumpable by explicit re-supply, and therefore exactly the
counter `FreshExhausted` already guards. The asymmetry is chosen, not overlooked.

**Storage tx id:** advances **once per commit that changed logical state**; a commit
whose delta is empty (all no-ops) does not advance it and does not invalidate any
image. It lives in `_meta` and commits atomically with the data. A successful no-op
commit still flushes any *dirty* fresh marks (`Q` values that advanced past their
committed base — allocations the closure may have returned to the host) in a
counters-only LMDB transaction: the tx id identifies query-visible state (`F/M/U/R`),
and `Q` marks are write-path bookkeeping no query reads, so every image and memo key
stays valid. Pending interns of a no-op commit are dropped — intern ids never escape.

Bulk load (`70-api.md` surface) is the same delta mechanism at scale — chunked into
multiple transactions (4096 facts each; a failing chunk aborts whole, prior chunks
stay committed, and the error carries the committed count). Chunking has a new
stated consequence under bidirectional containments: **a `==` statement's cluster
must be judged whole**, so a chunk boundary that splits a cluster mid-load fails
that chunk's commit loudly (never silently); the documented import order —
dependency-cluster order, owned by `70-api.md`'s ETL section — makes the failure
unreachable for well-formed exports. The fresh-database append-order fast path
stays deliberately unbuilt.

**Corrupt data is a hard error, never a skip:** an `F` value whose length differs from
the schema's fact width, a dangling intern id, an `M`/`F` disagreement, an
nonzero `bytes<N>` pad byte, an interval with `start ≥ end` — any of these aborts the
scan/query with a corruption error; an engine that silently skips undecodable rows
silently shrinks query results, which is the worse bug. Reopen-trusted counters are
additionally **bounded before they size anything**: the image build caps the claimed
`S` row count by the `_data` DBI entry count (`mdb_stat`, O(1)) — a witness that
over-approximates any one relation's rows because the DBI spans every namespace,
which is exactly what a ceiling is allowed to do — and a claim above it is the typed
`CounterDesync` corruption *before* any size-derived allocation; the F-scan
cross-check stays the exactness guarantee. The offline integrity checker,
`Db::verify_store`, then proves canonical F encodings, M↔F↔U↔R coherence,
pointwise disjointness, global scalar/coverage containments, virtual-relation
absence, counters, and dictionary bounds over one snapshot. The evidence that an
empty finding list means something is in-tree: every verifier pass has a
deterministic corruption fixture that plants the defect raw in `_data` and
asserts the named conviction (`crates/bumbledb/src/verify_store/tests.rs`) —
an empty finding list is backed by a fixture per claim, not by a smoke test.
(The packet-era coverage ledger this section once cited is retired; the
fixtures are its durable form.)

## The ephemeral store kind

A store IS a kind, marked on disk: `_meta` carries a kind byte (0 durable,
1 ephemeral) beside the format version and fingerprint, written at creation and
read at every open — after the version check, before the fingerprint. The durable
constructors (`Db::create`/`Db::open`) mint and open only durable stores;
`Db::ephemeral` (`70-api.md` § environment lifecycle) only ephemeral ones; the
cross-open matrix is four cells, all typed
(`crates/bumbledb/tests/ephemeral.rs`): open-on-durable and
ephemeral-on-ephemeral succeed, open-on-ephemeral and ephemeral-on-durable are the
typed `StoreKindMismatch` naming found and expected kinds. Parse, don't validate:
holding a `Db` handle proves the store's kind, so the durable surface can never
quietly read a store that skipped its fsyncs.

An ephemeral environment differs from a durable one in exactly one flag: the
LMDB `MDB_NOSYNC` (set inside `storage/env/open_env.rs`, the
one raw-open chokepoint, where the flags are DERIVED from the kind and the open
lane — `MDB_RDONLY` belongs to the read-only lane, R17 above — no flag
parameter exists, so the durable paths structurally cannot reach them; the unsafe
policy allowance and safety comments live there). `NOTLS`, the advisory lock, the
map size, the reader table, fingerprint verification, the whole write path, the
dependency judgment, and WriteTx point-read semantics are identical — proven by
the durable/ephemeral differential oracle (`60-validation.md`). The kind's one
other distinction is lifecycle, not environment: the dirty marker of the crash
contract below.

What the kind renounces is machine-crash (power-loss) durability and nothing
else. **The crash-sweep evidence** (banked; the sweep died with the fuzzing
apparatus, `60-validation.md` § the deletion record): the deterministic
crashpoint sweep — every named commit-pipeline point × the ops-prefix matrix —
ran against ephemeral stores too, and every combination
recovered all-or-nothing under `NOSYNC` exactly as the durable table
above claims: reopen, `verify_store` green, contents at the expected side, victim
replay — no third observable outcome. This is the expected LMDB
result — `NOSYNC` removes the fsync barrier, which only a power loss can
exploit, and never touches the meta-page commit protocol that atomicity stands
on — and the sweep was the proof the expectation was not doing the work.
(Retraction, cleanup-0.5.0 ruling 1: the flag set was `WRITEMAP|NOSYNC` —
WRITEMAP shipped on the 2026-07-15 sweep verdict with NOSYNC-only as the
recorded fallback. Ruling 1 promoted the fallback to the law: WRITEMAP's
open-time full-map ftruncate forced the eager capacity contract, whose price
at the 32 GiB map — 32 GiB of real disk or RAM per ephemeral open — no lane
could pay. The deterministic sweep and the kill smoke re-ran green under
NOSYNC-only; the ≥2,000-round statistical kill lane's recorded sessions
predate the flip, and a NOSYNC-only session is owed with the Measure phase.
Nothing was weakened.)

**The crash contract** (ruled 2026-07-23, R18): contents survive process
restarts, never machine crashes — and reopening after a crash yields a valid
empty store, always. Every ephemeral open sets a synced dirty marker before
trusting anything else, and a clean close clears it in a small synced commit —
the kind's only fsyncs, bracketing its lifetime. A reopen that finds the marker
set — power loss, or a process death that never reached clean close — wipes the
store and re-initializes it; the verified reopen (version, kind, fingerprint,
the same checks as `open`) is reserved for the marker-proven clean lineage.
What the marker refuses to open is the state `NOSYNC` makes possible and
`_meta` cannot see: a meta page flushed by incidental writeback over data pages
that never landed — fingerprint-valid over trees no committed transaction ever
contained. That state is unrepresentable, not detected: the possibly-torn store
is never opened at all. The kind's law is thereby exact — **an ephemeral store
never destroys data it promised to keep** — and the marker-clean lineage is the
promise's whole extent: a store that crossed a machine crash was already lost
by the store's own definition (below). The banked sweep evidence above keeps
its meaning — it proved `NOSYNC` never broke the meta-page commit protocol
under process kill — but reopen-after-unclean-death no longer stands on it: the
wipe happens before any data page is trusted.

The kind is **device-independent**: ephemeral-on-SSD is legitimate, and
ephemeral-on-ramdisk buys the flag's latency on top of the device's — the
device tax measured **1.1–1.6x** under `NOSYNC`-only (the R6 lane of
`crates/bumbledb/tests/ramdisk_phase_r.rs`, re-earned by the Measure phase
2026-07-19 across three interleaved sessions,
`bench-out/measure-ephemeral-r6/`; the retired `WRITEMAP|NOSYNC` figure was
~1.0–1.1x — and the device-independence *design* stands on the kind marker,
not the number). The kind
carries the no-durability claim, not the device, so no lie is possible — a
machine crash loses an ephemeral store by the store's own definition. (The
device-honesty rule for *timed* lanes is the orthogonal axis: `60-validation.md`.)
Capacity under the lazy map is the filesystem's own story: no open truncates
or preallocates anything (§ environment constants above), a store's data file
holds only committed pages, and a volume that fills surfaces as the failing
commit's typed `Lmdb` error. (Retraction, same ruling: the retired capacity
contract judged capacity once at open — real blocks for the full map,
`StorageFull` typed — to keep WRITEMAP's sparse ftruncate honest under
`NOSYNC`; with no dirty pages ever written through a mapping, the
unbackable-page hazard it guarded against is gone with the flag, and the
eager allocation would be pure cost.)

Lean owns none of this: durability and crash recovery are mechanism, outside the
model (`lean/README.md` § what Lean does NOT own), so the store kind adds no
Bridge row and the census expects no citation here — the sweep and the
differential oracle are the evidence.

**Decision: a distinct constructor and an on-disk KIND marker, `NOSYNC`.**
**Alternative 1:** a sync-mode flag on `create`/`open`. **Why it lost:** a mode is
a runtime claim nobody can read back; a kind is parsed at open and refuses
mismatches typed — and the durability law (`00-product.md`) stays whole: *no sync
mode exists on a durable store, and none may be born.* **Alternative 2:** the
earlier RAM-backed-device precondition (the phase-2 refusal's shape: ephemeral
only on RAM-backed paths). **Why it lost (superseded):** it tied the API's truth
conditions to device identity, which the kind marker makes unnecessary — the
marker, not the medium, carries the claim. **Alternative 3:** `WRITEMAP|NOSYNC` —
the flag set that originally shipped (the sweep convicted nothing and the R4
cells priced it fastest on the small-commit shape). **Why it lost (reversed,
cleanup-0.5.0 ruling 1):** WRITEMAP's open-time full-map ftruncate forced the
eager capacity contract, unpayable at the 32 GiB map; its measured price
advantage was earned under the old flag set, and the Measure phase re-earned
the kind's price under `NOSYNC`-only (2026-07-19, three interleaved R6
sessions, `bench-out/measure-ephemeral-r6/`: small-commit flags dividend
27–52x on SSD and 3.1–3.5x on the ramdisk, staging win 43–70x, device tax
1.1–1.6x — the WRITEMAP-era ~75–90x / ~4.2–4.4x band narrowed, the win
stands whole, so the rationale survives its own re-argument). The deterministic crash sweep and the kill
smoke re-ran green under `NOSYNC`-only while they lived (they died with the
fuzzing apparatus, `60-validation.md` § the deletion record), so the kind's
claim lost nothing. **Reverses if:** any
crashpoint ever shows a non-all-or-nothing recovery on an ephemeral store —
the kind and surface stay while the flag set answers for it.

## The columnar image cache (the hot representation)

The bridge to paper-faithful execution (`40-execution.md` D1):

- A **relation image** is **all columns** of a relation, decoded from one sequential
  `F`-prefix scan into whole-slab, 128-byte-aligned SoA vectors (one allocation per
  store, freed as a whole — the arena discipline without the arena type), plus the
  row count. **An interval field decodes into two parallel 8-byte columns**
  (start, end) — the image layer has no 16-byte column kind, membership and overlap
  lower to word comparisons over the pair (`40-execution.md`), and every existing
  kernel shape (predicate scan, compaction, gather, fold) applies unchanged. A
  fixed-width interval field (`interval<E, w>` — 8 stored bytes, the start)
  fills the SAME two columns: the image derivation computes
  `end = start + w` in the order-preserving word domain (the bias is
  additive, so the derived end is exact for either element), so membership
  and Allen classify over derived bounds through kernels that never learn a
  width existed. A
  `bytes<N>` field generalizes the same precedent: ⌈N/8⌉ parallel word columns
  (one plain word column for N ≤ 8), with the trailing pad validated zero at
  decode. The multi-byte unit exists only in `fact_bytes` and determinant keys, where
  ordering needs it.
  A build is linear in image bytes. The old anchor here — "at ~60 GB/s of
  single-core scan bandwidth a build is single-digit milliseconds per 100 MB,
  the number that makes the whole cache design sound" — is PENDING RE-TRUE on
  both counts (the incremental-images wave): it was bandwidth arithmetic, never
  a measurement of the decode-bound build path. The Wave-M record landed
  (2026-07-19, Apple M2 Max, `scripts/measure.sh`, scale S seed 1, durable
  stores, min-of-3 p50s): `cold_containment_walk` 1356.4 µs with
  copy-on-append vs 3405.2 µs with lineage disabled (the in-process A/B twin —
  since deleted with its number banked, the manifest's ruling-4
  gravestone — 2.54× family-level), and the
  rebuild-bearing `cold_containment_walk_delete` at 3540.6 µs — so a full
  scale-S ledger rebuild-plus-execute sits at ~3.4–3.5 ms where the append
  path pays ~1.4 ms. The per-100 MB normalization is still open (the
  `#[ignore]`d `image_build_split_evidence` harness, `image/tests/timing.rs`,
  has not run under measured conditions), and even where the figure holds, its conclusion died at the
  32 GiB ceiling — a full rebuild of a ceiling-scale (tens of GiB) image is
  seconds, not milliseconds, so what keeps the cache design sound is
  copy-on-append maintenance, with the delete-bearing rebuild as the priced
  exception. **Column strides are
  padded off 16 KiB multiples** (measured): L1D set congruence (256 sets × 64 B
  lines, bits 6–13) costs at most 1.55× on real lockstep scans — never the folklore
  10–20×, which requires a fully dependent load chain — while the hazard that
  actually matters is stream-prefetch trackers aliasing on low 16 KiB page-number
  bits: power-of-two-ish strides with small (1–3 line) staggers cost 4–6× on
  DRAM-tier lockstep scans (8.13 vs 1.78 ns/row). The rule:
  when a column-to-column stride within a slab is ≥ 64 KiB and lands within 384 B of a
  16 KiB multiple, round it up to the next exact multiple (exact multiples measured
  clean — the poison is the small offset). Immutable once built. Positions in the
  image are **dense scan ordinals**; `row_id`s exist only in LMDB keys and never
  appear in images (COLT offsets are image positions; the key-probe path reads `F`
  directly and never needs a translation).
  **Decision: full-width images, cache key `(relation_id, storage_tx_id)`.**
  **Alternative:** per-field-scope images. **Why it lost:** scope keys are combinatorial
  (defeating sharing and the "tiny key space" claim), overlapping scopes duplicate
  columns, and whole relations are the affordable unit — cheap outright within
  the validated envelope (≤1 GB), and kept affordable toward the 32 GiB ceiling
  (where one relation's image reaches tens of GiB and a full build reaches
  seconds) by copy-on-append maintenance, not by size. **Reverses if:** a wide-relation
  workload appears (it won't; BCNF relations are narrow).
- **Generation correctness:** a reader's generation T is the storage tx id read from
  `_meta` **inside its own snapshot** — never an in-process counter. This closes the
  open-snapshot/read-counter race that could poison the shared cache.
- The cache is a field of the `Db` handle, shared by reader threads through `&Db`
  (the handle is `Send + Sync`; no `Arc` of the cache itself is needed since one
  writing handle exists per path — the lock law is a writer law, top of this
  doc, and read-only opens never touch the cache). Two readers at the same T
  racing to build the same image:
  both may build; insert-if-absent, the loser adopts the winner's `Arc` and drops its
  own (accepted waste, no latch — priced at the validated ≤1 GB scale, where a
  duplicate build is milliseconds and megabytes; at a ceiling-scale relation the
  same race is a seconds-and-gigabytes event, restated honestly in § memory
  discipline — the no-latch decision itself stands, correctness unaffected).
  The insert re-checks the newest generation under
  the lock, so a reader racing a commit cannot re-insert an evicted generation.
- **Eviction:** at each state-changing commit the writer drops the entries of
  relations the commit **deleted from** — or inserted into below the retained
  base's boundary: under the one id allocator (§ key layout; ruled 2026-07-23,
  R16) an explicit fresh re-supply can land an `F` key under an append base,
  and a tail decode would silently miss it, so the non-tail check is one
  comparison per insert on the eviction path where the delete branch already
  lives. Delete-free, tail-only relations' images are retained as append
  bases — the next reader at the new generation copies columns and decodes
  only the tail (tail-only insertion is the prefix property, enforced by that
  eviction rather than assumed from counter shape), or carries the same `Arc`
  forward re-keyed when the relation is untouched. Readers still pinned at
  older generations keep their
  `Arc`s alive until their transactions end; a long-lived old-generation reader
  that needs an *unbuilt* image builds it query-locally without caching
  (accepted — the cost lands on the stale pinned reader alone and poisons
  nothing shared). The old parenthetical here — "writes are bursty and rare" —
  is RETRACTED: it was a workload assumption, never a measurement, and
  steady-write hosts are real; they are served by the copy-on-append path, not
  by an assumption about write frequency. There is **no memory-pressure
  eviction, ever** — no longer justified by the scale axiom but by the capacity
  plan stated in § memory discipline: the working set is the host's to
  provision, and a machine that cannot hold it is out of envelope for that
  store (`00-product.md`'s no-mmap-grace rule is what keeps this honest).
- **Filters:** on a cold relation with a filtered query, one *storage* scan produces
  both the cached unfiltered image and the query-local survivor view (the filter is a
  second pass over the decoded in-memory columns — the storage scan is the expensive
  part); on a warm relation the view is computed by scanning the cached image (NEON
  filter kernels). Views are survivor-position vectors in retained-capacity buffers,
  never cached; the prepared query additionally memoizes its views per (generation,
  resolved filters), so a warm re-execution skips even the in-memory re-scan.
- Invariant test: two sequential read
  transactions with no intervening write share identical image instances; plus the
  concurrent families in `60-validation.md`.

## Virtual relations (closed): the theory as storage

A closed relation's image is **synthesized, not built**: the sealed extension —
values canonically encoded ONCE, at validate — decodes through the ordinary
decode plan into the ordinary SoA layout (implicit `id` column `0..rows` first,
interval = two word columns, stride padding, lazy exact cardinality counters), with
**no LMDB transaction anywhere** (`image::synthesize_closed` takes none;
synthesis is pure). The fingerprint's preimage IS the storage: vocabulary can
never desync, never bloat, and never needs the sweeper — its "generation" is
the theory itself.

- **Cache behavior:** the synthesized image lives in a per-relation `OnceLock`
  slot on the cache, sized at cache construction from the schema and keyed
  *outside* the generation map — built on first touch, **never evicted, never
  rebuilt** (the production commit hook `ImageCache::advance` skips it by
  construction, as does its test-gated retain-newest twin `evict_older_than`;
  it is not in the generation-keyed map at all). `peek` answers it once
  resident, with no generation read.
- **Read surfaces:** `Snapshot::scan`/`scan_facts` yield the extension's
  canonical fact bytes directly (row id = declaration index); `WriteTx`
  point reads (`get_dyn`) resolve against the extension by re-deriving determinant
  bytes per row — ≤256 rows, L1-resident, an honest linear scan. There are no
  `U` determinants to probe: the closed auto-key is enforced by validation's
  duplicate-handle check, and the key-probe fast path refuses closed
  relations at classification (`40-execution.md` § access paths).
- **Write refusal, three layers:** the typed `ClosedRelationWrite` at every
  write-surface entry, the commit plan's debug assertion, and the sweeper's
  `ClosedRelationEntry` conviction (the namespace-table note above).

## Memory discipline

Images are whole-slab allocations freed as wholes; no per-value heap objects in
storage or images. Query scratch belongs to prepared queries (`40-execution.md`).
Steady-state process heap = LMDB's mmap + the newest generation's images +
at most one below-newest append base per delete-free relation (every cache
insert sweeps its relation's entries below its own generation in the same
critical section — `ImageCache::get_or_build` — so no entry outlives the next
insert above it: an epilogue-racing reader's full build supersedes the base
instead of stranding it, surplus is transient and bounded by concurrently
racing readers, and the map stays O(relations)) + per-prepared-query pools +
a constant. Prepared
queries hold current-generation
images only: prepare binds no image at all (`View::Unbound`), and each execution
reaps memoized bindings below its generation — old images die with the last pinned
reader or the first post-commit execution, whichever is later.

**The image-memory story at the 32 GiB ceiling, stated honestly.** A decoded
image is ≈ the relation's live fact-payload bytes (8 B per word-shaped field,
8·⌈N/8⌉ per `bytes<N>`, 1 B per bool — two slabs, padding aside), and the cache
holds the newest generation of every relation ever read with **no byte budget
anywhere** — no ceiling exists in `image/build.rs` below checked `usize`
arithmetic, and no memory-pressure eviction exists (above). On a full-map
32 GiB durable store the live payload plausibly spans ~15 % (narrow facts) to
~60 %+ (wide facts) of the file — **~4–20 GiB of decoded images**, with a
single dominating relation's image reaching 10–20 GiB. Transient multipliers on
top, each real:

- **the append path holds base + successor** — copy-on-append's 2× transient
  peak is ONE relation's image, per-relation not per-store, while the new `Arc`
  mints beside the old (today's evict-and-rebuild already reaches the same 2×
  whenever a pinned reader or parked memo binding holds the old generation
  during a rebuild — copy-on-append makes the overlap deterministic instead of
  reader-dependent; within the validated envelope that is ≤ ~2 × 1 GB, the
  accepted cost);
- **a racing same-generation double build** — two full slabs at once (the
  no-latch race above);
- **a pinned old-generation reader** — worst case one full extra image set per
  pinned generation;
- **parked prepared-query bindings** — `ViewMemo` parks up to 3 stale bindings
  per occurrence, each holding an image `Arc`, reaped only at the next bind: an
  idle prepared query strands its images' memory until it runs again — invisible
  at the validated scale, multi-GiB at ceiling scale.

Peak plausible on a full-map store: **2–3× the decoded payload, ~30–60 GiB** —
the canonical 96 GB machine holds one such store; the 16 GB minimum machine
cannot open-and-read one at all, and that is in-envelope by rule, not a bug:
the ceiling is headroom, the validated envelope (`00-product.md`) is unchanged,
and data beyond RAM stays a non-goal with no mmap grace. **Deferred, recorded:**
a byte-budget/eviction-under-pressure doctrine (and chunked columns with
structural sharing of full chunks, which would kill the append path's prefix
copy) is deliberately NOT designed — nothing today implies one exists. The
trigger to design it: a real workload whose working set is meant to approach
the ceiling, at which point the budget story is an owner ruling plus a design
item, not a patch (this paragraph is the durable copy of that deferral — the
prd-G1 file that first recorded it was deleted when cleanup-0.5.0 ruling 1
superseded the per-kind split).

## Operations

Backup = file copy of the environment (or `mdb_copy`) while the writer is quiesced.
Compaction and space reclamation = ETL into a fresh database (`70-api.md`
export/import surfaces). The LMDB file never shrinks; the dictionary leaks by
accepted design. That is the entire operational story, deliberately.

## Store-size anatomy and compaction

The store is larger than SQLite's for the same logical content, structurally and by
design — recorded so nobody re-derives it:

- **Freelist churn**: chunked bulk-load commits leave CoW residue as free pages.
  LMDB never shrinks its file; length reflects peak usage.
- **Several `_data` entries per fact by design**: fact (`F`) + membership hash
  (`M`) + one FD determinant (`U`) per key + one reverse edge (`R`) per satisfied
  containment direction and per window whose φ the fact satisfies. This is
  deliberate rent for O(log n) commit-time judgment checks and stays.
- **16 KB pages** on Apple Silicon (LMDB uses the OS page size) — chunkier
  B-tree overhead than SQLite's 4 KB pages with varint-packed rows.

The churn component is recoverable: `Db::compact(dest)` writes a live-pages-only
sequential copy through LMDB's `mdb_env_copy2(MDB_CP_COMPACT)` (copy-and-swap,
never in-place; refuses an existing destination; the copy is a first-class
writable store). The bench corpus cache loads into a scratch sibling and
compacts into place, so cached corpora ship live-sized. Auto-compaction of
live stores stays a non-goal — the door is tool-driven.
