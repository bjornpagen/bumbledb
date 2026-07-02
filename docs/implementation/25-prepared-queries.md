# PRD 25 — Prepared Queries, Parameters, and Results

Authority: `docs/architecture/20-query-ir.md` (prepared queries, pin-at-prepare,
per-execution literal resolution), `30-execution.md` (arena ownership, `!Sync`),
`60-api.md` (results, params).

## Purpose

The reusable execution object and the result surface — the thing the allocation
contract is written against.

## Technical direction

- `api::prepared`. `prepare(&ReadTxn, &Schema, &Query) -> Result<PreparedQuery>`:
  validate (PRD 14) → normalize (PRD 15) → build filtered-view stats + plan
  (PRD 16/17) → classify (PRD 23) → allocate all execution scratch (arenas, batch
  buffers, binding slots, sink state, counters=Noop) sized from the plan.
  `PreparedQuery` owns its arenas; `!Sync` by construction (interior scratch without
  Sync bounds); statistics pinned — no invalidation hooks exist (decision).
- `execute(&mut self, txn: &ReadTxn, params: &[Value], out: &mut ResultBuffer) ->
  Result<()>`: bind-time param check (count + structural type vs PRD 14's recorded
  types); resolve `PendingIntern` constants via read-only dict lookup (miss ⇒ the
  affected conjunct is unsatisfiable ⇒ empty result — short-circuit); acquire images
  via the cache (PRD 11) / views (PRD 12); run (PRD 21 path or guard probe); finalize
  sinks into `out`.
- `ResultBuffer`: caller-owned, reusable (`clear()` retains capacity — the zero-alloc
  path); columns = find terms in order; values decoded — String/Bytes resolve intern
  ids to bytes copied into the buffer's byte heap (the single sanctioned allocation
  site, growing only within the buffer); rows unordered. A convenience
  `execute_collect` allocates a fresh buffer.
- Arena reset discipline: every execution starts by resetting per-execution scratch
  regions (bump pointer rewind), never freeing; post-warmup executions must not grow
  arenas for same-shaped inputs (PRD 26 verifies mechanically).

## Non-goals

The `Db` facade and transactions API (PRD 28). Allocation *counting* (PRD 26).
Plan re-preparation policies (none exist — decision).

## Passing criteria

- Unit tests: prepare-once/execute-many with varying params gives correct,
  independent results; param count/type mismatches yield bind errors; a String param
  never interned → empty result, and after a commit interning it, the *same* prepared
  query finds rows (per-execution resolution rule); results decode intern ids to the
  original bytes; buffer reuse across executions returns identical results with
  retained capacity (capacity watermark asserted); executing against a newer-
  generation txn uses the new generation's images (pinned *plan*, fresh *data*).
- Global commands green.
