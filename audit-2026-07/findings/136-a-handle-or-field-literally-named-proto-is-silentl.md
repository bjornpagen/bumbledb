## A handle or field literally named `__proto__` is silently dropped by the input object literal while the type tier admits it

category: bug | severity: low | verdict: CONFIRMED | finder: ts:types

### Summary

`closed()`'s payload tier reads the handle roster off the axioms record with `Object.keys(axioms)` (ts/src/closed.ts:390). But in the user's object literal, a plain `__proto__: {...}` property is ECMA-262 Annex B.3.1's prototype setter, not a data property — it never becomes an own enumerable key, so the handle silently vanishes from the roster with no construction error. TypeScript's inference does NOT model the Annex B special case: the Handles type parameter infers as `"__proto__" | "Warn"` and the call compiles clean. The SDK carefully defends its OUTPUT records against this exact name (closed.ts:466-473: "a handle named \"__proto__\" would otherwise ride the Object.prototype accessor", minted via `Object.defineProperty` in `mintAxioms`, closed.ts:300) but the INPUT literal is the actual leak. The module doctrine "NO handle name is reserved: a vocabulary may legally contain handles named `match`, `where`, or `id`" (closed.ts:12-16) is silently false for exactly one name.

### Evidence (all verified by reading and by execution)

- ts/src/closed.ts:390 — `const handles = Object.keys(axioms)`: own enumerable keys only; the `__proto__:` spelling never creates one.
- ts/src/closed.ts:12-16 — the "NO handle name is reserved" doctrine.
- ts/src/closed.ts:466-474 — the output-side-only `__proto__` defense (own-property definition inside `mintAxioms`).
- ts/src/fields.ts:250-256 — `assertDeclarationOrderKey` rejects only integer-index names; `__proto__` never even reaches it because enumeration misses it.
- ts/src/fields.ts:223-224 — the downstream throw: `"${value}" is not a handle of ${closed.name}` when a type-legal `"__proto__"` literal meets the truncated roster (same message at query/lower.ts:1462 and marshal.ts:92).
- ts/src/relation.ts:202 and ts/src/closed.ts:442 — `Object.entries(fields/columns)`: the identical input hole for a field or column named `__proto__`.

Executed repro (repo's own tsc 5.x, `tsc --noEmit` exit 0; node --experimental-strip-types):

```ts
const Sev = closed("Sev", { pages: bool }, { __proto__: { pages: true }, Warn: { pages: false } })
// runtime:  handles: [ 'Warn' ]                        — one handle silently gone
// runtime:  Sev.axioms["__proto__"] === Object.prototype  → true
// type:     keyof typeof Sev.axioms  is exactly "__proto__" | "Warn"  — both literals assignable
```

### Failure scenario

`closed("Sev", { pages: bool }, { __proto__: { pages: true }, Warn: { pages: false } })` mints a roster of `["Warn"]` with no error. The type swears `"__proto__"` is a handle, so a referencing selection like `where({ sev: "__proto__" })` typechecks and then throws `'"__proto__" is not a handle of Sev — the roster is Warn'` at runtime (fields.ts:224). Worse, `Sev.axioms.__proto__` reads `Object.prototype` where the type promises an `AxiomRow` — a type-level lie on the readback surface. The same spelling in a `relation()` field block or a closed column block drops the field the same way.

### Doctrine lens

This is a make-illegal-states-unrepresentable failure at a trusted seam: the code already has admission guards at this exact boundary (`handleKeysOwn`, closed.ts:252-259, and `axiomsMinted`, closed.ts:268-282) but both iterate the already-truncated key list, so they verify the wrong invariant — they prove every enumerated key is own, never that every intended key was enumerated. The dropped key is representationally detectable and simply isn't checked.

### Suggested fix

At every declaration-record seam (payload axioms at closedPayload, closed columns at mintClosed, relation fields at relation()), judge the record's prototype before enumerating: `const p = Object.getPrototypeOf(record); if (p !== Object.prototype && p !== null) throw ...` — a non-default prototype on a declaration literal proves a plain `__proto__:` was spelled. Throw the same warm construction error `assertDeclarationOrderKey` throws for integer indices, with the escape hatch in the message: the computed spelling `["__proto__"]: {...}` creates an own data property and works correctly today (verified: `Object.defineProperty`/spread on the output side already handles it), restoring "no handle name is reserved" honestly. (The `p !== null` arm keeps `Object.create(null)`-built records admissible.)
