## isStatementValue's "no fact cell ever has data.kind" claim breaks on structurally-open interval values

category: bug | severity: low | verdict: CONFIRMED | finder: ts:core

### Summary

The keyed-get selector dispatch (`selectKeyRead`, the ONE dispatch both `Db.get` and the read scope's `get` route through) distinguishes a `key()` statement value from a primary-key object by probing the middle argument for an object-valued `data` property that carries a `kind` property (`isStatementValue`, ts/src/db.ts:514-522). The docstring asserts "no fact cell shape (bool, bigint, string, bytes, `{ start, end }`) ever does" (ts/src/db.ts:508-512). That claim is false: fact cells are structurally typed and open. An interval value with an excess `kind` property is a legal interval everywhere else in the SDK — but a primary-key `get` on a relation whose key projects an interval field literally named `data` misclassifies such a key object as a statement and throws the misleading error `keyed get with a statement selector also takes the key object — get(relation, keyStatement, key)` instead of performing the read.

This is also a philosophy violation: the dispatch relies on a probabilistic shape probe (a branch guessing at a representation) where a real representation — a module-private symbol brand on statement values — would make the misdispatch unrepresentable.

### Evidence (all verified against the working tree)

- **ts/src/db.ts:514-522** — `isStatementValue`: `if (typeof value !== "object" || !("data" in value)) return false; const data: unknown = value.data; return typeof data === "object" && data !== null && "kind" in data`. Docstring at 508-512 makes the "no fact cell shape ever does" claim.
- **ts/src/db.ts:539-548** — `selectKeyRead`: on the 2-arg (primary-key) path, `isStatementValue(keyOrStatement)` returning true throws the statement-selector error before any marshal runs.
- **ts/src/fields.ts:199-208** — `isIntervalValue` ("THE one interval predicate") narrows on `start`/`end` bigint presence only; excess properties are not sealed.
- **ts/src/marshal.ts:174-179** — `cellOf`'s interval arm accepts the same loose value and re-derives `{ start: value.start, end: value.end }`, stripping extras — the excess-`kind` value IS a legal cell at the write/lookup seam.
- **docs/architecture/30-dependencies.md:348** — the FD (key) validation rule permits "at most one interval-typed field, and it must be the final projection position": an interval-keyed relation is schema-legal, so the scenario is inhabitable (verified: the engine accepted `key(cfg, ["data"])` over `data: interval(u64)` at `Db.create`).
- **Runtime reproduction** (temp `node:test` against the built package, removed after the run):
  ```ts
  const cfg = relation("cfg", { data: interval(u64), value: u64 })
  const s = schema("S", { cfg }, [key(cfg, ["data"])])
  const db = await Db.create(dir, s)
  db.write(tx => { tx.insert(cfg, { data: { start: 1n, end: 2n }, value: 7n }) })
  db.get(cfg, { data: { start: 1n, end: 2n } })                    // ✓ returns the fact
  const withKind: { start: bigint; end: bigint; kind: string } = { start: 1n, end: 2n, kind: "window" }
  db.get(cfg, { data: withKind })                                   // ✗ throws
  ```
  The second get failed with cause `keyed get with a statement selector also takes the key object — get(relation, keyStatement, key)` raised at `selectKeyRead` (compiled `dist/db.js:198`). `pnpm exec tsc --noEmit` accepted the repro — the excess property is type-legal through a variable (TypeScript's excess-property check applies only to fresh object literals, not to structural assignment).

### Failure scenario

Schema declares `relation("cfg", { data: interval(u64), ... })` keyed by `key(cfg, ["data"])`. A host passes a key object whose interval value also serves another protocol and carries an extra `kind` property (e.g. `{ start, end, kind: "window" }`). The identical value inserts, marshals, and fingerprints identically to the clean one (cellOf strips extras), but `db.get(cfg, { data: value })` is misdispatched as a statement selector and throws a typed error that misdirects the user toward the 3-arg form. The bug requires the conjunction: field named exactly `data`, interval-typed, in the primary key, value carrying an excess `kind` property — hence severity low; but each conjunct is independently legal and the docstring's safety claim is simply wrong.

### Suggested fix

Replace the probabilistic exclusion with a representation. Either:
1. **Symbol brand** (preferred, matches the repo's make-illegal-states-unrepresentable doctrine): mint statement values in ts/src/statements.ts with a module-private `const statementBrand = Symbol(...)` property and have `isStatementValue` test `statementBrand in value` — no user-built fact cell can spell a module-private symbol, so the misdispatch becomes unrepresentable rather than improbable; or
2. **Seal the probe on the actual statement grammar**: require `value.data.kind` to be one of the three closed tags `"key" | "containment" | "window"` AND the co-present structural fields (`owner`/`projection` for key). This narrows the hole to adversarial construction but does not erase it.

Also correct the `isStatementValue` docstring: fact cell shapes are structurally open, so "never does" should not be claimed of a shape probe.
