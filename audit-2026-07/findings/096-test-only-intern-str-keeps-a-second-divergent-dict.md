## Test-only `dict::intern_str` keeps a second, divergent dictionary-write implementation alive

category: unification | severity: low | verdict: CONFIRMED | finder: engine:storage
outcome: fixed db9b934d

### Summary

The interning dictionary module ships two write implementations. The live one is the delta's provisional-id path: `WriteDelta::intern` mints ids against an in-memory copy of the `_meta` next-id counter and the commit flushes them via `dict::put_pending` plus one `put_dict_next_id`. The second is `dict::intern_str`, kept alive under `#[cfg(test)]`: the retired direct-write path that read-modify-writes the counter inside the transaction and performs its own copy of the forward/reverse double-put. All seven unit tests in `dict.rs` drive the fossil, so the module's own test suite pins the mint discipline of code production never executes — the two-implementations drift class the codebase's own doctrine comment forbids ("the sweeper's knowledge is the engine's knowledge, never a second implementation", `crates/bumbledb/src/verify_store.rs:6`).

### Evidence (verified against the code)

- `crates/bumbledb/src/storage/dict.rs:48-72` — `#[cfg(test)] pub fn intern_str`: forward-hit dedup probe, then `let id = txn.dict_next_id()?; txn.put_dict_next_id(id + 1)?;` and two `dict.put` calls (forward at :69, reverse at :70). The comment at :62-64 admits the divergence: "This read-modify-writes the `_meta` counter directly; the 50-storage doc re-homes it into the delta's in-memory-then-flush counter set."
- `crates/bumbledb/src/storage/env/writetxn.rs:42-45` — the enabler `dict_next_id` is likewise `#[cfg(test)]`, its doc comment stating it is "test-only since the delta's pending-intern set re-homed the live path in the 50-storage doc."
- Live path: `crates/bumbledb/src/storage/delta/intern.rs:55-79` (`WriteDelta::intern`, provisional ids, sentinel assert) flushed at `crates/bumbledb/src/storage/commit/write.rs:359-364` via `dict::put_pending` + `put_dict_next_id`. This matches the spec: `docs/architecture/50-storage.md` (the `_meta` paragraph, ~line 68) says "the delta's pending-intern design mints provisional ids against it from read snapshots."
- Literal duplication: the forward+reverse write pair exists twice — `dict.rs:69-70` (fossil) and `dict.rs:113-122` (`put_pending`, production).
- Tests pinned to the fossil: `dict.rs:186-288` — `interning_twice_returns_the_same_id`, `ids_strictly_increase_across_interns`, and `aborted_transaction_leaves_no_dictionary_entries` test the fossil's dedup, counter, and rollback behavior; the reader tests (`resolve_round_trips_interned_values`, `reverse_entries_carry_raw_bytes_with_no_tag`, `resolve_of_fabricated_id_is_corruption`) seed the store through the fossil writer instead of the production one.
- Two further fixture uses outside the module: `crates/bumbledb/src/storage/commit/tests/commit.rs:475` and `crates/bumbledb/src/storage/delta/tests.rs:248` call `dict::intern_str` to seed committed dictionary entries.

Mitigating facts found during verification (these bound the severity at low, they do not refute):

- `forward_key`/`reverse_key` (`dict.rs:27-39`) are shared by both writers, so the byte layout cannot drift — only the mint discipline and the double-put can.
- The live path is independently tested: `commit/tests/commit.rs:447-477` (`pending_interns_flush_at_commit_and_advance_the_counter`) covers flush, dedup, lookup, resolve, and counter advance; `commit.rs:230-259` covers the no-op commit's non-flush; `delta/tests.rs` covers `resolve` semantics.
- One detail of the original finding is wrong: the fossil's counter does NOT survive an abort. `dict.rs:272-288` demonstrates the LMDB transaction abort rolls the `_meta` write back and the next intern re-issues the id — semantically the same outcome as dropping the delta. The two paths' abort semantics agree.

### Failure scenario

Not a runtime bug — the fossil is unreachable in production. The exposure is drift: a change to the flush discipline (e.g., `put_pending` gains a write-order invariant, an extra key, or different dedup handling at flush) leaves `dict.rs`'s green tests asserting the behavior of a path production never runs, and the duplicated double-put must be updated in lockstep by hand or the module's tests silently certify stale semantics. The commit-level tests would likely catch functional regressions, but the module-level suite — the natural first place a maintainer looks — documents the wrong writer.

### Suggested fix

Delete `dict::intern_str` and `WriteTxn::dict_next_id`. Rewrite the dict.rs tests against the production surface: seed via `WriteDelta::intern_str` + `commit` (as `commit/tests/commit.rs:447` already does) or, for the module-local reader tests, via the production `put_pending` + `put_dict_next_id` pair directly. The two external fixture uses (`commit/tests/commit.rs:475`, `delta/tests.rs:248`) convert the same way. The reader API (`lookup`, `lookup_str`, `resolve`, `reverse_ids`, `put_pending`) remains the module's real production surface, and the fossil-only tests (monotonic ids, abort rollback) either move to the delta level or are already covered there.
