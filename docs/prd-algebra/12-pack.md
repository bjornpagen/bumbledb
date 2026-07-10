# PRD 12 — `Pack`: the coalescing fold

**Depends on:** 11 (the sweep), 05 (head shape), 02 (rays).
**Modules:** `crates/bumbledb/src/ir.rs` (`AggOp::Pack`), `ir/validate/`,
`exec/sink/aggregate/` (the pack group state + finalize), `api/` (result
shape).
**Authority:** `20-query-ir.md` (aggregates), `40-execution.md` (sinks),
architecture README (the OPEN item this discharges).
**Representation move:** the OPEN item's blocker dissolves under set
semantics. `Pack` stalled on "result shape (a set per group) unresolved" — but
a set-semantic query's result *is already a set of rows*, so the answer is that
**`Pack` is relation-shaped**: one result row per (group, maximal segment).
One-row-per-group was a convention, not a law; `ArgMax`'s tie sets were the
precedent. The aggregate that returns a relation is the representation the
temporal algebra was waiting for.

## Context (decided shape)

- `AggOp::Pack` over an interval-typed variable: per group, the result is the
  set of **maximal disjoint half-open segments** of the union of the group's
  interval point-sets (Snodgrass coalesce). Head shape: the group variables
  plus one interval-typed result position per `Pack` term (validation: at most
  one `Pack` per head — the multi-`Pack` product has no sighting and is
  refused with a trigger).
- Adjacency merges (`end == next.start`) — PRD 11's law. Rays pack correctly
  (a ray absorbs everything after its start; the packed ray is a ray — no
  measure is taken, so no `MeasureOfRay` interaction).
- Composition is the payoff: `Pack` output rows are ordinary interval values —
  a host feeds them to a second prepared query with `Allen` masks, or takes
  `Duration` of packed segments in the *same* query? **No** — aggregates of
  aggregates are refused (no nesting; standing aggregate law). Coalesced-time
  accounting (`Sum∘Duration∘Pack`) is two prepared queries or a host fold over
  packed rows; recorded with the trigger "a measured two-pass budget
  violation."
- Free time (`Gaps`) stays a host walk over sorted packed output — refused
  operator, README ledger.
- Fold mechanics: the group map holds an interval accumulation list
  (arena-chunked, the arg-restriction precedent); finalize sorts each group's
  list by start word and runs the sweep's emit continuation. Memory is
  O(group's claims), like every group-state sink; the allocation contract
  covers it as retained high-water scratch.
- Set-semantic dedup: binding dedup upstream is unchanged; identical intervals
  in one group collapse in the sweep for free.

## Technical direction

1. IR + validation (interval-typed input; head typing; the one-`Pack` rule).
2. Sink: `PackState` per group (chunked list + count); finalize = sort (words;
   the existing sort machinery or a pooled radix — measured choice, recorded)
   + sweep emit into head rows.
3. Naive model implements `Pack` independently from the point-set definition
   (union of point sets → maximal segments) — the oracle for everything here,
   since SQLite cannot express it (`60-validation.md` already assigns the
   naive model exactly this role).

## Passing criteria

- `[test]` Differential vs the naive model: randomized claim sets per group —
  overlapping, adjacent, nested, duplicate, ray-bearing; both element types.
- `[test]` Golden: the calendar shape — per-person busy claims in, coalesced
  busy out; a hand-checked fixture with adjacency and triple-overlap.
- `[test]` Group interaction: `Pack` groups by the non-aggregated head vars
  exactly as `Sum` does (shared group-map tests extended).
- `[shape]` The OPEN item is deleted from the architecture README (discharged,
  not amended); `Pack`'s result shape is stated in `20-query-ir.md`.
- `[gate]` Workspace gates green; alloc gate covers a warm `Pack` execution.

## Doc amendments (rule 5)

`20-query-ir.md`: `Pack` semantics, head shape, refusals (multi-`Pack`,
nesting, `Gaps`). `40-execution.md`: the sink. README: OPEN item discharged.
