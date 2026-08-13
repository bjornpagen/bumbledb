# 05 — Cutover

Lean complete and green first. Then docs as the present tense. Then Rust IR+validate+exec. Then macros/bindings. Then delete leftover Program. No half-cut: the tree must not contain a `Program` type that "is just one stratum," an `enum Program` in `prepared.rs`, or a `ReachProgram`.

Implementation order below is **this cut’s** engineering sequence. OPEN items are other cuts; they are not “the language refuses forever.” `01-language.md` already says mutual-linear is this-cut scope (`Option<Rec>`). This file listing it under OPEN is the same fact, not a contradiction.

## Order (normative)

### 0. This proposal

Land `proposals/` as-is. No Lean/Rust product change in that commit.

### 1. Lean — green before anything else

Follow `02-lean.md` literally. **One green tree** (one commit, or stacked commits that do not merge to main until green). Do not land `Atom.source` with `Program` still alive. Do not land `Program` gone with no `evalQuery`.

1. `Syntax.lean`: merge `Atom`/`PAtom`; add `Interior`/`Rec`/`InteriorId`; widen `Query` with `interiors` / `rec`; delete the program cut. `Query.plain` / `WellFormed` / `Rule.edbOnly`. Constructor grep (`a.relation`, `⟨n, rs⟩`).
2. `Denotation.lean`: `derives`/`ruleAnswers`/`rulesAnswers` over `F`; `sourceDen`/`edbEnv`/`tupleFact`; `factsOf` / `evalRule` taking `T`; retarget `eval_sound`. `Membership.lean`: `"relation"` JSON → `.edb`; theorems match `.edb`. Aggregates: `bindingSet`/`aggAnswers` over `F`.
3. Add `Exec/Reach.lean`. Delete `Exec/Fixpoint.lean`. `evalInteriors`, `reachOp`, `reachDen = lfpS`, `evalQuery`, `evalLinearReach`, `evalQueryList`, the agreement theorems, `evalQuery_plain`, `evalQuery_empty_rules`. **No fuel on those public defs.**
4. Plan/Dedup/Rewrites/Sweep: `queryAnswers` → `rulesAnswers` over `edbEnv` / `List Rule`; `⟨n, rs⟩` → `Query.plain n rs` or drop the Query wrapper; `a.source` match `.edb`. Do **not** grow interior-aware rewrites. Comments `Program::Empty` → `PreparedBody::Empty`.
5. Countermodels: odd off `Program`. Bridge rows. `Bumbledb.lean` / `Main.lean` / `lean/README.md` / `lean/conformance/README.md`. **Recut** `program-*.json` → `reach-*.json` per the ledger in `02-lean.md` — 27 in, **22** out: the five mutual shapes drop (`program-hand-mutual`; seeded `0000`/`0003`/`0010`/`0021`), the four three-predicate shapes unfold their middle predicate into main (seeded `0012`/`0013`/`0014`/`0016`), the rest recut 1:1 (identity main for `output = 0`; anti-join main for the strata-`[0,1]` cases). Keep source numbering; gaps mark the drops. CQuery `seeded-*.json` stay; `decodeAtom` maps `"relation"` → `.edb`. No Program decoder. **Do not run `generate_program_corpus` in this step.**
6. `lake build` green. `scripts/lean.sh` green. Census: docs still cite dead names — **do not** merge Lean to main with census red. That is why step 2 is next, not Rust. The census is CI-gated in the Lean lane, so steps 1 and 2 are one PR **by construction** — stack them.

If a theorem does not port in a day, **narrow and record** in the Reach module doc (law 5). Do not resurrect `Program` to keep `program_eval_sound` compiling.

### 2. Docs as normative present tense

`04-bindings-docs.md` architecture table, in one commit with or immediately after step 1, so the census matches the Lean names. Cookbook 24–25 citations. `00-product.md` deleted-vocabulary sentence. Chain-window OPEN stays OPEN. Architecture README gains the other OPEN rows (stacked linear, mutual-linear, named interior of finished rec, nonlinear-at-L). No "will be" / "after the cut" in architecture docs — those sentences belong only in `proposals/` until step 5 deletes this folder (or leaves it as history; pick: **delete `proposals/` in the final leftover-Program commit** so architecture docs are the only present tense, git holds the proposal).

### 3. Rust IR + validate + exec

`03-engine.md`. One engine PR (or a stacked pair: IR+validate, then prepare/execute — but **no** merge that exports both `Program` and `Query.interiors`). `ts/crate` and `cpp/bridge` are workspace-excluded but CI-gated and path-depend on `crates/bumbledb`: a step-3-only merge reddens both lanes, so steps 3 and 4 land as **one merge** (stacked commits, one PR). `Db::prepare(&Query)` only. Interiors-only lock (`PreparedBody::Rules` or `Empty`, never `Reach`). Reach driver. Delete `strata.rs`. Tests retargeted/refused per `04`. Adversarial sweep on `Query` (including huge `interiors.len()` — must not panic, must not invent `TooManyCtes`). Alloc gate: interiors-only and rec both in the windows.

