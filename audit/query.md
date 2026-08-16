# Query / IR / Plan / Exec representation audit (0.13)

Pre-publish audit of bumbledb QUERY / IR / PLAN / EXEC under the three
SPOVs in `docs/design/representation-first.md` (no `audit/REQUIRED-READING.md`
was present). Scope: `crates/bumbledb/src/ir/**`, `plan/**`, `exec/**`,
`crates/bumbledb-query/**`, `ts/src/query/**` where it duplicates engine IR.
Working tree + HEAD. Production code was not edited.

**Counts:** ship-blocker **0** · should-fix-before-0.13 **7** · later **11**

What is already parsed, so it is not a finding: `Query::{Cq, Reach}` (rec
unrepresentable on CQ); `AtomSource::{Edb, Interior}`; `Term::{Param, ParamSet}`;
`Role::{Positive, Negated, Eliminated, Folded}` (no polarity×eliminated flag);
`OccBind::{Edb, Finished, RecDelta, RecAcc}`; `ValidatedQuery` unconstructible
outside `ir/validate`; `ValidatedRec` nonempty + stored `self_occ`;
`ClassifiedComparison` as validation's sealed comparison language;
`ParamSpec::{Scalar, Set}`; `PreparedRule::{FreeJoin, KeyProbe}`;
`EitherSink::{Projection, Aggregate}`; `write_from` consumes `Snapshot`/`Witness`
(not a raw generation integer); TS `QueryStart` vs `QueryReachStart` (interior
after reach uncallable); Lean `LinearRec` with `RecStep.selfBindings`.

Prepare always runs `ir::validate::validate`. Illegal queries below fail at
prepare, not at execute — unless noted. The 0.13 question is whether the
*public* IR, the SDK/engine/Lean copies, and the mutation/prepared contract
are safe to publish, not whether the roster currently catches them.

---

### Q-01 (severity: should-fix-before-0.13)

**Location.** `crates/bumbledb/src/ir.rs` `FindTerm::Aggregate { op, over: Option<VarId> }`;
consumed in `ir/validate/finds.rs` and `ir/validate/validate.rs` `input_row`;
emitted by `bumbledb-query-macros` and smashed into by C (`bumbledb-c/src/query.rs`
`find_term_in`) and TS (`ts/crate/src/marshal.rs`).

**Illegal state.** `Count` with `Some(var)` and `Sum`/`Min`/`Max`/`Pack` with
`None`. Eight inhabitants, two legal families.

**Representation critique.** Nullary Count and a fold over a variable are
different terms. Encoding the distinction as `Option` is Hoare's mistake on
one field: every consumer re-matches `(op, over)` and the witness still
`expect`s ("validated: only Count is nullary"). The host surfaces already
parsed the split — the macro's `HeadTerm::{Count, Agg { over }}`, TS
`FindTermIr` (Count has no `over`; folds require it), C
`bdb_find_term_kind::{Count, Aggregate}` — then throw the proof away to
construct this Option.

**Better representation.**

```rust
enum FindTerm {
    Var(VarId),
    Measure(VarId),
    Count,
    Aggregate { op: FoldOp, over: VarId },       // Sum | Min | Max
    Pack { over: VarId },
    AggregateMeasure { op: FoldOp, over: VarId },
}
```

`FoldOp` is `Sum | Min | Max` only, so `AggregateMeasure { op: Count }`
(Q-11) dies with it.

**Evidence.** `finds.rs` 119–131: `(Count, Some(_))` → `CountWithVariable`,
`(Sum|Min|Max|Pack, None)` → `AggregateWithoutVariable`. Macro `lib.rs`
1893–1895 emits `over: None` for Count. TS `parse-ir.ts` 53–66 refuses the
split the engine then cannot spell. C `find_term_in` re-checks Count-as-Aggregate
and still builds `over: None`.

---

### Q-02 (severity: should-fix-before-0.13)

