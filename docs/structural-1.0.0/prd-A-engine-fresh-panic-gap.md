# PRD-A — Engine: the fresh-mint panic gap, closed by a drop-guard

Wave 1 · Repo: bumbledb (`crates/`, `lean/`, `docs/`) · depends on: — · standalone green

## Objective

Make the unconditional never-reissue law hold on EVERY termination of a write —
including a **panicking** `Db::write` closure. The mint model
(`lean/Bumbledb/Txn/Fresh.lean`) already proves one `Reachable.txn` transition:
every transaction persists its high-water, its fate irrelevant. The Rust honors
it on `Ok`/`Err`/reject/infra but NOT on panic — `crates/bumbledb/src/api/db/write.rs`
does `let out = match closure { Err => flush }`, and a `panic!` inside the closure
unwinds past that, dropping the delta without flushing, so a caught-and-retried
panicking write recycles a fresh id it already handed out. Close the gap; make the
Rust a true 1:1 with the model.

## Context (verified current state)

- `write_witnessed` in `crates/bumbledb/src/api/db/write.rs`:
  `let closure = f(&mut tx); let WriteTx { view, delta, .. } = tx; drop(view);
  let out = match closure { Ok(out) => out, Err(e) => { let _ = flush_escaped_fresh_ids(&self.env, &delta); return Err(e); } };`
  — the `Err` arm flushes; a panic never reaches it.
- `commit()` (`crates/bumbledb/src/storage/commit/write.rs`) already flushes on its
  own abort exits: `if outcome.is_err() { let _ = flush_escaped_fresh_ids(env, &delta); }`.
- The codebase's unwind-safety idiom is a `Drop` guard: `WriterThreadReset`
  (same file), which resets the writer-thread atomic on drop precisely to survive a
  panicking closure. `flush_escaped_fresh_ids` is `pub(crate)`.
- Model authority: `Fresh.lean :: never_reissue_observable`; the module doc's
  "every transaction persists its final mark ... `alloc` hands the id to the host
  before the commit's fate is known."

## Work

1. **Introduce an escaped-id burn guard** in `db/write.rs`, modeled on
   `WriterThreadReset`: a struct holding the minimal borrows to call
   `flush_escaped_fresh_ids(&env, &delta)`, with a `Drop` impl that flushes
   best-effort. It is armed for the region where the closure runs and the delta is
   alive-but-uncommitted, and DISARMED once the delta is moved into `commit()`
   (which owns the flush for paths that reach it). Use a taken-`Option` /
   `ManuallyDrop` / `Cell<bool>` disarm — whatever is cleanest.
2. **Panic-safe `Drop`.** No `unwrap`/`?`/allocation-that-can-panic in the guard's
   `Drop`; discard the `Result` (`let _ = …`). A double-panic aborts the process —
   the guard must never introduce one.
3. **Exactly one flush per termination.** The invariant: `Ok` (via `commit`'s own
   `flush_counters` inside its txn), `Err`, reject, infra-error, and **panic** each
   flush the escaped marks exactly once — never zero, never two. Verify drop order:
   the burn fires while the writer lock is still held (arrange declaration order so
   the guard drops before the lock guard). Fold the now-redundant explicit `Err`-arm
   flush into the guard; leave `commit()`'s own abort flush as-is (it owns the delta
   once moved in) or route it through the guard if the borrows allow — end state has
   ONE conceptual owner per region and no path flushing twice.
4. **A2 — record the narrowing.** The abort flush is best-effort (`let _ =`), so a
   disk-failure flush silently no-ops: the model claims unconditional persistence,
   the mechanism is unconditional-modulo-I/O-failure. Record this as a sanctioned
   narrowing in `Fresh.lean`'s narrowings section AND `docs/architecture/10-data-model.md`
   § fields — same class as the existing "the dirty-mark flush is mechanism" narrowing.
5. Update the mechanism doc comments (`db/write.rs`, `storage/delta/alloc.rs`) to
   name the guard and state the panic path explicitly. If any doc/lean citation
   names a renamed symbol, sweep it (`scripts/spec-census.sh`).

## Technical direction

- No lean theorem change is required — the model already claims the unconditional
  law; this PRD makes the Rust match it. No `sorry`/`admit`/`axiom` introduced.
- Zero new `unsafe` — the guard is safe Rust (matches `WriterThreadReset`).
- Do not alter `commit()`'s rejection-exit semantics (cited-fact decode still runs
  while pending interns resolve; interns are still NOT flushed on abort — only fresh
  ids burn).

## Passing criteria

- A new unit-shaped `#[test]` (part of this change) proves the panic path burns:
  inside `Db::write`, `alloc` an id, then `panic!`; `std::panic::catch_unwind` it;
  the next `Db::write` mints PAST the burned id. Mirrors the storage-level
  `fresh_ids_allocated_in_a_rejected_txn_are_burned`.
- Existing fresh tests stay green
  (`fresh_ids_allocated_in_a_rejected_txn_are_burned`,
  `a_noop_commit_flushes_escaped_fresh_ids_and_nothing_else`, alloc-advance tests).
- Single-flush proven (a counter/obs assertion that one aborted/panicked write
  advances the `Q` mark exactly once).
- `scripts/check.sh` exit 0 AND `scripts/lean.sh` exit 0 on the committed tree.
- Commit in the repo's voice (`Co-Authored-By: Claude Fable 5 <noreply@anthropic.com>`);
  push. This PRD stands alone green (the engine has zero open semantics after it).
