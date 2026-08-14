# lean-008: two decoders, two query types, two evaluators for one language

- **Severity:** medium
- **Tree:** lean
- **Status:** FIXED(3d42fdeb)
- **Source:** audit/lean.md M2
- **Depends on:** lean-001, lean-002 (decodes into their types)

## The bug

The conformance lane decodes the SAME language twice into DIFFERENT types. The seeded lane (`lean/Bumbledb/Conformance.lean:383-390`) parses atoms with a `relation` key into a local `CQuery`:

```lean
def decodeAtom (j : Json) : Except String Query.Atom := do
  let relation ← natKey j "relation"
  ...
  return { source := .edb ⟨relation⟩, bindings }
```

The reach lane (`lean/Main.lean:352-364`) parses `edb`/`interior` keys into the real `Query.Atom`:

```lean
  let source : Query.AtomSource ←
    if let some r := objKey? j "edb" then
      pure (.edb ⟨← r.getNat?⟩)
    else if let some c := objKey? j "interior" then
      pure (.interior ⟨← c.getNat?⟩)
    else .error "reach atom expects edb or interior"
```

And `Conformance.lean` carries a SECOND `evalQuery` over `CQuery` (`Conformance.lean:946-973`) plus a projection shim back to the real type (`Conformance.lean:927-930`):

```lean
def plainQuery (q : CQuery) : Query.Query :=
  Query.Query.plain ((q.rules.head?.map (·.finds.length)).getD 0)
    (q.rules.map fun r =>
      { r.body with finds := r.finds.filterMap CFind.plainVar? })
```

## Why it's wrong

One language, two representations: `CQuery` is a parallel query type with its own rule shape (`CFind` heads), its own evaluator, and a lossy projection (`filterMap CFind.plainVar?` silently DROPS aggregate heads) into the proved type. Everything proved about `evalQuery` covers the reach lane only; the seeded lane's evaluator is unverified code that merely *resembles* the denotation (Insight 2: two representations of one thing WILL drift; Insight 6: `plainQuery` throws away what decoding established).

## The fix

Per `audit/CONTRACT.md §C4` ("One decoder"):

- ONE atom decoder: `relation` (seeded spelling) and `edb`/`interior` (reach spelling) are two JSON spellings of the one `AtomSource` — a single `decodeAtom` accepts `relation` OR `edb`/`interior` keys and returns `Query.Atom`. Corpus unchanged. `Main.lean`'s `decodeReachAtom` merges into that function.
- ONE body type: seeded cases decode into `Query` (`.cq` after lean-001; interiors empty). Do **not** stuff aggregate finds into `Rule.finds` (`List VarId` — recorded PRD 04 narrowing). Keep a thin wrapper around that `Query` that carries (1) per-rule head shapes (`CFind` / successor), (2) the R2 `dnf : Bool` mark, (3) optional surface `width`. Those three are not in `Query` and must not be deleted. Renaming `CQuery` is fine; deleting the wrapper is not — `evalUnion` / `dnfBindings` / `ruleWidth` live on it.
- DELETE `plainQuery` (`Conformance.lean:927-930`). It is dead (no callers) and its `filterMap CFind.plainVar?` would drop aggregate heads if anyone used it.
- ONE evaluator *name*: rename Conformance's `evalQuery` (`:946`) so it no longer shadows the denotation. Dispatch stays:
  - **Do not pipe every plain-projection case through `Query.evalQueryList`.** `ruleStates` (`:559-567`) uses `joinAtoms` (pre-lowered `Matches`) on positives and **surface** anti-join (`surfaceMatchesB`) on negated atoms, including membership. `evalList` uses `Matches` on both polarities. The module doc (`Conformance.lean:15-25`) records that they coincide only on membership-free negation; `eval_sound` names that fragment. Blindly switching to `evalQueryList` changes negated-membership answers (corpus includes them). Membership-free-negation plain cases MAY call `evalQueryList`; negated-membership cases keep `ruleStates` / `evalPlain` (or lower then `evalQueryList`). Aggregate/measure cases keep the recorded glue over the same join states. DNF/union regimes (R2) unchanged.
- Reach cases (`Main.lean` `checkReachCase`) already run `Query.evalQueryList` on the proved type — keep that; share the atom decoder only.

## Acceptance criteria

- [x] Gone: `rg -nw 'plainQuery|decodeReachAtom' lean --glob '!conformance/cases/**'` → no matches; `rg -n 'def evalQuery' lean/Bumbledb/Conformance.lean` → no match under that name. A wrapper type may remain (heads + `dnf` + `width` + the `Query` body); it must not duplicate atom/condition/rule-body types.
- [x] One decoder: one function accepts `relation` or `edb`/`interior`.
- [x] Unchanged: ALL 268 cases green — 246 seeded + 22 reach — corpus byte-identical. Negated-membership seeded cases still use the surface anti-join (or an explicit lowering). Membership-free plain cases may use `evalQueryList`.
- [x] Commands green: `cd lean && lake build`; `lake exe conformance conformance/cases` (268, 0); `./scripts/lean.sh` fully green (build + battery + census + corpus + comparator).

## Constraints

- Corpus frozen. Aggregate/measure semantics identical (the recorded glue moves, not changes). DNF regime ruling (2026-07-23 R2) preserved verbatim — the wrapper keeps `dnf` and `width`.
- Do not treat `evalQueryList` as a drop-in for `evalPlain` on the membership roster.
- No Program vocabulary. Do not re-file CQuery-as-second-decoder under a new id.
