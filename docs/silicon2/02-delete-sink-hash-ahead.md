# PRD 02 — Delete the sink hash-ahead pipelines: a measured deletion

## Purpose

Exp 15 tested the exact shipped sink shape and returned a verdict the
silicon campaign's own confirm-runs had only half-found: the hash-ahead
ping-pong is a strict loss in its shipped configuration —
**+1.2–2.4 ns/row everywhere** (6.06 vs 5.10 ns/row at the 4 MB point)
— including on the mixed hit/miss dedup paths PRD silicon-04 kept it
for. The reasons compound: the SWAR window probe already removed the
fill-side flush exposure; the remaining mixed-stream exposure is
smaller than the pipeline's ceremony (second scratch row, swaps,
carried hash); and under const-arity (PRD 03) LLVM fuses the gather and
hash itself, which the ping-pong structure blocks. Exp 13 agrees from
the probe layer (hash-ahead's marginal value on the window probe at the
sink's L2-resident sizes ≈ 0 to negative). Deletion is the
optimization.

## Technical direction

`crates/bumbledb/src/exec/sink.rs`, `crates/bumbledb/src/exec/wordmap.rs`.

- **`ProjectionSink::emit_batch`**: remove the pipelined arm — back to
  the direct loop (assemble into `scratch`, `seen.insert(&scratch)` per
  survivor; keep the `stop_on_skip` first-row arm as is). Delete the
  `scratch_next` field, its constructor init, and the double outer
  prefill.
- **`AggregateSink::fold_batch_dedup_constant_group`**: same — direct
  per-row assemble + `seen.insert(&binding_scratch)`, survivors pushed
  on inserted. Delete `binding_scratch_next`.
- **`scan_run`** already lost its pipeline in the campaign (silicon-04
  Result); nothing to do there — verify by grep that no
  `insert_prehashed` call remains in `sink.rs`.
- **Keep the wordmap prehashed API** (`hash_of`,
  `get_or_insert_prehashed`, `insert_prehashed`): it is the seam
  PRD 03's const-arity internals and any future caller dispatch use,
  and its behavior-equivalence test stays. Only the sink-side CALLERS
  die.
- **Doc**: append a superseded-by note to
  `docs/silicon/04-hash-ahead.md`'s Result (the retention decision for
  dedup paths is overturned by exp 15's in-shape measurement; the
  premise-corrected microbench pin
  `hash_ahead_beats_inline_hashing_on_miss_heavy_fills` is deleted with
  the mechanism it pinned — a gate for removed code is noise).

## Passing requirements

1. grep gates: no `scratch_next`/`binding_scratch_next` fields remain;
   no `insert_prehashed`/`get_or_insert_prehashed` callers remain
   outside `wordmap.rs`'s own internals and tests.
2. Measured (vs post-01, min-of-3): stats p50 improves ≥ 2% (exp 15
   prices the ceremony at ~1.3 of stats' ~11 ns/row); range ≤ 28.5
   holds; skew p95 holds or improves; spread holds or improves; no
   family regresses > 5% (confirm-run).
3. Verify green; emits digests unchanged; zero-alloc holds (fields
   removed, nothing added).

## Out of scope

Const-arity (03 — lands next and multiplies this deletion's value);
wordmap-internal probe changes (04); any executor-side two-phase
hash/probe structure (that is the colt design and it stays — exp 01's
law: the executor's phase split IS hash-ahead at batch scale).

## Result

**Shipped**: both sink pipelines deleted — `ProjectionSink::emit_batch`
back to the direct per-row loop (the `stop_on_skip` first-row arm
preserved), `AggregateSink::fold_batch_dedup_constant_group` back to
direct assemble + insert; `scratch_next`/`binding_scratch_next` fields,
inits, and double prefills gone; the premise-corrected microbench pin
deleted with its mechanism. The wordmap prehashed API stays (PRD 03's
seam; `#[allow(dead_code)]` with PRD-10-ledger notes on the two
now-caller-less functions). Superseded note appended to
docs/silicon/04's Result.

**Grep gates**: zero `scratch_next`/`binding_scratch_next`; zero
`insert_prehashed`/`get_or_insert_prehashed` callers outside
`wordmap.rs` (its internals + tests only). ✓

**Measured** (min-of-3 vs post-01, `bench-out/s2p02-{1,2,3}`, verify
stamp `692ab607`):
- **stats 1,887.0 → 1,623.0 µs = −14.0%** — 7× the gate's ≥2%. Exp 15
  priced the visible ceremony at ~1.3 ns of the 11 ns/row floor; the
  deletion evidently also unblocked compiler fusion across the
  assemble+hash+insert chain (its projected mechanism for PRD 03).
- range 27.3 ≤ 28.5 ✓; skew p95 926.4 → 918.9 (p50 38.7 → 34.5) ✓;
  spread 10,843.1 (+0.8%, noise) ✓; triangle 11,771.0 (−0.2%) ✓;
  chain 111.6 → 102.3 (−8.3%) ✓; fk_walk p50 2.8 / p95 888.2 ✓;
  point/string/balance flat. No family regresses > 5%. ✓
- Verify 2,468 green; engine lib tests 299 green; check-asm green;
  zero-alloc holds (fields removed, nothing added).
