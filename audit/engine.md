# Engine representation audit

Brooks: the tables make the flowcharts obvious. Pike: data dominates; algorithms follow. Applied to `crates/bumbledb/` (IR, validate, prepare, execute) and the engine-facing bench oracles (`naive`, `querygen`, `conformance`, `translate`).

Program IR is gone. Query is interiors + optional Rec + main. The engine claims one loop (`run_rules` / `run_join`) with `run_reach` owning watermark/budget. The types still describe a stratified Program, and the control flow is still guarding combinations the new IR was supposed to make unwritable.

This-cut OPEN refusals (mutual recursion, nonlinear rec arms) are **not** counted as bugs. They are named only where the *witness* throws the proof away and every downstream site re-discovers it.

---

## The shape that is wrong

The lever is one missing sum. After parse, a query is not three independent pieces. It is a sequence of eval-once tables, optionally one linear rec SCC, then main. Today that fact is restated as flags:

```
Query { interiors: Vec<Interior>, rec: Option<Rec>, rules: Vec<Rule> }
ValidatedQuery { interiors, rec: Option<ValidatedRec>, main }
PreparedQuery { interiors: Vec<PreparedInterior>, body: Empty | Rules | Reach }
```

Every `rec.is_some()`, `interiors.is_empty()`, `matches!(body, Reach(_))`, `unreachable!("recursive rules live under Reach")`, and `expect("rec present")` is a branch guarding a state those products still admit. Interiors sit *beside* the body that was supposed to be the whole program. Rec is an InteriorId by pun (`interiors.len()`) and an `Option` by type. Recursive rules are a `PreparedRule` variant legal in every list, then forbidden in all but one.

The collapsing coordinate is homogeneous derived tables plus a parsed body that cannot spell the illegal combinations. Until that exists, the flowchart cannot get simpler than the table.

---

## Findings

### F1. Interiors live beside `PreparedBody`, so every consumer reconstitutes “derived?” from two flags

- **Where:** `crates/bumbledb/src/api/prepared.rs:217-218`; `execute.rs:90-110,160-172`; `reach.rs:133-205`; `introspect.rs:214-302`
- **What's wrong:** `PreparedQuery` carries `interiors: Vec<PreparedInterior>` *and* `body: PreparedBody`. `PreparedBody` is documented as the main body (`Empty` / `Rules` / `Reach`) with the note that interiors-only is `Rules` or `Empty` — never `Reach`. Execution then rebuilds the missing coordinate with a product of tests: `interiors.is_empty() && Empty`, `interiors.is_empty() && KeyProbe`, `!interiors.is_empty() || Reach`, `Reach` vs not. `run_derived` itself opens with `interiors.len() + usize::from(matches!(body, Reach(_)))`. Two independent fields, eight-ish states, a handful valid. This is Minsky’s three-boolean example with better names.
- **Collapsing representation:** one prepared pipeline, not a sidecar plus a tag.

```rust
enum PreparedPipeline {
    Empty { predicate: Predicate },
    Main { interiors: Vec<PreparedInterior>, rules: Vec<PreparedRule>, sink: EitherSink },
    Reach { interiors: Vec<PreparedInterior>, driver: ReachDriver }, // driver owns rec + main
}
```

`run_rules` becomes a match. `run_derived` is not a gate in front of the main loop; it is the Reach/Main arm. Interiors-only with a dead main is `Main { rules: vec![], .. }`, not `Empty` with a secret preamble.
- **Essential vs accidental:** accidental. The denotation *is* interiors then rec then main. The product type is the old Program leftover: interiors bolted onto a body enum that used to be the whole query.
- **Severity:** high

### F2. `PreparedRule::Recursive` is legal in every rule list; `unreachable!` is the typechecker

- **Where:** `crates/bumbledb/src/api/prepared.rs:331-338`; `execute.rs:299-301`; `introspect.rs:53-55,73-75,87-88,100-102,312-314,343-345`; `reach.rs:417-419`; `build.rs:424-429`
- **What's wrong:** `PreparedRule` is `FreeJoin | KeyProbe | Recursive`. Recursive is “a rec arm… Runs only under `PreparedBody::Reach`, in rounds ≥ 1.” The type admits Recursive in interiors, in base, in main, in `PreparedBody::Rules`, in `Empty`’s phantom list. Every walk then has a Recursive arm that panics, continues, or is “never.” `run_reach` even `continue`s on a non-Recursive rec arm (`reach.rs:417-419`) — a silent skip of an illegal state rather than a parse. Base/main Recursive is `unreachable!`. Rules-body Recursive is `unreachable!`. This is not an algorithm. It is a tag plus a forest of guards (Fowler’s type-code, not yet replaced).
- **Collapsing representation:** ordinary rules are `FreeJoin | KeyProbe`. Rec arms are a different type, only inhabitable in the rec-arm slot.

```rust
enum PreparedRule { FreeJoin(FreeJoinRule), KeyProbe(KeyProbeRule) }
struct RecArm { delta: OccId, rule: FreeJoinRule } // only ReachDriver.rec: Vec<RecArm>
```

The Recursive match arms delete. `continue` on the wrong variant becomes unrepresentable.
- **Essential vs accidental:** accidental. One positive self-atom per rec arm is essential (and this-cut nonlinear is an OPEN refusal, not a bug). The essential fact belongs on `RecArm`, not as a third kind of every rule.
- **Severity:** high

### F3. Rec is an `Option` *and* `InteriorId(interiors.len())` — a coordinate lie that every site recomputes

- **Where:** `crates/bumbledb/src/ir.rs:48-50,74-85,425-457`; `ir/validate/validate.rs:31,83-84`; `api/prepared/reach.rs:303,421-427`; `ir/render.rs:152-153`; `bumbledb-bench/src/naive/query.rs:265-269`; `bumbledb-bench/src/translate/reach.rs:42-44,53`
- **What's wrong:** `AtomSource::Interior(InteriorId)` names interiors *and* the rec. The rec’s id is a convention: `interiors.len()` after an overflow check, restated at validate, prepare, execute, render, naive, and SQL. `query.rec.is_some()` is the other half of the same fact. An `InteriorId` past the interiors vec is either the rec, or `UnknownInterior`, depending on a flag the id does not carry. Dijkstra: the off-by-one lived in the numbering. Here the special case lives in “last id, if Option is Some.” Homogeneous coordinates would make rec a derived table with a different *evaluation*, not a different *addressing*.
- **Collapsing representation:** dense derived ids over a single table list. Rec is not a pun on `len`.

