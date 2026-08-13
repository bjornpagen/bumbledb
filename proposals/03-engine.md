# 03 — Engine: IR, drivers, what dies

Lean is the meaning (`02-lean.md`). This file is the Rust cut after Lean is green and the architecture docs have been rewritten in the present tense (`05-cutover.md`). Do not half-cut: a Query with empty WITH and no rec is a Query, not a one-predicate Program. There is no internal Program, no `ReachProgram`, no `enum Program` in `prepared.rs`.

## IR (`crates/bumbledb/src/ir.rs`)

Delete: `Program`, `PredicateDef`, `PredId`, `ProgramRef`, `From<Query> for Program`, `From<&Query> for ProgramRef`, `From<&Program> for ProgramRef`, `MAX_PREDICATES`, `AtomSource::Idb`, `AtomSource::idb()`.

Add / change:

```rust
pub const MAX_CTES: usize = 16;
// Counts `with.len() + rec.is_some() as usize`. Excludes main.
// Not a rename of MAX_PREDICATES (that cap counted predicates
// including the output). See 01-language.md.

pub struct CteId(pub u16);

pub enum AtomSource {
    Edb(RelationId),
    Cte(CteId),
}

impl AtomSource {
    pub fn edb(self) -> Option<RelationId> { /* ... */ }
    pub fn cte(self) -> Option<CteId> { /* was idb() */ }
}

pub struct WithDef {
    pub head: Vec<HeadTerm>,
    pub rules: Vec<Rule>,
}

pub struct RecCte {
    pub head: Vec<HeadTerm>,
    pub base: Vec<Rule>,
    pub rec: Vec<Rule>,
}

pub struct Query {
    pub with: Vec<WithDef>,
    pub rec: Option<RecCte>,
    pub head: Vec<HeadTerm>,
    pub rules: Vec<Rule>,
}
```

`Query::single` sets `with: vec![]`, `rec: None`. Existing conjunctive construction sites compile with two extra empty fields or via `Query::single`.

`CteId(i)` = `with[i]`. Rec, if any, = `CteId(with.len() as u16)` **after** `TooManyCtes`. Count with `usize`. Never `u16::try_from(with.len()).unwrap()` on hostile IR (`01-language.md`). `MAX_CTES ≤ 16` keeps accepted ids in `u16`.

Public re-exports in `lib.rs`: drop `Program`, `PredicateDef`, `PredId`, `MAX_PREDICATES`, `ProgramRef`. Export `WithDef`, `RecCte`, `CteId`, `MAX_CTES`. `AtomSource` stays, variant renamed. Stats: drop `StratumStats` / `DeltaRows` if they were reachable through `ExecutionStats`; add `CteStats` / `ReachStats` as fields of `ExecutionStats` (no extra root re-export required unless the harness names them).

`ir::render::render` emits the grammar in `01-language.md`. `render_program` dies. Goldens recut. Handles/punning/Allen/Duration unchanged.

`ir::normalize`: `Idb` typing surface (`signatures[pred]`) becomes `CteSignatures` — a slice of sealed column types, indexed by `CteId`, filled in declaration order before the rec, rec after WITH, main last. An `AtomSource::Cte` binding reads `FieldId(i)` against that slice. Point-membership on a CTE interval column uses that sealed type (today's Idb membership in `normalize.rs`) — engine-only; Lean `Membership.lean` stays EDB (`02-lean.md`). Grounding still refuses CTE occurrences (not stored relations; no `U`/`M`; same as today's Idb refusal). `PlanOccurrence::relation()` panics on `Cte` the way it panics on `Idb` today — callers match `source`.

## Validate

`validate(schema, &Query) -> ValidatedQuery` is the **only** boundary. Delete `validate_program`, `ValidatedProgram`, `ir/validate/strata.rs` (the whole file: Tarjan, `condense`, `stratify`, the SCC tests).

The new roster is `01-language.md` § validation roster (canonical error names there). Implementation notes:

