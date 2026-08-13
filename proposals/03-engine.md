# 03 — Engine: IR, drivers, what dies

Lean is the meaning (`02-lean.md`). This file is the Rust cut after Lean is green and the architecture docs have been rewritten in the present tense (`05-cutover.md`). Do not half-cut: a Query with empty interiors and no rec is a Query, not a one-predicate Program. There is no internal Program, no `ReachProgram`, no `enum Program` in `prepared.rs`.

This cut: one linear SCC, one `ReachDriver`, interiors then rec then main. SQLite is not consulted.

## IR (`crates/bumbledb/src/ir.rs`)

Delete: `Program`, `PredicateDef`, `PredId`, `ProgramRef`, `From<Query> for Program`, `From<&Query> for ProgramRef`, `From<&Program> for ProgramRef`, `MAX_PREDICATES`, `AtomSource::Idb`, `AtomSource::idb()`.

**Do not add `MAX_CTES`.** `MAX_PREDICATES` dies and is not renamed. Interior count is uncapped (`01-language.md`). `MAX_RULES` stays, per rule-list, rec pooled.

Add / change:

```rust
pub struct InteriorId(pub u32);  // same width as RelationId; not u16

pub enum AtomSource {
    Edb(RelationId),
    Interior(InteriorId),
}

impl AtomSource {
    pub fn edb(self) -> Option<RelationId> { /* ... */ }
    pub fn interior(self) -> Option<InteriorId> { /* was idb() */ }
}

pub struct Interior {
    pub head: Vec<HeadTerm>,
    pub rules: Vec<Rule>,
}

pub struct Rec {
    pub head: Vec<HeadTerm>,
    pub base: Vec<Rule>,
    pub rec: Vec<Rule>,
}

pub struct Query {
    pub interiors: Vec<Interior>,
    pub rec: Option<Rec>,
    pub head: Vec<HeadTerm>,
    pub rules: Vec<Rule>,
}
```

`Query::single` sets `interiors: vec![]`, `rec: None`. Existing conjunctive construction sites compile with two extra empty fields or via `Query::single`.

`InteriorId(i)` = `interiors[i]`. Rec, if any, = `InteriorId(u32::try_from(interiors.len()).…)` **after** the overflow check. Count with `usize`. Never `as u32` / `try_from(...).unwrap()` on hostile IR (`01-language.md`). Derived-table count that does not fit `u32` is `InteriorIdOverflow`.

Public re-exports in `lib.rs`: drop `Program`, `PredicateDef`, `PredId`, `MAX_PREDICATES`, `ProgramRef`, and do not export `MAX_CTES`. Export `Interior`, `Rec`, `InteriorId`. `AtomSource` stays, variant renamed. Stats: drop `StratumStats` / `DeltaRows` if they were reachable through `ExecutionStats`; add `InteriorStats` / `ReachStats` as fields of `ExecutionStats` (no extra root re-export required unless the harness names them).

`ir::render::render` emits the grammar in `01-language.md` (`interior` / `recursive`, not `with`). `render_program` dies. Goldens recut. Handles/punning/Allen/Duration unchanged.

`ir::normalize`: `Idb` typing surface (`signatures[pred]`) becomes `InteriorSignatures` — a slice of sealed column types, indexed by `InteriorId`, filled in declaration order before the rec, rec after interiors, main last. An `AtomSource::Interior` binding reads `FieldId(i)` against that slice. Point-membership on an interior interval column uses that sealed type (today's Idb membership in `normalize.rs`) — engine-only; Lean `Membership.lean` stays EDB (`02-lean.md`). Grounding still refuses Interior occurrences (not stored relations; no `U`/`M`; same as today's Idb refusal). `PlanOccurrence::relation()` panics on `Interior` the way it panics on `Idb` today — callers match `source`.

## Validate

`validate(schema, &Query) -> ValidatedQuery` is the **only** boundary. Delete `validate_program`, `ValidatedProgram`, `ir/validate/strata.rs` (the whole file: Tarjan, `condense`, `stratify`, the SCC tests). Do not retain Tarjan “for interior cycles” — those are `InteriorNotPrior`. Do not retain k-minting “for a future nonlinear.”

The new roster is `01-language.md` § validation roster (canonical error names there). Implementation notes:

