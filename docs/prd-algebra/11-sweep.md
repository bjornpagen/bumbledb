# PRD 11 — The sweep: one walk, two callers

**Depends on:** nothing (pure refactor; prepares 12).
**Modules:** new `crates/bumbledb/src/interval/sweep.rs` (or sibling home),
`storage/commit/judgment.rs` (caller one), PRD 12's sink (caller two).
**Authority:** `30-dependencies.md` (coverage enforcement), `50-storage.md`.
**Representation move:** the anti-probe precedent, replayed. "No fact matches"
is one primitive with two owners (negation, the checker); "walk start-ordered
disjoint segments, tracking the covered frontier" is about to have two owners
(the coverage judgment, `Pack`'s finalize) — so it becomes one primitive
*before* the second owner exists, not after the copy drifts.

## Context (decided shape)

Extract the segment-walk core from the coverage judgment
(`judgment.rs` — entry-segment location + forward chain + gap detection) into
one module with a caller-neutral shape:

- Input: an ordered iterator of `(start, end)` word pairs (from LMDB guard
  cursors for the checker; from a sorted per-group slice for `Pack`), plus a
  target window `[s, e)` (the checker) or none (`Pack` emits everything).
- Core state: the covered frontier; the two outcomes: *gap at* (checker) /
  *emit maximal segment* (`Pack`). One loop, two continuation shapes —
  monomorphized via a small trait, no `dyn`, matching the sink/counter
  discipline.
- Adjacency law in one place: `end == next.start` continues the segment
  (half-open, PRD 02's denotation); overlap of inputs is legal for `Pack`
  (arbitrary claim sets) and impossible-by-key for the checker — the core
  handles both because max-frontier tracking subsumes disjoint chaining.
- The checker's behavior is bit-identical after the extraction — this PRD is
  strictly behavior-preserving for the commit path (the elegance-pass
  constraint applied locally: no semantics change, no error-shape change).

## Technical direction

1. Extract; parameterize by continuation; keep the checker's corruption checks
   at its call site (trust boundaries stay where the data enters).
2. Property tests on the core alone: random segment sets vs a naive point-set
   reference (coverage verdicts and packed output both).

## Passing criteria

- `[test]` Judgment suite unchanged and green (bit-identical verdicts on the
  existing fixtures, ray cases from PRD 02 included).
- `[test]` Core property test: packed output equals naive point-set
  union-then-maximal-segments; coverage verdict equals naive subset check —
  randomized, adjacency and overlap boundaries included.
- `[shape]` `judgment.rs` contains no inline frontier loop; exactly one sweep
  implementation exists in the crate.
- `[gate]` Workspace gates green.

## Doc amendments (rule 5)

`30-dependencies.md` enforcement summary: names the shared primitive (the
anti-probe sentence gains a sibling). `50-storage.md`: the coverage-walk
paragraph points at the module.
