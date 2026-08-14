# lean-010: `naiveIter`/`semiNaiveIter` are engine-shaped mechanism living in the meaning module

- **Severity:** medium
- **Tree:** lean
- **Status:** FIXED(b10d9df5)
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

Neither is used by `reachDen` (`lfpS`, line 203-205), by `evalLinearReach` (the fueled loop, 489-493), nor by any agreement proof — `evalLinearReach_eq_lfp` (659-734) goes straight from the fueled loop to `lfpS` without touching either chain. Bridge cites `semi_naive_agrees` twice (`Bridge.lean:588-596`) as the model-side warrant for the engine's delta rewrite. **Also:** `Countermodels.lean:1519, 1532-1533` uses `Query.naiveIter succOp` for the successor-chain walls (`succ_chain_ascends` / the infinite-prefix argument). Those theorems are operator-level and must keep compiling after the move.

## Why it's wrong

The meaning module hosts three ways to compute one fixpoint, two of which are decorative here: `reachDen = lfpS` is the ONE meaning (the file header says so, line 6-7), and strategy-agreement is an engine-correctness fact, not a denotation fact. Keeping mechanism beside meaning invites citing the wrong thing (Insight 1: meaning and mechanism in one namespace blur which is normative), and the duplicated Bridge row (two rows on the same theorem, 588 and 593) is the drift already showing.

## The fix

Per `audit/CONTRACT.md §C4` ("`naiveIter`/`semiNaiveIter` leave the meaning module; `reachDen = lfpS` is the one meaning"):

- MOVE `naiveIter`, `semiNaiveIter`, `semiNaive_delta`, `semi_naive_agrees`, `semi_naive_same_fixpoint` out of `Exec/Reach.lean` into a dedicated mechanism file (suggested: `lean/Bumbledb/Exec/SemiNaive.lean`). Statements unchanged. Namespace stays `Bumbledb.Query` so `Query.naiveIter` / `@Query.semi_naive_agrees` keep resolving.
- `setExt` in Reach.lean:262 is only spent by `semi_naive_agrees` — move it with the chains. Do NOT touch Plan.lean's private `setExt` (`Plan.lean:345`).
- Countermodels must import the new file (do not re-export the chains from `Reach.lean`; that would leave them in the meaning module). `succ_chain_ascends` / `succ_prefixed_infinite` keep calling `Query.naiveIter`.
- The two Bridge rows stay two rows with distinct claims; each `@Query.semi_naive_agrees` still elaborates. Census does not key on the Lean file path for these symbols (the `@` is the Lean half).
- `Exec/Reach.lean`'s header note ("Level 1 … proved equal to Level 0") stops implying the chains are part of the reach denotation.

## Acceptance criteria

- [x] Moved: `rg -nw 'naiveIter|semiNaiveIter' lean/Bumbledb/Exec/Reach.lean` → no matches; definitions exist in the new mechanism file with statements textually unchanged; `rg -n 'Query.naiveIter' lean/Bumbledb/Countermodels.lean` still matches.
- [x] Bridge honest: `./scripts/spec-census.sh` green; both `semi_naive_agrees` rows resolve.
- [x] Commands green: `cd lean && lake build`; `lake exe conformance conformance/cases` (268, 0).

## Constraints

- Pure motion + doc edits; zero statement changes. No assertion weakened; no Bridge row deleted.
- Countermodels successor walls (`succ_chain_ascends`, `succ_prefixed_infinite`) must still see `Query.naiveIter` after the move.