Do not "temporarily" `impl From<Program> for Query`. Do not keep `validate_program` as a wrapper that wraps the output predicate. Do not add `MAX_CTES`.

### 4. Macros / bindings / oracles — same merge as step 3

`query!`, TS, C++, napi marshal, C++ foreign view, `translate_query`, naive eval, conformance corpus builder (`reach-*.json`), cookbook tests, primer-shaped `reach(x,x)` lock. `04`. Fingerprint file regenerated. C++ recipe parity. **Now** the generator may overwrite `reach-*.json` — it emits the Lean Reach shape, not `predicates`/`output`.

Sugar caps stay sugar (`max_query_rules = 4`; optional C++ `max_interiors = 4`). Engine: `MAX_RULES = 16`, rec pools `MAX_RULES` across `base`+`rec`, **no interior-count cap**.

### 5. Leftover Program — grep-clean

```
rg 'Program|PredId|PredicateDef|AtomSource::Idb|AtomSource::idb|validate_program|render_program|evalProgram|degenerate_embedding|MAX_PREDICATES|MAX_CTES|TooManyCtes|ProgramRef|FixpointProgram|ReachProgram|bdb_program|strata\.rs|\.idb\(|STRATUM|VALIDATE_STRATIFY|StratumStats|DeltaRows|AggregationInRecCte|RecCte|CteId|WithDef|Query\.with\b' \
  --glob '!proposals/**'
```

excluding git history. Zero hits in `lean/`, `crates/`, `ts/`, `cpp/`, `docs/` except comments that say the word is gone, if any — prefer zero. Then delete this folder.

`PreparedBody` and `ReachDriver` are the replacement names. `Program::Empty` comments in Lean Rewrites become `PreparedBody::Empty` in step 1. `Cte*` names are not a halfway house — the IR is `Interior` / `InteriorId` / `interiors`.

## No half-cut

Forbidden intermediate states, even on a branch that "will clean up":

- `Program` with `predicates.len() == 1` as the Query encoding
- `From<Query> for Program` or the reverse
- `validate_program` calling `validate`
- `enum AtomSource { Edb, Idb, Interior }` or `{ Edb, Idb, Cte }`
- `enum Program` in `prepared.rs` (use `PreparedBody`)
- Tarjan retained "for interior cycles" (interior cycles are `InteriorNotPrior`)
- k-variant minting retained "for a future nonlinear"
- `evalProgram` alias in Lean
- `ReachProgram` as the driver type (the name is `ReachDriver`)
- Named heads in `query!` still lowering to anything
- Interiors-only executing inside `run_reach` / `run_fixpoint`
- Docs saying Query is "the degenerate Program"
- `Atom.relation : RelId` total accessor that returns `⟨0⟩` on `.interior`
- Dumping `reach-*.json` into the CQuery glob (`seeded-*.json` decoder)
- Engine field named `with` (Lean keyword; CTE dialect)
- `MAX_CTES` / `TooManyCtes` / a 16-slot `INTERIOR` obs array
- Specifying the language by writing `WITH RECURSIVE`

The IR change is a break. ETL / regenerate. Not a compatibility shim.

## Success criteria

1. **Lean.** `Exec/Fixpoint.lean` gone. `Program` gone. `evalQuery_plain`, `evalQuery_sound`, `evalQuery_empty_rules`, `evalLinearReach_eq_lfp`, `reachOp_mono`, `reach_den_finite`, `wellFormed_interior_reads_real` proved; walls retargeted (`odd_not_monotone`, `odd_no_fixpoint`, `succ_prefixed_infinite`). `lake build` green. Conformance agrees on the recut `reach-*.json` corpus (22 files) via `evalQueryList`. Seeded CQuery cases still pass (`"relation"` → `.edb`). No fuel parameter on those public defs. No public def whose **docs** mention fuel as Lean incompleteness.
2. **Language.** `query!` all-bare still compiles. Named head without `interior`/`recursive` is a compile error. Cookbook 24–25 native forms use `recursive`. Primer `reach(x,x)` recuts 1:1. TS `program`/`rec`/`output` gone. C++ `bdb::program`/`rec<>`/`output` gone. C ABI `bdb_program` gone.
3. **Engine.** `prepare(&Query)` only. Interiors-only prepared body is `PreparedBody::Rules` or `Empty`, never `Reach`. Rec path: `ReachDriver`, one `DeltaVariant` per rec arm, watermark Δ, `TransientImage`, existing rule loop. `FixpointBudgetExceeded` vs `reachDen`, not vs Lean fuel. **No `MAX_CTES`.** Rec pools `MAX_RULES`. More than 16 interiors validate. No `strata.rs`. `Db::render_program` gone. `obs::STRATUM` gone. No 16-slot interior span array.
4. **Oracles.** SQLite translator emits `WITH [RECURSIVE]` for this cut’s fragment (`CLOSURE`, `CLOSURE_ROOTS` inlined main); no `UNION ALL`. That SQL is not the language. Naive complete lfp (empty base ⇒ empty lfp is the iteration). Three-way on rec cases. Interval derived-column still a translator limit.
5. **Grep.** No `Program` / `PredId` / `evalProgram` / `degenerate_embedding` / `MAX_PREDICATES` / `MAX_CTES` / `ReachProgram` / `bdb_program` / `.idb(` / `STRATUM` / `RecCte` / `CteId` in product trees.
6. **Unchanged on purpose.** Union = set union of rules, spanning seen-set, one sink per rule-list. No bags. No second engine. Chain-window still OPEN. Host-loop closure still cookbook 24's other dialect. During-walk negation in rec is refused, not rewritten into main.