```rust
struct DerivedId(u32); // index into derived: Vec<Derived>
enum Derived { Interior(Interior), Rec(Rec) } // Rec last, at most one — or a parsed QueryShape
enum AtomSource { Edb(RelationId), Derived(DerivedId) }
```

If this-cut “at most one rec, last” is the coordinate, the parser emits `Query { derived: Vec<Interior>, rec: Rec, main }` vs `Query { derived, main }` as a sum, and `DerivedId` is in-range by construction on the witness. No site re-adds `len + is_some()`.
- **Essential vs accidental:** the *existence* of at most one linear rec SCC is essential this cut. Encoding it as `Option` beside a vec, then punning its identity as `len`, is accidental. `AtomSource::Interior` naming the rec is a leftover predicate/IDB vocabulary.
- **Severity:** high

### F4. `Query.rec: Option<Rec>` admits “a Rec that is an Interior”; validation says so and then keeps the type

- **Where:** `crates/bumbledb/src/ir.rs:431-437`; `error.rs:859-862`; `ir/validate/validate.rs:245-251,340-345`
- **What's wrong:** `Rec { base: Vec<Rule>, rec: Vec<Rule> }` can have empty `base` or empty `rec`. `EmptyRecursiveStep`’s own doc: “`Rec.rec` is empty: that is an interior — write an interior.” The error is the representation talking. The type still allows the state; `rec_roster` rejects empty lists; `lower_rec_pool` rejects them *again* after DNF (a second emptiness check on a different stage, same fact). SPOV 3: the special case is a Rec that isn’t recursive. Change the coordinate and it is an Interior, not a roster item.
- **Collapsing representation:** at the *witness* (parsed) layer, rec arms are nonempty by type (`NonEmpty<Rule>` or a `Rec` constructor that only exists with both lists nonempty). At the untrusted `Query` layer, empty lists are hostile input — parse them into `EmptyRecursiveBase/Step` once, and `ValidatedRec` cannot spell them. Do not re-test after DNF with the same variant: a DNF-emptied pool is a different parsed error or a dead rec, not a second `EmptyRecursiveStep`.
- **Essential vs accidental:** empty-base ⇒ empty lfp is essential mathematics (`T(∅)=∅`). Empty-step ⇒ “this is an Interior” is accidental splitting of one derived-table kind. Nonlinear/missing-self on a rec *arm* are this-cut refusals (not bugs); empty-step is not that class — the engine already knows it is the other type.
- **Severity:** high

### F5. Validation discards the unique-self proof; prepare searches again

- **Where:** `crates/bumbledb/src/ir/validate/validate.rs:258-265`; `api/prepared/build.rs:407-413`; `ir/validate.rs:523-540,555-563`
- **What's wrong:** King: `validateNonEmpty` returns `()` and every caller re-checks. `rec_roster` counts positive self-atoms and rejects 0 (`RecArmMissingSelf`) or ≥2 (`NonlinearRecArm` — OPEN this cut, not a bug). The witness is still `ValidatedRec { rec: Vec<LoweredRule> }` with no `OccId` of the self-atom. `prepare_reach` walks occurrences and `.expect("RecArmMissingSelf judged at validate")`. `rec_base_rule` / `rec_step_rule` are `self.rec.as_ref().expect("rec present")` while `rec_base_rules` uses `map_or(0, …)` so the iterator is safe only because the count is zero when rec is missing. The proof never made it into the type.
- **Collapsing representation:** parse the rec arm into `ValidatedRecArm { self_occ: OccId, rule: LoweredRule, typing: RuleTyping }`. Nonlinear/missing-self die at the boundary. Prepare reads `arm.self_occ`. `ValidatedQuery` is `enum { Plain {..}, Rec { rec: ValidatedRec, .. } }` so `rec_base_rule` has no `expect`.
- **Essential vs accidental:** unique positive self-atom is essential this cut. Re-finding it is accidental. Nonlinear remaining a *validation error* (not a runtime bug) is in-scope as a refusal, out of scope as a defect.
- **Severity:** high

### F6. `InteriorSignatures` uses `Option` holes and screens twice

