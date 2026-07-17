# PRD-C1 — heed flags: NO_MEM_INIT + bulk-lane APPEND

Wave 2 · Repo: bumbledb · depends on: idle machine + owner go · measurement-owned

## Objective

Settle the two remaining measurement-owned engine candidates surfaced in the heed
audit, each a twin under the measurement landing bar: land the win or record the
gravestone. Nothing here is a design change — it is the measured verdict on two
env/put flags the audit flagged as plausible-but-unproven.

## The two candidates

1. **`NO_MEM_INIT` on durable stores.** LMDB's `MDB_NOMEMINIT` skips zeroing write
   buffers before overwrite; documented consistency-safe (the only cost is heap
   garbage in file slack, meaningless for a single-user local store). Site: the
   env open flags in `crates/bumbledb/src/storage/env/open_env.rs` (the sanctioned-
   unsafe storage module) — add it to the durable-kind flag set alongside the
   existing derivation. Honest prior: likely ~nil, because durable commits are
   `F_FULLFSYNC`-dominated and the memset is noise beside the barrier — but the
   `bulk` lane may see it.
2. **`MDB_APPEND` on the bulk-load put path.** The classic LMDB bulk trick:
   append-mode puts skip the page-split search when keys arrive in key order. The
   bulk path today uses plain puts and sets no `PutFlags`. IF the bulk loader can
   feed keys in key order (verify — it may already sort, or `bulk_load_dyn`'s input
   order may be controllable), append-mode is the one place a real write-path win
   plausibly hides. Site: the bulk insert path in `crates/bumbledb/src/storage/`
   (the applier/commit put calls the bulk lane uses).

## Work (per candidate — both are twins under §measurement)

1. Implement the flag/put-flag behind the twin (a small, isolated change).
2. Measure through `scripts/measure.sh` (the machine-wide mutex), interleaved
   same-session A/B, fresh data per rep, ±2% band. `NO_MEM_INIT`: the write
   families (`commit_single/witnessed/batch`, `bulk`) and a durable-read
   spot-check for neutrality. `MDB_APPEND`: the `bulk` family specifically, key-
   ordered vs unordered input. State the tier of every number.
3. **Landing bar:** semantics untouched (conformance + naive parity + lean
   three-way green for anything touching the write path); predicted sign outside
   ±2% at the regime that matters; no other family loses >2%; fmt + clippy -D +
   alloc gate if hot paths moved. A win lands with its numbers in the commit; a
   LOSS/NEUTRAL lands its **gravestone** — the experiment, the numbers, the
   mechanism, recorded (a doc note at the flag site + the commit body).
4. `MDB_APPEND` correctness caveat: append mode ERRORS if a key arrives out of
   order — so it lands ONLY if the bulk lane provably feeds sorted keys (prove it,
   or land a sort, or gravestone). A wrong-order append must never silently corrupt.

## Technical direction

- Storage's one sanctioned-unsafe module is `env/open_env.rs`; flag changes there
  keep the SAFETY comments. No new unsafe elsewhere.
- Durability law is untouched: `NO_MEM_INIT` is consistency-neutral (it does not
  weaken the `F_FULLFSYNC` barrier); do NOT touch `NO_META_SYNC`/`MAP_ASYNC` (those
  are unrepresentable through the durable constructors by design).
- These are independent — either can land or gravestone without the other.

## Passing criteria

- Each candidate has a recorded verdict: WIN (landed, numbers in the commit, no
  regression) or gravestone (numbers + mechanism recorded). No candidate left
  "pending."
- `scripts/check.sh` + `scripts/lean.sh` green on the resulting tree.
- Measurement discipline honored (measure.sh, interleaved A/B, tier stated) — a
  verdict that violated it is void and re-run.
- Commit(s) in the repo's voice; push.
