## Statement is unbranded, so schema() admits forged statements that bypass the roster wall the engine cannot backstop

category: bug | severity: medium | verdict: CONFIRMED | finder: ts:types

### Summary

`Statement` (ts/src/statements.ts:78-80) is a plain structural interface — `{ readonly data: StatementData }` — with no admission brand. The two construction-time runtime walls the module itself documents as load-bearing, `assertArityAgreement` (statements.ts:126) and `assertRosterAgreement` (statements.ts:150), run only inside the constructors `contained()`/`mirrors()`/`window()` (statements.ts:214-215, 239-240, 266-267). `schema()` (ts/src/schema.ts:299-313) re-runs membership, implied/duplicate, handle-roster, and closed-reference checks, but never re-runs the arity or roster walls. Because TypeScript typing is structural (and this project mandates hard structural typing), any object of the right shape *is* a `Statement` — so a fully well-typed literal, no casts, walks past both walls into `schema()`.

This is exactly the failure mode `count.ts` already solves for `Count`: count.ts:30-37 introduces a module-private `unique symbol` brand ("the admission brand") with the documented rationale that "`WindowSpec` is a public wire type, so without this brand every banned spelling ... would be writable as a plain object literal." `Statement` is the same kind of public value with the same kind of construction-time law enforcement — and lacks the brand.

The severity is carried by the roster wall specifically: statements.ts:142-145 states "the engine cannot backstop this one — the wire carries plain u64s, no rosters." The arity wall at least gets the engine's "colder refusal" at `Db.create` (cleanup-0.5.0 ruling 9, statements.ts:117-124); the roster wall has no second judge anywhere.

### Evidence (all verified by direct execution against the built 0.6.0 SDK)

- ts/src/statements.ts:78-80 — `interface Statement { readonly data: StatementData }`, no brand.
- ts/src/statements.ts:150-164 — `assertRosterAgreement`, whose doc (lines 134-148) states the invariant: "a plain u64 column could alias a closed vocabulary through a declared containment ... and every descriptor-keyed closed judgment — the orderable ban, the name↔id marshal, answer decode — would silently miss it," and that "the engine cannot backstop this one."
- ts/src/schema.ts:299-313 — the full `schema()` validation loop: `verifyMembership`, implied/seen duplicate checks, `verifyHandles`, then `verifyClosedReferences` and `computeClasses`. No arity or roster re-check.
- ts/src/law.ts:446-462 — `unionSlot` skips unpaired positions (line 448: `if (targetField === undefined) return`) and unions coordinates with no roster comparison; the only wall is two-generators-per-class (line 457), which a bare (non-fresh) u64 column never trips since it is not a generator. This confirms both the roster bypass and the silent arity truncation.
- ts/src/count.ts:37-47 — the `admitted: unique symbol` brand precedent, with the "public wire type" rationale in its doc comment.
- Execution repro (performed during verification): with `Kind = closed("Kind", ["Checking","Savings"])` and `R = relation("R", { k: u64 })`, the forged literal
  ```ts
  const forged: Statement = {
    data: { kind: "containment", source: on(R, "k").data, target: on(Kind, "id").data, bidirectional: false }
  }
  ```
  typechecks under the project's own `tsc --noEmit` with zero errors (no casts; it also passes `LawfulStatements`), and `schema("Forge", { Kind, R }, [forged])` returns
  `classes: {"Kind":{"id":"Kind.id"},"R":{"k":"Kind.id"}}`, with `lower()` emitting `{"name":"R","fields":[{"name":"k","valueType":{"kind":"u64"},"newtype":"Kind.id","fresh":false}]}` — the bare u64 column now carries the vocabulary's wire newtype.
  Control: `contained(on(R,"k"), on(Kind,"id"))` on the identical faces throws `R.k is a bare column but Kind.id is a Kind reference — closedness rides the descriptor ...` — proving the wall exists and is genuinely bypassed, not absent.
- docs/architecture/10-data-model.md (closed relations) is the spec `assertRosterAgreement`'s doc cites: a closed reference at the engine encoding is a plain u64 column plus a declared containment — which is why nothing downstream can distinguish the forged pairing from a legitimate one once schema() has admitted it.

### Failure scenario

Any code path that spells a statement structurally instead of through the constructors — programmatic statement assembly, round-tripping statements through a serialized/wide shape, or a test helper building fixtures — hands `schema()` a containment pairing a bare u64 with a closed id. `schema()` accepts; the class map and the lowered wire `newtype` claim the vocabulary class for a field whose descriptor carries no roster. From then on the SDK's descriptor-keyed judgments (name↔id marshal, answer decode, orderable ban — the exact list statements.ts:141-143 enumerates) treat `R.k` as a raw u64 while the class map, query class-name comparisons, and the engine's newtype label all treat it as a `Kind` reference — the precise aliasing the roster wall exists to make impossible, on the one wall with no engine backstop. Secondarily, a forged arity-mismatched containment silently truncates to the shorter projection in `computeClasses` (law.ts:448) until `Db.create`'s colder engine refusal — the exact silent-truncation behavior cleanup-0.5.0 ruling 9 moved to construction time.

### Suggested fix

Apply the project's own representation-first doctrine (docs/design/representation-first.md — make illegal states unrepresentable) using the pattern already in the codebase: give `Statement` the same module-private `unique symbol` admission brand `Count` carries (count.ts:37). The four constructors `key()`/`contained()`/`mirrors()`/`window()` become the only mints, so a statement that skipped the arity/roster walls is unspellable — one `admitted` symbol serves both mechanisms, and no re-running of the asserts at `schema()` is needed. This is strictly stronger than re-checking at the `schema()` seam, and it is the fix the count.ts doc comment already argues for in the general case.
