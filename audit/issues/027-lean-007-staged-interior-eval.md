# lean-007: `evalInteriorsAt` is a `Nat`-staged rebuild with a `none => False` arm, not a fold

- **Severity:** medium
- **Tree:** lean
- **Status:** FIXED(9b21ea6a)
- **Source:** audit/lean.md M1
- **Depends on:** none (textually conflicts with lean-001 in `Exec/Reach.lean`; coordinate)

## The bug

`lean/Bumbledb/Exec/Reach.lean:132-144` — the Level 0 interior semantics is indexed by a stage `Nat` and re-derives "which interior am I" by arithmetic at every lookup:

```lean
def evalInteriorsAt (C : Classify) (defs : List Interior) (I : Instance)
    (ρ : ParamEnv) : Nat → InteriorEnv
  | 0 => InteriorEnv.empty
  | n + 1 =>
    let prev := evalInteriorsAt C defs I ρ n
    fun c t =>
      if h : c.id < n then
        prev c t
      else if c.id = n then
        match defs[n]? with
        | some d => t ∈ rulesAnswers C d.rules (sourceDen I prev) ρ
        | none => False
      else False
```

The `defs[n]?` partial lookup and its `none => False` arm exist only because the stage index can exceed the list. A stack of stage-bookkeeping lemmas exists purely to discharge that coordinate: `evalInteriorsAt_stable` (154-180), `evalInteriorsAt_agree_prefix` (798-806), `evalInteriorsAt_out` (811-830), and the 56-line trichotomy proof `evalInteriorTables_step` (833-888) reconciling the staged spec with the Level 1 fold. Meanwhile Level 1 already IS the structural fold (`evalInteriorTables.go`, 759-768).

## Why it's wrong

The declaration-order dependency is structural in the list; encoding it as a `Nat` cursor over the list makes "stage out of range" and "stage vs. id mismatch" representable, then spends ~150 lines proving those states never matter (Insight 5: a special case in the wrong coordinate multiplies proofs). The `none => False` arm is a guard for a state the structural fold cannot express.

## The fix

Per `audit/CONTRACT.md §C4` ("Denotation"): make Level 0 a structural fold in declaration order, mirroring Level 1's shape:

```lean
def evalInteriorsFold (C : Classify) (I : Instance) (ρ : ParamEnv) :
    Nat → List Interior → InteriorEnv → InteriorEnv
  | _, [], W => W
  | i, d :: ds, W =>
      evalInteriorsFold C I ρ (i + 1) ds
        (W.update ⟨i⟩ (fun t => t ∈ rulesAnswers C d.rules (sourceDen I W) ρ))

def evalInteriors (C : Classify) (q : Query) (I : Instance) (ρ : ParamEnv) :
    InteriorEnv :=
  evalInteriorsFold C I ρ 0 q.interiors InteriorEnv.empty
```

(The `i` counter names each interior's publish slot — dense ids stay per §C5; there is no partial lookup and no `False` arm.)

- DELETE `evalInteriorsAt`, `evalInteriorsAt_zero`, `evalInteriorsAt_stable`, `evalInteriorsAt_agree_prefix`, `evalInteriorsAt_out`, and rebuild `evalInteriorTables_sound` as a straight two-fold induction (Level 1's `go` and Level 0's fold now have the SAME recursion structure, so `evalInteriorTables_step`'s trichotomy collapses).
- A later or out-of-range interior read is empty because the env was never updated there — same phantom semantics, now by construction of the fold rather than by a `False` arm.

## Acceptance criteria

- [x] Gone: `rg -nw 'evalInteriorsAt|evalInteriorsAt_stable|evalInteriorsAt_out|evalInteriorsAt_agree_prefix' lean` → no matches; `rg -n 'none => False' lean/Bumbledb/Exec/Reach.lean` → no matches.
- [x] Unchanged: `evalQuery`'s value on every input (the fold computes the same env — provable, and witnessed by the 268-case conformance staying green); `evalInteriorTables_sound` survives under that name relating Level 1 to the new Level 0.
- [x] Commands green: `cd lean && lake build`; `lake exe conformance conformance/cases` (268, 0); no `sorry`/`admit`.

## Constraints

- Semantics identical (phantom reads preserved); dense `InteriorId` stays (§C5 — no `Fin`-indexing).
- Coordinate with lean-001 (same file, `evalQuery` body); land either order but expect merge friction.