- **Where:** `crates/bumbledb/src/ir/validate.rs:284-343`; `ir/validate/validate.rs:38,90-105`; `ir/validate/context.rs:487-525`
- **What's wrong:** `sealed: &'a [Option<Predicate>]` — “A `None` slot is a table not yet sealed.” Rec is pushed as `None`, base is typed, then the hole is filled, then rec arms type. `column` calls `screen` *again* after `check_atoms` already screened the atom. `screen` fails `index >= derived_count`; `column` then `arities.get(index)` as `UnknownInterior` again; then `sealed.get(index).and_then(Option::as_ref)` as `UnknownInterior` a third time. Three encodings of “this id is live”: count, arity slot, Some predicate. Shotgun parsing of a linear seal.
- **Collapsing representation:** two slices, not one Option-padded array. Type interiors against `&[Predicate]` already sealed (declaration order). Type rec *base* against that same slice (base must not read rec — unrepresentable if rec isn’t in the slice yet). Type rec *arms* against `sealed + rec_predicate`. `UnknownInterior` is out-of-range on that slice. No `None` hole, no second screen inside `column`.
- **Essential vs accidental:** sealing in declaration order is essential (DAG). The Option hole is a phase flag stuffed into the data — accidental. Double screen is accidental.
- **Severity:** high

### F7. k-variant leftover: one delta, still wrapped as “the variant”

- **Where:** `crates/bumbledb/src/api/prepared.rs:341-357,522-567`; `api/prepared/build.rs:414-429,651-674,718-719`; `reach.rs:4,499,582`; `exec/introspection.rs:88-94`; `introspect.rs:36-40`; `plan/selectivity.rs:89-110,134-135`
- **What's wrong:** Comments insist “No k-variant minting.” The types still mint one: `RecursiveRule { variant: DeltaVariant { delta, rule } }`. Accessors say “any variant speaks for the rule,” “variant 0 speaks,” “every variant shares the slot layout.” `prepare_rule` is a wrapper that passes `delta: None` into `prepare_rule_variant`. Introspection still explains a fixpoint program as “a recursive rule as its delta variants” with labels of the form `predicate p0 rule 1 delta variant 0`, and claims the counted surface is `stats.strata` — a field that does not exist (`ExecutionStats` has `reach: Option<ReachStats>`). Selectivity still names “a variant’s marked occurrence.” The representation of “one rec arm, one delta occurrence” is a singleton of a list that used to be k-wide.
- **Collapsing representation:** delete `DeltaVariant`. `RecArm { delta: OccId, rule: FreeJoinRule }`. `prepare_rule` takes `delta: Option<OccId>` only if you must share the pipeline — better, `prepare_rec_arm(..., delta: OccId)` with the floor choice as data on the occurrence, not a side channel. Introspection labels `reach rec {i} (delta occ {d})` without “variant.” `stats.strata` comments die with the field they name.
- **Essential vs accidental:** one delta occurrence per rec arm is essential (semi-naive). A `Variant` newtype around a single value is accidental residue of k-magic-set / k-variant minting.
- **Severity:** high

### F8. `execute` and `profile` are two copies of the same protocol, with different predicates

- **Where:** `crates/bumbledb/src/api/prepared/execute.rs:78-144,154-182`; `introspect.rs:202-376`
- **What's wrong:** `profile` comments that it “mirrors `run_rules` with per-rule accounting inline” — then it does not call `run_bound`. Empty short-circuit is copied (`interiors.is_empty() && Empty`). Key-probe fast lane is copied **wrong**: execute requires `KeyProbe { key_probe_finds: Some(_) }` (plain-var finds); profile matches `[PreparedRule::KeyProbe(_)]` (any single key-probe, including aggregate/measure probes that execute keeps on the sink path). Reach is a third copy (`bind` + `run_rules` + `ReachCounters`). Interiors-only non-Reach is a fourth: `run_derived` then a hand-rolled main loop that `unreachable!`s Recursive. Two representations of “which access path is this?” — the body enum, and a forest of `if`s that disagree.
- **Collapsing representation:** one execution function parameterized by `Counters`. Fast lanes are parsed into the pipeline enum (F1), not re-detected. Profile is `execute` with `CountingCounters` / `ReachCounters`. The key-probe direct path is a property of `PreparedRule::KeyProbe { key_probe_finds: Some(_) }` — one predicate, both callers.
- **Essential vs accidental:** ANALYZE must count; that is essential. Duplicating the protocol, and widening the fast-lane predicate on the counted path, is accidental. The divergence is a defect the product type invited.
- **Severity:** high

### F9. `run_reach` re-matches `PreparedBody::Reach` because interiors stole the other borrow

- **Where:** `crates/bumbledb/src/api/prepared/reach.rs:292-450`
- **What's wrong:** The function asserts Reach, then re-matches to reset, re-matches for round 0, re-matches every loop iteration (`unreachable!("matched above")` ×3). The cause is F1: `self.interiors`, `self.derived`, `self.body` are sibling fields; a mutable `ReachDriver` and `DerivedScratch` cannot be borrowed together without stuffing the driver back into the enum. Control flow is compensating for a layout that will not split. Greenspun: a bad interpreter of “which piece of self is live.”
- **Collapsing representation:** `ReachDriver` owns rec scratch and the rec sink. `run_reach(&mut self.driver, &mut self.derived, …)` after a match that *stays* matched — or `PreparedPipeline::Reach` holds `{ interiors, driver, derived }` so the Reach arm has everything it needs. The inner `let PreparedBody::Reach(driver) = &mut self.body else { unreachable! }` loop vanishes.
- **Essential vs accidental:** split borrows are a Rust tax. Four identical discriminant checks are accidental layout.
- **Severity:** high

### F10. `fill_plan_images` takes three independent `Option`s for one rec bind

- **Where:** `crates/bumbledb/src/api/prepared/reach.rs:454-561,568-583`; `run_join.rs:31-32,96-99`
- **What's wrong:** `fill_plan_images(plan, derived, rec_id: Option<usize>, rec_delta: Option<&Arc<RelationImage>>, rec_acc: Option<&Arc<RelationImage>>)` admits rec_id without images, images without rec_id, delta without acc. The body is `if rec_id == Some(q) { if rec_delta.is_some() && rec_acc.is_some() { acc } else { finished } }` then a second walk that overwrites self-atoms with delta when both rec_id and delta are Some. `fill_finished_images` still takes `variant_delta: Option<OccId>` and discards it (`let _ = variant_delta`). `run_into_projection` still takes `rec_delta: Option<OccId>` and discards it (`let _ = rec_delta`). The delta occurrence is now filled in `fill_plan_images`, but the old parameters remain as unused Option soup — k-variant bind leftover. `run_join` then takes `idb_images: &[Option<Arc<RelationImage>>]` (zombie IDB name) and `.expect("the reach driver supplies every Interior occurrence's image")` — None is representable for an Interior occ, then panicked.
- **Collapsing representation:**

```rust
enum DerivedBind<'a> {
    Finished(&'a DerivedScratch),
    Rec { id: usize, delta: &'a Arc<RelationImage>, acc: &'a Arc<RelationImage> },
}
```

`occ_images: Vec<Arc<RelationImage>>` sized to Interior occurrences only, or a dense map from `OccId` — not `Vec<Option<_>>` with expect. Drop `variant_delta` / `rec_delta` parameters that nobody reads. Rename `idb_*` to `derived_*`.
- **Essential vs accidental:** delta vs accumulated vs finished is essential semi-naive. Three Options plus two unused leftovers plus IDB naming is accidental.
- **Severity:** high

### F11. Zombie Program / strata / IDB / predicate vocabulary still structures live data

- **Where:** `crates/bumbledb/src/api/prepared.rs:191,219,275`; `api/prepared/build.rs:23,107,145,597`; `api/stats.rs:14-52`; `exec/introspection.rs:15,87-94`; `exec/introspection/display.rs:23-25,153,186`; `introspect.rs:36-40`; `api/prepared/run_join.rs:31-32`; `ir/normalize/normalize.rs:16-18`; `plan/fj/validate.rs:216-217`; `tests/reach_finalize_hunt.rs:203`; `bumbledb-bench/src/conformance/reach.rs:11`; `bumbledb-bench/src/closure.rs:502`
- **What's wrong:** Program IR is deleted. The tables still say Program. `ground_program` names the main-rule grounding. Comments say “inert when `rec` is `None`” on a type that has `body`, not `rec`. Introspection: “fixpoint program,” “query-shaped programs,” “counted surface is `stats.strata`,” unit label example `predicate p0 rule 1 delta variant 0`. Display still prints `predicate p{id}` for `AtomSource::Interior` and sends fixpoint counts to “the strata section.” `run_join` parameters are `idb_images` / `idb_retired`. `normalize()` (dead_code) claims “no Interior occurrence exists in a sealed ValidatedQuery (the query boundary has no predicate address space)” — false. `fj/validate.rs` repeats it: “a sealed ValidatedQuery carries no Interior occurrence.” Conformance reach docs boast “No `predicates` / `output` / `strata` / `idb`” while the engine still speaks those words. Closure families skip profile because “the profile path is query-shaped; rec queries skip it” — F8 made flesh: rec is still a side path. `reach_finalize_hunt.rs`: “the two-strata program became.”
- **Collapsing representation:** rename to the Query vocabulary and delete the dead `normalize` empty-surface path. `ground_main`. `stats.reach`. `derived_images`. Display `interior {id}` / `rec {id}`. Profile rec queries through the same counters seam (F8). A comment that says the witness has no Interior occurrences is a lie; delete the function it excuses.
- **Essential vs accidental:** accidental. The denotation did not keep Program, strata, or IDB. The names did.
- **Severity:** high (the false “no Interior on ValidatedQuery” invariant and `stats.strata` / query-shaped profile skip still divert control flow; the rest of the vocabulary would be low if it were comments only)

### F12. `ExecutionStats` is three independent fields; interior/reach rule tables are dead

- **Where:** `crates/bumbledb/src/api/stats.rs:16-75`; `introspect.rs:251-252,281-290,373-375,379-388`; `exec/introspection/reach_counters.rs:20-28`
- **What's wrong:** `ExecutionStats { rules, interiors: Vec<InteriorStats>, reach: Option<ReachStats> }`. Legal-looking combos: `reach: Some` on interiors-only, `interiors: []` on a query that ran interiors, `reach: None` on Reach. Profile constructs them with the same flag forest as execute. `InteriorStats.rules` is always `Vec::new()` (`interior_stats`). `ReachStats.rules` is always `into_reach(Vec::new())`. The fields are the old per-stratum rule tables, kept, never filled. Display therefore cannot print per-interior rule stats; it prints `interior p{}: {} emits` and `reach: {} rounds`. Structured stats “are the interiors block — there is no parallel span-or-stats fork” — except `rules` on those structs is exactly that fork, empty.
- **Collapsing representation:** stats shaped like the pipeline enum (F1). `InteriorStats { id, emits }` without a ghost `rules`. `ReachStats { rounds }` without a ghost `rules`. `reach: Option` only on a `Pipeline::Reach` stats arm — unrepresentable on interiors-only.
- **Essential vs accidental:** per-interior emits and per-round delta/emitted/absorbed are essential. Empty `rules` vecs and a parallel Option are accidental strata residue.
- **Severity:** high

---

### F13. Dual scratch layouts for the same derived-image protocol

- **Where:** `crates/bumbledb/src/api/prepared/reach.rs:49-102,395-415`
- **What's wrong:** `DerivedScratch` holds `finished_slot: Vec<TransientImage>` (working) *and* `finished: Vec<Option<Arc<RelationImage>>>` (published), plus `occ_images: Vec<Option<Arc<...>>>` (per-occurrence bind) plus `retired`. `ReachScratch` holds `delta: [TransientImage; 2]`, `acc: [TransientImage; 2]`, `acc_filled`, `flip`, `watermark`, *and* `round_delta: Option<Arc<...>>`, `round_acc: Option<Arc<...>>`. Two copies of “TransientImage ping-pong plus published Arc.” Rec’s published images are Options independent of the arrays; `begin()` nulls them. `DerivedScratch.finished[i] = None` is a hole for a derived id that `stash_finished` was supposed to fill. Dual layout, dual None.
- **Collapsing representation:** one `DerivedImages` with a working `TransientImage` per derived id and a published `Arc<RelationImage>` after each table closes. Rec ping-pong is two slots of that, indexed by `flip: bool` (or a `PingPong<T>`). Published images are `Arc`, not `Option<Arc>` — stash is the parse. Round delta/acc are locals in `run_reach`, not fields that persist as None between rounds.
- **Essential vs accidental:** ping-pong and a watermark are essential semi-naive. Two structs, Option published slots, and round Arcs as fields are accidental.
- **Severity:** med

### F14. `rounds_budget` is an inert field on every prepared query

- **Where:** `crates/bumbledb/src/api/prepared.rs:219-224`; `reach.rs:116-123,386`
- **What's wrong:** “Rec-round budget. Inert when `rec` is `None`.” There is no `rec` field. The budget lives on every `PreparedQuery`; `set_derived_budget` writes both axes always; the rounds axis is ignored unless `body` is Reach. A flag that is only meaningful in one arm, stored on the product. Tuples budget *is* universal (interiors-only trips it) — that axis belongs on the query. Rounds belong on Reach.
- **Collapsing representation:** `PreparedPipeline::Reach { rounds_budget, tuples_budget, .. }`; non-Reach carries only `tuples_budget`. `set_derived_budget` on a non-Reach query does not pretend to set rounds.
- **Essential vs accidental:** two budget axes are essential. Inert rounds on CQ/interiors-only is accidental.
- **Severity:** med

### F15. `ReachDriver.main` is stuffed after construction; main has two homes

- **Where:** `crates/bumbledb/src/api/prepared/build.rs:61-66,215-221,263-270,443-446`; `api/prepared.rs:314-322,440-455`
- **What's wrong:** `prepare` builds `Option<ReachDriver>` with `main: Vec::new()`, then `prepare_witnessed` prepares main rules, then `if let Some(mut driver) = rec { driver.main = rules; Reach }`. Main lives on `PreparedBody::Rules` *or* on `ReachDriver.main`, never both, selected by Option. `PreparedBody::rules()` is a match that reaches into the driver. Interiors were not stolen this way (F1) — only main was. Asymmetric special case: rec hijacks main because someone needed a place to put it when the body became Reach.
- **Collapsing representation:** main always on the query / pipeline. `ReachDriver` is base + rec arms + rec sink/scratch, not a container for main. `body.rules()` is `main`, always.
- **Essential vs accidental:** running main after rec closes is essential order. Moving main into the driver is accidental packaging.
- **Severity:** med

### F16. Prepare still branches `if witness.rec().is_some()` and `expect`s

- **Where:** `crates/bumbledb/src/api/prepared/build.rs:61-66,94,371`
- **What's wrong:** After validation, prepare is supposed to be a total function over a witness. It still does `if witness.rec().is_some() { signatures.push(witness.rec().expect("rec present").predicate()); Some(prepare_reach(...)) }`. `prepare_reach` `expect`s again. The Option was not parsed away. Same King move as F5, one layer up.
- **Collapsing representation:** `match witness { ValidatedQuery::Plain {..} => prepare_plain, ValidatedQuery::WithRec { rec, .. } => prepare_reach(rec) }`. No expect.
- **Essential vs accidental:** accidental.
- **Severity:** med

### F17. `AtomSource::interior()` and every `source.edb().is_none()` treat rec as an interior

- **Where:** `crates/bumbledb/src/ir.rs:97-105`; `api/prepared/run_join.rs:96`; `api/prepared/reach.rs:483-485,527`; `api/prepared/build.rs:722`; `plan/selectivity.rs:139`
- **What's wrong:** `interior()` returns `Some` for the rec. `edb().is_none()` is the Interior/rec bind path in `run_join` (“The Interior bind”). Selectivity’s “THE GUARD” is `let Some(relation) = occurrence.source.edb() else { floor }`. The complement of EDB is not one thing — it is finished-interior vs rec-delta vs rec-acc. Those three are recovered later by Option soup (F10) because the source type collapsed them.
- **Collapsing representation:** `enum AtomSource { Edb(RelationId), Derived(DerivedId) }` with bind kind decided by whether `DerivedId` is the live rec and whether this occ is the marked delta — data on the occurrence (`delta: bool` on `RecArm`’s occ, or a `Role` extension), not `edb().is_none()`.
- **Essential vs accidental:** EDB vs derived is essential. Calling derived “Interior” is accidental naming that flattens the bind kinds.
- **Severity:** med

### F18. Selectivity keeps two names for one floor

- **Where:** `crates/bumbledb/src/plan/selectivity.rs:95-110,134-135`; `api/prepared/build.rs:723-727,929`
- **What's wrong:** `DELTA_PLANNING_ROWS = 1`, `ACCUMULATED_PLANNING_ROWS = 16`, `INTERIOR_PLANNING_ROWS = ACCUMULATED_PLANNING_ROWS`. The third constant exists to preserve a distinction the numbers deny. Comments still say “delta floor for the variant’s marked occurrence.” `prepare_rule_variant` picks between delta and INTERIOR by `delta == Some(occ_id)` — a side channel (F7) instead of a property of the occurrence.
- **Collapsing representation:** two floors, named delta vs finished-derived. Put the floor on the occurrence at normalize (`Occurrence { planning: Delta | Finished }`) so prepare does not pass `Option<OccId>` beside the rule.
- **Essential vs accidental:** two floors are essential (frontier vs table). The alias and the Option side channel are accidental.
- **Severity:** med

### F19. Naive eval: `if let Some(rec)` forest; `InteriorWorld` holds rec tables

- **Where:** `crates/bumbledb-bench/src/naive/query.rs:170-183,256-287,331-334`
- **What's wrong:** The oracle is allowed to be dumb. It is not allowed to lie about the tables. `InteriorWorld` is “finished interior *and rec* tables.” `query` loops interiors, then `if let Some(rec)` allocates `rec_id = sets.len() - 1` (the same pun as F3), then `rows_for` is “the query dispatch… the fixpoint calls it per round.” Rec is an afterthought flag on a CQ evaluator. The naive lfp re-evaluates *base+rec* every round (full T(I), not Δ). That last part is essential for a definitional oracle (engine is the semi-naive image). The naming and the Option branch are not.
- **Collapsing representation:** `DerivedWorld { sets: Vec<BTreeSet<Tuple>>, interval: Vec<Vec<bool>> }` with derived ids (F3). Eval is `for d in derived { match d { Interior => eval once, Rec => lfp } }; eval main`. Same `rows_for`. No `if let Some(rec)`.
- **Essential vs accidental:** nested-loop lfp is essential for the oracle. InteriorWorld-as-rec and the Option branch are accidental.
- **Severity:** med

### F20. Querygen: reach/interiors is a side entry, not a shape

- **Where:** `crates/bumbledb-bench/src/querygen.rs:19-22,54-75`; `querygen/construct.rs`; `querygen/shapes_recursive.rs:1-31,99-122`
- **What's wrong:** “The reach/interiors arm (`random_reach_query`) is its own entry beside `random_query`, not a `Shape` row.” Interiors/rec are a second generator bolted on, exactly as interiors sit beside `PreparedBody`. `RecursiveVariant` includes `InteriorsDag`, `InteriorsAntiJoin`, `ManyInteriors` — interiors-only shapes living in a *Recursive* enum. The grammar’s product is CQ-shapes × (maybe reach later).
- **Collapsing representation:** `Shape` includes interiors/rec forms, or `enum QueryClass { Cq(Shape), Derived(DerivedShape) }` as the generator’s top sum — one `random_query`. `InteriorsDag` is not a `RecursiveVariant`.
- **Essential vs accidental:** coverage of rec vs CQ is essential. A side entry and a Recursive tag on interiors-only is accidental.
- **Severity:** med

### F21. Translator: interiors/rec are a gate on the CQ translator

- **Where:** `crates/bumbledb-bench/src/translate/query.rs:37-39`; `translate/reach.rs:1-63`; `translate.rs:39-44`
- **What's wrong:** `translate` opens `if !query.interiors.is_empty() || query.rec.is_some() { return reach::translate_query(...) }`. Two flags, one WITH path. `translate_query` then loops interiors, `if let Some(rec)`, then `let recursive = query.rec.is_some()` to pick `WITH` vs `WITH RECURSIVE` — the same Option, third time. Interiors-only still goes through the “reach” module. `sqlite_reach_expressible` is named for rec and used for interiors. Comments: “the four rec gates died with the stratified IR.”
- **Collapsing representation:** one translator over Query. CTEs are derived tables in order; `WITH RECURSIVE` iff the parsed shape has a Rec. Module name `derived` / `cte`, not `reach`. The CQ function does not test `interiors.is_empty()`.
- **Essential vs accidental:** SQL WITH vs WITH RECURSIVE is essential SQLite. Gating the CQ translator on two flags and naming interiors “reach” is accidental.
- **Severity:** med

### F22. `rec_roster` then `lower_rec_pool` re-check emptiness; `measure_in_rec` is a second walk

- **Where:** `crates/bumbledb/src/ir/validate/validate.rs:85-130,245-293,306-346`
- **What's wrong:** `rec_roster` rejects empty base/step, self-in-base, missing/nonlinear self, negation. `lower_rec_pool` rejects empty base/step *again* after DNF, plus `MAX_RULES` on the pool, empty head, nesting, DNF width. `measure_in_rec` walks conditions a third time after typing. Parse, don’t validate: each check should refine the type. Emptiness-after-DNF is a *different* fact (a nonempty written pool can collapse) and deserves a distinct error or a dead-rec parse, not a reuse of `EmptyRecursiveBase`. Nonlinear is an OPEN refusal — keeping it as a roster item is correct; re-testing self-count after DNF would be the shotgun (currently prepare does that, F5).
- **Collapsing representation:** one rec parser: written roster → lower → typed `ValidatedRec` with nonempty `NonEmpty` arms and `self_occ` per arm. Measure-in-rec is a typed refusal on `ClassifiedComparison` during `type_rules` with a rec-body flag, or unrepresentable if rec conditions cannot carry `Term::Measure` in the rec-arm type.
- **Essential vs accidental:** the roster items are essential this cut (including nonlinear as refusal). Double emptiness and a post-pass measure walk are accidental.
- **Severity:** med

### F23. Dead-main vs empty-query vs interiors-preamble: `Empty` does too much

- **Where:** `crates/bumbledb/src/api/prepared.rs:314-318`; `execute.rs:84-96,162-171`; `introspect.rs:211-216,391-414`; `build.rs:198-200,266-267`
- **What's wrong:** `PreparedBody::Empty` means “every **main** rule was statically refuted.” Comments still say “the statically-empty program… Always the whole program: this variant is built only when every rule died” and then contradict: interiors-only with a dead main still runs the preamble. `run_rules` special-cases `Reach` then `Empty` then the main loop. `empty_stats` assumes no interiors (`interiors: Vec::new()`). A dead main with live interiors is Empty+nonempty interiors (F1). A dead everything with no interiors is Empty+empty interiors. Same tag.
- **Collapsing representation:** `Main { rules: Vec<PreparedRule> }` — empty vec is dead main. The no-work path is `interiors.is_empty() && rules.is_empty() && not Reach`, or a parsed `Pipeline::Empty` that is only the latter. Stats for dead-main+live-interiors include interior emits.
- **Essential vs accidental:** statically-empty main is essential (fold.rs). Overloading Empty as “the empty program” is accidental, and the comments have not caught up.
- **Severity:** med

### F24. Dual ray-probe loops (main vs interior) copy the latch protocol

- **Where:** `crates/bumbledb/src/api/prepared/execute.rs:189-256`; `reach.rs:212-279`
- **What's wrong:** `run_ray_probes` (main) and `run_interior_ray_probes` are the same protocol: take probes, resolve interns, resolve_filters / fast-eligible, `run_join` into `RayArbiter`, restore on error. Interior version additionally `fill_plan_images`. Two copies because interiors are a sidecar (F1) with their own `ray_probes: Vec<RayProbeSet>`. Rec cannot have measure (`MeasureInRec`) so ReachDriver has no probes — another special case of “measure is a main/interior thing.”
- **Collapsing representation:** `fn run_ray_probes(probes, images, …)` once. Each prepared stage that can measure owns probes. Rec’s lack of measure is the rec-arm type (no `Term::Measure` in rec conditions post-parse), not a missing field plus a roster item.
- **Essential vs accidental:** R6 probes after the rule loop are essential. Two loops are accidental duplication.
- **Severity:** med

### F25. `visit_rules` / `PreparedRule` accessors re-match Recursive to unwrap `.variant.rule`

- **Where:** `crates/bumbledb/src/api/prepared.rs:458-569`
- **What's wrong:** `visit_rules` and `visit_rules_mut` match Empty/Rules/Reach and walk interiors separately (F1). Every `PreparedRule` method (`finds`, `slot_count`, `distinct_witness`, `dedup_spans`, `pinned`) has a Recursive arm that forwards through `rule.variant.rule`. Comments: “Variants project one head,” “variant 0 speaks.” The accessor forest is F2+F7 in miniature. `distinct_witness` on Recursive is `None` by policy (no key coverage) — a branch that would be the RecArm type’s inherent lack of that field.
- **Collapsing representation:** F2. Accessors exist only on `FreeJoinRule` / `KeyProbeRule`. RecArm exposes `.rule: FreeJoinRule`.
- **Essential vs accidental:** accidental.
- **Severity:** med

### F26. `EitherSink` match on the hot main path; interiors/rec are projection-only by a different function

- **Where:** `crates/bumbledb/src/api/prepared/execute.rs:351-381`; `reach.rs:563-648`; `api/prepared.rs:661-676`
- **What's wrong:** Main `run_rule` matches `EitherSink::Projection | Aggregate` to monomorphize `run_join`. Interiors and rec always go through `run_into_projection` (projection sink). That split is real — derived tables are projection-shaped (folds through cycles refused). The representation does not say so: `PreparedInterior` has `ProjectionSink` (good) but `PreparedRule` in interiors can still be KeyProbe/Recursive. `run_into_projection` handles KeyProbe and unwraps Recursive to FreeJoin. Two run functions because two sink types, but the rule enum is shared (F2).
- **Collapsing representation:** interior/rec rules are `FreeJoin | KeyProbe` into a `ProjectionSink`. Main rules are `FreeJoin | KeyProbe` into `EitherSink`. Do not share `PreparedRule` if Recursive only exists in rec arms.
- **Essential vs accidental:** projection-shaped derived tables are essential this cut. Sharing one rule enum across sink kinds is accidental.
- **Severity:** med

### F27. Hostile IR still represents empty main, empty interiors, empty finds; witness types still could

- **Where:** `crates/bumbledb/src/ir.rs:417-466`; `ir/validate/validate.rs:48-50,141-144,379-387`
- **What's wrong:** Untrusted `Query` with `rules: []`, `Interior { rules: [] }`, `head: []` is correct *input* — parse to `EmptyRuleSet` / `EmptyInterior` / `EmptyFinds`. `ValidatedMain` / `ValidatedInterior` still hold `Vec<LoweredRule>` that the constructor guarantees nonempty but the type does not. Same King gap as F4/F5, milder because downstream rarely `expect`s nonempty — it indexes `[0]` for `Predicate::derive(&lowered[0], &typings[0])`.
- **Collapsing representation:** `NonEmpty` lowered lists on the witness, or `Predicate` stored without re-deriving from `[0]`. Untrusted Query stays open.
- **Essential vs accidental:** empty-as-error at the boundary is essential. Vec-that-happens-to-be-nonempty on the witness is accidental.
- **Severity:** med

### F28. Derived-count / rec-id restated instead of stored

- **Where:** `crates/bumbledb/src/ir.rs:48-50`; `ir/validate/validate.rs:31`; `error.rs:848-850`; `api/prepared/reach.rs:133-134,303`; `api/prepared/build.rs:373-375`; `ir/render.rs:153`; `bumbledb-bench/src/naive/query.rs:269`; `bumbledb-bench/src/translate/reach.rs:43`
- **What's wrong:** `derived = interiors.len() + usize::from(rec.is_some())` is the well-formedness screen, then the execution bind, then the rec_id, then the SQL CTE id, then the naive set index, then render. A coordinate (F3) recomputed as arithmetic everywhere. Overflow is judged at validate (`InteriorIdOverflow`) then `expect("derived count fits u32")` / `expect("overflow judged at validate")` at prepare. Proof discarded.
- **Collapsing representation:** witness carries `derived_count: u32` and `rec_id: Option<DerivedId>`. Execution copies those. No `len + is_some()`.
- **Essential vs accidental:** u32 id-width is essential. Recomputing and re-expecting is accidental.
- **Severity:** med

### F29. Introspection unit labels and “query-shaped vs fixpoint” are a tag not yet a sum

- **Where:** `crates/bumbledb/src/exec/introspection.rs:83-95`; `introspect.rs:41-106`; `exec/introspection/display.rs:20-38,184-187`
- **What's wrong:** `IntrospectionReport { rules, unit_labels, stats }`. `unit_labels` empty means query-shaped (label is rule index); nonempty means fixpoint (labels are `reach base {i}`, `reach rec {i} (delta occ {})`, `main {i}`). Parallel arrays: `unit_labels.get(rule_idx)` vs `None if multi => rule {i}`. Display: if no `stats.rules[i]`, skip counts because “fixpoint plan units… counted surface is the strata section.” Two modes of one report, encoded as “are the labels empty?”
- **Collapsing representation:** `enum ReportBody { Main { rules: Vec<(RulePlan, RuleStats)> }, Reach { units: Vec<(String, RulePlan)>, interiors, reach: ReachStats } }`. No parallel `unit_labels`. No `stats.rules.get` miss as a mode bit.
- **Essential vs accidental:** different counted surfaces for CQ vs reach are essential (one counter spanning many plans). Parallel labels-or-not is accidental.
- **Severity:** med

### F30. `normalize()` empty-surface path is dead and false

- **Where:** `crates/bumbledb/src/ir/normalize/normalize.rs:14-28`; `plan/fj/validate.rs:200-217`; `api/prepared/build.rs:661`
- **What's wrong:** `normalize(schema, query)` passes `&[]` signatures and is `#[allow(dead_code)]`. Doc: query path has no Interior occurrences. Production uses `normalize_predicate` / `normalize_rules` with signatures. `fj::validate` (test-only) vs `validate_with_signatures` is the same split: “query path passes the empty surface.” The query path *is* interiors/rec/main. A dead function encoding a deleted Program invariant.
- **Collapsing representation:** one `normalize_rules`. Delete `normalize`. Test `fj::validate` calls `validate_with_signatures` or takes signatures always (empty only for EDB-only fixtures, as data, not as “the query path”).
- **Essential vs accidental:** accidental leftover.
- **Severity:** med

