## Keyed-get traced lane ships empty engine captures: the snapshot point-read path is wholly dark

observability | medium | CONFIRMED | obs-estate
outcome: fixed e6d6f877

### Summary

The snapshot point-read surface — `Snapshot::get_dyn`, `Snapshot::get`, `Snapshot::contains` in `crates/bumbledb/src/api/db/snapshot.rs` — carries zero obs instrumentation. The scenarios trace lane for `Surface::KeyedGet` (`crates/bumbledb-bench/src/scenarios/trace.rs`) traces exactly this call, so its capture contains only the harness `sample` span; after `split_harness` the engine stream is empty and the shipped artifacts are 0-byte `.folded` files plus a header-only flame embed (`total wall 0.000 us`). The p5 lane exists to profile the point read and structurally cannot attribute a nanosecond of it. The baseline authors noticed — `TALLY.md` marks it as an "honest gap" — but documenting the blank is not lighting the path.

### Evidence (verified)

- **No obs anywhere in snapshot.rs**: `grep obs crates/bumbledb/src/api/db/snapshot.rs` is empty. `get_dyn` (snapshot.rs:204-255) runs determinant encode → U probe / F fetch (or closed-extension / fresh-row `fact_at`) → decode with no span at any phase; same for typed `get` (:281) and `contains` (:160-189).
- **The traced lane traces exactly this call**: `crates/bumbledb-bench/src/scenarios/trace.rs:89-119` (`Surface::KeyedGet` arm) wraps `db.read(|snap| snap.get_dyn(...))` in only the harness `sample` span (cold) / `harness::traced_sample` (warm).
- **Shipped artifacts on disk**: `bench-out/baseline-2026-07-25/traced/scenarios/points/trace/scenarios/points/p5_keyed_get.warm.folded` and `.cold.folded` are both **0 bytes**; both `.json` files hold exactly one event — the tid-2 harness `sample` (`{"name":"sample","cat":"harness",...}`) — and no engine event. Siblings p1–p4 in the same directory have real engine streams (246–5876-byte artifacts).
- **The empty artifacts ship without refusal**: `trace_out::emit_pair` (`crates/bumbledb-bench/src/trace_out.rs:56-68`) writes the pair unconditionally and renders `FlameSummary::compute(&engine).render_top(10)` — with an empty engine stream that is a header plus `total wall 0.000 us` (`trace_out/flame_summary.rs:111`). The `--trace` honesty rule (`crates/bumbledb-bench/src/driver/trace.rs:18-22`, "An obs-less capture is empty — refuse before writing span-free artifacts") guards **obs-less builds** (`!cfg!(feature = "obs")`) only; an obs-enabled capture that yields zero engine spans evades it entirely.
- **Downstream rendering**: `scripts/flame.py:190,231` renders an "(empty)" SVG for empty folded input; the baseline carries 147 flame SVGs, with the KeyedGet ones blank.
- **The gap is asymmetric**: the query-path twin is lit — `KEY_PROBE` span at `crates/bumbledb/src/exec/dispatch/key_probe_fact.rs:264` — and api-layer spans already exist (`EXECUTE`/`BIND_PARAMS`/`FINALIZE` in `crates/bumbledb/src/api/prepared/execute.rs:34,38,130`), so instrumenting the snapshot surface is squarely inside the existing obs seam (zero-cost off, per docs/architecture/40-execution.md observability doctrine).
- **Known, not fixed**: `bench-out/baseline-2026-07-25/TALLY.md:22` records "p5_keyed_get traces empty: … zero engine spans by design … attribution is below span coverage (the whole op is sub-µs)". So the blank is acknowledged, not silent — but the same TALLY calls comparable attribution holes "observability finding material," and the acknowledgment does not make the lane's artifacts real.

### Failure scenario / impact

Run `bumbledb-bench scenarios --trace` (or `scripts/flame.sh` on the points scenario): p5's warm/cold folded artifacts are 0 bytes, `flame.py` renders "(empty)" SVGs, the report embeds a header-only table with `total wall 0.000 us`, and every KeyedGet query in the traced baseline is a blank. Where-the-time-goes is unanswerable for the point-read lane — the flagship 0.5.0 typed-key read surface — even though the harness span proves the op takes ~1.1–1.4 µs (recorded `dur` in the shipped JSONs), i.e. well within what a single span can attribute.

### Suggested fix

Light the snapshot point-read at pass granularity through the existing obs seam (ZST-off, raw-tick stamps): one `GET` span (`Category::Execute`, args: relation id, hit=1/miss=0) around the body of `get_dyn`/`get`/`contains` — mirroring `KEY_PROBE` on the query path — optionally splitting determinant-encode vs U→F fetch vs decode as sub-spans if the sub-µs stamps prove resolvable. Then the KeyedGet traced artifacts become real, the TALLY "honest gap" entry retires, and consider extending the honesty rule so an obs-enabled lane whose engine stream is empty is a refusal too, not a 0-byte file.