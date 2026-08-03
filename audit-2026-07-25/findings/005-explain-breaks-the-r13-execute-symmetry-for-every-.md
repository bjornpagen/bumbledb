## explain() breaks the R13 execute-symmetry for every query carrying a set param or a literal membership array — execute succeeds, explain always throws

observability | high | CONFIRMED | ts-surface-fresh
outcome: fixed 1bb35923

### Summary

`db.explain(p, params)` is documented as `=== db.read(snap => snap.explain(p, params))` — the R13 symmetry rule (ts/src/db.ts:412) — and its signature takes the exact same `Params` record as `execute` (db.ts:340, 413). But the bridge's explain entry refuses every set-shaped param, and a literal membership array — the canonical closed-roster spelling — is folded into the param registry as a prebuilt `{kind:"set"}` constant that crosses on every call regardless of the user's params object. Result: any prepared query using `r.inSet(...)` OR a membership array like `match(Item, { kind: ["A","B"] })` executes fine but throws unconditionally from `explain()`, with no type-level signal and no test covering it.

### Evidence (verified file:line)

- **One marshaling path for both verbs.** `wireParams` (ts/src/query/run.ts:57-82) is called by both `execute` (ts/src/db.ts:1034) and `explain` (db.ts:1043). A membership entry returns its prebuilt frozen `{kind:"set", values}` by reference (run.ts:59-61) — the host's params object is never consulted, so even `db.explain(prepared, {})` ships a set.
- **Membership arrays are set params.** Lowering builds the registry entry with `membership = Object.freeze({ kind:"set", values: [...] })` (ts/src/query/lower.ts:1412-1427).
- **The bridge refuses sets for explain only.** `explain_stats` (ts/crate/src/lib.rs:743-753) calls `bind_scalars` (lib.rs:620-632), which errors on `OwnedParam::Set` with "bumbledb: preparedExplain binds scalar params only (the engine's profile entry has no param-set spelling)". The execute path's `param_args` (lib.rs:607-615) handles `Set` via `ParamArg::Set`.
- **Root cause is the engine surface.** `Snapshot::profile` takes `&[BindValue]` only (crates/bumbledb/src/api/db/snapshot.rs:94-100); `execute_args`/`execute_collect_args` take `&[ParamArg]` (snapshot.rs:51-71). Internally, `PreparedQuery::profile` binds via scalar-only `bind_params` (api/prepared/introspect.rs:186-205) while `execute_args` binds via `bind_param_args` and both share the same `run_bound` execution body (api/prepared/execute.rs:54-90) — the set arm is simply not plumbed into profile.
- **Runtime reproduction** (built darwin-arm64 addon, fresh temp store): for BOTH a membership array (`kind: ["A","B"]` on a `closed` reference) and an ordinary-field `r.inSet` param, `db.execute` succeeded and `db.explain` threw the exact chain `bumbledb read → explain bumbledb prepared query → bumbledb: preparedExplain binds scalar params only (the engine's profile entry has no param-set spelling)`.
- **No test coverage, no type wall.** The only explain tests are param-free (ts/test/db.test.ts:584-598) or scalar-param (ts/test/expressibility-operand-views.test.ts:291). `Prepared<Rels,Row,Params>` carries no set-freeness; explain's signature is execute's twin.

### Failure scenario / impact

Any host that introspects its prepared plans — the flagship R13 diagnostic loop — and uses membership arrays or ∈-set params (the standard closed-vocabulary idiom throughout the cookbooks; cookbook tests use `r.inSet` in semi-naive frontier queries) gets a hard runtime throw from `explain()` on a query that executes correctly. The doc comment at db.ts:412 promises symmetry; the type system promises the same params; the runtime breaks both. The failure is total (every call throws), silent until runtime, and undocumented on the TS surface.

### Suggested fix

Engine-side unification, exactly as the finder proposes: give profile/introspect the `ParamArg` roster execute already takes — `PreparedQuery::profile` swaps `bind_params` for the existing `bind_param_args` (the instrumented run already shares `run_bound`-style plumbing with execute), `Snapshot::profile` takes `&[ParamArg]`, and the bridge's `bind_scalars` + its refusal delete outright (`explain_stats` calls `param_args`, the same marshaling execute uses). Zero-backwards-compat policy makes the signature change free. Land with the missing test: an explain over a membership-array query and over an `r.inSet` query, asserting stats arrive and emits match execute. Failing the engine change, the asymmetry must become a type wall (Prepared carries set-freeness) — but that enshrines a hole in the R13 surface rather than closing it.