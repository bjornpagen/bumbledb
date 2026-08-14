# lean-021: Membership and key-probe collapse `AtomSource` to `RelId ⟨0⟩`

- **Severity:** high
- **Tree:** lean
- **Status:** FIXED(a73bbe07)
- **Source:** audit/lean-rest.md H1
- **Depends on:** none for the ⟨0⟩ kill / key-probe (membership stays EDB). Coordinate with lean-022 (`keyProbeEval` / Plan take the same `F`).
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

`Typing.membership` returns `Bool`; every lemma re-tests it. A fourth reconstruction of "interiors have no RelId": `Conformance.scalarAnchored` (`Conformance.lean:539-541`) takes `| .interior _ => false` (never interval) — the recorded "interior membership is engine-only" narrowing (`Syntax.lean:48-49`) — while `SurfaceMatches` still consults relation 0's header. Two different lies for one constructor.

## Why it's wrong

Insight 4: two identities glued by a sentinel (`⟨0⟩`) admit states that are not the language (interior membership against the wrong header; interior key-probe licensed by an unrelated stored key). Insight 5: absence of a `RelId` is stuffed into every interior atom as a value, so every consumer branches. Insight 6: membership status is validated as a `Bool` and discarded, so the check repeats at every lemma instead of living in a parsed binding type. Syntax.lean:100-103 already recorded that statements cannot name interiors — key-probe then mints a fake `Statement.functionality ⟨0⟩` to dodge that law.

## The fix

Per `audit/CONTRACT.md §C4` (one `AtomSource`) and the recorded narrowing `Syntax.lean:48-49` ("Membership stays EDB … Interior membership is engine-only"):

**Do not invent Lean interior membership.** Keying `Header.fieldType` / `Typing.membership` by `AtomSource` and typing interiors against derived heads would add a denotation the spec recorded as engine-only and would disagree with `scalarAnchored`'s `interior _ => false`. That is a semantic change.

- Membership: kill the `⟨0⟩` sentinel. Interior `SurfaceMatches` is value-equality (`Matches`) — membership-free, same reading as `lowerNegated`'s current `filters := []` arm and as `scalarAnchored`. `Header` / `Typing.membership` stay `RelId`-indexed. Theorems already carry `hedb : ∃ R, a.source = .edb R` (`surfaceMatches_iff_antiMatches`); keep that gate. A parsed value-vs-membership binding form on the **EDB** arm is in scope (Insight 6) but must not grow an interior field-type observer.
- `AntiOccurrence`: stop stuffing `relation := ⟨0⟩` on interiors. Sum or equivalent: EDB carries `RelId` + domain + filters; interior is membership-free domain (anti-join against the interior `AtomSource`, not against stored relation 0). `AntiMatches` on the EDB arm stays as today. `AntiOccurrence.rejects` must not take a `RelId` and an `AtomSource` that can disagree.
- `KeyProbeShape.declared` is `a.source = .edb R ∧ Statement.functionality R K ∈ T.statements`. Interior key-probe is unrepresentable (stored-relation probe; Syntax.lean:100-103). `keyProbeEval` takes the same `F : AtomSource → Set Fact` as `ruleAnswers` — `InteriorTables.empty` as a mode bit dies. This half is the dual-coordinate kill; it does not add interior key-probe.

`groundSplit`'s `interior _ => none` (Rewrites.lean:432) stays: interiors are not closed EDB extensions. That match is the sum, not a collapse. `Conformance.scalarAnchored`'s `interior _ => false` stays (it is the narrowing, not a third sentinel).

## Acceptance criteria

- [x] Gone: `rg -n 'interior _ => ⟨0⟩' lean/Bumbledb` → no matches; `rg -n 'Statement.functionality ⟨0⟩' lean` → no matches; `rg -n 'InteriorTables.empty' lean/Bumbledb/Exec/Rewrites.lean` → no matches in `keyProbeEval` / `keyprobe_equiv_join`. `AntiOccurrence` has no interior `relation : RelId` (sum, or `source : AtomSource` with filters empty on interior).
- [x] Unchanged: `membership_lowering_preserves` / `_negated` still EDB-only (same surface = lowered content); `keyprobe_equiv_join` still probe = join under a stored key. `scalarAnchored` still `interior _ => false`. No new interior-membership theorems. 268-case conformance green; corpus frozen.
- [x] Commands green: `cd lean && lake build`; `lake exe conformance conformance/cases` (268, 0); `./scripts/lean.sh` fully green (build + battery + census + corpus + comparator). No `sorry`/`admit`.

## Constraints

- Semantics identical: bivalent membership on stored interval fields unchanged; interior bindings in Lean stay value-equality (engine-only membership is not newly modeled). Phantom interior reads stay empty until a real interior environment is supplied (C5: identities stay dense `Nat`; no Fin-telescope).
- No C5 split: dual coordinate dies; do not Fin-index `Header.sig`; do not add derived-head interval types.
- No Program vocabulary. Key-probe remains a stored-relation fast path (C3 engine `KeyProbe` is EDB). Lean-022 parameterizes Plan with the same `F`.
- Must not weaken `keyprobe_equiv_join`'s key uniqueness or `Safe` premises.