### F31. Key-probe direct path re-matches what the gate already matched

- **Where:** `crates/bumbledb/src/api/prepared/execute.rs:97-110,406-418`
- **What's wrong:** `run_bound` matches a single `KeyProbe { key_probe_finds: Some(_) }` then calls `execute_key_probe_direct`, which slice-matches the same pattern and `return Ok(())` on failure — a silent empty result if the gate and the body disagree. Parse, don’t re-validate. The silent `Ok(())` is a dropped answers path, not a typed empty program.
- **Collapsing representation:** after F1, `Pipeline::Main { rules: [KeyProbe { finds: Some(t), .. }] }` *is* the direct path. The function takes `&KeyProbeRule`. No second match, no `else { return Ok(()) }`.
- **Essential vs accidental:** a point lane is essential. Re-matching and swallowing mismatch is accidental.
- **Severity:** med

### F32. `DerivedScratch.occ_images` / `finished` as `Vec<Option<_>>`

- **Where:** `crates/bumbledb/src/api/prepared/reach.rs:53-67,521-544`
- **What's wrong:** Per-occurrence `Option<Arc<RelationImage>>`, resized to `plan.occurrences().len()`, None for EDB and discharged. Bind expects Some for Interior. `finished` is `Vec<Option<Arc>>` indexed by derived id, None until stash. Absence of an image for a live Interior occ is representable. Hoare: null in every slot.
- **Collapsing representation:** EDB occs do not have a slot in `occ_images`. Derived occs have `Arc<RelationImage>` (stash before bind). `finished: Vec<Arc<RelationImage>>` filled in order (interior 0..n, then rec).
- **Essential vs accidental:** not every occurrence is derived — essential. Option as the way to say that — accidental.
- **Severity:** med

