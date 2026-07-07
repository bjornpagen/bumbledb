# PRD 07 — Sink lanes and the SLP audit: keep hot state renameable

## Purpose

bumblebench exp 12: the per-item toll of call-shaped sinks is not the
call — it is state shape. A renamed scalar store-to-load round trip is
exactly 1.00 cycle; if LLVM's SLP vectorizer merges adjacent accumulator
fields into a q register, the round trip becomes ~12 cycles and
un-renameable; a `bl/ret` boundary disables renaming even for scalar
state (2.1 → 7.5 cycles on identical chains). And splitting one hot
accumulator into K independent lanes overlaps the forwarding chains at
12/K cycles (floor 3.3 at K=4). bumbledb's batch-emit design already
avoids the worst case (SLP flips sign under batching: +5.3×), but the
per-item paths that remain — `Sink::emit`, per-survivor accumulator
updates, guard-lane pushes — must be audited to the machine code and
pinned there.

## Technical direction

`crates/bumbledb/src/exec/sink.rs` (AggregateSink accumulators,
ProjectionSink push paths), `crates/bumbledb/src/exec/run.rs` (leaf
per-item paths), `scripts/check-asm.sh`.

- **Audit first, to the disassembly.** objdump every per-item hot symbol
  (`emit`, the pinned-row leaf path, guard-lane cell pushes, per-survivor
  accumulate in non-batch arms): flag any loop-carried
  `str q … ldr q` round trip on sink/accumulator state, and any `bl`
  reachable per item. Record the audit table (symbol → verdict) in
  `## Result`.
- **Break SLP merges on per-item state.** Where fields are being merged
  (e.g. adjacent `sum: u64, count: u64` updated together): the durable
  fix is to keep the loop-carried state in scalar locals inside the batch
  loop and write the struct back once per batch — this is both the
  renaming fix and better code. Where a per-item struct write genuinely
  must remain, separate the fields (intervening field, or update through
  distinct locals) so SLP cannot pair them — and pin the shape with the
  asm gate, since attribute-level tricks (`#[repr]`, `black_box`) are not
  guaranteed across rustc versions.
- **K-lane accumulators where a single hot chain binds.** The
  constant-group fold arm (single group, one accumulator, every row hits
  it) is the 12/K case: fold into 4 independent lane accumulators merged
  at batch end. The PRD-06 NEON kernels already do this internally for
  dense runs; this item covers the non-dense arms that stay scalar
  (`fold_batch_constant_group` over survivors, dedup-fed folds).
- **Batched-callee toll is fine — leave it.** ~23 cycles per batch call
  is 0.16 ns/item at batch 128; do NOT chase it. The enum-dispatch
  (`EitherSink`) compiles to `csel`-class dispatch (+0.13 ns even
  mispredicted) — confirmed cheap, leave it. Record both as
  measured-and-accepted in `## Result` so nobody "optimizes" them later.

## Passing requirements

1. asm gate (`check-asm.sh` extended): per-item hot symbols carry no
   loop-carried q-register store-to-load on accumulator state and no
   per-item `bl`; the audit table is committed in `## Result`.
2. Measured (vs post-06, min-of-5): balance p95 holds; stats p50 −3%
   further or documented; spread p50 −3% or documented; skew finalize
   path unchanged (finalize is out of scope — guard it with the no-regress
   rule).
3. No family regresses >5% (confirm-run); verify green; emits digests
   byte-identical.

## Out of scope

Finalize/ResultBuffer (already batched in perf-PRD 08); making `emit`
batch-only (the batch path exists; per-item `emit` remains for
correctness paths); any `dyn` — stays banned, and this PRD documents why
(unpredictable `blr` ~28 cycles/miss).
