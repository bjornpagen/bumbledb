# Cut: delete Program — Query, interiors, one linear reach

This folder is **one** proposal. Not a wiki. Read in order, then open Lean.

The cut: user-facing `Program` / stratified Datalog / `evalProgram` / the strata-witness denotation die. (The fuel was never a public parameter — today's lie is `40-execution.md`'s “sufficient fuel” sentence and a public `fueledLoop`; both die with it.) The one execute target is `Query`. Named **interiors** are a DAG of conjunctive rule-lists, each evaluated once. Recursion is at most one **linear** SCC, denoted by a least fixpoint with **no fuel**, executed by specializing the existing rule loop — one `DeltaVariant`, watermark Δ, `TransientImage`, Free Join. Then delete the Program system entire: no zombie one-stratum `Program`, no `From<Query> for Program`, no Tarjan, no k-variants. Mutual recursion is unwritable **this cut** (OPEN, not a wall).

Do not implement from this folder until `02-lean.md` is the working spec and the Lean tree is green. Docs after Lean. Rust after docs. Bindings after Rust. Leftover `Program` last.

**Lean work is one green tree**, not five mergeable half-states. The numbered Lean steps below are the queue *inside* that commit (or stacked commits that do not land on main until `lake build` is green). Do not merge `Atom.source` with `Program` still alive, and do not merge `Program` gone with no `evalQuery`.

SQLite is a **lossy translator** of this cut’s fragment. It is never the denotation and never the language. Do not specify the IR by writing `WITH RECURSIVE`.

## Three knobs (normative; every file uses these words)

Do not collapse these into one “the language refuses.” A this-cut refusal is not a wall. An OPEN item is not unwritable forever.

**Walls (conceptual, forever).** Bound heads / creation quarantine — the lfp is finite because heads project bound variables (`reach_den_finite`; `succ_prefixed_infinite` is the wall). No negation or aggregation **through the cycle** — nothing negates or folds the rec table itself (`reachOp_mono`; `odd_not_monotone` / `odd_no_fixpoint` are the walls). Not a Datalog runtime: no stored programs, no magic sets, no demand rewrite, no host-loop internalization. Denotation is `lfp`, not fuel. Recursion is never the answer (main is). Chain-window stays OPEN (created head). Filtering during the walk is not anti-joining the finished lfp.

**This cut (scope, not morality).** Query = named interiors (DAG, eval once) + **one** recursive SCC + main. Linear arms (exactly one positive rec occurrence per rec rule) because k=1 Δ-variant × Free Join at 10⁷ / 10 ms — the better TC algorithm for sighted graphs, not because SQLite. One `ReachDriver`. Interiors then rec then main. Primer's `reach(x,x)` shape recuts 1:1 (in-tree lock; the Primer repo's own recut is a filed issue, not a gate). `NegationInRec` refuses **every** negation in the rec SCC — wider than the wall on purpose (one negation path in the driver); the monotone finished-table case is OPEN below, not immoral. `MAX_RULES` applies to every rule-list in interiors ∪ rec ∪ main. **No interior-count cap** — one derived-tuples ledger (`DerivedBudgetExceeded`, né `FixpointBudgetExceeded`; 10⁷) prices interior ∪ rec materialization instead, judged after each interior and between rec rounds; rounds axis rec-only. The answer stays priced in bytes. No derived table is unguarded.

**OPEN (same complexity class; workload trigger; not unwritable forever).** Stacked sequential linear lfps. Mutual-linear (one SCC, each rule ≤1 rec atom) — refused **this cut** so Tarjan / k-variants die with Program. During-walk anti-join of **finished** tables in rec arms — monotone (the negated source never reads Δ), refused this cut for one negation path. Named interior of a **finished** rec (inlining is equivalent). Nonlinear refused until a measured L-scale query makes the linear encoding unnatural **and** the work still fits the tuple budget.

`01-language.md` states the language, then this-cut narrowing, then the roster. `05-cutover.md` does not contradict that: mutual-linear is OPEN / other cuts; this cut’s IR cannot write it.

## Read order

| File | Job |
|---|---|
| `00-manifesto.md` | Why Program is the wrong model; the three knobs; why linear; why SQLite is not the language |
| `01-language.md` | Language + this-cut IR; grammar as host sugar; mapping; errors |
| `02-lean.md` | **The core.** Types, eval, theorems to keep/retarget/delete. Start here in the tree |
| `03-engine.md` | IR after Program; drivers; files/types that die |
| `04-bindings-docs.md` | `query!`, TS, C++, cookbook, oracles, refusals |
| `05-cutover.md` | Ordered plan (this cut’s engineering sequence). Success criteria. OPEN / out of cut |

## Locked (do not re-open)

- Query-only. Lean and engine field `interiors`. Do not spell the IR `with` (`with` is a Lean keyword; that collision was the tell that the last draft was a CTE dialect).
- This cut: one linear `Rec`. No `UNION` keyword. No fuel in any public Lean denotation.
- Specialize the existing driver. No FFI / Q-mark / C20 sprawl.
- Delete Program utterly — including `enum Program` in `prepared.rs` and any “internal one-stratum Program”.
- **No `MAX_CTES`.** `MAX_PREDICATES` dies with Program. Do not replace it with a second 16 on interiors. `MAX_RULES` (16) is the remaining query-shaped cap, per rule-list; the rec SCC pools it across `base`+`rec`. The price of interiors is the derived-tuples ledger, not a counter. C++/TS sugar caps stay sugar (`max_query_rules = 4` on rules); named interiors are uncapped at every layer — no `max_interiors` anywhere.
- Conformance: existing `seeded-*.json` stay the CQuery arm (`"relation"` → `.edb`). The 27 `program-*.json` recut to **22** `reach-*.json` evaluated by `evalQueryList` — five mutual shapes drop, four three-predicate shapes unfold their middle predicate into main (`02-lean.md` owns the ledger; keep source numbering, gaps mark the drops). Do not dump them into the CQuery glob.

## Lean-first sequence (the only sequence)

1. Widen `Atom` to `AtomSource` (`edb | interior`). Generalize `derives` / `ruleAnswers` over `F : AtomSource → Set Fact`. Recover today's lemmas at `edbEnv I`. Delete `PAtom` / `PRule` / the even-odd coding transport. Executable join reads `factsOf W T` — never re-pun `InteriorId` into `RelId`. Every `a.relation` site matches `.edb R` (Membership, Plan, Rewrites, Dedup, Sweep, Conformance `decodeAtom`).
2. Add `Interior`, `Rec`, `InteriorId`. Widen Lean `Query` with `interiors : List Interior` and `rec : Option Rec`. Engine IR field is `interiors`. Delete `Program` / `PredicateDef` / `PredId` / `Query.toProgram`.
3. Define `evalInteriors`, `reachOp`, `reachDen = lfpS`, `evalQuery`. Executable `evalLinearReach` / `evalQueryList` proved equal to those denotations. **No fuel in any public denotation.**
4. Retarget Bridge, Countermodels, conformance (`evalProgram` → `evalQueryList` on `reach-*.json`). `lake build` green. Census green (docs in the next commit of the same PR if needed — do not merge Lean to main with red census).
5. Then — and only then — architecture docs as the new present tense, then Rust IR+validate+exec **and** macros/bindings as one merge (`ts/crate` and `cpp/bridge` are workspace-excluded but CI-gated and path-depend on `crates/bumbledb` — a Rust-only merge is a red tree), then grep-clean `Program`.

`02-lean.md` is sufficient to begin step 1 without the rest of this folder in hand. The rest exists so the Rust cut does not re-invent a Program.
