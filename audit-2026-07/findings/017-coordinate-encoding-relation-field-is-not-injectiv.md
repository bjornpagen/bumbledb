## Coordinate encoding `${relation}.${field}` is not injective — dotted names corrupt the class map at both tiers

category: bug | severity: high | verdict: CONFIRMED | finder: ts:types
outcome: fixed 4d30dbda

### Summary

The law-typing engine (`ts/src/law.ts`) identifies every field slot by the string coordinate `` `${relation}.${field}` `` — in the runtime union-find AND in the type-tier template-literal machinery. Nothing anywhere rejects a `.` inside a relation or field name: the SDK's one name-judging seam (`assertDeclarationOrderKey`) rejects only integer-index keys, and the engine's schema validation checks only duplicate names. The encoding is therefore not injective — relation `"A.B"` field `"x"` and relation `"A"` field `"B.x"` are ONE coordinate `"A.B.x"` — and both verified consequences follow:

1. **Soundness hole:** two unrelated law classes silently merge into one class name, so query joins between fields the laws never unified wrongly pass at both the type tier (`JoinOk`) and the runtime twin (`fieldJoins`), with no engine backstop.
2. **Tier drift:** a lawful schema with two independent `fresh` generators whose coordinates alias throws the one-generator wall at runtime with the nonsense message "A.B.id and A.B.id", while `tsc --noEmit` exits 0.

This is an "illegal state left representable" violation of the project's representation-first doctrine (docs/design/representation-first.md): the flat string coordinate is a representation whose collision class a character ban (or a pair key) would erase.

### Evidence (all verified against the working tree, 2026-07-23)

**The encoding.**
- `ts/src/law.ts:432` — runtime seeding: `` const coord = `${member.relation}.${field.name}` ``
- `ts/src/law.ts:451-452` — pair union: `` const coordA = `${source.owner.name}.${fieldName}` `` / `` const coordB = `${target.owner.name}.${targetField}` ``
- `ts/src/law.ts:466, 471, 479` — class naming and map minting reuse the same template.
- `ts/src/law.ts:138` — type tier `ZipCoords`: `` readonly [`${SN}.${SH}`, `${TN}.${TH}`] ``; `ts/src/law.ts:304` — `` ClassOfCoord<Comps, Gens, `${N}.${F}`> ``. Both tiers share the collision.

**No name grammar anywhere.**
- `ts/src/fields.ts:250-256` — `assertDeclarationOrderKey` tests only `/^(?:0|[1-9][0-9]*)$/` (the ECMA-262 integer-index reorder hazard). It is the only name judgment: `relation.ts:203` (fields), `closed.ts:388/392/443` (columns, handles), `schema.ts:32` (relation record keys). Relation names passed to `relation(name, …)` are judged by nothing at all.
- `crates/bumbledb/src/schema/validate.rs:90` (duplicate relation names) and `:1405-1412` (duplicate field names) are the engine's only name checks — no identifier grammar, so the engine admits `"A.B"` and re-judges nothing (its query judgment compares structural value types only; both aliased slots are u64).

**Executed repro (a) — class merge.** `relation("A.B", { x: u64 })`, `relation("A", { "B.x": u64 })`, plus `contained(on(P,"p"), on(AB,"x"))` and `contained(on(Q,"q"), on(A,"B.x"))` — two containments with no law connecting them — produced:

```
classes: {"A.B":{"x":"A.B.x"},"A":{"B.x":"A.B.x"},"P":{"p":"A.B.x"},"Q":{"q":"A.B.x"}}
```

One class for two independent laws. I also confirmed the TYPE tier merges identically: a probe asserting mutual assignability of `(typeof t)["classes"]["P"]["p"]` and `(typeof t)["classes"]["Q"]["q"]` typechecked (`tsc --noEmit` exit 0) — at the type tier the two components `["P.p","A.B.x"]` and `["Q.q","A.B.x"]` union through the shared aliased coordinate exactly as the runtime does.

**The class name is the whole join judgment.** `ts/src/query/scope.ts:300` (`JoinOk`) and its runtime twin `ts/src/query/scope.ts:326-340` (`fieldJoins`, `a.class === b.class`) compare kind/width/element/roster/class — the class string off this map and nothing else. A `find()` variable minted at `P.p` therefore wrongly joins `Q.q` at both tiers, and the engine has no domain notion to catch it.

**Executed repro (b) — spurious wall + tier drift.** The fresh variant — `relation("A.B", { id: u64.fresh })`, `relation("A", { "B.id": u64.fresh })`, two containments into the two DIFFERENT generators — threw at construction:

```
schema T2: the statements unify two generators into one class — A.B.id and A.B.id (two mints cannot share a carrier) — C(x) <= A.B(id)
```

while the identical schema passed `tsc --noEmit` (exit 0). Mechanism: at the type tier the two generator coordinates are the SAME string literal, so `IsMulti<Extract<H, Gens>>` (law.ts:221, 254) sees one union member and the `ClassWall` never fires; at runtime `markGenerator` (law.ts:397-400) is called twice on the one aliased coordinate, the roster reaches length 2, and the first union throws (law.ts:456-461) — naming the same coordinate twice.

**Doc check.** docs/architecture/70-api.md (§ the `schema!` grammar) spells fields as `name: type` — Rust identifiers, which cannot contain `.`. The TS SDK is the only surface that can mint dotted names, so banning the character is exact macro parity, not a new restriction. law.ts's own header comment ("the two tiers are the same computation by construction", law.ts:4-7) is violated by repro (b).

### Failure scenario

```ts
const AB = relation("A.B", { x: u64 })
const A  = relation("A",   { "B.x": u64 })
schema("T", { "A.B": AB, A, P, Q }, [
  contained(on(P, "p"), on(AB, "x")),
  contained(on(Q, "q"), on(A, "B.x"))
])
```

`P.p` and `Q.q` receive the SAME class `"A.B.x"`, so a query variable minted at `P.p` joins `Q.q` at both tiers despite no law relating them — silent wrong-typechecking queries. With `fresh` marks on the aliased coordinates, a lawful schema that typechecks clean is REFUSED at runtime by the one-generator wall with a message naming one coordinate twice.

### Suggested fix

Make the illegal state unrepresentable at the one name-judging seam that already exists: extend `assertDeclarationOrderKey` (ts/src/fields.ts:250) to also reject `.` in the name, and route relation names through it too (today `relation(name, …)` and `closed(name, …)` judge their own name by nothing). This is exact macro parity — Rust identifiers cannot contain dots — and it makes the `${relation}.${field}` template injective by construction at BOTH tiers. (Keying the runtime union-find on pairs would fix only the value tier; the type tier's template-literal coordinates at law.ts:138/304 need the character ban regardless.) A matching grammar check in `validate.rs` would close the hand-built-descriptor path defense-in-depth, but the SDK seam is the load-bearing fix since the class map exists only at the TS tier.
