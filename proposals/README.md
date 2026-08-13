# Cut: delete Program — Query + WITH + one linear reach

This folder is **one** proposal. Not a wiki. Read in order, then open Lean.

The cut: user-facing `Program` / stratified Datalog / `evalProgram` / fuel-as-semantics die. The one execute target is `Query`. Named views are non-recursive `WITH` (a DAG, evaluated once). Recursion is at most one **linear** `WITH RECURSIVE`, denoted by a least fixpoint with **no fuel**, executed by specializing the existing rule loop — one `DeltaVariant`, watermark Δ, `TransientImage`, Free Join. Then delete the Program system entire: no zombie one-stratum `Program`, no `From<Query> for Program`, no Tarjan, no mutual recursion, no k-variants.

Do not implement from this folder until `02-lean.md` is the working spec and the Lean tree is green. Docs after Lean. Rust after docs. Bindings after Rust. Leftover `Program` last.

**Lean work is one green tree**, not five mergeable half-states. The numbered Lean steps below are the queue *inside* that commit (or stacked commits that do not land on main until `lake build` is green). Do not merge `Atom.source` with `Program` still alive, and do not merge `Program` gone with no `evalQuery`.

## Read order

| File | Job |
|---|---|
| `00-manifesto.md` | Why Program is the wrong model; what elegance means here |
| `01-language.md` | Normative Query + WITH + RecCte; grammar; mapping; errors |
| `02-lean.md` | **The core.** Types, eval, theorems to keep/retarget/delete. Start here in the tree |
| `03-engine.md` | IR after Program; drivers; files/types that die |
| `04-bindings-docs.md` | `query!`, TS, C++, cookbook, oracles, refusals |
| `05-cutover.md` | Ordered plan. Lean green first. Success criteria. Out of cut |

## Locked (do not re-open)

- Query-only. Lean field `views`; engine / C ABI / TS IR field `with`. `with` is a Lean keyword.
- One linear `RecCte`. No `UNION` keyword. No fuel in any public Lean denotation.
- Specialize the existing driver. No FFI / Q-mark / C20 sprawl.
- Delete Program utterly — including `enum Program` in `prepared.rs` and any “internal one-stratum Program”.
- `MAX_CTES = 16` excludes main. Rec CTE pools `MAX_RULES` across `base`+`rec`. C++/TS `max_ctes = 4` is sugar.
- Conformance: existing `seeded-*.json` stay the CQuery arm (`"relation"` → `.edb`). Former `program-*.json` become `reach-*.json` evaluated by `evalQueryList`. Do not dump them into the CQuery glob.

## Lean-first sequence (the only sequence)

1. Widen `Atom` to `AtomSource` (`edb | cte`). Generalize `derives` / `ruleAnswers` over `F : AtomSource → Set Fact`. Recover today's lemmas at `edbEnv I`. Delete `PAtom` / `PRule` / the even-odd coding transport. Executable join reads `factsOf W T` — never re-pun `CteId` into `RelId`. Every `a.relation` site matches `.edb R` (Membership, Plan, Rewrites, Dedup, Sweep, Conformance `decodeAtom`).
2. Add `WithDef`, `RecCte`, `CteId`. Widen Lean `Query` with `views : List WithDef` and `rec : Option RecCte`. Engine IR field is `with`. Delete `Program` / `PredicateDef` / `PredId` / `Query.toProgram`.
3. Define `evalWith`, `reachOp`, `reachDen = lfpS`, `evalQuery`. Executable `evalLinearReach` / `evalQueryList` proved equal to those denotations. **No fuel in any public denotation.**
4. Retarget Bridge, Countermodels, conformance (`evalProgram` → `evalQueryList` on `reach-*.json`). `lake build` green. Census green (docs in the next commit of the same PR if needed — do not merge Lean to main with red census).
5. Then — and only then — architecture docs as the new present tense, then Rust IR+validate+exec, then macros/bindings, then grep-clean `Program`.

`02-lean.md` is sufficient to begin step 1 without the rest of this folder in hand. The rest exists so the Rust cut does not re-invent a Program.