- DNF / nesting / per-rule roster run **independently** on each `Interior.rules`, on `Rec.base`, on `Rec.rec`, and on `Query.rules`.
- **`MAX_RULES`:** each `Interior.rules` and the main `Query.rules` independently ≤ 16 (structural then DNF, today's query). The rec SCC is **one pool**: `base.len() + rec.len() ≤ MAX_RULES` (`TooManyRules { count: sum }`) and `dnf_width(base) + dnf_width(rec) ≤ MAX_RULES` (`DnfExceedsRules { produced: sum, cap }`). Not 16+16. Not a redundant per-arm structural cap on top of the pool. Not a parallel cap on how many interiors exist.
- Seal interiors in declaration order. Rec seals from `base` then `rec`. No sealing loop, no `UnresolvedPredicateSignature`.
- `InteriorSignatures` is a `Vec` of length `derivedCount`, not a chaotic fixpoint.
- Adversarial sweep: hostile `Query` values (random `InteriorId`s, self in base, two self-atoms, `interiors.len() == 100_000`). `rec` is `Option` and `interiors` is a prefix list — interior-after-rec is unrepresentable; inject out-of-range `InteriorId`s instead. Trust-boundary law unamended: no panic from IR data. 100_000 interiors is legal if each list respects `MAX_RULES`; it is slow prepare, not a typed `TooManyCtes`.
- Params unify once, query-global, after every rule of every interior and main has typed.

**Measure (pick A, matches `01-language.md`).** Interior **bodies** may contain measure comparisons; a ray raises `MeasureOfRay` after **that** interior finishes (then later interiors, rec, main still run only if the host continues — same as today's per-query Ray: the execute aborts). Rec refuses **every** measure site (`MeasureInInterior` on heads, `MeasureInRec` on bodies) before execute. Measure **finds** and folds only on **main**. Ray probes: after each interior, then after main. No rec probes. Delete "recursive programs defer the pass."

Error renames (`error.rs`), census-scanned: `01-language.md` table. `FixpointBudgetExceeded` **stays** — rec driver resource abort vs `reachDen`. Payload drops `stratum`: `{ rounds, tuples }`. One rec SCC, no strata.

### `ValidatedQuery` — three parts, not one lowered list

Today's `ValidatedQuery { lowered, predicate, rules, param_types, ... }` cannot branch interiors vs rec vs main. After:

```rust
pub struct ValidatedInterior {
    lowered: Vec<LoweredRule>,
    predicate: Predicate,       // ir::validate::Predicate — sealed column types
                                // of this interior head. NOT PredicateDef. NOT PredId.
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
    interiors: Vec<ValidatedInterior>,
    rec: Option<ValidatedRec>,
    main: ValidatedMain,
    param_types: BTreeMap<ParamId, ValueType>,
    set_params: BTreeSet<ParamId>,
    point_params: BTreeSet<ParamId>,
}
```

`validate` returns this. Prepare reads `interiors` then `rec` then `main`. `ValidatedProgram` dies — including `witness(PredId)` / `output()`. Accessors: `interiors[i]`, `rec`, `main`. `PreparedQuery::predicate()` remains the **main/answer** head (today's output signature). Do not reuse today's single `lowered` by concatenating interior rules into the main list.

`normalize_predicate` becomes per-interior / rec / main `normalize` against `InteriorSignatures`. `CountAcrossRules` is a main-query error (interior/rec heads cannot carry Count — `AggregateInInterior`).

## Prepare / execute

Today (`api/prepared.rs`):

```rust
enum Program { Empty, Rules(Vec<PreparedRule>), Fixpoint(Box<FixpointProgram>) }
```

After:

```rust
struct PreparedQuery {
    interiors: Vec<PreparedInterior>,  // declaration order; empty iff Query.interiors is
    body: PreparedBody,                // Empty | Rules(main) | Reach(driver)
    // ... scratch, schema pin, rendered, budget (ignored when rec is None)
}

enum PreparedBody {
    Empty,                          // main emits nothing; interior preamble still runs
    Rules(Vec<PreparedRule>),       // main rules; rec is None
    Reach(Box<ReachDriver>),        // rec: Some
}
```

Interior units are **not** inside `PreparedBody`. An interiors-only query is `interiors.len() > 0` and `body` is `Rules` or `Empty` — never `Reach`. The lock `an_interiors_only_query_does_not_enter_reach` asserts `!matches!(body, Reach(_))`.

**Rename the inner enum.** Today's `Program` as a prepared-body name is a leftover lie. Grep-clean `enum Program` in `prepared.rs`. The driver type is `ReachDriver`, not `ReachProgram`.

### Interiors-only (`rec == None`)

Must **not** build `Reach` / enter `run_reach`. Path:

1. For each `Interior` in order: prepare as today's per-rule pipeline (key probe or Free Join), sink = `ProjectionSink` (bound-var head). Interior occurrences of earlier interiors cost on the selectivity floors (row count unknown at prepare — same as Idb). Pin-at-prepare unamended.
2. Prepare main rules. Interior occurrences of any named interior: floors.
3. Execute: for each interior, `run_rules` into its projection sink, transpose to `TransientImage` (`synthesize`/`refill` from seen-set rows — today's interior-predicate image), bind that image for later interiors and main `Interior` occurrences (`Colt::reset`, bypass `memo.bind`, recycle `spare_buffers` — the Idb arm in `run_join`, renamed Interior). Then `run_rules` main into the head-owned sink. Ray probes: each interior is an ordinary query — probe after that interior finishes; main probes after main. No deferred-Idb carve-out.

Statically empty: a dead interior is the empty table (later readers see nothing). A dead main with live interiors still runs the interior preamble (params/bind errors must surface; an interior measure comparison can Ray). A query of only dead **main** rules with empty interiors is today's `Empty` plan. Pick: interiors-only where every **main** rule is dead → `Empty` after the interior units are still prepared (bind + ray on interiors still run). Do not skip interiors when main is empty-plan: an interior can be the only measure site. Simpler pick that matches "eval once": always run the interior preamble when `interiors` is nonempty; `Empty` means main emits nothing and has no live rule, not "skip the query."

`EmptyRuleSet` is validate of IR (`Query.rules` empty). `PreparedBody::Empty` is plan after static deadness. Both exist; they are not the same.

### Rec present

`ReachDriver` is a **specialization** of today's `FixpointProgram`, not a second evaluator. The name is `ReachDriver`, not `ReachProgram` — grep for `Program` must stay zero (except comments that say the word is deleted, and git history).

- No `strata_members`, no `top_stratum`, no per-predicate SCC, no `recursive: bool` per predicate.
- Interior preamble as above (finished images, eval once). Then rec. Then main.
- One rec predicate: `base: Vec<PreparedRule>` (round 0, ordinary rule loop into the rec `ProjectionSink`), `rec: Vec<RecursiveRule>` (rounds ≥ 1).
- **`RecursiveRule` holds exactly one `DeltaVariant`.** Delete `variants: Box<[DeltaVariant]>`. The unique positive self-atom is the delta occurrence. Nonlinear k-minting in `prepare_program` (`prepare_rule_variant` loop over same-stratum Idb atoms) dies. Extra EDB / interior atoms on a rec arm are accumulated/EDB, never a second delta.
- Frontier = rec sink's seen-set, watermark, `answers_since`, `TransientImage` ping-pong for delta vs accumulated of **that one SCC**. Delete per-predicate arrays sized by `MAX_PREDICATES`; size 1 for rec plus `interiors.len()` finished slots (the interior slot count is data, not a 16-slot array).
- Stop: empty Δ. That is Lean `reachStep acc ⊆ acc` / `T(acc) ⊆ acc` (`02-lean.md`). Budget: `DEFAULT_FIXPOINT_ROUNDS` / `DEFAULT_FIXPOINT_TUPLES` stay (rename constants only if the module is renamed; values stay — 2¹⁶ rounds, 10⁷ tuples). Size is the wall, not round count. `set_fixpoint_budget` stays on `PreparedQuery` and is ignored when `rec` is `None` (no new error; hosts copy-paste).
- Then main rules, binding the finished rec image like an interior.
- Ray probes: rec is measure-free by roster — no rec probes. Interiors as above. Main after main. Delete "recursive programs defer the pass."

`run_join`'s Idb arm is the Interior arm. One code path for interior images and rec images: an `Interior` occurrence binds a `RelationImage` the driver stuffed in `interior_images[id]`. Generation map never learns derived tables exist (today's Idb law).

### What the driver is not

- Not `evalProgram` over strata.
- Not a host-loop internalization (ParamSet-per-round). Cookbook 24's host idiom stays in the host.
- Not k delta-variants.
- Not Tarjan.
- Not mutual recursion (OPEN, other cuts — do not leave a corpse).
- Not a second Free Join.
- Not named `ReachProgram`.
- Not entered by interiors-only.
- Not SQLite's recursive CTE evaluator.

`lean/Bumbledb/Exec/Reach.lean: evalLinearReach_eq_lfp` and `semi_naive_agrees` at `reachOp` are the agreement. `reachOp_empty` is round 0 = base. Lean `evalLinearReach` is the naive chain; this driver is the semi-naive realization.

## Files / types that die

| Path / type | Fate |
|---|---|
| `ir::{Program, PredicateDef, PredId, ProgramRef, MAX_PREDICATES}` | Delete. **No `MAX_CTES`.** |
| `AtomSource::Idb` / `AtomSource::idb` | `Interior` / `interior` |
| `From<Query> for Program` and `ProgramRef` From impls | Delete. No embedding |
| `ir/validate/strata.rs` | Delete the file |
| `validate_program`, `ValidatedProgram`, `IdbSignatures` | Delete / become `InteriorSignatures` on the one `validate` |
| `ir/render.rs` `render_program` | Delete |
| `Db::render_program` (`api/db.rs`) | Delete. Only `render_query` |
| `api/prepared.rs` `enum Program` | `PreparedBody`; drop `Fixpoint` |
| `api/prepared/fixpoint.rs` | Rewrite as `reach.rs` (or gut in place and rename). Drop strata, k-variants, per-pred SCC. Keep watermark, TransientImage, budget, round-0 base / round-r Δ |
| `FixpointProgram`, `FixpointPredicate`, `FixpointScratch` (multi-pred) | `ReachDriver`, one rec scratch + interior slots |
| `RecursiveRule.variants: Box<[DeltaVariant]>` | one `DeltaVariant` |
| `prepare_program` in `build.rs` | `prepare` handles Query; branch `rec.is_some()` vs interiors-only vs plain |
| `Db::prepare(..., impl Into<ProgramRef>)` | `prepare(&Query)` only. Hostile nesting still screened on `&Query` before any clone — the ProgramRef borrow rationale is gone because the clone-into-Program path is gone |
| `obs.rs` `STRATUM` array, `MAX_PREDICATES == STRATUM.len()`, `VALIDATE_STRATIFY` | Delete. `VALIDATE_SEAL` becomes declaration-order interior sealing (one span, not a chaotic loop) or dies if sealing is not worth a span — pick: keep one `VALIDATE_SEAL` over interior count. **No `INTERIOR[16]` array** and no `CTE.len() == 16` assert — that would be `MAX_CTES` in obs clothing. Counted path: one `INTERIORS` span (args: count, emits) + one `REACH` span + `fixpoint_round` children + `RULE[j]` (main, still `MAX_RULES`-bounded). Per-interior detail lives in structured `ExecutionStats.interiors`. No `or silent` fork |
| `ExecutionStats.strata`, `StratumStats`, `DeltaRows` | `interiors` / `reach` / `RoundStats.delta: u64` |
| Naive `NaiveDb::program` (`naive/query.rs`) | Extend `NaiveDb::query`: interiors then rec lfp then main. Delete the program entry |
| `translate_program` | `translate_query`: lossy SQL of this cut's fragment (`04`). Gate shrinks: mutual / nonlinear / in-cycle fold are unwritable this cut; keep interval-column translator limit. SQLite spelling is not the IR |
| `querygen/shapes_recursive.rs` | Rewrite to emit `Query` (interiors + optional Rec + identity main). Drop mutual and nonlinear variants. Keep linear, main-negation of finished rec, main fold, empty-Δ, budget-trip, primer-shaped `reach(x,x)` |
| Conformance `generate_program_corpus` / `conformance/program.rs` | Emit Reach JSON (`reach-*.json`). Lean corpus already recut in the Lean commit; do not run the old generator until this rewrite |
| C ABI `bdb_program` / `bdb_predicate` / `BDB_ATOM_SOURCE_KIND_IDB` | `bdb_query { interiors, rec, head, rules }`; `BDB_ATOM_SOURCE_KIND_INTERIOR`. `bdb_prepare(db, const bdb_query *)`. Delete `prepare(bdb_program)` in `raii.cc` |
| C++ `.idb` / `.not_idb` | `.interior<"n">` / `.not_interior<"n">` |
| `cpp/bridge` `program_in` | `query_in` |

`lib.rs` public surface after: `Query`, `Rule`, `Interior`, `Rec`, `InteriorId`, `AtomSource`, `MAX_RULES`. No Program. No `MAX_CTES`. `AtomSource::interior()` not `idb()`.

## Introspection

`INTROSPECTION_VERSION` 3 → 4 (content change). Byte-identical within the new version.

Delete `ExecutionStats.strata` / `StratumStats` / `DeltaRows`. After:

```rust
pub struct ExecutionStats {
    pub introspection_version: u16,
    pub interiors: Vec<InteriorStats>, // named interiors, declaration order; empty if interiors.is_empty()
    pub reach: Option<ReachStats>,     // None iff rec is None
    pub rules: Vec<RuleStats>,         // MAIN rules only
    pub emits: u64,                    // main sink
    pub disjoint_rules: Option<DisjointRules>, // MAIN rules only
    pub subsumed: Vec<SubsumedRule>,   // MAIN lowered-rule indices
    pub dead: Vec<DeadRule>,           // MAIN; interior dead-ness is empty table, not this list
}

pub struct InteriorStats {
    pub interior: u32,                 // InteriorId
    pub rules: Vec<RuleStats>,         // that interior's rule loop
    pub emits: u64,
}

pub struct ReachStats {
    pub rules: Vec<RuleStats>,         // rec: base arms then rec arms, declaration order
    pub rounds: Vec<RoundStats>,       // round 0 = base; ≥1 = Δ
}

pub struct RoundStats {
    pub delta: u64,                    // frontier size; one rec SCC, not Vec<DeltaRows>
    pub emitted: u64,
    pub absorbed: u64,
}
```

No `or` between interior spans and an `interiors:` block: the structured stats **are** the `interiors:` block. Counted-path spans are `INTERIORS` then `REACH` then `RULE[j]` (main). Drop `STRATUM` and the `MAX_PREDICATES == STRATUM.len()` assert. Do not replace it with `INTERIOR.len() == 16`. Interiors-only: `reach: None`, no `REACH` span.

## Allocation

Interior images are retained-capacity pools on the `PreparedQuery`, same contract as today's interior transients: high-water over `(generation, param envelope, iteration shape)`. Interiors-only has no iteration-shape axis (eval once). Rec keeps the axis. Warm interiors-only is allocation-silent once interior and main high-waters have been seen.

## Selectivity

Interior occurrences pin nothing at prepare. Floors: reuse `DELTA_PLANNING_ROWS` / `ACCUMULATED_PLANNING_ROWS` for rec delta vs acc; add `INTERIOR_PLANNING_ROWS` equal to the accumulated floor for finished interiors (and for main's read of finished rec). Do not invent histograms. Document the constant at its definition.

## Tests that move with this file

`ir/validate/tests/program.rs` → `interior.rs` / `rec.rs`. Mutual / SCC / `UnresolvedPredicateSignature` / `UnknownOutputPredicate` tests become refusals or deletions (`04`). `a_degenerate_program_executes_as_its_query` → `a_plain_query_executes_as_today`. `prepare_executes_recursion_under_the_driver` → reach driver. `a_tight_fixpoint_budget_trips_with_the_typed_error` stays (rec only). A new lock: `an_interiors_only_query_does_not_enter_reach` — assert `!matches!(body, PreparedBody::Reach(_))` (interiors-only is `Rules` or `Empty`). Budget methods remain on every `PreparedQuery` and are ignored when `rec` is `None` (no new error; hosts copy-paste). A new lock: many interiors (more than 16) still validate if each list respects `MAX_RULES` — the `MAX_CTES` corpse must not return as `TooManyCtes`.

## C ABI (`cpp/foreign/bumbledb_c.h`)

```c
typedef enum bdb_atom_source_kind {
  BDB_ATOM_SOURCE_KIND_EDB,
  BDB_ATOM_SOURCE_KIND_INTERIOR,
} bdb_atom_source_kind;

typedef struct bdb_interior {
  const struct bdb_head_term *head;
  size_t head_count;
  const struct bdb_rule *rules;
  size_t rule_count;
} bdb_interior;

typedef struct bdb_rec {
  const struct bdb_head_term *head;
  size_t head_count;
  const struct bdb_rule *base;
  size_t base_count;
  const struct bdb_rule *rec;
  size_t rec_count;
} bdb_rec;

typedef struct bdb_query {
  const struct bdb_interior *interiors;
  size_t interior_count;
  const struct bdb_rec *rec;   /* nullable */
  const struct bdb_head_term *head;
  size_t head_count;
  const struct bdb_rule *rules;
  size_t rule_count;
} bdb_query;
```

Delete `bdb_program`, `bdb_predicate`, `bdb_prepare(..., const bdb_program *)`. `bdb_prepare(db, const bdb_query *)`. C++ sugar `interior<"n">` / `not_interior<"n">` lowers to `INTERIOR`. This is bindings work in the same Rust-phase commit as the IR (`04`), listed here so the die-list is grep-complete. Regenerated header: cbindgen, not a hand edit.
