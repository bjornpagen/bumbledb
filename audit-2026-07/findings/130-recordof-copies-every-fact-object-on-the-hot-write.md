## recordOf copies every fact object on the SDK verb paths; the copy is dead weight on 5 of 6 sites and load-bearing only in insert

category: perf | severity: low | verdict: CONFIRMED | finder: ts:core
outcome: fixed 3ba803f4

### Summary

`recordOf` (ts/src/marshal.ts:75-77) is implemented as `Object.fromEntries(Object.entries(fact))` — per call it allocates the entries array, N two-element tuple arrays, and a fresh result object. It sits on every SDK verb: `tx.insert`, `tx.delete`, `contains`, both `get` arms, and every `execute`'s params object. On five of the six call sites the downstream consumers only read properties, so the copy buys nothing and is a hidden per-fact allocation tax — a doctrine violation in a repo whose stated reason for Rust is allocation control (docs/design/representation-first.md lineage). The finder's premise that the copy buys nothing *everywhere* is wrong in one place: on the insert path the copy is load-bearing, because `mintFreshCells` deliberately mutates the record in place. Any fix must preserve that isolation.

### Evidence (verified)

- ts/src/marshal.ts:75-77 — the copy:
  ```ts
  function recordOf(fact: object): Record<string, unknown> {
      return Object.fromEntries(Object.entries(fact))
  }
  ```
- Call sites, all verified: ts/src/db.ts:835 (`contains`), :867 and :876 (both `get` arms), :937 (`execute` params), :1092 (`insert`), :1108 (`delete`).
- Read-only consumers, verified property-read-only: `rowOf` (ts/src/marshal.ts:189-197), `keyRowOf` (ts/src/marshal.ts:205-225), `wireParams` (ts/src/query/run.ts:57-87). None writes to the record — identity is safe for contains/get/get/execute/delete.
- The one mutating consumer: `mintFreshCells` (ts/src/db.ts:707-737). Its jsdoc at db.ts:705 states "Mutates `values` in place with the minted cells" and db.ts:727 performs `values[declared.name] = cell`. The insert flow at db.ts:1092-1094 is `values = recordOf(fact)` → `mintFreshCells(..., values)` → `rowOf(relation.data, values)`; the copy is what keeps the minted id out of the caller's own fact object.
- Bench registry check: docs/architecture/61-bench-lanes.md — every lane in the night table (`bench-durable`, `scenarios`, `storage`, `curves`, `write-throughput`, …) is a Rust-engine lane. No lane exercises the TS SDK verb path, so the finder's "benchmark lanes improve" claim does not attach to any registered lane; the cost lands on real SDK users, unmeasured.

### Bench impact

Per verb call: ~2N+2 short-lived allocations (entries array, N tuples, result object) that a representation fix erases on the read/delete/execute paths. In bulk-write or point-read loops through the SDK this is steady nursery-GC pressure. The absolute win is modest — each verb also crosses the napi bridge (`native.txInsert` etc.), which likely dominates — hence severity low. No committed bench lane would show the delta.

### Suggested fix

Split the two roles the copy currently conflates:

1. For the five read-only sites, make `recordOf` an allocation-free identity via an overload (public signature `(fact: object): Readonly<Record<string, unknown>>`, implementation `return fact`) — the same no-cast doctrine, zero copies. Note one semantic edge: identity exposes prototype/getter properties to `rowOf`'s property reads, which the entries snapshot did not; for plain-object facts (the SDK's contract) behavior is identical.
2. For `insert`, keep the isolation but cheapen it: a single `{ ...fact }` spread (one object allocation, no intermediate arrays) — or, more representation-first, have `mintFreshCells` stop mutating and return the minted cells for `rowOf` to read as an overlay (`fact[name] ?? fresh[name]`), erasing the copy entirely and making "insert mutates its input record" an unrepresentable state rather than a documented caveat (db.ts:705).

The finder's one-line identity fix applied verbatim would be a behavior bug: on insert it would write minted fresh ids into the user's fact object, or throw if the caller passed a frozen object.
