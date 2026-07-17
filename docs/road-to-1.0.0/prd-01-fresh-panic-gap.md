# PRD-01 — Engine: the fresh-mint panic gap, closed by a drop-guard

Repo: bumbledb · depends on: — · blocks: nothing (independent; clears the last
open semantic)

## Objective

Make the never-reissue law hold on EVERY termination of a write, including a
**panicking** `Db::write` closure. Today the mint model
(`lean/Bumbledb/Txn/Fresh.lean`) proves a single unconditional `Reachable.txn`
transition — every transaction persists its high-water, its fate irrelevant — but
the Rust honors it only on the paths that reach the `match` in `Db::write` or the
abort exits in `commit()`. A panic unwinds past those, so a caught-and-retried
panicking closure recycles the fresh ids `alloc` already handed out. Close the
gap so the Rust is a true 1:1 with the model.

## Context

- `crates/bumbledb/src/api/db/write.rs :: write_witnessed` currently reads
  `let closure = f(&mut tx);` then, after destructuring `tx`, flushes escaped
  fresh ids only in the `Err` arm of `match closure`. A `panic!` inside `f`
  unwinds the whole frame — `tx`/`delta` drop, the `match` is never reached, no
  flush occurs.
- `commit()` (`storage/commit/write.rs`) already flushes on its own abort exits
  (reject/infra) via `if outcome.is_err() { let _ = flush_escaped_fresh_ids(...) }`.
- The codebase's established unwind-safety idiom is a `Drop` guard:
  `WriterThreadReset` (same function, `db/write.rs`) resets the writer-thread
  atomic on drop precisely to survive a panicking closure.
- The model's authority: `never_reissue_observable` and the module doc's "every
  transaction persists its final mark ... because `alloc` hands the id to the host
  before the commit's fate is known."

## Work

1. **Introduce an escaped-id burn guard** in `db/write.rs`, modeled on
   `WriterThreadReset`: a struct holding `&Environment` and `&WriteDelta` (or the
   minimal borrows needed to call `flush_escaped_fresh_ids`), with a `Drop` impl
   that flushes the delta's escaped fresh marks best-effort (`let _ = …`; a flush
   failure must never turn into a panic-in-drop). The guard is armed for the whole
   region in which the closure runs and the delta is alive-but-uncommitted.
2. **Disarm it on the paths that persist the marks themselves**: a successful
   state-changing `commit()` flushes inside its own txn (`flush_counters`), so the
   guard must not double-flush there. Provide an explicit `disarm()` (e.g. a
   `Cell<bool>`/`ManuallyDrop` or a taken-`Option`) called once the delta has been
   handed to `commit()` — or structure so the guard only covers the
   pre-`commit` window and `commit()` remains the sole owner of the flush for
   paths that reach it. Either shape is acceptable; the invariant is: **exactly
   one flush of the escaped marks on every termination — success, `Err`, reject,
   infra-error, and panic — never zero, never two.**
3. **Collapse the now-redundant explicit flush sites** into the guard where doing
   so is cleaner: the `Err`-arm flush in `write_witnessed` folds into the guard
   (the guard fires on `Err`-drop too). The `commit()` abort flush MAY remain
   (commit owns the delta once it is moved in) OR also route through the guard if
   the borrow structure allows — pick the spelling that leaves ONE conceptual
   owner per region and document it. The end state has no path that reaches
   `flush_escaped_fresh_ids` twice for one delta.
4. **Update the doc comments** that describe the mechanism to name the guard and
   state the panic path explicitly (`db/write.rs`, and the `alloc.rs` doc if it
   narrows the set of covered paths). No lean change is required — the model
   already claims the unconditional law; this PRD makes the Rust match it. If any
   doc/lean citation names a symbol you rename, sweep it (`scripts/spec-census.sh`).

## Technical direction

- The guard's `Drop` MUST be panic-safe: no `unwrap`, no `?`, no allocation that
  could panic; `flush_escaped_fresh_ids` returns `Result` — discard it with
  `let _ =`. A double-panic (panic during unwind) aborts the process; the guard
  must never introduce one.
- `flush_escaped_fresh_ids` opens its own counters-only write txn under the
  held writer lock; on the panic path the writer lock is still held (its
  `WriterThreadReset`/`MutexGuard` drop after this guard by declaration order, or
  arrange order so the flush runs while the lock is held). Verify drop order:
  the burn guard must drop (flush) BEFORE the writer lock is released.
- Do not change `commit()`'s rejection-exit semantics (the cited-fact decode still
  runs while pending interns resolve; interns still are NOT flushed on abort).
- Zero new `unsafe`. The guard is safe Rust (matches `WriterThreadReset`).

## Passing criteria

- A new `#[test]` (unit-shaped, part of this change — not a smoke/e2e PRD) proves
  the panic path burns: inside `Db::write`, `alloc` an id, then panic; catch the
  panic with `std::panic::catch_unwind`; the next `Db::write` mints PAST the burned
  id. This mirrors the existing `fresh_ids_allocated_in_a_rejected_txn_are_burned`
  storage test at the public-API layer.
- The existing fresh tests stay green:
  `fresh_ids_allocated_in_a_rejected_txn_are_burned`,
  `a_noop_commit_flushes_escaped_fresh_ids_and_nothing_else`, and the alloc-advance
  tests.
- No double-flush: assert (via the counters or an obs count) that a single
  aborted/panicked write advances the `Q` mark exactly once.
- `scripts/check.sh` exit 0 and `scripts/lean.sh` exit 0 on the committed tree
  (fmt, clippy -D, workspace tests, alloc gate, crashpoint + kill sweeps, feature
  matrices; lean build + zero-sorry + census + conformance + three-way).
- Commit in the repo's voice; push (the engine has zero open semantics after this).
