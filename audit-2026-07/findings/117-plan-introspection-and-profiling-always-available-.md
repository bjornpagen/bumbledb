## Plan introspection and profiling (EXPLAIN/ANALYZE) have no TypeScript surface

category: missing-free-feature | severity: low | verdict: CONFIRMED | finder: cross:free-features

### Summary

The engine's plan introspection — EXPLAIN/ANALYZE, which the architecture docs call "the debugging story" that "exists from day one" — is fully built and free on the Rust surface (`snap.introspect(..)` returns the rendered `introspection v3` report; `snap.profile(..)` returns structured `ExecutionStats`), but the napi bridge marshals none of it. A TypeScript host — the project's primary consumer — has no way to see the join plan, folded ψ member sets, pinned access paths, per-rule stats, or unresolved-literal diagnostics for a prepared query. The engine already renders the artifact to a plain `String` and the stats to a plain struct, so the missing piece is pure marshaling: one snapshot-worker request variant and one bridge function.

### Evidence (verified against the code)

- **Rust surface exists exactly as documented:**
  - `crates/bumbledb/src/api/db/snapshot.rs:79-85` — `Snapshot::introspect(&mut PreparedQuery, params) -> Result<(Answers, String)>` (ANALYZE semantics: executes with counting instrumentation).
  - `crates/bumbledb/src/api/db/snapshot.rs:94-100` — `Snapshot::profile(..) -> Result<(Answers, ExecutionStats)>`.
  - Backing impls: `crates/bumbledb/src/api/prepared/introspect.rs:28` (`introspect`) and `:186` (`profile`); `ExecutionStats` is a plain data struct at `crates/bumbledb/src/api/stats.rs:16`.
- **Docs promise availability and lean on it:**
  - `docs/architecture/70-api.md:862-872` — "Plan introspection — EXPLAIN, colloquially — is always available through `snap.introspect(..)` ... `Snapshot::profile` returns the same execution as structured `ExecutionStats`". (The "always available" contrast is with the two feature-gated observability surfaces, `alloc-counter` and `trace`.)
  - `docs/architecture/40-execution.md:914` — "**Plan introspection exists from day one** and is the debugging story."
  - `docs/cookbook.md:330` — recipe text relies on its output: "plan introspection prints the set, not a count".
- **TS surface has nothing:**
  - `ts/crate/src/lib.rs` prepared roster (verified by grep of every `pub fn`): `db_prepare` (:1238), `prepared_execute` (:1276), `prepared_staleness` (:1299), `prepared_close` (:1315). No introspect, no profile.
  - The snapshot worker's request enum handles only `Scan`/`Contains`/`Get`/`Execute`/`Staleness`/`Witness`/`Close` (`ts/crate/src/lib.rs:613-653`) — there is no introspection variant even at the worker layer.
  - `ts/src/db.ts:302-310` — the `Prepared` interface carries only `staleness(snap)`. Grep of `ts/src`, `ts/test`, `ts/README.md`, `ts/COOKBOOK.md` finds zero occurrences of introspect/profile/ExecutionStats.

### Failure scenario

A TS host with a slow or surprising prepared query (or wanting to verify a ψ fold like the cookbook's `folded: Kind{mastered == true} → {DirectPass, JudgedPass}`) has no engine-side diagnostic at all. `prepared.staleness(snap)` reports drift counts but never the plan. The host's only recourse is to rebuild the schema and query in a Rust harness to run `snap.introspect` there — a full second toolchain to answer "what did the engine do with my query".

### Context that bounds the severity

The API freeze operates under a recorded trigger law (`docs/architecture/70-api.md:1041-1055`): surface lands only when a real consumer reaches for it, and "unfired speculative sugar would itself be debt". So shipping this requires a ruling, and low severity is right. But note the OPEN ledger never censused TS-side introspection — the "Resolved by ruling or implementation" line (:1160-1163) records only the Rust spelling `snap.introspect(&mut prepared, params) -> (Answers, String)`. This gap is an unexamined omission, not a recorded DECLINE, and the asymmetry cuts against "the debugging story exists from day one" for the host that actually runs the primary workload.

### Suggested fix

Marshaling only, no engine change: add a `SnapReq::Introspect { prepared, params }` variant to the snapshot worker and a `prepared_introspect` bridge function returning `(rows, report: String)` (optionally `prepared_profile` returning `ExecutionStats` as plain data), surfaced on the TS side as `snap.introspect(prepared, params)` with the same shape as `execute`. The report is already a rendered `String` and the stats a plain struct, so the bridge cost is one reply variant and one `#[napi]` function.
