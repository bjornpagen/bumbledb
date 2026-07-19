# PRD-I1 — Copy-on-append image maintenance

Repo: bumbledb (`crates/`, `fuzz/`, `docs/`) · depends on: — · engine-only, no
SDK surface · gates: `scripts/check.sh` + `scripts/lean.sh` (no lean change —
images are below the model) · measurement owner-gated (idle machine, Wave M).

## Objective

On a commit whose delta contains **no delete for relation R**, the next reader's
image of R is: a fresh frame sized at the new row count, a per-column
`copy_from_slice` of the previous image's columns, and a decode of **only the
tail rows** via the existing per-fact decode plan over a suffix range scan. A
commit that deletes anything from R keeps today's evict-and-rebuild for R,
exactly. A commit that never touched R carries R's image forward at zero copy
(the same immutable `Arc`, re-keyed). Memos, the crash story, and the
no-memory-pressure-eviction axiom are deliberately zero-diff.

Soundness in one paragraph: F keys are `F | relation(u32 BE) | row_id(u64 BE)`
(`storage/keys.rs:285-290`) and the image build is one F-prefix cursor in
row-id order (`storage/read/scan.rs:29-36`, `image/decode.rs:159-182` —
positions are scan ordinals). Row ids come from the `S | rel | RowIdHighWater`
allocator (`storage/commit/applier.rs:276-294`; flushed in phase 4,
`commit/write.rs:337-340`; deletes never touch it, `applier.rs:14-70`; the
sweeper convicts a live id ≥ the high-water, `verify_store.rs:185`). The
committed high-water is monotone forever, every row of an insert-only commit
sorts strictly after every surviving row, fact bytes are immutable (the only F
writers are insert-at-fresh-id and delete, `applier.rs:95-96`/`25-34`) — so
gen-G's row sequence is a **logical prefix** of gen-G+1's: same ordinals, same
column words. Citation discipline: this is the row-id high-water's
monotonicity, NOT the `Q` fresh never-reissue law (`storage/delta/alloc.rs` —
field values; aborted commits DO re-mint row ids, harmlessly).

## The change sites

A. **The discriminator.** The delta's net-disposition invariant
   (`storage/delta.rs:85-100`; cancellation exact, `delta/insert.rs:9-14`,
   `delta/delete.rs:9-14`) means "no `(R, _) → Delete` entry" is precisely
   "commit removes no fact from R" — no false negatives from cancelled pairs.
   `WriteDelta` gains one `pub(crate)` accessor beside `deletes()`
   (`delta/accessors.rs:30-32`): `dirty_relations()` — deduplicated relations
   with ≥ 1 `Delete` disposition (one ordered pass over the `(rel, hash)`-keyed
   `BTreeMap`, ≤ one small `Vec`). Classified in `write_witnessed`
   (`api/db/write.rs:169-250`) between `burn.disarm()` and `commit(...)`; every
   write shape funnels through this one body (`write`, `write_from`,
   `bulk_load`'s 4096-fact chunks) — covered by construction.

B. **The cache becomes lineage-aware.** `evict_older_than`
   (`image/cache/evict_older_than.rs:18-36`) is replaced at the one wiring
   point (`api/db/write.rs:239-247`) by `advance(new_generation, dirty)`:
   dirty relations' entries drop (today's behavior exactly); all others are
   **retained as append bases** — the deliberate answer to the eviction-timing
   adversity (today the commit instantly empties the cache of the exact gen-G
   Arcs an extend would want; never scavenge them from memo bindings). Map
   value grows to `Cached { image: Arc<RelationImage>, row_id_next: u64 }`
   (`cache.rs:36`; the high-water read in the build txn). The **lineage law**,
   documented on `CacheInner`: an entry at generation g < newest exists only if
   every state-changing commit in (g, newest] was delete-free for its relation.
   Corollary: ≤ 1 below-newest entry per relation — the map stays O(relations),
   the scale axiom's no-eviction stance unstrained.