**Location.** `crates/bumbledb/src/ir.rs` `Rec { base: Vec<Rule>, rec: Vec<Rule> }`;
roster in `ir/validate/validate.rs` `rec_roster`. Contrast
`lean/Bumbledb/Query/Syntax.lean` `RecRule` / `RecStep` / `LinearRec`.

**Illegal state.** Empty base, empty step, self-atom on a base arm, zero or
two+ positive self-atoms on a step arm, any negated atom in either list.
Five named refusals (`EmptyRecursiveBase`, `EmptyRecursiveStep`, `SelfInBase`,
`RecArmMissingSelf`, `NonlinearRecArm`, `NegationInRec`) for states the type
admits. After DNF, `rec_arm_self_occ` `expect`s the unique self the roster
already proved.

**Representation critique.** A rec step *is* "exactly one positive self-atom
plus other atoms, no negation." Encoding it as "a `Rule`, and we will search
for the self" is tag+search. Lean already reified it: `RecStep.selfBindings`
*is* the unique positive self-atom; `RecRule` has no self field and no
`negated`; both lists are nonempty by type (`A × List A`). The engine
witness (`ValidatedRecArm.self_occ`) stores the proof and then throws it
away from the public IR. TS `RecData` uses `NonEmpty<RuleData>` (empty
unrepresentable) but still `RuleData` without a reified self-atom — closer,
not done.

**Better representation.** Public `Rec` matches Lean: nonempty base arms
with no self/negation in the type; step arms carry `self: Atom` (or
`self_bindings: Vec<(FieldId, Term)>`) plus remaining atoms. Downstream
`self_occ` is an index into a list that cannot fail to contain it.

**Evidence.** `validate.rs` 401–438 walks written rules counting self-atoms.
Lean `Syntax.lean` 268–288 and `RecStep.toRule` reconstructs the self-atom
from `selfBindings` so missing/nonlinear self cannot be written.
`ValidatedRecArm` already carries `self_occ` — the parse exists; the input
type does not.

---

### Q-03 (severity: should-fix-before-0.13)

**Location.** `ir.rs` `Interior.head: Vec<HeadTerm>`, `Rec.head: Vec<HeadTerm>`;
`refuse_derived_head` in `ir/validate/validate.rs`.

**Illegal state.** `HeadTerm::Aggregate(_)` on an interior or rec head;
`FindTerm::Measure` / `AggregateMeasure` in those rule lists. Comments say
"bound-variable positions only (every `HeadTerm::Var`)." The type disagrees.

**Representation critique.** Interior and rec heads are projection-shaped by
the creation-quarantine law. Sharing `HeadTerm` (var | aggregate-op) with
main is the special case belonging to the representation. Lean `Interior`
has only `rules`; head width is `finds.length`. Lean `Rule.finds : List VarId`
makes aggregate/measure finds unwritable at that level. The engine roster
(`AggregateInInterior`, `MeasureInInterior`) discharges a class Lean cannot
write — recorded as a narrowing, which is the finding: the engine still
writes it.