## This is not in the cut

Do not open these while the Lean/Rust Program deletion is in flight. Mixing them is how this sprawls.

**Walls — not a later cut either:**

- Bound heads / creation quarantine; negation or aggregation through the cycle (the rec SCC's own table); fuel as denotation; rec as implicit answer; stored programs / magic sets / demand rewrite / host-loop internalization; bags / `UNION ALL`

**OPEN — other cuts, same complexity class, workload trigger:**

- **Stacked sequential linear lfps** (A finishes; B reads A as a finished set). Two least fixpoints, two drivers, or one Query with `List Rec`. *Trigger:* a workload where host two-prepares is the pain (not a translator gap).
- **Mutual-linear** (one SCC, several names, each rule ≤1 rec atom). Same class as self-rec; even/odd encodes as one linear predicate with a parity column. Refused **this cut** so Tarjan / k-variants / multi-pred scratch die with Program. *Trigger:* a sighted query that is unnatural as one name **and** is still linear. Admitting it is a new IR (not `Option<Rec>`), not a resurrection of `Program`.
- **Named interior of a finished rec** (inlining equivalent). This cut inlines into main. *Trigger:* two main-shaped queries over one rec that want to share a named projection without a second prepare — still not a second SCC.
- **Nonlinear rec** (`P(x,z) ← P(x,y), P(y,z)`). Semi-naive still agrees; it is a worse TC algorithm at 10⁷ / 10 ms (k FJ plans × |Acc|). *Trigger:* a measured L-scale query where the linear encoding is unnatural **and** `|Δ ⋈ Acc|` still fits 10 ms / `DEFAULT_FIXPOINT_TUPLES`. Not “SQLite grew a second reference.”
- **During-walk anti-join of finished tables** (EDB / an earlier interior negated in a rec arm). Monotone — the negated source is constant in the operator's argument (`stratumOp_mono`'s stratified content; `reachOp_mono` does not witness against it). Refused this cut so `NegationInRec` covers the whole SCC, the driver keeps one negation path, and `recLinear` stays one line. *Trigger:* a workload whose during-walk exclusion cannot be written positively. Admitting it weakens `reachOp_mono`'s premise to no-negated-self; the wall (self) is untouched.

**Already OPEN, unchanged:**

- Chain-window (`w = w₁ ∩ w₂` in a rec head) — created head; `20-query-ir.md` trigger stands
- During-walk negation **of the rec table itself** (through the cycle) — the wall (`odd_not_monotone` / `odd_no_fixpoint`); never a rewrite into main. The finished-table case is the OPEN row above, not this wall

**Other work, not this feature:**

- FFI UAF, Q-mark abort, napi lifetime bugs
- `ArgKey::Measure` / measure-in-binding follow-ons
- C20 and any capacity-statement work
- A full Datalog runtime (Soufflé-shaped): stored programs, magic sets, demand transformation — a wall, not OPEN
- Replacing Free Join; a new watermark structure; a worklist besides the seen-set suffix
- Shrinking engine `MAX_RULES` to the C++ sugar cap (4) — sugar caps stay sugar
- Inventing `MAX_CTES` / `MAX_INTERIORS` as a product cap
- Plan/COLT/grounding redesign; teaching grounding to fold interior atoms at prepare; growing interior-aware rewrite rules; interior membership theorems in `Membership.lean`. The `queryAnswers` → `rulesAnswers` / `Query.plain` / `.edb` match retarget **is** in the Lean commit
- Smashing CQuery (aggregate finds) into `Syntax.Query`
- Making `FixpointBudgetExceeded` a Lean error or putting fuel back in `reachDen`
- Fingerprint stability; on-disk compatibility
- Deleting the **host-loop** closure idiom from the cookbook (it remains the depth-bounded answer)

The next step is `lean/Bumbledb/Query/Syntax.lean`, not a scheduling meeting.