### F33. Render/display still say `interior p{id}` / `predicate p{id}` / `recursive p{id}`

- **Where:** `crates/bumbledb/src/ir/render.rs:143-158`; `exec/introspection/display.rs:84-88,153`
- **What's wrong:** Diagnostic surface keeps Datalog predicate numbering. Rec is `recursive p{interiors.len()}`. Stats print `interior p{}`. Occurrence source prints `predicate p{}`. The type is Interior/Rec/Query. The strings are Program.
- **Collapsing representation:** `interior {id}`, `rec`, `main`. Same ids as `DerivedId`.
- **Essential vs accidental:** accidental vocabulary. Harmless if isolated; it trains every reader that rec is predicate pN.
- **Severity:** low

### F34. `ground_program` and “the whole program” in prepare

- **Where:** `crates/bumbledb/src/api/prepared/build.rs:23-26,107,145,581,597,1217-1221`; `plan/ground.rs:402`; `plan/ground/tests.rs:688-691`
- **What's wrong:** Main-rule grounding is `ground_program`. Interiors/rec ground via `ground_rules`. The name says the deleted IR. Tests: `grounded_program`. Comments: “Validation and normalization see the whole program.”
- **Collapsing representation:** `ground_main` / `ground_rules`. “Query” in comments.
- **Essential vs accidental:** accidental naming.
- **Severity:** low