C. **The three arms in `get_or_build`** (miss at `generation == newest`;
   `get_or_build.rs:34-119`): probe the relation's below-newest base under the
   first lock (Arc clone out — map-ops-only); outside the lock read
   `claimed = read::row_count(txn, rel)` with the untrusted-counter discipline
   intact (`data_entries` ceiling / `CounterDesync`, `build.rs:188-205`; tail
   cross-checked against the scan, `RowCountMismatch`):
   - `claimed == base.row_count()` ⇒ **carry-forward**: insert
     `Arc::clone(&base.image)` at `(rel, generation)`, remove the base key.
     Sound because images are immutable (`image.rs:6`) and insert-only lineage
     + equal counts ⇒ identical content.
   - `claimed > base.row_count()` ⇒ **append**: new `image::append(txn, schema,
     rel, &base)` beside `build` — fresh frame at `claimed` (fresh
     `StridePadder`; layouts are address-dependent, `image/stride.rs:25-53`, so
     the copy unit is the **column**, never the slab), per-column prefix
     `copy_from_slice` (exactly two column kinds, `image.rs:37-47` — Words and
     Bytes; interval = two word columns; strings hold intern ids and the dict
     is append-only, `storage/dict.rs:15`; `distincts` must NOT copy — `seal`
     mints fresh `OnceLock`s, the `TransientImage::refill` precedent,
     `build.rs:301-306`), tail fill from the new
     `read::scan_from(txn, schema, rel, base.row_id_next)` (a `range()` from
     `fact_key(rel, from)`, same 13-byte parse / `check_width` /
     fuse-on-first-corruption as `scan`) at positions `base.row_count()..claimed`
     via the generalized `fill_columns` (takes iterator + starting position).
     Insert with the existing newest re-check + insert-if-absent adoption,
     removing the base key in the same critical section. **Amended as built
     (the review wave):** the insert sweeps EVERY older entry of its relation
     in that critical section, not one remembered base key — remove-by-key
     stranded one whole image per commit-epilogue race won (a full build whose
     snapshot ran ahead of `advance` probes no base and removed nothing),
     monotone forever on a never-deleted relation; under the sweep no entry
     outlives the next insert above it — surplus is transient and bounded by
     concurrently racing readers, never monotone.
   - `claimed < base.row_count()` ⇒ typed `Corruption` (`RowCountMismatch`) —
     under the lineage law only corruption shrinks a count; hard error, never a
     skip.
   Readers below `newest`: exactly as today (query-local, never inserted),
   except they may now HIT a retained base at their generation — a strict
   improvement with no code. `peek` untouched. Trace counters (`cache/stats.rs`,
   feature `trace`) gain `appends` and `carries` beside `builds`.

