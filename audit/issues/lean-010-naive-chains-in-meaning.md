# lean-010: `naiveIter`/`semiNaiveIter` are engine-shaped mechanism living in the meaning module

- **Severity:** medium
- **Tree:** lean
- **Status:** OPEN
- **Source:** audit/lean.md M4
- **Depends on:** lean-002 (reachOp's signature changes under it)

## The bug

`lean/Bumbledb/Exec/Reach.lean:266-318` defines two iteration *strategies* beside the denotation:

```lean
def naiveIter {α : Type u} (T : Set α → Set α) : Nat → Set α
  | 0 => fun _ => False
  | k + 1 => fun a => naiveIter T k a ∨ T (naiveIter T k) a

def semiNaiveIter {α : Type u} (T : Set α → Set α) :
    Nat → Set α × Set α
  ...
theorem semi_naive_agrees {α : Type u} (T : Set α → Set α) :
    ∀ k, (semiNaiveIter T k).1 = naiveIter T k
```

Neither is used by `reachDen` (`lfpS`, line 203-205), by `evalLinearReach` (the fueled loop, 489-493), nor by any agreement proof — `evalLinearReach_eq_lfp` (659-734) goes straight from the fueled loop to `lfpS` without touching either chain. Their only consumers are two `Bridge.lean` rows (`Bridge.lean:588-596`) citing `semi_naive_agrees` as the model-side warrant for the engine's delta rewrite.

## Why it's wrong

The meaning module hosts three ways to compute one fixpoint, two of which are decorative here: `reachDen = lfpS` is the ONE meaning (the file header says so, line 6-7), and strategy-agreement is an engine-correctness fact, not a denotation fact. Keeping mechanism beside meaning invites citing the wrong thing (Insight 1: meaning and mechanism in one namespace blur which is normative), and the duplicated Bridge row (two rows on the same theorem, 588 and 593) is the drift already showing.

## The fix

Per `audit/CONTRACT.md §C4` ("`naiveIter`/`semiNaiveIter` leave the meaning module; `reachDen = lfpS` is the one meaning"):

- MOVE `naiveIter`, `semiNaiveIter`, `semiNaive_delta`, `semi_naive_agrees`, `semi_naive_same_fixpoint` (and `setExt` if unused elsewhere) out of `Exec/Reach.lean` into a dedicated mechanism file (suggested: `lean/Bumbledb/Exec/SemiNaive.lean`), importing the operator definitions. Statements unchanged.
- Namespaces stay `Bumbledb.Query` so the Bridge symbol names survive; the two Bridge rows update their citation text if the census keys on file paths, and the DUPLICATE pair merges into rows with distinct claims (one row = naive is enough for the model; one row = one-delta-per-arm walks the chain) or stays two rows with distinct mechanism columns — but each must cite the file it now lives in.
- `Exec/Reach.lean`'s header note ("Level 1 … proved equal to Level 0") stops implying the chains are part of the reach story.

## Acceptance criteria

- [ ] Moved: `rg -nw 'naiveIter|semiNaiveIter' lean/Bumbledb/Exec/Reach.lean` → no matches; definitions exist in the new mechanism file with statements textually unchanged.
- [ ] Bridge honest: `./scripts/spec-census.sh` green; both `semi_naive_agrees` rows resolve.
- [ ] Commands green: `cd lean && lake build`; `lake exe conformance conformance/cases` (268, 0).

## Constraints

- Pure motion + doc edits; zero statement changes. No assertion weakened; no Bridge row deleted.
