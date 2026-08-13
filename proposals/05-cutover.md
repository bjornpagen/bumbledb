# 05 — Cutover

Lean complete and green first. Then docs as the present tense. Then Rust IR+validate+exec. Then macros/bindings. Then delete leftover Program. No half-cut: the tree must not contain a `Program` type that "is just one stratum," an `enum Program` in `prepared.rs`, or a `ReachProgram`.

## Order (normative)

### 0. This proposal

Land `proposals/` as-is. No Lean/Rust product change in that commit.

### 1. Lean — green before anything else

Follow `02-lean.md` literally. **One green tree** (one commit, or stacked commits that do not merge to main until green). Do not land `Atom.source` with `Program` still alive. Do not land `Program` gone with no `evalQuery`.

1. `Syntax.lean`: merge `Atom`/`PAtom`; add `WithDef`/`RecCte`/`CteId`; widen `Query` with `views` / `rec`; delete the program cut. `Query.plain` / `WellFormed` / `Rule.edbOnly`. Constructor grep (`a.relation`, `⟨n, rs⟩`).
2. `Denotation.lean`: `derives`/`ruleAnswers`/`rulesAnswers` over `F`; `sourceDen`/`edbEnv`/`tupleFact`; `factsOf` / `evalRule` taking `T`; retarget `eval_sound`. `Membership.lean`: `"relation"` JSON → `.edb`; theorems match `.edb`. Aggregates: `bindingSet`/`aggAnswers` over `F`.
3. Add `Exec/Reach.lean`. Delete `Exec/Fixpoint.lean`. `evalWith`, `reachOp`, `reachDen = lfpS`, `evalQuery`, `evalLinearReach`, `evalQueryList`, the agreement theorems, `evalQuery_plain`, `evalQuery_empty_rules`.
4. Plan/Dedup/Rewrites/Sweep: `queryAnswers` → `rulesAnswers` over `edbEnv` / `List Rule`; `⟨n, rs⟩` → `Query.plain n rs` or drop the Query wrapper; `a.source` match `.edb`. Do **not** grow CTE-aware rewrites. Comments `Program::Empty` → `PreparedBody::Empty`.
5. Countermodels: odd off `Program`. Bridge rows. `Bumbledb.lean` / `Main.lean` / `lean/README.md` / `lean/conformance/README.md`. **Recut** `program-*.json` → `reach-*.json` (Lean shape: `views`/`cte`/`finds`, identity main, drop `program-hand-mutual.json`). CQuery `seeded-*.json` stay; `decodeAtom` maps `"relation"` → `.edb`. No Program decoder. **Do not run `generate_program_corpus` in this step.**
6. `lake build` green. `scripts/lean.sh` green. Census: docs still cite dead names — **do not** merge Lean to main with census red. That is why step 2 is next, not Rust. A stacked pair in one PR is fine.

If a theorem does not port in a day, **narrow and record** in the Reach module doc (law 5). Do not resurrect `Program` to keep `program_eval_sound` compiling.

### 2. Docs as normative present tense

`04-bindings-docs.md` architecture table, in one commit with or immediately after step 1, so the census matches the Lean names. Cookbook 24–25 citations. `00-product.md` deleted-vocabulary sentence. Chain-window OPEN stays OPEN. No "will be" / "after the cut" in architecture docs — those sentences belong only in `proposals/` until step 5 deletes this folder (or leaves it as history; pick: **delete `proposals/` in the final leftover-Program commit** so architecture docs are the only present tense, git holds the proposal).

### 3. Rust IR + validate + exec

`03-engine.md`. One engine PR (or a stacked pair: IR+validate, then prepare/execute — but **no** merge that exports both `Program` and `Query.with`). `Db::prepare(&Query)` only. WITH-only lock (`PreparedBody::Rules` or `Empty`, never `Reach`). Reach driver. Delete `strata.rs`. Tests retargeted/refused per `04`. Adversarial sweep on `Query` (including huge `with.len()`). Alloc gate: WITH-only and rec both in the windows.

Do not "temporarily" `impl From<Program> for Query`. Do not keep `validate_program` as a wrapper that wraps the output predicate.

### 4. Macros / bindings / oracles

`query!`, TS, C++, napi marshal, C++ foreign view, `translate_query`, naive eval, conformance corpus builder (`reach-*.json`), cookbook tests. `04`. Fingerprint file regenerated. C++ recipe parity. **Now** the generator may overwrite `reach-*.json` — it emits the Lean Reach shape, not `predicates`/`output`.

Sugar caps stay sugar (`max_ctes = 4`, `max_query_rules = 4`). Engine `MAX_CTES = 16`, `MAX_RULES = 16`. Rec CTE pools `MAX_RULES` across `base`+`rec`.

### 5. Leftover Program — grep-clean

```
rg 'Program|PredId|PredicateDef|AtomSource::Idb|AtomSource::idb|validate_program|render_program|evalProgram|degenerate_embedding|MAX_PREDICATES|ProgramRef|FixpointProgram|ReachProgram|bdb_program|strata\.rs|\.idb\(|STRATUM|VALIDATE_STRATIFY|StratumStats|DeltaRows|AggregationInRecCte' \
  --glob '!proposals/**'
```

