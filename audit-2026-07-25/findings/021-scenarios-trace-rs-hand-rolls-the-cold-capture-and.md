## scenarios/trace.rs hand-rolls the cold capture and leaks the live capture on every error path — harness::traced_sample already exists with the drain-either-way discipline

unification | medium | CONFIRMED | cross-branching-new
outcome: fixed ea3d6792

### Summary

`capture_query` in `crates/bumbledb-bench/src/scenarios/trace.rs` reimplements the cold traced sample by hand in both surface arms (`start_capture` + SAMPLE Harness span + `finish_capture`), and every `?` exit between the start and the drain returns with the thread-local capture buffer still live. `crate::harness::traced_sample` (harness/traced.rs:10-21) provides byte-identical semantics — SAMPLE span included — with the capture drained before the closure's error propagates ("the capture is drained either way", traced.rs:9), and the very same file already uses it for both warm halves (trace.rs:86, 117). These two cold arms are the only drain-on-error violations in the bench crate.

### Evidence (verified against v0.9.0-era code on `bugbash-perf`)

- `crates/bumbledb-bench/src/scenarios/trace.rs:57-71` (Query cold arm): `obs::start_capture()` (57) → SAMPLE span (58) → `db.prepare(&q).map_err(...)?` (59-61) → `db.read(...).map_err(...)?` (63-64) → `cold_span.end()` (65) → `obs::finish_capture()` inside `emit(...)` (66-71). Both `?` exits skip the drain.
- `crates/bumbledb-bench/src/scenarios/trace.rs:92-102` (KeyedGet cold arm): same shape; the `?` at line 95 (`get_dyn` map_err) skips the `finish_capture()` at line 100.
- `crates/bumbledb-bench/src/harness/traced.rs:10-21`: `traced_sample` runs `start_capture` → SAMPLE span → `f()` → `span.end()` → `finish_capture()` → `Ok((result?, events))` — the drain happens unconditionally before the error propagates. `traced.rs:86` and `:117` in scenarios/trace.rs already call it for the warm halves.
- `crates/bumbledb/src/obs.rs:483-492`: `start_capture` is idempotent-extend (`get_or_insert_with`) — "a nested (or unwound-over) start extends the live capture, never destroys it." A leaked buffer is therefore inherited by the next capture on the thread and its foreign events drain into that capture's artifact.
- `crates/bumbledb/src/obs.rs:506-512`: `record` pushes into any live buffer; `obs.rs:11-14` sanctions recording allocation only because "capture is never enabled inside a measured allocation window" — a leaked always-on capture breaks that mutual-exclusion contract for every subsequent engine call on the thread.
- Crate-wide `start_capture` survey: capacity/windowed/crud/lawful/writes lanes reach capture only through `harness::measure` → `traced_sample` (harness/measure.rs:112); `sweep.rs:341-353` calls `start_capture` raw but drains before its `?` (`let outcome = db.write(...); let events = obs::finish_capture(); outcome.map_err(...)?`) — correct discipline, just not the shared helper; `tripwires.rs:55/161/201` are tests using `.expect()` (panic, no `?` escape). scenarios/trace.rs cold arms are the sole violations.
- Sole caller: `crates/bumbledb-bench/src/scenarios/run_query.rs:348-350` propagates `capture_query`'s error with `?`, so today the leak is latent — the run aborts before the polluted buffer is observed.

### Failure scenario / impact

A scenario query's cold prepare or first execute fails mid-run (schema drift, bad fixture). The capture stays live on the thread. Today the whole run aborts, so the damage is unrealized; the moment any caller downgrades the per-query error to a warning (the natural evolution of a multi-scenario trace pass), the next query's cold artifact drains the failed query's events with a foreign timeline, and until then every engine invocation on the thread records-and-allocates into the leaked buffer — inside any subsequently measured window, violating the obs.rs allocation sanction. Separately, the duplication means two owners of the cold-capture shape in one crate: a future change to the SAMPLE discipline (e.g. args, category) must be made twice or silently diverges.

### Suggested fix

Delete both hand-rolled cold blocks and route through the shared helper. For the Query arm, hoist the prepared query through an `Option` slot so the warm half can reuse it:

```rust
let mut slot = None;
let (_, cold) = crate::harness::traced_sample(&mut || {
    let mut prepared = db.prepare(&q).map_err(...)?;
    let mut buffer = Answers::new();
    db.read(|snap| snap.execute(&mut prepared, &bind_values(&sets[0]), &mut buffer))
        .map_err(...)?;
    slot = Some((prepared, buffer));
    Ok(0)
})?;
emit(&dir, &format!("{}.cold", sq.name), cold, false)?;
let (mut prepared, mut buffer) = slot.expect("cold sample succeeded");
```

The KeyedGet arm is a direct one-closure replacement. Identical semantics (SAMPLE span included), drain-on-error for free, and `traced_sample`/`traced_cold_sample` become the only capture owners in the timed lanes.