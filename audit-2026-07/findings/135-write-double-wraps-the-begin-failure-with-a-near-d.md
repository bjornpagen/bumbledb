## write() double-wraps the begin failure with a near-duplicate context

category: inelegance | severity: low | verdict: CONFIRMED | finder: ts:core
outcome: fixed c31416e1

### Summary

`write()` in `ts/src/db.ts` wraps a failed `dbWriteBegin` twice with the same phrase. The inner `bridged("begin bumbledb write transaction", ...)` call already wraps any native throw with that context — that is the bridge guard's entire job. The outer `errors.trySync` in `write()` then catches the already-wrapped error and wraps it again with `begin bumbledb write transaction (live snapshots at fault: N)`. The rendered chain repeats the phrase verbatim; the live-snapshot census is the only new information, and the whole outer trySync/wrap layer exists solely to attach it.

### Evidence

- `ts/src/db.ts:1185-1194` — the double layer:
  ```ts
  const begun = errors.trySync(function beginDelta() {
      return bridged("begin bumbledb write transaction", function begin() {
          return native.dbWriteBegin(handle)
      })
  })
  if (begun.error) {
      throw errors.wrap(begun.error, `begin bumbledb write transaction (live snapshots at fault: ${liveSnapshots})`)
  }
  ```
- `ts/src/native.ts:549-555` — `bridged` is itself `errors.trySync(run)` + `errors.wrap(result.error, context)`; its doc comment (native.ts:543-548) calls it "THE one wrapper every native call crosses". So the context string is guaranteed to already be on the chain when `begun.error` is inspected.
- `@superbuilders/errors` (node_modules, `dist/index.js`, `createErrorChainToString`) renders the cause chain as `"outer: inner: root"`, so the duplicate is user-visible in every log line and thrown message.
- Contrast within the same file: the witnessed begin at `ts/src/db.ts:1246` crosses `bridged("begin witnessed bumbledb write transaction", ...)` once, with no outer re-wrap — single-wrap is the file's own established pattern.
- No test asserts on either message string (grep over `ts/test` for "live snapshots" / "begin bumbledb write" is empty), so collapsing the layers breaks nothing.

### Failure scenario

Diagnostics only. When LMDB refuses the begin (e.g. EINVAL from a leaked writer, the exact situation the census exists to diagnose — see the reader-slot comment at db.ts:985), the operator reads:

```
begin bumbledb write transaction (live snapshots at fault: 3): begin bumbledb write transaction: <native error>
```

— the phrase twice, with the census buried inside the repetition it was meant to sharpen.

### Suggested fix

Fold the census into the single bridged context and delete the outer layer:

```ts
function write(fn: DeltaBuild<Rels>): WriteResult<Rels> {
    const tx = bridged(
        `begin bumbledb write transaction (live snapshots: ${liveSnapshots})`,
        function begin() { return native.dbWriteBegin(handle) }
    )
    return runDelta(tx, fn)
}
```

This is safe: `liveSnapshots` is in closure scope (declared db.ts:960), the context string is evaluated when `write()` is entered, and the path is synchronous, so the census value is identical to what the current catch-time wrap would report. In representation-first terms (docs/design/representation-first.md): `bridged`'s context parameter is the representation slot for per-call diagnostic data; the outer trySync/wrap is a control-flow layer re-doing what that slot already carries. Adjacent (out of scope here but worth a look in the same pass): the commit paths at db.ts:1156/1173 and db.ts:1204/1216 also re-wrap with the identical string — there the outer trySync earns its keep by running the abort, but the duplicate wrap message is the same inelegance.
