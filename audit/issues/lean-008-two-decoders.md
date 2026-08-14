# lean-008: two decoders, two query types, two evaluators for one language

- **Severity:** medium
- **Tree:** lean
- **Status:** OPEN
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

- ONE atom decoder: `relation` (seeded spelling) and `edb`/`interior` (reach spelling) are two JSON spellings of the one `AtomSource` — a single `decodeAtom` accepts `relation` OR `edb`/`interior` keys and returns `Query.Atom`. Corpus unchanged.
- ONE query type: the seeded lane decodes into `Query` (`.cq` arm after lean-001). Aggregate/measure head shapes (`CFind.agg`/measures) are the real gap — they are head-level, not rule-body-level: keep the head-shape layer as a thin wrapper AROUND `Query` (heads + the `Query` it projects), not a parallel query with its own rules/atoms/conditions. `plainQuery`'s lossy `filterMap` dies; plain-projection cases feed `evalQueryList` directly.
- ONE evaluator entry: the conformance `evalQuery` (`Conformance.lean:946`) is renamed (it shadows the denotation's name) and becomes dispatch: plain-head cases → the PROVED `evalQueryList`; aggregate/measure cases → the recorded glue over the same join states, applied to the one query type. The DNF/union regimes keep their recorded rulings.
- `Main.lean`'s reach lane reuses the shared decoder module; `decodeReachAtom`/`decodeAtom` merge.

## Acceptance criteria

- [ ] Gone: `rg -nw 'CQuery|plainQuery|decodeReachAtom' lean --glob '!conformance/cases/**'` → no matches (head-shape wrapper may keep `CFind` for find-shapes only — it must not carry rules/atoms); `rg -n 'def evalQuery' lean/Bumbledb/Conformance.lean` → no match under that name.
- [ ] One decoder: `rg -n 'relation' lean/Bumbledb/Conformance.lean lean/Main.lean` shows one shared atom-decode site.
- [ ] Unchanged: ALL 268 cases green — 246 seeded + 22 reach — with corpus byte-identical; the plain-projection cases now exercise the proved `evalQueryList` (this is the point: strictly MORE cases flow through the proved path).
- [ ] Commands green: `cd lean && lake build`; `lake exe conformance conformance/cases` (268, 0); `./scripts/lean.sh` fully green (build + battery + census + corpus + comparator).

## Constraints

- Corpus frozen. Aggregate/measure semantics identical (the recorded glue moves, not changes). DNF regime ruling (2026-07-23 R2) preserved verbatim.
- No Program vocabulary.
