# PRD 18 — COLT

Authority: `docs/architecture/30-execution.md` (COLT adoption + the chunked-chain
Deviation), paper §4.2 (read `04-optimizations.tex` first).

## Purpose

The Column-Oriented Lazy Trie over images/views: the executor's data structure.

## Technical direction

- `exec::colt`, arena-backed (PRD 06's bump arena), **no `unsafe`** — index-based
  arena references (`NodeRef(u32)`), never pointers. This module's design replaces
  v5's UnsafeCell aliasing UB (post-mortem §36) representationally.
- `ColtNode` = `Unforced { positions }` | `Forced { map }` where `positions` is either
  the root view (all/survivors, borrowed) or a chunked child list; `map` is **open
  addressing, power-of-two capacity, inline u64-word keys** (multi-var keys: fixed
  small arity — keys are 1..=3 words for sane plans; store inline `[u64; K]` via const
  generics or an enum by arity — pick const-generic arity monomorphized from the trie
  schema), linear probing, no tombstones (build-once, never deleted from).
- **Single-pass force** (the Deviation): iterate positions, decode key words from
  image columns, insert-or-get map slot, append position to that key's **chunked child
  list** (chunks of 64 positions arena-allocated, chained by NodeRef; count in the
  slot). Singleton optimization: first position stored inline in the slot; chunk
  allocated on second.
- `get(key) -> Option<NodeRef>` (forces if unforced); `iter()` — over map keys if
  forced, else over positions **only when the node's remaining schema is a suffix**
  (paper rule: suffix-iterate without forcing; otherwise force then iterate);
  `iter_batch(n)` filling caller slices (PRD 21's входной point); `key_count() ->
  Exact(u64) | Estimate(u64)` (forced map len vs position count — labeled, never
  conflated: post-mortem §40).
- Iteration over a forced map yields (key words, child NodeRef) pairs — **the child
  comes with the key; no re-probe of the map just enumerated** (post-mortem §34).

## Non-goals

The recursion (PRD 19). Vectorized probing (PRD 21). Any eager build.

## Passing criteria

- Unit tests: laziness — constructing a COLT over a 10⁶-position view allocates O(1)
  until first `get` (assert via arena watermark); force builds exactly one level;
  suffix iteration never forces (arena watermark unchanged); get/iter agree with a
  naive HashMap oracle built in the test over random data incl. duplicate-heavy keys;
  chunked lists round-trip >64-duplicate keys; singleton keys allocate no chunk;
  key_count labels correct in both states; iteration yields children without map
  probes (API shape makes re-probing inexpressible — assert by API review comment).
- Global commands green; module remains `unsafe_code = deny`.
