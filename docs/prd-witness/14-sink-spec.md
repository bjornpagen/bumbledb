# PRD 14 — SinkSpec: the rewrite returns a narrower type

**Depends on:** baseline (self-contained in the sink modules; independent
of 09–13; if 12 landed, `FindSpec` lives on the rule structs — the
boundary below is identical either way).
**Modules:** `crates/bumbledb/src/exec/sink.rs`,
`exec/sink/aggregate/new.rs` (mechanism name: `rewrite_measures`),
`exec/sink/aggregate/{finalize,fold_batch,groups}.rs`,
`api/prepared/build.rs` (the `FindSpec` construction),
`exec/sink/tests/*`.
**Authority:** the audit's finding 4 (~12 sites): `rewrite_measures`
parses measure finds (`Duration`/`AggDuration` → slot-word forms) IN
PLACE into the same `FindSpec` enum, and every downstream consumer
asserts "rewrite_measures ran" / "the constructor's rewrite ran" — a
two-stage type sharing one enum.
**Representation move:** the constructor is already the parse boundary;
give its output its own type. Pre-rewrite `FindSpec` (what build
produces, measures included) and post-rewrite `SinkSpec` (what the sink
executes, measures already lowered to slot arithmetic) stop sharing a
vocabulary, and the "rewrite ran" asserts become unrepresentable.

## Context (decided shape)

```rust
/// What prepare hands the sink: the head's find terms, measures still
/// symbolic. Constructed in build.rs; consumed exactly once, by the
/// sink constructors. (Today's FindSpec, unchanged.)
enum FindSpec { /* as today, Duration/AggDuration included */ }

/// What the sink executes: every measure lowered to its slot-word form.
/// Minted only by the sink constructors' parse of FindSpec; the
/// Duration/AggDuration variants DO NOT EXIST here.
enum SinkSpec {
    Var { slot: … },                    // scalar find: one slot word
    Interval { start_slot: … },         // two-word find
    FixedBytes { slot, len },           // multi-word find
    Str { slot },                       // resolving find
    Agg { op: AggKind, slot: … },       // fold input, measure-free
    // …one variant per post-rewrite shape the sink actually executes —
    // enumerate from today's post-rewrite arms; the measure lowering
    // (interval slots → end−start subtraction, ray check) becomes data
    // on the Agg/Var variant it feeds (e.g. `source: SlotSource::Width
    // { start_slot }` vs `SlotSource::Word { slot }`), so finalize and
    // fold_batch match structure, not history.
}
```

`rewrite_measures` becomes `fn parse_finds(&[FindSpec]) -> Vec<SinkSpec>`
(or per-spec `impl From` where total), living where the rewrite lives
today; the aggregate AND projection sink constructors both consume it
(projection finds also carry the two-stage split today — verify; if
projection never sees measures, its parse is the identity refactor and
the type split still deletes its "resolving path" asserts only where
they guard rewrite state, not where they guard byte-heap routing — the
byte-heap routing asserts in `word_cell`/`push_word` are recorded
derivation-refusals and STAY).

Dying asserts (audit's list, reconcile at execution): "rewrite_measures
ran" ×6 (`finalize.rs`, `sink.rs`, `fold_batch.rs`), "the constructor's
rewrite ran" ×4, `exec/sink.rs` ×2.

## Technical direction

1. Read `rewrite_measures` and every match on `FindSpec` downstream of
   sink construction; write the exact `SinkSpec` variant set from what
   those matches DO (the sketch above is directional).
2. Land `SinkSpec`; constructors parse; `finalize`, `fold_batch`,
   `groups`, and the projection emit path match `SinkSpec` totally.
   `MeasureOfRay` keeps firing where it fires today (execution-time, on
   the slot values) — the error's TIMING is a recorded behavior
   (PRD-07-era ruling: folding measures would move it); assert unchanged
   by the existing measure tests.
3. `build.rs` keeps producing `FindSpec` — zero changes upstream of the
   sink boundary.
4. Tests: sink/aggregate suites re-anchor mechanically; add the totality
   pin — constructing a sink from every `FindSpec` shape yields the
   expected `SinkSpec` (a table test, one row per variant), so a future
   find kind extends the parse or fails here, never grows an assert.

## Passing criteria

- `[shape]` `grep -rn "rewrite_measures ran\|constructor's rewrite"
  crates` → zero hits; `SinkSpec` has no `Duration`/`AggDuration` variant
  and no post-parse match mentions either symbolic variant. The parse
  site, ray-error path, and required parser table tests may name them.
- `[test]` The FindSpec→SinkSpec table test; every existing
  measure/aggregate/pack test green with unchanged value and
  `MeasureOfRay`-timing assertions.
- `[shape]` The byte-heap routing refusals in
  `result_buffer.rs`/`word_cell` are untouched (they are recorded
  derivations, not rewrite-state guards).
- `[gate]` Workspace gates green at campaign close.

## Doc amendments (rule 5)

`20-query-ir.md` § the measure (or 40-execution's sink paragraph —
whichever states the lowering): one sentence — the measure lowers at sink
construction into a measure-free execution vocabulary; the sink never
re-checks that the lowering happened.