### F35. Tests and comments still say “empty program,” “multi-rule program,” “degenerate program”

- **Where:** `crates/bumbledb/src/api/prepared/tests/statically_empty.rs:206-210`; `tests/folded.rs:251-254`; `tests/rules.rs:59,85`; `ir/validate/tests/rules.rs:1,230,262`; `tests/api.rs:1534-1538`; `tests/adversarial_ir.rs:545,654-775`; `exec/wordmap/clear.rs:47`
- **What's wrong:** Zombie Program as the name of a Query. `TooManyCtes` is explicitly gone (`adversarial_ir.rs` asserts it must not return) — good — but the test is still framed as the CTE/Program cap’s ghost. `iter_since` documents “a non-recursive program cannot observe it” — the watermark hook is rec-only; the comment is the inert-field pattern (F14) in a sentence.
- **Collapsing representation:** say Query. Watermark is on the rec sink’s seen-set; CQ queries do not have that sink.
- **Essential vs accidental:** accidental naming. `TooManyCtes` absence is a regression pin, not a type.
- **Severity:** low

### F36. `_either_sink_marker` dead import hush

- **Where:** `crates/bumbledb/src/api/prepared/reach.rs:651-653`
- **What's wrong:** `#[allow(dead_code)] fn _either_sink_marker(_: &EitherSink) {}` because reach imported `EitherSink` for a layout that does not use it. A type imported to keep a product aligned with execute. Null object for a missing use.
- **Collapsing representation:** don’t import `EitherSink` in reach. Interiors/rec are projection (F26).
- **Essential vs accidental:** accidental.
- **Severity:** low

