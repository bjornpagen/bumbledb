## NaiveDb::apply stages every accepted delta twice — two full-database deep clones per write op

category: perf | severity: low | verdict: CONFIRMED | finder: r2:differential-apparatus-soundness
outcome: fixed 6353e6fc

### Summary

`NaiveDb::apply` derives the candidate final state twice for every accepted write. It first calls `self.violations(delta)`, which internally calls `self.staged(delta)` — cloning the entire model database — judges it, and discards the staged state. On the accept path, `apply` then calls `self.staged(delta)` a second time to rebuild the exact same object. Since `staged` is pure and deterministic, the second derivation is byte-identical redundant work: two full O(total-facts) deep clones (including per-value heap allocations for strings and fixed-bytes) where one suffices. The module's own doc comment (naive.rs:252-253) declares the "one derivation" doctrine for the violation set; the staging computation itself breaks it.

### Evidence

All lines verified in `/Users/bjorn/Documents/bumbledb`:

- `crates/bumbledb-bench/src/naive.rs:255-269` — `apply` calls `self.violations(delta)` (line 256), then on the accept path calls `self.staged(delta)` again (line 260) and assigns `self.relations = next`.
- `crates/bumbledb-bench/src/naive.rs:287-297` — `violations` calls `self.staged(delta)` (line 295) and judges the result; the staged `next` is dropped on return.
- `crates/bumbledb-bench/src/naive.rs:303-317` — `staged` begins `let mut next = self.relations.clone();` (line 304), where `relations: Vec<BTreeSet<Tuple>>` (line 62).
- `crates/bumbledb-bench/src/naive/tuple.rs:14` — `pub struct Tuple(pub Vec<Value>);` and `crates/bumbledb-theory/src/value.rs:20` — `Value` carries `String(Box<[u8]>)` / `FixedBytes(Box<[u8]>)`, so the clone is a true deep copy with heap allocation per string/bytes value, not a cheap refcount bump.

Write-heavy call sites paying the double clone on their dominant (commit) path:

- `crates/bumbledb-bench/src/differential.rs:108-124` — the differential runner calls `naive.apply(delta)` per `Op::Write`.
- `crates/bumbledb-bench/src/differential/tests.rs:1,160,647` — the fixed 200-op random stream.
- `crates/bumbledb-bench/src/verify/run_naive.rs:55-77,238` — `load_ops` replays the whole corpus as 32-fact insert deltas, all of which commit (so all hit the double-staging path).
- `crates/bumbledb-bench/src/verify/run_calendar.rs:98,271` — same chunked-load pattern for the calendar slice.
- `crates/bumbledb-bench/src/lawful/tests.rs:170` — lawful parity seeding via `apply`.

Doctrine cross-check: `docs/architecture/30-dependencies.md`'s "judged on final states" phase structure (cited in the `violations` doc comment, naive.rs:276-281) constrains WHAT is judged, not how many times the final state is materialized — the fix does not touch judgment semantics. The naive model is deliberately simple in its judgments; duplicated staging is not judgment simplicity, it is the same pure computation run twice.

### Bench impact

Not a correctness issue. Every accepted write in every differential/verify lane deep-clones the whole model database twice instead of once — ~2x the clone work on the load phase of the verify naive slice, the verify calendar slice, the 200-op differential stream, and lawful seeding. Corpus loads grow the database as they replay, so the redundant clone is O(facts-so-far) per 32-fact chunk — quadratic total, doubled. Rejected writes stage only once (the `violations` early return at naive.rs:257-259 skips the second staging), and no timed perf lane uses the naive model, hence severity low.

### Suggested fix

Stage once. A private `fn judged(&self, delta: &Delta) -> (Vec<BTreeSet<Tuple>>, Vec<BTreeSet<Tuple>>, Vec<Violation>)` that runs the closed-relation preemption check, stages, and judges; `violations` returns its third element, `apply` consumes the first and third — reusing the `next` it already judged. Behavior-identical (staged is pure; the ClosedRelationWrite preemption at naive.rs:288-294 already precedes staging), and it restores the module's own one-derivation doctrine to the staging computation, not just the violation set.
