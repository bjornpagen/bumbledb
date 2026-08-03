## pump counts zero-yield batch draws its leaf twin never counts, skewing the batches/batch_entries observable

observability | low | CONFIRMED | gj-split-live
outcome: fixed 6c32f1f0

### Summary

`pump` (the middle-node pass) fires `counters.batch(node_idx, yielded)` unconditionally after every `iter_batch` draw, including draws that yield 0; the leaf pass `run_node` breaks on `yielded == 0` before counting. Since `CountingCounters::batch` increments the batch count with no zero guard, middle nodes record phantom batches the leaf never would, deflating per-node mean batch fill in the introspection surface.

### Evidence

- `crates/bumbledb/src/exec/run/pump.rs:139-147` — the draw and the unguarded counter:
  ```rust
  let (yielded, next) = colts[cover_occ].iter_batch(
      cover_cursor, cover_level, token,
      &mut scratch.entry_keys[fill * cur_arity..],
      &mut scratch.children[fill..],
      want,
  );
  counters.batch(node_idx, yielded);
  ```
- `crates/bumbledb/src/exec/run/pump.rs:156-177` — the resume path that manufactures the zero-yield draw: when `fill == self.batch` the batch flushes, `fill = 0`, and `if !gate_cover && yielded == want { continue; }` resumes the entry's token; an entry whose fanout exactly filled `want` then yields 0 on the next draw, which is counted at line 147 before the `yielded < want` break at lines 175-177. An empty cover view (`colts[occ].start()` on an empty occurrence) produces the same counted zero draw on the first iteration.
- `crates/bumbledb/src/exec/run/run_node.rs:193-196` — the leaf twin guards:
  ```rust
  if yielded == 0 {
      break;
  }
  counters.batch(node_idx, yielded);
  ```
- `crates/bumbledb/src/exec/introspection/counters.rs:8-11` — no zero guard downstream: `self.batches[node][0] += 1; self.batches[node][1] += len`.
- `crates/bumbledb/src/exec/introspection/into_stats.rs:48-52` — `batches`/`batch_entries` flow into `NodeStats`, the stable stats surface (`api/stats.rs:224-228` documents `batches ≪ batch_entries` as the batching-engaged signal).
- `crates/bumbledb/src/exec/run.rs:187-189` — the counter's contract: "at batch size B over N tuples this fires ~N/B times". At a middle node whose entries' fanouts are exact multiples of B, it fires 2N/B times.
- The only other `counters.batch` call sites (`run/leaf.rs:73`, `run/scan_table.rs:66`) pass fixed nonzero lengths, so the asymmetry is exactly pump-vs-run_node. `run_node.rs:60-62` declares `pump` the pipelined twin of the leaf pass (the explicit "kept line-parallel" block at run_node.rs:380-387 covers the probe section vs `probe_pass.rs`; the draw-loop twinning is the line-60 declaration).

### Failure scenario / impact

Introspection-only (the `Counters` generic no-ops under `NoCounters`, so no hot-path cost). A profile reader comparing per-node mean batch fill (`batch_entries / batches`) across nodes, or across a plan-shape change that moves a node between middle and leaf position, sees an artifactual fill drop: an entry with fanout exactly 128 records `batches = 2, batch_entries = 128` at a middle node but `batches = 1, batch_entries = 128` at the leaf — up to 2x the true batch count, reading as "batching not engaged" where the executor batched perfectly. This contradicts the observable's own contract (run.rs:187-189) and the observability doctrine's premise that the introspection representation reports the mechanism as it ran.

### Suggested fix

Guard pump's counter exactly as the leaf does — count only nonzero yields (e.g. `if yielded > 0 { counters.batch(node_idx, yielded); }` at pump.rs:147, or hoist the count above the resume `continue` so only fill-advancing draws count), keeping the twins line-parallel. Pin with a `CountingCounters` test driving a middle node whose entry fanout is an exact multiple of the batch size, asserting `batches` matches the leaf's count for the same shape.