**Better representation.** `Interior` / `Rec` heads are `Vec<VarId>` (or
omitted: derive from rule 0's finds, which are `Vec<VarId>`). Main keeps
`Vec<HeadTerm>` / `Vec<FindTerm>`.

**Evidence.** `ir.rs` 412–415, 431–432 comments vs `HeadTerm::{Var, Aggregate}`.
`refuse_derived_head` 364–383. Tests `validate/tests/interior.rs` and
`rec.rs` construct `head: vec![HeadTerm::Var, HeadTerm::Aggregate(Count)]`
to trigger the roster. Lean module doc lines 16–23, 263–266.

---

### Q-04 (severity: should-fix-before-0.13)

**Location.** Public `crates/bumbledb/src/ir.rs` `Query` / `Rule` / `Term` /
`Comparison` / `ConditionTree`; the actual parse is
`ir/validate.rs` `ValidatedQuery` (unconstructible outside the module).
Hosts construct `Query`: `Db::prepare(&Query)`, C `bdb_query`, TS
`lowerQuery` → `QueryIr` → engine `Query`. `crates/bumbledb-query` is a
re-export of `query!`, which emits this wide IR.

**Illegal state.** The public type admits (non-exhaustive): empty main,
empty finds, no positive atoms, duplicate `FieldId` in one atom, `Term::Measure`
in a binding, `ParamSet` under `Lt`, `ParamId` used as both scalar and set,
constant comparisons, unbound finds, aggregates on interiors, the whole rec
roster (Q-02), Count-with-variable (Q-01), unknown relation/field/interior
ids, condition trees deeper than `MAX_CONDITION_DEPTH`. Validation's roster
is ~20 items of "this type can say that."

**Representation critique.** This is the textbook parse-don't-validate miss
at the *publish* boundary. Internally they did the work: DNF → `LoweredRule`
(no `Or`); typing → `ClassifiedComparison`; rec → `ValidatedRec` with
`NonEmpty` and `self_occ`. Hosts never receive that type. `prepare` is
"validate this bag, then trust the witness." Three SDKs (Rust `query!`, TS
builder, C structs) each re-implement a partial parse, then lower into the
bag so the engine can parse again. King's rule: every host re-checks because
the check's result was not a type.

**Better representation.** The crate-root IR *is* `ValidatedQuery` (or a
public, unconstructible-except-via-parse twin). `query!` / TS / C inhabit
it directly. Raw `Query` stays a private CST if a data-in trust boundary
is still required for hostile FFI — but then C/TS marshal into the CST and
only `ValidatedQuery` crosses into plan/exec, which is already true
internally. Publishing `Query` as "the IR a host needs to build" (`lib.rs`
153–155) is what makes the CST the product.

**Evidence.** `ir/validate.rs` 1–3: "IR in, `ValidatedQuery` witness out.
Everything downstream trusts the witness and re-checks nothing." `api/db/prepare.rs`
12–13: "Validation is `validate` on `&Query` only." `lib.rs` re-exports
`Query`, `Rule`, `Term`, … not `ValidatedQuery`. Macro `find()` (Q-01)
lowers a parsed `HeadTerm::Count` into the Option CST.

---

### Q-05 (severity: should-fix-before-0.13)

**Location.** Four IRs of one query: engine `ir::Query`; Lean
`Bumbledb.Query.Query`; TS `QueryData` (`ts/src/query/lower.ts`) then wire
`QueryIr` (`ts/src/native.ts`); C `bdb_query` views.

**Illegal state / drift.** The copies do not denote the same language.

| Fact | Engine | Lean | TS `QueryData` | TS `QueryIr` |
| --- | --- | --- | --- | --- |
| Rec self-atom | search `Rule.atoms` | `RecStep.selfBindings` | `RuleData` items | same as engine |
| Interior/rec finds | `FindTerm` (aggregates legal) | `List VarId` | construction wall | `HeadTermIr` still has aggregate |
| Count | `Aggregate { over: None }` | (PRD 05, not in syntax) | `Agg<"count", undefined>` then split | `{ kind: "aggregate", op: { kind: "count" } }` no `over` |
| Vars | dense `VarId` | dense `VarId` | object identity | dense ids |
| Interior names | anonymous (`InteriorId`) | anonymous | `InteriorData.name` | anonymous |
| Allen mask | `u16` bits | `List AllenRel` | `number` | `number` |
| Body order | parallel `atoms` / `negated` / `conditions` | same | `RuleItem` stream (atom \| negated \| interior \| cond) | parallel lists |
| Closed membership array | no node (becomes `ParamSet`) | no node | `literalSet` → synthetic param | `paramSet` |
| Phantom interior | `UnknownInterior` | reads empty (negated phantom holds) | construction lookup | engine roster |

**Representation critique.** Dual representations of the same query are the
complexity. TS `QueryData` is in several places the *better* IR (named
interiors, reference-identity joins, `RuleItem` as one stream so negation
is a position, `NonEmpty` rec, `QueryReachStart` making post-reach
`interior` uncallable). `lowerQuery` then erases those proofs into engine
CST. `parseQueryIr` is a brand cast (`return ir as ParsedQuery`) — validation
that throws away the parse: `ParsedQuery` is still `QueryIr`, not a new type.
`isTypedScope` (`typeof scope.match === "function"`) is a fake parse, always
true. Closed-handle membership arrays are a third spelling of `ParamSet`
(literal set folded to a content-addressed registry entry that execute never
consults). Lean records the engine's extra width as "narrowings" rather than
keeping the engines in bijection.

**Better representation.** One kernel IR = Lean syntax ≅ engine
`ValidatedQuery` ≅ TS `QueryIr` after a *constructing* parse (new type, not
a brand). SDK `QueryData` is a host CST that *parses* into that kernel (keep
names, refs, `RuleItem` until the boundary, then produce the narrow type).
Do not smash `FindTermIr`'s Count split back into `Option`. Stop minting
synthetic params for program-constant membership arrays — a `Term::LiteralSet`
(or fold to `In(Literal)` at the kernel) is one meaning, one node.

**Evidence.** `ts/src/query/parse-ir.ts` 12–30 brand cast. `lower.ts` 2127–2186
`lowerQuery` → `parseQueryIr`. `atom.ts` 55–60 `literalSet` vs `setParam`.
`lower.ts` 1929–1940 both become `{ kind: "paramSet" }`. Lean `Syntax.lean`
14–73 recorded narrowings, including the unknown-interior gap (negated
phantom holds in Lean, refused in engine). `native.ts` 121–131 `FindTermIr`
already split.

---

### Q-06 (severity: should-fix-before-0.13)

**Location.** `image/view.rs` `FilterPredicate`; `image/view/apply.rs`
evaluator; `ir/validate.rs` `ClassifiedComparison` (the parse);
`ir/normalize/place_comparisons.rs` (the consumer).

**Illegal state.** `FilterPredicate::Compare { op: CmpOp::Allen { .. } | PointIn, .. }`;
`Compare` of an interval field under `Lt`; `PointVar` in a list the view
evaluator is specified never to see; `DurationCompare { op: CmpOp::Eq }`
(comment: "op is an order operator … by validation"). `CmpOp::compare`
(`ir.rs` 319–334) `unreachable!`s Allen/PointIn. `apply.rs` 134–137, 188–191
`unreachable!("validated: interval constants compare under Eq only")`.

**Representation critique.** Validation *parses* comparisons into
`ClassifiedComparison` (nine legal shapes, sealed, "no shape is re-derived
downstream"). Normalization then lowers into `FilterPredicate` + `CmpOp`,
which re-admits the illegal combinations the classification just excluded.
That is throwing away the parse proof. `PointVar` is a staging token in the
runtime filter enum ("plan validation lifts this … The view evaluator never
sees it") — a compile-phase node living in the execute AST.
`FilterPredicate` is otherwise the right move (shapes as kinds, no expression
tree); the hole is leaving `CmpOp` (the input language) inside `Compare` /
`FieldsCompare` / `DurationCompare` / `PlacedComparison`.

**Better representation.** Execution predicates use the classified language
(or a further-lowered subset): `EqWord`, `OrderWord`, `EqInterval`,
`InSet`, … — no `CmpOp` that includes Allen. `PointVar` is not a
`FilterPredicate`; it is a `PointProbe` candidate on the occurrence, which
plan validation already produces. `PlacedComparison.op` is
`Eq | Ne | Lt | Le | Gt | Ge` only.

**Evidence.** `ir/validate.rs` 200–217: classified is "pipeline-internal …
consumed by … place_comparisons with a **total** match." `view.rs` 148–256
`Compare { op: CmpOp }` plus `PointVar` "the view evaluator never sees it"
(`apply.rs` 206–208 `unreachable!`). `ir.rs` 330–331 interval operators
"never reach single-word evaluation." Plan node comments in `plan/fj.rs`
236–245 already refuse merging residual *lists* by kind for batching — that
refusal is sound; it does not justify `CmpOp` inside those lists.

---

### Q-07 (severity: should-fix-before-0.13)

**Location.** Mutation cutover vs prepared handles: `api/db/write.rs`
`write_from`; `api/prepared.rs` pin-at-prepare; `api/prepared/staleness.rs`
(`#[doc(hidden)]`); `docs/architecture/70-api.md` "Plan diagnostics —
WITHDRAWN"; `docs/architecture/20-query-ir.md` § prepared queries.
Working tree adds `api/db/mutation.rs` (`MutationReport` / `FreshRange`) and
touches `api/prepared/tests/staleness.rs`.

**Illegal state.** Not a wrong-answer state. The representable product
state is: a `PreparedQuery` whose plan was costed on generation G, executed
on generation G+k after `write`/`write_from`, with **no host-visible type
or API** that the plan is the one costed on G. `Staleness` exists and is
withdrawn from embedding API. `key_probe_direct: bool` on `PreparedQuery`
is a second spelling of "this pipeline is a no-interior single key probe"
beside `PreparedPipeline` + `PreparedRule::KeyProbe`.

**Representation critique.** Pin-at-prepare plus generational view rebinding
is the right *correctness* representation (memo keys `(ViewGeneration, filters)`;
tests `prepared/tests/snapshot.rs` `pinned_plan_reads_fresh_data_at_newer_generations`;
`view_memo.rs` reaps `parked.generation < generation`). `write_from` is the
right *witness* representation (private generation, `ForeignSnapshot`,
`GenerationMoved`, TS `writeFrom` refuses a dead or foreign scope). What
0.13 ships to hosts is write-from without the compensating control the
architecture named. Hosts doing the cookbook read-compute-`write_from` loop
cannot ask "has this prepared handle's plan drifted?" except through
harness-only `#[doc(hidden)]` `staleness`. That is a boolean flag on the
*product*: mutation is public, plan-drift is not. The engine will not
re-prepare (accepted); hiding the signal means hosts cannot either.

**Better representation.** Keep pin-at-prepare and generation-keyed memos
(do not auto-reprepare). If mutation is embedding API, the drift report
is too — a pull `PreparedQuery::staleness(&Snapshot) -> Staleness` as
public as `write_from`, or a `Prepared` that is explicitly
`Pinned { at: GenerationId, .. }` so "this plan is of that generation" is
in the type even if execute remains legal across generations. Delete
`key_probe_direct`; `run_bound` matches `PreparedPipeline::Cq { interiors,
rules: [PreparedRule::KeyProbe(_)] }` with empty interiors — the parse
already happened at build (`execute.rs` 84–87, 419 "does not re-gate").

**Evidence.** `prepared.rs` 6–8: "Plans pin the statistics read at prepare
time and are never invalidated by writes." `staleness.rs` 1–8, 78–83:
"The engine never calls this and no threshold exists"; "Harness-only (not
embedding API)." `70-api.md` 1166–1171 withdrawn. `20-query-ir.md` 1181–1187
names `staleness` as the compensating control. `write.rs` 159–223 witness
compare inside the writer lock. TS `db.ts` 1369–1388 live-scope check.
`prepared.rs` 289–293 `key_probe_direct` "Execute and profile consume this
flag; they do not re-match the pipeline at run time."

Correctness of execute-after-write was not found broken. This is a ship
gap relative to the mutation cutover, not a silent wrong-answer bug.

---

### Q-08 (severity: later)

**Location.** `ir.rs` `Term`; `ir/validate/context.rs` placement walls.

**Illegal state.** `Term::Measure` in an atom binding (`DurationInBinding`);
`Term::ParamSet` under any operator but `Eq`; two `Measure`s in one
comparison; `Measure` under `Allen`/`PointIn`/`Eq`.

**Representation critique.** `Term` is one sum used in bindings and
comparisons. Legal positions differ. A binding term is `Var | Param |
ParamSet | Literal`; a comparison side is `Var | Param | ParamSet | Literal |
Measure` with operator-specific subsets. One `Term` makes every illegal
placement a roster item.

**Better representation.** `BindingTerm` vs `CmpTerm` (or `CmpTerm` =
`BindingTerm | Measure`). Operator-specific constructors (already the TS
surface: `eq` allows `SetParam`, `lt` does not).

**Evidence.** `context.rs` 598–637 `DurationInBinding`; 758–826 measure
operator screen; validate.rs roster item 8.

---

### Q-09 (severity: later)

**Location.** `ir.rs` `Atom.bindings: Vec<(FieldId, Term)>`; duplicate check
in `ir/validate/context.rs`.

**Illegal state.** Two bindings of the same `FieldId` on one atom
(`DuplicateFieldBinding`). Absence-as-wildcard is the right idea; a `Vec`
of pairs does not make absence a map invariant.

**Better representation.** `BTreeMap<FieldId, BindingTerm>` (or a
per-relation dense slot array with `Option<BindingTerm>` per field). Duplicate
keys unrepresentable; wildcard = missing key, as the docs already say.

**Evidence.** `ir.rs` 145–169 "Absence of a field *is* the wildcard."
`validate.rs` roster item 3. `context.rs` 504 `DuplicateFieldBinding`.

---

### Q-10 (severity: later)

**Location.** `ir.rs` `FindTerm::AggregateMeasure { op: AggOp, over: VarId }`;
`ir/validate/finds.rs` 101–105.

**Illegal state.** `op` is `Count` or `Pack` (`DurationAggregateOp`).

**Representation critique.** Same Option/tag overload as Q-01 on a second
constructor. The comment already says "the only three ops the measure admits
(`Count` is nullary)."

**Better representation.** `op: FoldOp` (Q-01). Covered if Q-01 lands.

**Evidence.** `finds.rs` 103–105; reject tests around `DurationAggregateOp`.

---

### Q-11 (severity: later)

**Location.** Option/flag overloads adjacent to Q-01 that are *post-witness*
or bind-time, not the public CST.

- `SignatureColumn { ty, op: Option<AggKind> }` — projection vs fold as null.
  `enum Column { Project(ValueType), Fold { op: AggKind, ty: ValueType } }`.
- `ParamSpec::Scalar { ty, point: bool }` / `Set { elem, point: bool }` —
  a point-domain U64 is not a boolean on a scalar. `bind.rs` even tests
  `*point` on `FixedBytes` (ceiling word), so `point: true` is representable
  on non-element types. `enum ParamSpec { Scalar(ValueType), Point { element }, Set { .. } }`.
- `LoweredRule.written: Option<u16>` — `None` means "collapsed across written
  rules"; `minted: Vec<u16>` is already the uncompressed form. Prefer
  `enum Provenance { Single(u16), Merged(NonEmpty<u16>) }` or just `minted`.
- `ViewMemo.generation: Vec<Option<ViewGeneration>>` — `None` = unbound.
  A `ViewBinding::{Unbound, Bound { generation, .. }}` per occurrence
  (they already have `View::Unbound` at the image layer).

**Evidence.** `ir/validate.rs` 156–166 SignatureColumn "Kept together
deliberately." `prepared.rs` 602–608 ParamSpec. `bind.rs` 131–132 point on
bytes. `dnf.rs` 30–47 written vs minted.

---

### Q-12 (severity: later)

**Location.** `plan/fj.rs` `FjPlan` / `Node` / `Subatom` as plain `pub` data;
`PlanError` validate boundary.

**Illegal state.** Broken partition, missing occurrence, non-participating
occurrence in a node, unplaced residual, selection on a filtered field, …
The module says so: "Plans built by `binary2fj` + `factor` are valid by
construction; this boundary exists because `FjPlan` is plain data anyone
can construct."

**Representation critique.** Internal CST + validate, same pattern as Q-04,
crate-private (`plan` is `pub(crate)`). `ValidatedPlan` is the parse. Fine
to leave until `FjPlan` would otherwise leak; do not re-export it.

**Evidence.** `plan/fj.rs` 64–120 `PlanError` list.

---

### Q-13 (severity: later)

**Location.** `ir.rs` `Query::interiors_mut` / `head_mut` / `rules_mut`
(public); all IR structs have public fields.

**Illegal state.** After `Query::single(legal_rule)`, `rules_mut().clear()`
is `EmptyRuleSet` waiting for prepare. Reach interiors remain mutable as a
shared `Vec` across arms.

**Representation critique.** "Queries are plain data" does not require
post-construction mutation of the CST. Accessors that re-unify Cq/Reach
fields (`interiors()`, `head()`, `rules()`) are total and fine; the `_mut`
variants invite invalidation. `PreparedPipeline` repeats the same
`interiors_mut` pattern internally.

**Better representation.** Private fields, `Query::single` / builders, or
document the CST as write-once. Drop public `_mut` accessors.

**Evidence.** `ir.rs` 505–540.

---

### Q-14 (severity: later)

**Location.** Membership: "a typing rule, not a node" (`ir.rs` 158–168);
`TypeSlot::{Mono, Bivalent}` then `resolve_bivalents`; Lean
`Query/Membership.lean`.

**Illegal state.** A point variable bound only by membership (no enumerable
domain) — roster item 11. Bivalent slots that never resolve.

**Representation critique.** Recorded decision with a Lean preservation
proof (`membership_lowering_preserves`). The SPOV still objects: interval
field + element-typed term *is* `PointIn`, not `Eq` waiting on inference.
The bivalent slot is the illegal state (term is interval-or-element until
resolution). They parse it at validation (`TypeSlot` consumed into
`var_types` — phase change, good). The CST never had a membership node to
start with, so every host and the naive model must replay the typing rule.

**Better representation (horizon).** Binding `(field, term)` splits at the
CST: `BindValue` vs `BindPoint` when the field is interval-typed, or keep
bivalent only in an untyped CST and make `ValidatedAtom` carry
`Equality | Membership`. Do not relitigate while Membership.lean is the
arbiter; do not add a third spelling (TS `literalSet` is already one, Q-05).

**Evidence.** `ir.rs` 158–168; Lean `Syntax.lean` 38–49; `TypeSlot` 897–911
"CONSUMED by `resolve_bivalents`."

---

### Q-15 (severity: later)

**Location.** Accidental interpreters vs the explicit evaluator.

The **intentional** small evaluator is `exec/run/run_node.rs` over
`ValidatedPlan` (nodes, covers, residual lists grouped by kind — the
`fj.rs` 236–245 refusal of a merged `RejectionFilter` enum is the
batching law as data). Kernel `exec/kernel/filter.rs` is per-shape
functions, not a tag switch.

The **accidental** ones: `image/view/apply.rs` ~500-line `match predicate`
with `unreachable!("validated")` arms (Q-06); `exec/dispatch/classify.rs`
eligibility as a pile of negative predicates (residuals nonempty, measure
filter, ParamSet, Interior, closed relation, …) returning `Option<KeyProbePlan>`
instead of parsing `NormalizedQuery` into `KeyProbe | FreeJoin` once at
prepare (they *do* store `PreparedRule::{KeyProbe, FreeJoin}` — then
`key_probe_direct` re-encodes a subset as a bool, Q-07); `plan/ground.rs`
walking `Role` with fold/eliminate special cases that `Role` already
makes disjoint.

**Better representation.** Classification is a parse (`NormalizedQuery` →
`Access::{KeyProbe(plan), Join(normalized)}`) with no `Option` and no later
bool. View evaluation matches a predicate enum that cannot spell Allen-in-
`Compare`. Grounding is already mostly data (`Role::Folded(FoldedMark)`);
keep new cases as `Role` variants, not flags.

**Evidence.** `classify.rs` 26–102 fall-through `None`. `prepared.rs` 289–293
`key_probe_direct`. `apply.rs` 123–208.

---

### Q-16 (severity: later)

**Location.** C ABI `crates/bumbledb-c/src/query.rs` `bdb_term`, `bdb_atom`,
`bdb_find_term` — tag plus unused payload fields.

**Illegal state.** `kind = Var` with a live `literal`; `source_kind = Edb`
with a live `interior` id. C cannot spell sums.

**Representation critique.** Expected for C. They parse by kind and document
that leftover payloads are never read (`query.rs` 424–429 condition trees).
Keep marshal as the parse (it already is for Count vs Aggregate). Do not
let leftover payloads become a second IR.

**Evidence.** `bdb_term` 48–57; `bdb_atom` 80–88; `find_term_in` 372–397.

---

### Q-17 (severity: later)

**Location.** TS `ts/src/query/find.ts` `Agg<Op, Over extends … | undefined>`;
`lower.ts` `isTypedScope`.

**Illegal state.** `Agg<"count", AnyVar>` is writable in the type (only
`count()` constructs `undefined`). `isTypedScope` never fails.

**Representation critique.** Same Option overload as Q-01 at the SDK value
layer, while `FindTermIr` already split. The fake type guard is a
validate-don't-parse fig leaf on a single runtime chain.

**Better representation.** `type Agg = { agg: "count" } | { agg: "sum"|"min"|"max"; over: AnyVar | Duration } | { agg: "pack"; over: AnyVar }`
(no `undefined`). Drop `isTypedScope`; the raw builder *is* the runtime
and the typed interfaces are the face (they already say this).

**Evidence.** `find.ts` 33–57. `lower.ts` 1250–1252.

---

### Q-18 (severity: later)

**Location.** `FieldId` on interior atoms addresses head positions; on EDB
atoms addresses stored fields. Same type, documented as "never a pun" for
`InteriorId` vs `RelationId`, but `FieldId` *is* the pun.

**Illegal state.** `FieldId(i)` past the target signature
(`InteriorColumnOutOfRange`) or naming an EDB field that does not exist.

**Representation critique.** Lean records "an interior atom's bindings
address HEAD POSITIONS positionally — `FieldId i` is column `i`." Same
newtype, two meanings. Interior membership is engine-only (Lean theorems
match `a.source = .edb`).

**Better representation (horizon).** `enum Place { Edb { relation, field }, Interior { id, column: u16 } }`
on the binding, or keep `FieldId` but wrap interior bindings as
`Vec<Term>` in head order (no field ids; position is the index). TS already
lowers interior atoms by head order (`lowerInteriorAtom`).

**Evidence.** `ir.rs` 74–80, 47–51 "separate identity, never a pun" (about
`InteriorId`/`RelationId`, not `FieldId`). Lean `Syntax.lean` 97–114.

---

## Mutation / prepared — what is safe to ship

Not findings; recorded so Q-07 is not misread as "write_from is wrong."

- `write_from` / `write_from_witness`: environment identity, then generation
  compare inside the writer lock, `f` never runs on mismatch. Snapshot
  fields private. TS `writeFrom` requires a live read scope of the same
  store.
- Prepared execute: `ForeignPreparedQuery` on env mismatch; view memo
  generation-keyed; closed relations bind a sentinel generation and do not
  phantom-drift (`staleness.rs` tests). Pin-at-prepare is **correctness-
  preserving**; only costed join order can go stale.
- `Prepared` in TS is an identity token (`Object.freeze({})` + `WeakMap`);
  the native handle is not a public value; `FinalizationRegistry` is
  reclamation-only. Cross-store execute is refused via `owner`.
- Reach measure conditions are refused (`MeasureInRec`); interior/rec
  heads refuse measure/aggregates (Q-03). Main-rule ray probes still build
  when main is nonempty (`build.rs` 231–235). The `prepared.rs` comment
  "Empty for … Reach queries" describes the *top-level* field not carrying
  rec probes (interiors have their own `ray_probes`), not a Reach-main skip.

---

## Working tree vs HEAD

`ir.rs` is unchanged in the snapshot git status. In-flight query-adjacent
edits are plan/exec tests, `plan/ground.rs`, `plan/fj.rs`, prepared
staleness tests, and `api/db/mutation.rs` (write-path reports, not IR).
Findings above hold at HEAD and on the working tree. Q-07 is the one
finding that *becomes* load-bearing because mutation is what this cutover
publishes.