### F37. `Query::single` is the right coordinate — and everything else re-special-cases it

- **Where:** `crates/bumbledb/src/ir.rs:450-478`; `execute.rs:97-100`; `introspect.rs:218-244`
- **What's wrong:** `Query::single` is homogeneous: empty interiors, no rec, one rule. That is Dijkstra’s `[a,b)` — the CQ is not a different type, it is the empty prefix. Execution then special-cases “interiors empty AND single key-probe” as a fast lane *outside* that coordinate (F8, F31). The constructor did the right thing; the engine reintroduced the special case as a branch.
- **Collapsing representation:** keep `Query::single`. Fast lanes are parsed `Pipeline` arms (F1), not `is_empty() && matches!`.
- **Essential vs accidental:** CQ-as-empty-prefix is essential and *already done*. The extra branches are accidental. Listed so the audit does not treat `Query::single` as a violation.
- **Severity:** low (the constructor is correct; the branches are charged to F1/F8)

### F38. Conformance reach JSON dropped Program fields; engine stats did not

- **Where:** `crates/bumbledb-bench/src/conformance/reach.rs:7-11`; `crates/bumbledb/src/api/stats.rs:48-52`; `exec/introspection.rs:90`
- **What's wrong:** Reach cases are `interiors / rec / main rules`; “No `predicates` / `output` / `strata` / `idb`.” The oracle corpus parsed. The engine’s counted surface still talks `strata` and `reach: Option`. Split-brain: Lean/JSON got the new tables; introspection kept the old flowchart labels.
- **Collapsing representation:** F12. Align `ExecutionStats` with the JSON: interiors, optional rec rounds, main rules.
- **Essential vs accidental:** accidental drift between oracles and engine stats.
- **Severity:** low (docs/comments unless you count F12)

