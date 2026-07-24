## Exhume scan's row/descriptor pairing throws on short rows but silently drops extra cells

category: incoherence | severity: low | verdict: CONFIRMED | finder: ts:core

### Summary

The SDK's exhume scan decodes each positional engine row by pairing it against the persisted descriptor's field names. A row SHORTER than the field list gets the pointed `bumbledb exhume drift` throw; a row LONGER than the field list decodes silently, its trailing cells discarded. Every other row-decode seam in the SDK gates on exact arity first — exhume, the one surface whose explicit reason to exist is store/descriptor drift, is the only one that does not.

### Evidence

- `ts/src/exhume.ts:210-224` — the pairing walk:
  ```ts
  rows.map(function factOf(row): ExhumedFact {
      const fact: Record<string, FactValue> = {}
      names.forEach(function pair(name, index) {
          const cell = row[index]
          if (cell === undefined) {
              throw errors.new(
                  `bumbledb exhume drift: relation ${relation} row has no value at position ${index} (${name})`
              )
          }
          fact[name] = cell
      })
      return Object.freeze(fact)
  })
  ```
  No `row.length !== names.length` check anywhere in `scan` (ts/src/exhume.ts:202-225). Note the `cell === undefined` check is the type-narrowing check indexed access forces on every seam (`row[index]: FactValue | undefined`); the deliberate arity gate is absent entirely, not merely one-sided.
- `ts/src/marshal.ts` `factOf` — the contrast seam: `if (row.length !== data.fields.length) throw errors.new(\`relation ${data.name}: row arity ${row.length} does not match the ${data.fields.length} declared fields\`)` — the exact-arity gate PLUS the per-cell undefined check.
- `ts/src/query/run.ts:114-116` `decodeAnswers` — same pattern: `if (row.length !== finds.length) throw errors.new(...)` plus the per-cell check.
- `crates/bumbledb/src/api/db/exhume.rs:74-83` — the engine's exhume doctrine is belt-and-suspenders decode fidelity: after the fingerprint hash gate, the decoded descriptor must re-encode to the exact stored bytes, with the comment "a decoder drift can never silently misread a store." The SDK-side row pairing is the one link in that chain that tolerates a silent misread (of row shape) in one direction.
- `docs/architecture/70-api.md` § exhume (around line 416-426) frames the contract as: scan rows come back in the descriptor's field declaration order, and pairing positions against field names IS the name-keyed reading — i.e. the descriptor's field list is the row's declared shape, not a lower bound on it.
- No test covers the mismatch: `ts/test/exhume.test.ts` has no drift or arity case.

### Failure scenario

Not reachable through today's engine: `exhume_scan` (ts/crate/src/lib.rs:456-477) decodes rows per the same in-process `SchemaDescriptor` the manifest is rendered from, so widths agree by construction — which is also true of the rows `factOf` and `decodeAnswers` receive, yet those seams keep the gate. The exposure is exactly the class exhume is built for: a bridge/engine version skew or marshal bug that yields rows wider than the persisted descriptor declares would be reported as clean facts with trailing cells silently dropped, instead of the pointed `exhume drift` error the shorter-row case gets. On a forensic surface, "clean-looking but truncated" is the worst failure shape.

### Suggested fix

Add the same exact-arity gate the other two decode seams use, before the pairing walk in `scan` (ts/src/exhume.ts:211):

```ts
if (row.length !== names.length) {
    throw errors.new(
        `bumbledb exhume drift: relation ${relation} row arity ${row.length} does not match the ${names.length} descriptor fields`
    )
}
```

One line, no representation change needed — this is restoring consistency with the seam pattern already established in marshal.ts and query/run.ts.
