## Type-tier roster slot compares handle-name unions; runtime compares roster identity — well-typed statements throw at construction

category: incoherence | severity: medium | verdict: CONFIRMED | finder: ts:types
outcome: fixed fb7e5073

### Summary

The TypeScript SDK's type tier and runtime tier disagree on what "same vocabulary" means for closed references. The type tier — both `ShapeOf`'s roster slot (`ts/src/face.ts:135`, feeding `SameShapes` for containment/window statements) and `RosterOf` (`ts/src/query/scope.ts:285-289`, feeding `JoinOk` for query joins) — extracts only the handle-name UNION from a `ClosedIdField`. The runtime twins judge roster VALUE IDENTITY (`ts/src/statements.ts:158` `sourceRoster !== targetRoster`; `ts/src/query/scope.ts:338` `rosterA === rosterB`). Two distinct vocabularies sharing a handle set are therefore identical at the type tier but distinct at runtime: a well-typed statement compiles with zero errors and throws at construction.

This inverts the codebase's own contract. Both runtime asserts document themselves as "the runtime twin" of the type-tier judgment (`statements.ts:135` — "The runtime twin of {@link SameShapes}'s roster slot"; `scope.ts:319` — "The runtime twin of {@link JoinOk}"), i.e. a backstop for untyped callers. Here the runtime is strictly stronger than the type for WELL-TYPED callers — the compile-tier wall the machinery already has a slot for is demoted to a runtime surprise.

The gap has one root cause: `ClosedRoster` (`ts/src/fields.ts:59-62`) declares `name: string` wide, discarding the one literal that distinguishes same-shaped vocabularies — even though `mintClosed` (`ts/src/closed.ts:423`, `:440`, `:465`) has the `Name extends string` literal in hand when it freezes `{ name, handles }` into the roster and builds `ClosedIdField<Handles>`.

### Evidence (all verified against the code, and by execution)

- `ts/src/face.ts:131-136` — `ShapeOf`'s fourth slot: `F extends { readonly closed: { readonly handles: readonly (infer H extends string)[] } } ? H : undefined`. Only `handles` is read; the roster's `name` never enters the comparand.
- `ts/src/query/scope.ts:285-289` — `RosterOf` is the identical handles-only extraction; `JoinOk` (`:300-316`) compares kind/class/width/element/roster. Two lawless closed references both carry class `undefined`, so the class slot does not distinguish them either.
- `ts/src/statements.ts:150-164` — `assertRosterAgreement` compares `rosterOf(...)` results with `!==` and throws; applied by `contained`/window constructors at `:215`, `:240`, `:267`. Doc comment at `:135` names it the runtime twin of `SameShapes`'s roster slot.
- `ts/src/query/scope.ts:326-340` — `fieldJoins` compares `rosterA === rosterB` (doc at `:319-321`: "the roster by VALUE IDENTITY").
- `ts/src/fields.ts:59-62` — `interface ClosedRoster<H extends string = string> { readonly name: string; readonly handles: readonly H[] }`; `ts/src/fields.ts:139-142` — `ClosedIdField<H>` carries no name parameter.
- `ts/src/closed.ts:440` — `const roster: ClosedRoster<Handles> = Object.freeze({ name, handles: handleList })` and `:465` — `const id: ClosedIdField<Handles> = Object.freeze({ kind: "u64", closed: roster })`: the `Name` literal is present in `mintClosed`'s type parameters (`:423`) and dropped at both construction sites.
- **Executed repro** (verifier-run): `closed("Kind", ["Yes","No"])`, `closed("Answer", ["Yes","No"])`, `relation("R", { k: Kind.id })`, then `contained(on(R,"k"), on(Answer,"id"))`. `tsc --noEmit` produced zero diagnostics for this file; running it threw at construction: `Error: R.k is a Kind reference but Answer.id is a Answer reference — closedness rides the descriptor ... — R(k) <= Answer(id)` from `assertRosterAgreement`.

Doctrine cross-check: this is not a nominal-typing request — under the hard-structural-typing doctrine the vocabulary's name IS part of its encoding (the runtime roster value carries `{ name, handles }`, and `schema()` names the generator class `"Kind.id"` off it, `fields.ts:133-134`). The type tier currently encodes only half the structure the runtime compares; carrying the name literal makes the type the faithful encoding of the value, per `docs/design/representation-first.md`'s representation-over-control-flow lens (the compile-time representation should erase this runtime throw for typed callers).

### Failure scenario

Any pair of vocabularies sharing a handle set — `Yes/No` polar answers, `Low/Medium/High` bounded scales duplicated across domains — where a containment, window, or query join pairs a reference to one against the other's id: the program typechecks clean, then throws `...is a Kind reference but ... is a Answer reference...` at statement construction (or a join refusal in the rule builders). The error is deterministic and early, so no data corruption — but a statically-decidable mismatch that the type machinery already has a comparand slot for surfaces only at runtime.

### Suggested fix

Carry the vocabulary name as a literal through the roster type: `ClosedRoster<Name extends string, H extends string>` with `readonly name: Name` (and `ClosedIdField<Name, H>` accordingly) — `mintClosed` already holds the `Name` literal at `closed.ts:423`. Then read the name into both type-tier comparands: `face.ts` `ShapeOf`'s roster slot and `query/scope.ts` `RosterOf` become e.g. `readonly [Name, H]` instead of bare `H`. Distinct same-shaped vocabularies then mismatch at compile time, matching the runtime identity judgment exactly. The only residual runtime-only case is a same-name forgery (two rosters minted with the same name), which is already owned at schema membership: `verifyMembership` (`ts/src/schema.ts:65`, applied at `:301`) — so the type/runtime twin contract closes fully for typed callers.