excluding git history. Zero hits in `lean/`, `crates/`, `ts/`, `cpp/`, `docs/` except comments that say the word is gone, if any — prefer zero. Then delete this folder.

`PreparedBody` and `ReachDriver` are the replacement names. `Program::Empty` comments in Lean Rewrites become `PreparedBody::Empty` in step 1.

## No half-cut

Forbidden intermediate states, even on a branch that "will clean up":

- `Program` with `predicates.len() == 1` as the Query encoding
- `From<Query> for Program` or the reverse
- `validate_program` calling `validate`
- `enum AtomSource { Edb, Idb, Cte }`
- `enum Program` in `prepared.rs` (use `PreparedBody`)
- Tarjan retained "for WITH cycles" (WITH cycles are `CteNotPrior`)
- k-variant minting retained "for a future nonlinear"
- `evalProgram` alias in Lean
- `ReachProgram` as the driver type (the name is `ReachDriver`)
- Named heads in `query!` still lowering to anything
- WITH-only executing inside `run_reach` / `run_fixpoint`
- Docs saying Query is "the degenerate Program"
- `Atom.relation : RelId` total accessor that returns `⟨0⟩` on `.cte`
- Dumping `reach-*.json` into the CQuery glob (`seeded-*.json` decoder)

The IR change is a break. ETL / regenerate. Not a compatibility shim.

## Success criteria

1. **Lean.** `Exec/Fixpoint.lean` gone. `Program` gone. `evalQuery_plain`, `evalQuery_sound`, `evalQuery_empty_rules`, `evalLinearReach_eq_lfp`, `reachOp_mono`, `reach_den_finite`, `wellFormed_cte_reads_real` proved. `lake build` green. Conformance agrees on the recut `reach-*.json` corpus via `evalQueryList`. Seeded CQuery cases still pass (`"relation"` → `.edb`). No fuel parameter on those public defs. No public def whose **docs** mention fuel as Lean incompleteness.
2. **Language.** `query!` all-bare still compiles. Named head without `with`/`with recursive` is a compile error. Cookbook 24–25 native forms use `with recursive`. TS `program`/`rec`/`output` gone. C++ `bdb::program`/`rec<>`/`output` gone. C ABI `bdb_program` gone.
3. **Engine.** `prepare(&Query)` only. WITH-only prepared body is `PreparedBody::Rules` or `Empty`, never `Reach`. Rec path: `ReachDriver`, one `DeltaVariant` per rec arm, watermark Δ, `TransientImage`, existing rule loop. `FixpointBudgetExceeded` vs `reachDen`, not vs Lean fuel. `MAX_CTES = 16`. Rec CTE pools `MAX_RULES`. No `strata.rs`. `Db::render_program` gone. `obs::STRATUM` gone.
4. **Oracles.** SQLite `WITH [RECURSIVE]` whole cte-list goldens (`CLOSURE`, `CLOSURE_ROOTS` inlined main); no `UNION ALL`. Naive complete lfp. Three-way on rec cases. Interval CTE column still a translator limit.
5. **Grep.** No `Program` / `PredId` / `evalProgram` / `degenerate_embedding` / `MAX_PREDICATES` / `ReachProgram` / `bdb_program` / `.idb(` / `STRATUM` in product trees.
6. **Unchanged on purpose.** Union = set union of rules, spanning seen-set, one sink per rule-list. No bags. No second engine. Chain-window still OPEN. Host-loop closure still cookbook 24's other dialect. During-walk negation in rec is refused, not rewritten into main.

## This is not in the cut

Do not open these while the Lean/Rust Program deletion is in flight. They are other cuts; mixing them is how this sprawls.

- FFI UAF, Q-mark abort, napi lifetime bugs
- `ArgKey::Measure` / measure-in-binding follow-ons
- C20 and any capacity-statement work
- Chain-window (`w = w₁ ∩ w₂` in a rec head) — stays OPEN; trigger unchanged (`20-query-ir.md`)
- Mutual recursion, nonlinear rec, stratified Datalog, Soufflé-shaped runtime
- During-walk negation in the rec CTE (refused; not a rewrite)
- `UNION ALL`, bags, outer join, a merge node
- A second eval path: ParamSet-per-round host-loop internalized in the engine; magic sets; demand transformation
- Replacing Free Join; a new watermark structure; a worklist besides the seen-set suffix
- Shrinking engine `MAX_RULES` to the C++ sugar cap (4) — sugar caps stay sugar
- Plan/COLT/grounding redesign; teaching grounding to fold CTE atoms at prepare; growing CTE-aware rewrite rules; CTE membership theorems in `Membership.lean`. The `queryAnswers` → `rulesAnswers` / `Query.plain` / `.edb` match retarget **is** in the Lean commit
- Smashing CQuery (aggregate finds) into `Syntax.Query`
- Making `FixpointBudgetExceeded` a Lean error or putting fuel back in `reachDen`
- Fingerprint stability; on-disk compatibility
- Deleting the **host-loop** closure idiom from the cookbook (it remains the depth-bounded answer)

The next step is `lean/Bumbledb/Query/Syntax.lean`, not a scheduling meeting.
