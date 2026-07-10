# PRD 03 — Virtual images: the theory as storage

**Depends on:** 01 (sealed extensions, pre-encoded), 02 (a theory to test with).
**Modules:** `crates/bumbledb/src/image/` (cache, build), `storage/read/`
(scan/dyn surfaces), `api/prepared/run_join.rs` (view bind), `exec/dispatch`
(guard-probe classification).
**Authority:** `50-storage.md`, `40-execution.md` (Deviation D1, the image
cache), the README boundary clause.
**Representation move:** the store contains zero vocabulary bytes. A closed
relation's image is synthesized from the sealed extension — the fingerprint's
preimage IS the storage — so vocabulary can never desync, never bloat, never
need the sweeper. Its "generation" is the theory itself.

## Context (decided shape)

- **Synthesis, not build**: a closed relation's `RelationImage` is
  constructed from `Relation.extension` (values pre-encoded at validate, PRD
  01) — columns laid out by the existing `FactLayout`/span rules (interval =
  two word columns), 128-byte-aligned SoA like any image, distinct counters
  exact. The synthesized image contains the implicit `id` column
  (0..rows) first.
- **Cache behavior**: keyed outside the generation map — a separate
  `closed: Box<[OnceLock<Arc<RelationImage>>]>` (indexed by closed-relation
  ordinal) on the cache; `get_or_build` on a closed `RelationId` returns the
  synthesized Arc, building on first touch, **never evicted, never rebuilt**
  (`evict_older_than` skips it by construction — it is not in the
  generation-keyed map at all).
- **View memo interaction** (`run_join`): a view over a closed relation keys
  its memo on `(GENERATION_CLOSED, filters)` where `GENERATION_CLOSED` is a
  sentinel generation that never advances — warm forever; the stale-reaping
  path never touches it (reaping compares generations; the sentinel is
  maximal). Weaker-model note: do NOT thread `Option<Generation>` through the
  memo — use the sentinel constant so every existing comparison keeps
  compiling and meaning the right thing.
- **Read surfaces**: `Snapshot::scan`/`scan_facts` over a closed relation
  iterate the synthesized image (they currently cursor `F` — add the branch
  at the entry, yielding decoded rows from the image); typed/`get_dyn` point
  reads by key resolve against the extension (a linear ≤256 scan or the
  guard... there are no `U` guards for closed relations — the auto-key is
  enforced by validation's duplicate-handle check, so point reads scan the
  extension: ≤256 rows, L1-resident, O(rows) is honest and tiny).
- **Storage refusal hardening**: `keys::`-level debug assertion + the
  `verify_store` check from PRD 01 — no F/M/U/R namespace entry may name a
  closed `RelationId`.
- **Guard-probe classification** (`exec::dispatch::classify`): a single-atom
  query fully binding a closed relation's key does NOT take the guard-probe
  fast path (there are no guards); it classifies as Free Join and hits the
  virtual image — which PRD 07 then folds anyway.

## Technical direction

1. `image/build.rs`: `synthesize_closed(relation) -> RelationImage` — walk
   the sealed rows, write the id column then each declared column's words
   (interval = two), reusing the decode-plan column layout so downstream
   (`ColumnView`, distinct counters, pitch padding) is untouched. No LMDB
   txn parameter — synthesis is pure.
2. `image/cache.rs`: the `closed` OnceLock array sized at cache construction
   from the schema; `get_or_build` branches on `relation.is_closed()` before
   touching the generation map; `byte_size` accounting includes them once.
3. `storage/read/scan.rs` + `api/db/get.rs`: the closed branches (image
   iteration; extension scan for point reads) with the same typed error
   surface as ordinary relations for unknown ids.
4. `run_join.rs`: the sentinel-generation constant
   (`Generation::CLOSED = Generation(u64::MAX)`) and the one-line branch
   selecting it; confirm `view_memo::bind`'s reaping comparison treats it as
   never-stale (it does, by maximality — write the test anyway).
5. Tests live beside each module; fixtures use a PRD-02 theory with all
   three tiers.

## Passing criteria

- `[test]` Synthesis: a closed relation's image has `rows == extension len`,
  id column = 0..n, every declared column's words equal to the canonical
  encodings from validate (compare against `encoding::encode` directly).
- `[test]` Cache: two `get_or_build` calls return the same Arc; a
  state-changing commit + `evict_older_than` leaves it untouched; the view
  memo stays warm across generations (bind → commit → bind: zero rebuilds,
  asserted via the obs counters or the memo's slot state).
- `[test]` `scan` over a closed relation yields exactly the extension;
  `get_dyn` by id returns the row; unknown id is the existing typed error.
- `[test]` `verify_store` fixture: a hand-planted `F` entry under a closed
  `RelationId` is reported as corruption.
- `[shape]` No LMDB write path can reach a closed relation (PRD 01's refusal
  + grep: `synthesize_closed` takes no txn).
- `[gate]` Workspace gates green at campaign close.

## Doc amendments (rule 5)

`50-storage.md`: the virtual-relations section ("the fingerprint's preimage
is the storage"); the namespace table notes closed relations' absence.
`40-execution.md`: D1 gains the closed carve-out; the view-memo section
gains the sentinel generation.
