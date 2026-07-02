# PRD 21 — Vectorized Execution

Authority: `docs/architecture/30-execution.md` (D4 — two-phase batched probing,
branchless compaction, batch as the default path), paper §4.3.

## Purpose

Batch the cover-iterate/probe loop for MLP. This *replaces* the scalar node loop as the
one path (scalar = batch size 1), it does not sit beside it — no mode enum
(post-mortem §31).

## Technical direction

- Rework `exec::run`'s node loop around batches: `iter_batch` (PRD 18) fills
  arena-backed batch buffers — per-batch: cover key words (SoA: one array per key
  word), child NodeRefs, survivor mask/positions.
- **Two-phase probing per sibling subatom**: phase 1 computes all probe hashes from
  binding slots + batch key columns (pure ALU loop, no loads); phase 2 probes all —
  the loads are independent and the OoO window overlaps them. Misses clear survivor
  slots.
- **Branchless compaction** between siblings and after residual evaluation: scalar
  cursor-write compaction here (the `if`-free pattern from PRD 12); the NEON variant
  swaps in via PRD 22 behind the same function signature.
- Recursion happens per surviving batch element (paper: batch within a node, recurse
  per tuple); binding slots are written per element before descent and the undo
  journal is per-element — keep the scalar journal discipline, batched only within
  the node.
- Batch size: a `const BATCH: usize = 128` in one place with a doc comment citing D4's
  model and the OPEN measurement item; buffers sized once per prepared query.
- Equality across batch sizes is the module's contract: parameterize the executor's
  tests over batch ∈ {1, 2, 64, 128, 1024-capped} — identical sink outputs.

## Non-goals

NEON (PRD 22). Cross-node-entry batch accumulation (explicitly future work per D4 —
do not attempt).

## Passing criteria

- All PRD 19/20 tests re-run parameterized by batch size with identical results
  (including skew, empty relations, partial final batches, batch > row count).
- A test Counters assertion that phase-1 hash computation completes for the whole
  batch before any phase-2 probe (instrument via counter ordering).
- No `dyn`, no mode enums; `rg -n "ExecutionMode|Scalar" src` finds nothing
  (post-mortem §31 guard, checked manually not by CI gate).
- Global commands green.
