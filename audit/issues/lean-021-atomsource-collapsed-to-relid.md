# lean-021: Membership and key-probe collapse `AtomSource` to `RelId ⟨0⟩`

- **Severity:** high
- **Tree:** lean
- **Status:** OPEN
- **Source:** audit/lean-rest.md H1
- **Depends on:** lean-001, lean-002 (interior field width is `finds.length` after lean-006; membership typing of interiors needs that)
- **Conflicts with:** none (Membership.lean / Rewrites.lean KeyProbeShape were not in wave 1)

## The bug

The cut's atom is a sum (`AtomSource = edb RelId | interior InteriorId`). Membership typing and key-probe acceptance never learned the second constructor.

`lean/Bumbledb/Query/Membership.lean:331-333` — every interior atom is stored relation 0:

```lean
def SurfaceMatches (Γ : Typing) (f : Fact) (a : Atom) (σ : Assignment)
    (ρ : ParamEnv) : Prop :=
  ∀ b, b ∈ a.bindings → Term.selectsAt Γ ρ σ (match a.source with | .edb R => R | .interior _ => ⟨0⟩) b.1 b.2 (f b.1)
```

That match is cloned through `surfaceMatches_iff_occurrence`, `Atom.membershipFree`, `lowerAtoms`, `stepLower`, `memCount`, and `surfaceMatchesB` (~40 sites). `Typing.membership` / `Header.fieldType` (`Membership.lean:120-121`, `176-197`) are `RelId`-indexed, so the collapse is forced by the type.

`lean/Bumbledb/Query/Membership.lean:1477-1484` — the negated lowering is a *different* reconstruction of the same illegal identification:

```lean
def Atom.lowerNegated (Γ : Typing) (a : Atom) : AntiOccurrence :=
  match a.source with
  | .edb R =>
    { relation := R
      domain := a.bindings.filter fun b => !(Γ.membership R b.1 b.2)
      filters := a.bindings.filter fun b => Γ.membership R b.1 b.2 }
  | .interior _ =>
    { relation := ⟨0⟩, domain := a.bindings, filters := [] }
```

`AntiOccurrence.relation : RelId`. Positive interiors consult relation 0's header; negated interiors become membership-free (filters emptied). Two lies for one constructor.

`lean/Bumbledb/Exec/Rewrites.lean:1286-1288` and `1302-1304` — key-probe is a third:

```lean
  declared : (match a.source with
      | .edb R => Statement.functionality R K
      | .interior _ => Statement.functionality ⟨0⟩ K) ∈ T.statements
```

```lean
def keyProbeEval ... :=
  match (factsOf W InteriorTables.empty a.source).find? (probeHitB ρ a K) with
```

An interior singleton-atom rule is "declared" iff relation 0 happens to carry key `K`, then evaluated against empty interior tables. `keyprobe_equiv_join` (`Rewrites.lean:1367-1369`) then equals `ruleAnswers` under `edbEnv`.

`Typing.membership` returns `Bool`; every lemma re-tests it.

## Why it's wrong

Insight 4: two identities glued by a sentinel (`⟨0⟩`) admit states that are not the language (interior membership against the wrong header; interior key-probe licensed by an unrelated stored key). Insight 5: absence of a `RelId` is stuffed into every interior atom as a value, so every consumer branches. Insight 6: membership status is validated as a `Bool` and discarded, so the check repeats at every lemma instead of living in a parsed binding type. Syntax.lean:100-103 already recorded that statements cannot name interiors — key-probe then mints a fake `Statement.functionality ⟨0⟩` to dodge that law.

## The fix

Per `audit/CONTRACT.md §C4` (one `AtomSource`; interiors are ordinary data) and §C7 (no Program/EDB-only coordinate after the parse):

- `Typing.membership` / `Header.fieldType` (or a derived-head observer) keyed by `AtomSource`. Interior field types come from the derived head after lean-002/006 (`finds.length` / the body's bindings), never `Header.sig ⟨0⟩`.
- `AntiOccurrence.source : AtomSource`. Delete `relation : RelId`. The interior arm of `lowerNegated` is the same partition as the edb arm, against the interior's field types.
- `KeyProbeShape.declared` is `a.source = .edb R ∧ Statement.functionality R K ∈ T.statements`. Interior key-probe is unrepresentable (stored-relation probe). `keyProbeEval` takes the same `F : AtomSource → Set Fact` as `ruleAnswers` — `InteriorTables.empty` as a mode bit dies.
- Membership `Bool` screen becomes a parsed binding form (value vs membership) so `SurfaceMatches` does not re-test `Γ.membership` at every site. The `match a.source | .interior _ => ⟨0⟩` family is gone.

`groundSplit`'s `interior _ => none` (Rewrites.lean:432) stays: interiors are not closed EDB extensions. That match is the sum, not a collapse.

## Acceptance criteria

- [ ] Gone: `rg -n 'interior _ => ⟨0⟩' lean/Bumbledb` → no matches; `rg -n 'AntiOccurrence' -A6 lean/Bumbledb/Query/Membership.lean` shows `source : AtomSource` (no `relation : RelId`); `rg -n 'Statement.functionality ⟨0⟩' lean` → no matches; `rg -n 'InteriorTables.empty' lean/Bumbledb/Exec/Rewrites.lean` → no matches in `keyProbeEval` / `keyprobe_equiv_join`.
- [ ] Unchanged: `membership_lowering_preserves`, `membership_lowering_preserves_negated`, `keyprobe_equiv_join` survive restated with the same mathematical content (surface = lowered; probe = join under a stored key). 268-case conformance green; corpus frozen.
- [ ] Commands green: `cd lean && lake build`; `lake exe conformance conformance/cases` (268, 0); `./scripts/lean.sh` fully green (build + battery + census + corpus + comparator). No `sorry`/`admit`.

## Constraints

- Semantics identical: bivalent membership on stored interval fields unchanged; phantom interior reads stay empty until a real interior environment is supplied (C5: identities stay dense `Nat`; no Fin-telescope).
- No C5 split: dual coordinate dies; do not Fin-index `Header.sig`.
- No Program vocabulary. Key-probe remains a stored-relation fast path (C3 engine `KeyProbe` is EDB). Lean-022 parameterizes Plan the same way; land membership/key-probe first or together.
- Must not weaken `keyprobe_equiv_join`'s key uniqueness or `Safe` premises.