### F39. `prepare_rule_variant`’s `delta: Option<OccId>` is a boolean with an id stuffed in

- **Where:** `crates/bumbledb/src/api/prepared/build.rs:642-674,722-727`
- **What's wrong:** `None` = not a delta plan; `Some(id)` = this occ gets the delta floor. Option-as-flag. Callers: `prepare_rule` always None; rec arms Some. A rec arm prepared through the CQ function with a side channel instead of a rec-arm function (F7).
- **Collapsing representation:** `prepare_rule(...)` for CQ/interior/base; `prepare_rec_arm(..., delta: OccId)` for rec. Floor is not an argument; it is which function you called.
- **Essential vs accidental:** accidental packaging.
- **Severity:** low (charged primarily to F7; listed because the Option is a distinct illegal-state: Some(id) on a rule that has no such occ)

### F40. `ReachScratch` size-1 comment vs `[2]` ping-pong

- **Where:** `crates/bumbledb/src/api/prepared/reach.rs:82-91`
- **What's wrong:** “Rec ping-pong: delta vs accumulated of the one SCC. Size 1.” The fields are `[TransientImage; 2]`. The comment is the k-SCC leftover (one SCC, so “size 1”) colliding with ping-pong width 2. Special case belonging to the coordinate: one SCC means one pair of buffers, not a length-1 array of pairs.
- **Collapsing representation:** `PingPong { a: TransientImage, b: TransientImage, flip: bool }` or two named fields `delta_working`, `acc_working`. Comment matches.
- **Essential vs accidental:** accidental comment/layout mismatch. One SCC is essential this cut (mutual is OPEN, not a bug).
- **Severity:** low

---

## Not counted as bugs (this-cut OPEN)

- **Mutual recursion / multiple rec SCCs.** `Query.rec: Option<Rec>` makes a second SCC unrepresentable. That is the cut. Do not “fix” by adding `Vec<Rec>`.
- **Nonlinear rec arms** (`NonlinearRecArm`, ≥2 positive self-atoms). Roster item, untrusted IR. Defect is only F5 (proof not parsed into the witness), not the refusal.
- **Negation in rec / measure in rec / folds through cycles.** Refusals. Representability at `Query` is the hostile surface; the witness should not carry them (F5/F22).
- **Naive full-lfp vs engine semi-naive.** Two representations of the same denotation; required for the oracle.

---

## Counts

| Severity | Count |
|----------|------:|
| high     |    12 |
| med      |    20 |
| low      |     8 |
| **total**|  **40** |

High: F1–F12. Med: F13–F32. Low: F33–F40.

F37 is a non-violation of the constructor, recorded so CQ-as-empty-prefix is not “fixed” into a separate type. Charge its branches to F1/F8.

---

## The one table that would delete the flowchart

```rust
enum QueryShape<I, R, M> {
    Cq { interiors: I, main: M },
    Rec { interiors: I, rec: R, main: M },
}

// Untrusted: I = Vec<Interior>, R = Rec, M = (head, Vec<Rule>)
// Witness:   I, R, M nonempty/typed; R arms carry self OccId
// Prepared:  I = Vec<PreparedInterior>
//            R = ReachDriver { base, rec: Vec<RecArm>, sink, scratch }  // no main
//            M = Vec<PreparedRule>  // FreeJoin | KeyProbe only
```

Interiors are data in both arms, not a sidecar. Rec cannot appear without `Rec`. Recursive rules cannot appear in main. `rec.is_some()` has nothing to test. `stats.strata`, `idb_images`, `DeltaVariant`, and `prepare_rule_variant` have nowhere to stand.

Brooks: show the tables. This is the table. The `if rec.is_some()` forest is the flowchart it makes obsolete.
