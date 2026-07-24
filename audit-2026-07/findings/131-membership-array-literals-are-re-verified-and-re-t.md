## Membership-array literals are re-verified and re-translated through the roster on every execute

category: perf | severity: low | verdict: CONFIRMED | finder: ts:core
outcome: fixed 3ba803f4

### Summary

A membership-array literal (a closed-reference `[...]` binding folded into the query program) stores its raw handle NAMES on the `ParamEntry` (`members: readonly string[]`). Because `wireParams` runs on every `preparedExecute`, each member name is re-translated name → row id through the roster (`Array.prototype.indexOf`) and re-tagged into a fresh `TaggedValue` object on every execution, plus a fresh `{ kind: "set", values }` wrapper — N indexOf scans and N+1 object allocations per execute for values that are frozen build-time constants of the prepared plan. The per-execute recomputation is the representation-first violation: the entry stores the pre-image (names) instead of the image (tagged ids), forcing the same translation branch to re-run forever.

One correction to the original finder's framing: the construction-time judge (`membershipSet`) validates string-ness, arity (≥ 2), and distinctness — but NOT roster membership. The `indexOf` roster check in `taggedHandleId` is the only verification point, so an out-of-roster handle name in a membership array currently throws at the FIRST execute rather than at `prepare`. This strengthens the fix rather than weakening the finding: resolving members at build time both erases the per-execute work and moves the error to where the mistake was made.

### Evidence (verified)

- `ts/src/db.ts:934-941` — `execute` calls `wireParams(plan.params, recordOf(params))` on every prepared execute; nothing is cached between executes.
- `ts/src/query/run.ts:58-66` — the membership branch: `if (entry.members !== undefined) { return { kind: "set", values: entry.members.map(... wireValue(entry, ..., member)) } }`. The host params object is never consulted for these values; they are program constants supplied by the SDK itself (the file's own doc comment at run.ts:50-55 says so).
- `ts/src/query/run.ts:36-43` — `wireValue` → `taggedCmpLiteral`.
- `ts/src/query/lower.ts:1451-1466` — `taggedHandleId`: `closed.handles.indexOf(value)` per call plus a fresh `{ kind: "u64", value: BigInt(id) }` allocation; reached via `taggedCmpLiteral` → `taggedLiteral` (lower.ts:1493-1496) for a rostered anchor field.
- `ts/src/query/lower.ts:442-475` — `membershipSet` at construction: checks string-ness, `length >= 2`, distinctness, and mints the content-addressed name; it does NOT check the names against `roster.handles`.
- `ts/src/query/lower.ts:559-570` — the `ParamEntry` for a membership array carries `members` (frozen) and `anchor: declared.field`; the roster is part of the schema and fixed at build, so the tagged values cannot differ across executes of the same plan.
- Docs check: `docs/architecture/20-query-ir.md` and `70-api.md` contain no contract requiring execute-time roster verification for membership arrays; the "THE single roster-verification point" doctrine (lower.ts:1447-1450) is about there being one FUNCTION, which a build-time call preserves.

### Bench impact

Query-execution lanes whose programs carry membership-array filters pay, per execute, N roster `indexOf` scans (linear in roster size) and N+1 short-lived object allocations that a prebuilt frozen `QueryParam` would erase entirely. N and rosters are small in practice, hence severity low — but the work is pure waste, in the hot marshal seam of every execute, and Rust-side allocation discipline is undercut by hidden per-execute allocation on the TS side. Secondary correctness-adjacent benefit of the fix: an invalid handle name in a membership array fails at `prepare` instead of first `execute`.

### Suggested fix

Resolve each membership array to its frozen `{ kind: "set", values: TaggedValue[] }` `QueryParam` ONCE — either at registry time on the `ParamEntry` (store the prebuilt param instead of, or alongside, the raw names) or when the `PreparedPlan` is assembled in `db.prepare`. `taggedHandleId` remains the single roster-verification point; it just runs at build. `wireParams`'s membership branch then returns the prebuilt frozen value by reference — the representation (image, not pre-image) erases the per-execute re-translation.
