## pump's inner batch loop never checks all_cancelled — a whole-execution D2 skip still iterates and probes the entire remaining node-0 cover

category: perf | severity: high | verdict: CONFIRMED | finder: engine:run

### Summary

The executor's whole-execution poison flag `all_cancelled` — documented at `crates/bumbledb/src/exec/run.rs:585-586` as "A skip crossed the virtual root: the whole execution is done" — is consulted at only two granularities: once per pending entry at the top of pump's entry loop (`pump.rs:54-56`) and once per survivor in probe_pass's routing loop (`probe_pass.rs:492-494`). The inner token loop in pump (`pump.rs:129-170`) that draws cover batches and flushes them through `probe_pass` has no check at all. Node 0 always holds exactly ONE pending entry — the virtual root (`execute.rs:446-449`) — so for node 0 the entry-granularity check can never re-fire once iteration has begun. When the poison is set mid-iteration, pump keeps drawing every remaining batch of node 0's cover and `probe_pass` keeps running the full hash + sibling-probe + residual + anti-probe pipeline over each one, only to discard all survivors when the routing loop breaks at k=0.

### Evidence (all verified in the working tree)

- `crates/bumbledb/src/exec/run/pump.rs:129-170` — the inner loop: `loop { let want = ...; let (yielded, next) = colts[cover_occ].iter_batch(...); ... if fill == self.batch { self.probe_pass(...); fill = 0; if !gate_cover && yielded == want { continue; } } if gate_cover || yielded < want { break; } }`. No `self.all_cancelled` test appears anywhere in this loop; the only pump check is `pump.rs:54` at each entry's turn.
- `crates/bumbledb/src/exec/run/execute.rs:446-449` — `run_pipeline` seeds node 0 with a single virtual-root entry (`pending_len = 1`) and calls `self.pump(&tables, plan, 0, ...)`. Node 0's pump is the top-level driver of the whole execution.
- Poison setters, all reachable mid-batch: `probe_pass.rs:561` (leaf `Flow::SkipSuffix` under `tables.absorb == None`), `probe_pass.rs:486` (origin-mint overflow), `probe_pass.rs:360` and `run_node.rs:393` (MeasureOfRay ray poison). Lines 360 and 486 carry the comment "stops the pump loops upstream" — which the code does not fully deliver.
- `probe_pass.rs:492` — the `all_cancelled` check sits at the top of the survivor ROUTING loop only. The sibling hash/gather/probe passes (`probe_pass.rs:56-235`), the five residual pass families (`probe_pass.rs:239-416`), and the anti-probe pass (`probe_pass.rs:423-442`) all run unconditionally before it. Every post-cancellation batch therefore pays the full vectorized pipeline and then throws the survivors away.
- Scenario constructibility: `plan/fj/validate.rs:271-275` marks a node `SuffixSkip::Licensed` whenever none of its `new_vars` are sink vars, so a boolean/existential head (`q() :- r(x,y), s(y,z)`) makes every node Licensed; `pipe_tables.rs:45-47` computes `absorb` as the last Forbidden node, hence `None` for such plans; `leaf.rs:96-103` returns `SkipSuffix` on the projection sink's first emit (the projection sink advertises `SkipCapability::Licensed`, `exec/sink/projection/sink.rs:72`); `probe_pass.rs:558-561` maps that to `self.all_cancelled = true` when `absorb` is `None`.
- Spec check: docs/architecture/40-execution.md, Deviation D2 (§ "set semantics") — "the executor may skip a plan suffix after the first witness ... the emitted fact cannot change". For the all-existential (absorb=None) case the entire execution's answer is fixed at the first witness; the code's continued cover iteration contradicts both this deviation's intent and the flag's own doc comment at run.rs:585-586. (The Free Join paper itself is bag-semantic and has no D2; this is bumbledb's own documented deviation, so the doc is the spec here.)

### Failure scenario / Bench impact

Boolean projection `q() :- r(x,y), s(y,z)`: node 0 covers `r` with a sibling probe into `s`; node 1 is the leaf. The first full batch (128 elements) reaches the leaf, the projection sink emits its one witness, the leaf returns `SkipSuffix`, `probe_pass.rs:561` sets `all_cancelled` — and pump then draws the ENTIRE remaining cover of `r` at 128 keys/batch, running per batch: the gather+hash pass, one bucket probe per element into `s`, residuals, anti-probes, and an immediate k=0 routing break. First-answer latency degrades from O(first match) to O(|cover|) — for a million-key base relation, ~7,800 fully-priced dead batches. The MeasureOfRay error path (`probe_pass.rs:360`, `run_node.rs:393`) and the origin-overflow path (`probe_pass.rs:486`) pay the identical bill before their typed errors surface from `execute` (`execute.rs:402-409`). Any existence-style projection over a large base relation exhibits it; correctness is unaffected (the routing break discards everything), which is why no test catches it.

### Suggested fix

Add the same check the two existing granularities already carry, at the top of pump's inner token loop (`pump.rs:129`):

```rust
loop {
    if self.all_cancelled {
        break;
    }
    let want = if gate_cover { 1 } else { self.batch - fill };
    ...
}
```

Optionally mirror it as an early return at the head of `probe_pass` so a poisoned tail flush (pump.rs:172-188) and the deeper pump recursions skip their sibling/residual work too. The poison-flag representation itself is right — one flag, no per-tuple Result — it is simply not consulted at the one granularity where node 0, the whole execution's driver, actually lives.
