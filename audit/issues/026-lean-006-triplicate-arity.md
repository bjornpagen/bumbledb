# lean-006: arity is stored three times and trusted zero times

- **Severity:** high
- **Tree:** lean
- **Status:** FIXED(e134b917)
- **Source:** audit/lean.md H6
- **Depends on:** lean-001, lean-002 (the fields live on the types those issues rewrite; land with or after)

## The bug

Arity is declared on `Interior`, `Rec`, and `Query` (`lean/Bumbledb/Query/Syntax.lean:267-269, 271-276, 289-293`):

```lean
structure Interior where
  arity : Nat
  rules : List Rule
```

yet no evaluator or theorem reads any of them. The evaluators size everything from `finds.length`: `recCands` (`Exec/Reach.lean:372-375`):

```lean
def recCands (rec : Rec) (W : ListInstance) (V : InteriorTables) :
    List AnswerTuple :=
  (rec.base ++ rec.rec).flatMap fun r =>
    allTuples (recDom rec W V) r.finds.length
```

and the module's own narrowing note (`Exec/Reach.lean:20-24`) documents the dodge: "agreement does not assume head-arity equals `Rec.arity`." The decoder dutifully fills the dead fields (`lean/Main.lean:379-382, 385-391`). Mismatched widths are representable — an `Interior` with `arity := 5` and rules whose `finds.length = 2` decodes fine — and are papered over downstream by `fillerValue`/`tupleFact` totalization (`Denotation.lean:669-677`).

## Why it's wrong

A stored value that must equal a computable value is a proof obligation the type never discharges (Insight 9: derived data stored beside its source drifts). Here the library resolved the drift by *ignoring the stored copy*, leaving three dead `Nat`s whose only effect is to make "arity disagrees with finds" a representable state that every theorem must silently not depend on.

## The fix

Per `audit/CONTRACT.md §C4` ("Arity"):

- DELETE `Interior.arity` and the `LinearRec` arity (after lean-002 there is no `Rec.arity` field to keep). A derived head's width is its rules' `finds.length` — uniform under acceptance, which is what the evaluator already trusts.
- `Query`'s main `arity` survives ONLY if a denotation genuinely reads it; grep says nothing does — delete it too, and drop the corresponding constructor argument from lean-001's sum (`cq (interiors) (rules)` / `reach (interiors) (r) (rules)`). If a conformance comparator or Bridge row genuinely needs the declared width, keep main `arity` only and record why in the constructor doc.
- Decoder (`Main.lean` `decodeInterior`/`decodeRec`/`decodeReachQuery`): still *parses* the JSON `arity` keys (corpus frozen) but discards them — or, better, checks them against `finds.length` and refuses mismatches at decode (a real parse). Either is acceptable; refusing is preferred since the corpus is all-consistent.
- **Refused half (do NOT attempt):** `AnswerTuple` as `Vector Value arity` / length-indexed rules — refused per CONTRACT §C5 R-DENSE; `tupleFact`/`fillerValue` totalization stays (it is the recorded phantom semantics).

## Acceptance criteria

- [x] Gone: `rg -n 'arity' lean/Bumbledb/Query/Syntax.lean` → no `Interior.arity`/rec-arity fields (main arity only if the documented exception is taken); `rg -n 'Rec.arity|\.arity' lean/Bumbledb/Exec/Reach.lean` → no matches.
- [x] Unchanged: 268-case conformance green with the corpus byte-identical (`arity` keys still parsed or checked, never required absent); the narrowing note at `Reach.lean:20-24` deleted (nothing left to narrow).
- [x] Commands green: `cd lean && lake build`; `lake exe conformance conformance/cases` (268, 0); no `sorry`/`admit`.

## Constraints

- Corpus JSON frozen — the decoder's treatment of `arity` keys changes, the files do not.
- Semantics identical (widths were already `finds.length`-driven).