D. **The memo story is zero-diff.** `ViewMemo` re-forces exactly as today (any
   generation change is a miss, `run_join.rs:137-151`); what changes is only
   the COST of its miss. Extending Colts (re-point + suffix `force_ingest`,
   relaxing `gather.rs:186-188`'s never-re-target contract) is the named
   follow-on **PRD-I-next (memo extension)**, not this PRD — as is the
   O(delta) in-place slab (capacity-framed strides + a published-length
   watermark, a new memory-ordering invariant; `Arc::get_mut` uniqueness can
   essentially never fire on a cache-resident image and MUST not be forced —
   bumping `row_count` under a pinned gen-G reader breaks snapshot isolation).
   ResolveMemo, plan statistics/OccurrencePin, fixpoint `TransientImage` slots,
   and the parked-LMDB-reader epilogue order (cache hook BEFORE
   `CommitSeq::advance`, `write.rs:243-246`) are all unaffected — preserved
   verbatim by `advance()`.

E. **Edge cases, decided.** Empty base appends normally; never-read relation =
   full build; k chained insert-only commits = ONE append from the old base
   (the tail scan covers all k — monotone ids, no chain bookkeeping);
   `bulk_load` with interleaved readers pays one append per read (the O(n)
   prefix copy is the recorded cost the slab follow-on removes); tail rows land
   in fact-hash order (unchanged — both paths are row-id order); closed
   relations branch before the map, untouched; the cache is process-local, so
   reopen/crash equals today.

## Correctness referee

An appended image must be indistinguishable from a rebuilt one **at the column
granularity** (slab capacities, starts, strides legitimately differ per
allocation and are unobservable through `RelationImage::column(i)`,
`image.rs:189-198`):

> For every generation g reached by insert-only commits: `appended(g)` and
> `rebuilt-from-scratch(g)` agree on `row_count`, `spans`, column count, every
> column's full slice byte-for-byte, and every forced `distinct(c)`.

Set-semantic query parity (the ops fuzz differential) already catches
wrong/missing/extra tail rows — but an append landing the right multiset at
wrong positions, or corrupting lazily-observed metadata, passes every existing
oracle. The column differential is genuinely new coverage: nothing in-tree
compares two fill paths.

**New coverage shipped:**
1. **The column differential test** (integration): every field shape
   (u64/i64/str/bool/bytes<3>/bytes<20>/interval/fixed-interval), k chained
   insert-only commits with reads between, per-generation compare vs a
   from-scratch `build()` in the same read txn; carry-forward proven by
   `Arc::ptr_eq`.
2. **The delete-fallback pin** (feature `trace`): a delta with one delete for R
   increments `builds`, not `appends`, on R's next read; insert-only increments
   `appends`/`carries`; mixed multi-relation deltas take the right arm per
   relation. Pins the discriminator so appended-across-a-delete cannot exist
   silently.
3. **Fuzz extension** (the ops lane): (a) an insert-streak scenario bias in
   `corpus_gen/opgen.rs` so append-on-append chains stop being rare, and
   neutralize the 1-in-10 closed-case arm that silently injects a delete into
   an "insert" batch (`opgen.rs:269`); (b) a sixth ops oracle — after each
   state-changing commit, per touched relation, the column differential (engine
   image vs fresh `build()`) at the ops target's Tiny scale. Corpus replay
   tests inherit both.
4. **Crash-safety: untouched, and here is why.** Images are pure in-memory
   state (no persistence code under `image/`); the append runs lazily inside
   LATER read transactions, strictly after the victim commit's
   `mdb_txn_commit`; the Post-side crashpoints already bracket the hook. No new
   crashpoints; the sweep matrix is unchanged; the only diff is the
   `CRASHPOINTS` prose for `after-memo-update` ("eviction" → "advance",
   `storage/commit.rs:81`).

## Concurrency (no new invariant)

The cache mutex still covers map operations only, every critical section
panic-free (`cache.rs:44-48`): the new under-lock ops are a probe + Arc clone,
an insert, a remove. The prefix copy and tail decode run OUTSIDE the lock where
`build()` runs today. The base is immutable shared state; racing appenders both
append from the same base, insert-if-absent, loser adopts — the documented
accepted-waste rule (`get_or_build.rs:15-17`, `50-storage.md:520-523`). A
commit landing mid-append is handled by the existing newest re-check. Removing
the base key only releases the map's reference — pinned readers keep their
Arcs. Zero new unsafe; the allowlist is not touched (a fresh-Arc memcpy needs
none of it).

## The doc retraction (lands WITH the code)

- `image/cache.rs:37-39` and `docs/architecture/50-storage.md:524-528`:
  "writes are bursty and rare" — RETRACTED in place, with the reason (a
  workload assumption, never a measurement; steady-write hosts are real and
  served by copy-on-append, not by an assumption about write frequency).
- `50-storage.md` § eviction rewritten to the new rule: the writer drops
  entries of relations the commit deleted from; delete-free relations' images
  are retained as append bases; untouched relations carry the same Arc forward.
- `50-storage.md:496-497` ("single-digit milliseconds per 100 MB") — flagged as
  bandwidth arithmetic; the measurement below re-trues or retracts it.
- Prose hygiene where touched: row ids live in F keys AND M/U values and R key
  tails; they still never enter images (`decode.rs:171` discards at the
  boundary).

## Measurement — spec only (Wave M; a bench repin may own the machine)

The "237 ms / 190 MB" transcript number is recorded NOWHERE in the repo — it is
not cited; the baseline is re-established. Family: `cold_containment_walk`
(`bumbledb-bench/src/writebench.rs:174-208`) — the only family whose timed
region contains a post-commit rebuild. No cold-vs-warm digits are asserted
here either: session-run numbers without `scripts/measure.sh` conditions and
a tier are not a record (the landing bar), so the baseline row IS the first
record. Twin: an `ImageCache` bench/test-only knob that disables lineage
(every `advance` behaves as `evict_older_than`) — the
`StridePadder::with_tolerance` falsifier precedent — so A/B lay out interleaved
in ONE process under `scripts/measure.sh` (±2% band; fresh data per rep;
clock-proxy bracketing; tier stated). Expected: `org_touch` is one pure insert,
so every other ledger relation carries forward and Org appends one row — the
cold walk's p50 should collapse toward the warm regime; predicted sign ≥ 5× at
scale S, **measured, never promised**. The no-family-loses check (>2% = the
gravestone lands, not the code): every warm family (the hit path gains zero
instructions), every commit family (`advance` + classification in the epilogue),
ALL-WIN untouched (write families are Report-class).

## Passing criteria

- Column differential green across all field shapes, k-chained commits,
  carry-forward `Arc::ptr_eq`, forced distincts.
- Delete-fallback pin green under `trace` (per-relation arm selection exact).
- Ops fuzz extended (streak bias + post-commit image oracle); existing corpora
  replay green; crash + kill sweeps green UNCHANGED.
- `scripts/check.sh` exit 0 (the append path allocates only the new frame —
  alloc gate green) and `scripts/lean.sh` exit 0.
- Both retraction sites landed; `CRASHPOINTS` prose swept; 50-storage eviction
  bullet rewritten.
- Wave M (owner go): baseline established, interleaved twin, win reported with
  tier, no-family-loses within band; numbers + machine conditions recorded. A
  NEUTRAL/LOSS verdict lands the gravestone, not the code.

## Size

**M overall.** Pieces: delta classification + hook (S) · cache lineage (M) ·
`image::append` + `scan_from` + `fill_columns` generalization (M) · referee
tests (S–M) · fuzz extension (M) · docs (S) · measurement (M, owner-gated).
The split line if it exceeds the bar: the fuzz extension separates (the C2
precedent); measurement rides Wave M regardless. The append path without its
differential referee or its honest doc sentence does not land.