- DNF / nesting / per-rule roster run **independently** on each `WithDef.rules`, on `RecCte.base`, on `RecCte.rec`, and on `Query.rules`.
- **`MAX_RULES`:** each `WithDef.rules` and the main `Query.rules` independently ≤ 16 (structural then DNF, today's query). The rec CTE is **one pool**: `base.len() + rec.len() ≤ MAX_RULES` (`TooManyRules { count: sum }`) and `dnf_width(base) + dnf_width(rec) ≤ MAX_RULES` (`DnfExceedsRules { produced: sum, cap }`). Not 16+16. Not a redundant per-arm structural cap on top of the pool.
- Seal WITH in declaration order. Rec seals from `base` then `rec`. No sealing loop, no `UnresolvedPredicateSignature`.
- `CteSignatures` is a `Vec` of length `cteCount`, not a chaotic fixpoint.
- Adversarial sweep: hostile `Query` values (random `CteId`s, self in base, two self-atoms, `with.len() == 100_000`). `rec` is `Option` and `with` is a prefix list — WITH-after-rec is unrepresentable; inject out-of-range `CteId`s instead. Trust-boundary law unamended: no panic from IR data.
- Params unify once, query-global, after every rule of every CTE and main has typed.

**Measure (pick A, matches `01-language.md`).** WITH **bodies** may contain measure comparisons; a ray raises `MeasureOfRay` after **that** CTE finishes (then later WITH, rec, main still run only if the host continues — same as today's per-query Ray: the execute aborts). Rec CTE refuses **every** measure site (`MeasureInCte` on heads, `MeasureInRecCte` on bodies) before execute. Measure **finds** and folds only on **main**. Ray probes: after each WITH, then after main. No rec probes. Delete "recursive programs defer the pass."

Error renames (`error.rs`), census-scanned: `01-language.md` table. `FixpointBudgetExceeded` **stays** — rec driver resource abort vs `reachDen`. Payload drops `stratum`: `{ rounds, tuples }`. One rec CTE, no strata.

### `ValidatedQuery` — three parts, not one lowered list

Today's `ValidatedQuery { lowered, predicate, rules, param_types, ... }` cannot branch WITH vs rec vs main. After:

```rust
pub struct ValidatedCte {
    lowered: Vec<LoweredRule>,
    predicate: Predicate,       // ir::validate::Predicate — sealed column types
                                // of this WITH head. NOT PredicateDef. NOT PredId.
    rules: Vec<RuleTyping>,
}

pub struct ValidatedRec {
    base: Vec<LoweredRule>,
    rec: Vec<LoweredRule>,
    predicate: Predicate,       // sealed rec head (same Predicate type)
    base_typing: Vec<RuleTyping>,
    rec_typing: Vec<RuleTyping>,
}

pub struct ValidatedMain {
    lowered: Vec<LoweredRule>,
    predicate: Predicate,       // answer head
    rules: Vec<RuleTyping>,
}

pub struct ValidatedQuery {
    with: Vec<ValidatedCte>,    // Lean: views; Rust field matches IR
    rec: Option<ValidatedRec>,
    main: ValidatedMain,
    param_types: BTreeMap<ParamId, ValueType>,
    set_params: BTreeSet<ParamId>,
    point_params: BTreeSet<ParamId>,
}
```

`validate` returns this. Prepare reads `with` then `rec` then `main`. `ValidatedProgram` dies — including `witness(PredId)` / `output()`. Accessors: `with[i]`, `rec`, `main`. `PreparedQuery::predicate()` remains the **main/answer** head (today's output signature). Do not reuse today's single `lowered` by concatenating CTE rules into the main list.

`normalize_predicate` becomes per-CTE / rec / main `normalize` against `CteSignatures`. `CountAcrossRules` is a main-query error (WITH/rec heads cannot carry Count — `AggregateInCte`).

## Prepare / execute

Today (`api/prepared.rs`):

```rust
enum Program { Empty, Rules(Vec<PreparedRule>), Fixpoint(Box<FixpointProgram>) }
```

After:

```rust
struct PreparedQuery {
    with: Vec<PreparedCte>,  // declaration order; empty iff Query.with is
    body: PreparedBody,      // Empty | Rules(main) | Reach(driver)
    // ... scratch, schema pin, rendered, budget (ignored when rec is None)
}

enum PreparedBody {
    Empty,                          // main emits nothing; WITH preamble still runs
    Rules(Vec<PreparedRule>),       // main rules; rec is None
    Reach(Box<ReachDriver>),        // rec: Some
}
```

WITH units are **not** inside `PreparedBody`. A WITH-only query is `with.len() > 0` and `body` is `Rules` or `Empty` — never `Reach`. The lock `a_with_only_query_does_not_enter_reach` asserts `!matches!(body, Reach(_))`.

**Rename the inner enum.** Today's `Program` as a prepared-body name is a leftover lie. Grep-clean `enum Program` in `prepared.rs`. The driver type is `ReachDriver`, not `ReachProgram`.

### WITH-only (`rec == None`)

Must **not** build `Reach` / enter `run_reach`. Path:

1. For each `WithDef` in order: prepare as today's per-rule pipeline (key probe or Free Join), sink = `ProjectionSink` (bound-var head). CTE occurrences of earlier WITH cost on the selectivity floors (row count unknown at prepare — same as Idb). Pin-at-prepare unamended.
2. Prepare main rules. CTE occurrences of any WITH: floors.
3. Execute: for each CTE, `run_rules` into its projection sink, transpose to `TransientImage` (`synthesize`/`refill` from seen-set rows — today's interior-predicate image), bind that image for later CTE and main `Cte` occurrences (`Colt::reset`, bypass `memo.bind`, recycle `spare_buffers` — the Idb arm in `run_join`, renamed Cte). Then `run_rules` main into the head-owned sink. Ray probes: each WITH is an ordinary query — probe after that CTE finishes; main probes after main. No deferred-Idb carve-out.

Statically empty: a dead WITH is the empty table (later readers see nothing). A dead main with live WITH still runs the WITH preamble (params/bind errors must surface; a WITH measure comparison can Ray). A query of only dead **main** rules with empty WITH is today's `Empty` plan. Pick: WITH-only where every **main** rule is dead → `Empty` after the WITH units are still prepared (bind + ray on WITH still run). Do not skip WITH when main is empty-plan: WITH can be the only measure site. Simpler pick that matches "eval once": always run the WITH preamble when `with` is nonempty; `Empty` means main emits nothing and has no live rule, not "skip the query."

`EmptyRuleSet` is validate of IR (`Query.rules` empty). `PreparedBody::Empty` is plan after static deadness. Both exist; they are not the same.

### Rec present

`ReachDriver` is a **specialization** of today's `FixpointProgram`, not a second evaluator. The name is `ReachDriver`, not `ReachProgram` — grep for `Program` must stay zero (except comments that say the word is deleted, and git history).

- No `strata_members`, no `top_stratum`, no per-predicate SCC, no `recursive: bool` per predicate.
- WITH preamble as above (finished images, eval once). Then rec. Then main.
- One rec predicate: `base: Vec<PreparedRule>` (round 0, ordinary rule loop into the rec `ProjectionSink`), `rec: Vec<RecursiveRule>` (rounds ≥ 1).
- **`RecursiveRule` holds exactly one `DeltaVariant`.** Delete `variants: Box<[DeltaVariant]>`. The unique positive self-atom is the delta occurrence. Nonlinear k-minting in `prepare_program` (`prepare_rule_variant` loop over same-stratum Idb atoms) dies. Extra EDB / WITH atoms on a rec arm are accumulated/EDB, never a second delta.
- Frontier = rec sink's seen-set, watermark, `answers_since`, `TransientImage` ping-pong for delta vs accumulated of **that one CTE**. Delete per-predicate arrays sized by `MAX_PREDICATES`; size 1 for rec plus `with.len()` finished slots.
- Stop: empty Δ. That is Lean `reachStep acc ⊆ acc` / `T(acc) ⊆ acc` (`02-lean.md`). Budget: `DEFAULT_FIXPOINT_ROUNDS` / `DEFAULT_FIXPOINT_TUPLES` stay (rename constants only if the module is renamed; values stay). `set_fixpoint_budget` stays on `PreparedQuery` and is ignored when `rec` is `None` (no new error; hosts copy-paste).
- Then main rules, binding the finished rec image like a WITH.
- Ray probes: rec CTE is measure-free by roster — no rec probes. WITH as above. Main after main. Delete "recursive programs defer the pass."

`run_join`'s Idb arm is the Cte arm. One code path for WITH images and rec images: a `Cte` occurrence binds a `RelationImage` the driver stuffed in `cte_images[id]`. Generation map never learns CTEs exist (today's Idb law).

### What the driver is not

- Not `evalProgram` over strata.
- Not a host-loop internalization (ParamSet-per-round). Cookbook 24's host idiom stays in the host.
- Not k delta-variants.
- Not Tarjan.
- Not mutual recursion.
- Not a second Free Join.
- Not named `ReachProgram`.
- Not entered by WITH-only.

`lean/Bumbledb/Exec/Reach.lean: evalLinearReach_eq_lfp` and `semi_naive_agrees` at `reachOp` are the agreement. `reachOp_empty` is round 0 = base. Lean `evalLinearReach` is the naive chain; this driver is the semi-naive realization.

## Files / types that die

| Path / type | Fate |
|---|---|
| `ir::{Program, PredicateDef, PredId, ProgramRef, MAX_PREDICATES}` | Delete |
| `AtomSource::Idb` / `AtomSource::idb` | `Cte` / `cte` |
| `From<Query> for Program` and `ProgramRef` From impls | Delete. No embedding |
| `ir/validate/strata.rs` | Delete the file |
| `validate_program`, `ValidatedProgram`, `IdbSignatures` | Delete / become `CteSignatures` on the one `validate` |
| `ir/render.rs` `render_program` | Delete |
| `Db::render_program` (`api/db.rs`) | Delete. Only `render_query` |
| `api/prepared.rs` `enum Program` | `PreparedBody`; drop `Fixpoint` |
| `api/prepared/fixpoint.rs` | Rewrite as `reach.rs` (or gut in place and rename). Drop strata, k-variants, per-pred SCC. Keep watermark, TransientImage, budget, round-0 base / round-r Δ |
| `FixpointProgram`, `FixpointPredicate`, `FixpointScratch` (multi-pred) | `ReachDriver`, one rec scratch + WITH slots |
| `RecursiveRule.variants: Box<[DeltaVariant]>` | one `DeltaVariant` |
| `prepare_program` in `build.rs` | `prepare` handles Query; branch `rec.is_some()` vs WITH-only vs plain |
| `Db::prepare(..., impl Into<ProgramRef>)` | `prepare(&Query)` only. Hostile nesting still screened on `&Query` before any clone — the ProgramRef borrow rationale is gone because the clone-into-Program path is gone |
| `obs.rs` `STRATUM` array, `MAX_PREDICATES == STRATUM.len()`, `VALIDATE_STRATIFY` | Delete. `VALIDATE_SEAL` becomes declaration-order CTE sealing (one span, not a chaotic loop) or dies if sealing is not worth a span — pick: keep one `VALIDATE_SEAL` over CTE count. `CTE` spans `MAX_CTES`-bounded (counted path, like `RULE`). One `REACH` span + `fixpoint_round` children. No `or silent` fork |
| `ExecutionStats.strata`, `StratumStats`, `DeltaRows` | `ctes` / `reach` / `RoundStats.delta: u64` |
| Naive `NaiveDb::program` (`naive/query.rs`) | Extend `NaiveDb::query`: WITH then rec lfp then main. Delete the program entry |
| `translate_program` | `translate_query`: SQL `WITH [RECURSIVE]` then the **whole** cte-list (`04`). Gate shrinks: mutual / nonlinear / in-cycle fold are unwritable; keep interval-column translator limit |
| `querygen/shapes_recursive.rs` | Rewrite to emit `Query` (WITH + optional RecCte + identity main). Drop mutual and nonlinear variants. Keep linear, main-negation of finished CTE, main fold, empty-Δ, budget-trip |
| Conformance `generate_program_corpus` / `conformance/program.rs` | Emit Reach JSON (`reach-*.json`). Lean corpus already recut in the Lean commit; do not run the old generator until this rewrite |
| C ABI `bdb_program` / `bdb_predicate` / `BDB_ATOM_SOURCE_KIND_IDB` | `bdb_query { with, rec, head, rules }`; `BDB_ATOM_SOURCE_KIND_CTE`. `bdb_prepare(db, const bdb_query *)`. Delete `prepare(bdb_program)` in `raii.cc` |
| C++ `.idb` / `.not_idb` | `.cte<"n">` / `.not_cte<"n">` |
| `cpp/bridge` `program_in` | `query_in` |

`lib.rs` public surface after: `Query`, `Rule`, `WithDef`, `RecCte`, `CteId`, `AtomSource`, `MAX_RULES`, `MAX_CTES`. No Program. `AtomSource::cte()` not `idb()`.

## Introspection

`INTROSPECTION_VERSION` 3 → 4 (content change). Byte-identical within the new version.

Delete `ExecutionStats.strata` / `StratumStats` / `DeltaRows`. After:

```rust
pub struct ExecutionStats {
    pub introspection_version: u16,
    pub ctes: Vec<CteStats>,          // WITH units, declaration order; empty if with.is_empty()
    pub reach: Option<ReachStats>,    // None iff rec is None
    pub rules: Vec<RuleStats>,        // MAIN rules only
    pub emits: u64,                   // main sink
    pub disjoint_rules: Option<DisjointRules>, // MAIN rules only
    pub subsumed: Vec<SubsumedRule>,  // MAIN lowered-rule indices
    pub dead: Vec<DeadRule>,          // MAIN; WITH dead-ness is empty CTE table, not this list
}

pub struct CteStats {
    pub cte: u16,                     // CteId
    pub rules: Vec<RuleStats>,        // that WITH's rule loop
    pub emits: u64,
}

pub struct ReachStats {
    pub rules: Vec<RuleStats>,        // rec CTE: base arms then rec arms, declaration order
    pub rounds: Vec<RoundStats>,      // round 0 = base; ≥1 = Δ
}

pub struct RoundStats {
    pub delta: u64,                   // frontier size; one rec CTE, not Vec<DeltaRows>
    pub emitted: u64,
    pub absorbed: u64,
}
```

No `or` between CTE spans and a `ctes:` block: the structured stats **are** the `ctes:` block; counted-path spans are `CTE[i]` then `REACH` then `RULE[j]` (main). Drop `STRATUM` and the `MAX_PREDICATES == STRATUM.len()` assert. `CTE.len() == MAX_CTES`. One `REACH` span, not an array. WITH-only: `reach: None`, no `REACH` span.

## Allocation

WITH images are retained-capacity pools on the `PreparedQuery`, same contract as today's interior transients: high-water over `(generation, param envelope, iteration shape)`. WITH-only has no iteration-shape axis (eval once). Rec keeps the axis. Warm WITH-only is allocation-silent once CTE and main high-waters have been seen.

## Selectivity

CTE occurrences pin nothing at prepare. Floors: reuse `DELTA_PLANNING_ROWS` / `ACCUMULATED_PLANNING_ROWS` for rec delta vs acc; add `CTE_PLANNING_ROWS` equal to the accumulated floor for finished WITH (and for main's read of finished rec). Do not invent histograms. Document the constant at its definition.

## Tests that move with this file

`ir/validate/tests/program.rs` → `cte.rs` / `rec.rs`. Mutual / SCC / `UnresolvedPredicateSignature` / `UnknownOutputPredicate` tests become refusals or deletions (`04`). `a_degenerate_program_executes_as_its_query` → `a_plain_query_executes_as_today`. `prepare_executes_recursion_under_the_driver` → reach driver. `a_tight_fixpoint_budget_trips_with_the_typed_error` stays (rec only). A new lock: `a_with_only_query_does_not_enter_reach` — assert `!matches!(body, PreparedBody::Reach(_))` (WITH-only is `Rules` or `Empty`). Budget methods remain on every `PreparedQuery` and are ignored when `rec` is `None` (no new error; hosts copy-paste).

## C ABI (`cpp/foreign/bumbledb_c.h`)

```c
typedef enum bdb_atom_source_kind {
  BDB_ATOM_SOURCE_KIND_EDB,
  BDB_ATOM_SOURCE_KIND_CTE,
} bdb_atom_source_kind;

typedef struct bdb_with_def {
  const struct bdb_head_term *head;
  size_t head_count;
  const struct bdb_rule *rules;
  size_t rule_count;
} bdb_with_def;

typedef struct bdb_rec_cte {
  const struct bdb_head_term *head;
  size_t head_count;
  const struct bdb_rule *base;
  size_t base_count;
  const struct bdb_rule *rec;
  size_t rec_count;
} bdb_rec_cte;

typedef struct bdb_query {
  const struct bdb_with_def *with;
  size_t with_count;
  const struct bdb_rec_cte *rec;   /* nullable */
  const struct bdb_head_term *head;
  size_t head_count;
  const struct bdb_rule *rules;
  size_t rule_count;
} bdb_query;
```

Delete `bdb_program`, `bdb_predicate`, `bdb_prepare(..., const bdb_program *)`. `bdb_prepare(db, const bdb_query *)`. C++ sugar `cte<"n">` / `not_cte<"n">` lowers to `CTE`. This is bindings work in the same Rust-phase commit as the IR (`04`), listed here so the die-list is grep-complete. Regenerated header: cbindgen, not a hand edit.
