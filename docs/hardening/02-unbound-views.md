# PRD 02 — Unbound views: prepare pins nothing

Findings fixed (docs/audit/): **sink-pipeline MEDIUM** and **concurrency-crash
MEDIUM** (the same finding, both reports): "Parked placeholder COLTs pin
prepare-generation images for the prepared query's lifetime."

## Purpose

`build_view_memo` fills one active + three parked COLTs per occurrence with
`View::All(Arc<RelationImage@prepare>)` placeholders. For selection-only
occurrences (the common ledger shape) the parked Arcs are **never touched
again** — full-width column slabs pinned for the prepared query's lifetime,
across every commit and eviction, breaching 40-storage's steady-state-heap
claim and invisible to `cache_resident()`. The elegant fix is not to drop the
Arcs earlier — it is for prepare to never hold an image at all: a view that
has not been bound to a generation should be *unrepresentable as data*.

## Technical direction

- **`View::Unbound`.** `image/view.rs`: a third variant carrying nothing.
  `len() == 0`; `image()` becomes unreachable from it (panic with a named
  programmer-invariant message — no code path may ask an unbound view for its
  image; audit the two callers of `image()` and confirm both sit behind a
  bound view by construction). `position_at` unreachable likewise.
- **Prepare stops building images.** `api/prepared.rs::build_view_memo`
  (`:455-469`): delete the `cache.get_or_build` call entirely; every COLT —
  active and parked — is constructed over `View::Unbound`. The `BUILD_COLTS`
  phase becomes pure column-schema construction (faster prepare; the span
  stays, its meaning documented). The stats phase's `peek` is already
  build-free (docs/perf); prepare now provably never touches an image. First
  execution rebuilds via the existing memo-miss path (generation `None` never
  matches), paying the image build exactly where a cold execution already
  pays it. Note the doc consequence: 30-execution's prepare-pipeline
  description ("build_colts … against the current images") is amended here,
  in the same change.
- **Parked slots become `Option<ParkedView>`.** Empty at prepare (three `None`
  slots — no placeholder Colts at all, deleting the ballast the audit named).
  `ViewMemo::bind`:
  - Parked-hit search over `Some` entries only.
  - Parking (active is current-generation, misses): prefer an empty (`None`)
    slot; else the stale-first/LRU victim as today. First park of a slot
    constructs its `ParkedView` in place (allocation inside the view-rebuild
    window — already the sanctioned class; steady-state rotation still swaps
    without allocating).
  - **Stale reaping:** at the top of `bind`, drop (`= None`) every parked
    entry whose generation is below the requested one — the audit's own
    proof: "their unhittability is already proven." This frees the Colt pools
    *and* the image Arcs the moment a new generation is first executed,
    restoring 40-storage's story: old images die with the last pinned reader
    or the first post-commit execution, whichever is later.
- **The active placeholder** is covered by the same change: `View::Unbound`
  holds no Arc, so a prepared-but-never-executed query pins nothing.
- Zero-warm-alloc audit: probing/hit/swap paths allocate nothing (unchanged);
  reaping deallocates only on generation change (the sanctioned rebuild
  window — but note the alloc gate asserts zero *deallocations* too, within a
  generation: reaping fires only when the generation moved, which the gate
  never does mid-window; state this in the code comment).

## Non-goals

Evicting the *active* binding's image mid-generation (it is live state);
changing the memo's key or LRU semantics (docs/perf/03's protocol tests must
pass verbatim); cache-level eviction policy.

## Passing criteria

- **The pinning is gone, asserted by refcount:** prepare a selection-only
  query against a store, grab `Arc::strong_count`-observable state via the
  cache (or: build the image, note `cache_resident()`, prepare, drop the
  cache's copy via a commit + eviction, assert the image's weak/strong count
  through a held `Arc` in the test — concretely: test holds one `Arc` to the
  generation-T image; after prepare + one commit + one execution at T+1,
  `Arc::strong_count(&held) == 1` — the prepared query holds nothing).
- Prepare builds no images: trace-lane test — `prepare` on a cold cache emits
  **zero** `image_build`/`cache_hit` events; the first execution emits them.
- Reaping: execute at T (binding built), commit, execute at T+1 → a test-only
  accessor (or the refcount pattern above) shows no parked entry below T+1
  survives.
- Every docs/perf/03 LRU protocol test passes verbatim (four windows build
  once then hit; fifth evicts LRU; generation bump invalidates) — the reaping
  must not disturb same-generation rotation.
- The alloc gate passes verbatim in release (rotation paths untouched).
- 30-execution and 40-storage amendments landed. `scripts/check.sh` green.
