# The representation contract — pinned before any fix lands

One decision per cross-cutting coordinate. Lean, engine, SDKs, and docs
implement THIS, not their audit's local suggestion, where the two
differ. Semantics are identical throughout: denotations, walls (no
negation/aggregation through the cycle), this-cut OPEN refusals
(mutual/nonlinear/stacked/named-interior-of-finished-rec), the
derived-tuples ledger, budget values, and the locked names
(`DerivedBudgetExceeded`, `set_derived_budget`, `DEFAULT_DERIVED_TUPLES`,
`DEFAULT_REACH_ROUNDS`). Representation changes; meaning does not. No
new caps; `MAX_CTES`/`MAX_INTERIORS` stay dead; no Program vocabulary
returns.

---

## C1. The boundary IR is open and dense; every trusted layer is a sum

There are two kinds of query representation and exactly one of each:

* **The hostile boundary** — `crates/bumbledb/src/ir.rs::Query
  { interiors, rec: Option<Rec>, head, rules }`, the JSON corpus
  encoding, the C ABI `bdb_query` (nullable `rec`), and the TS wire
  type. These stay **shape-unchanged**. C and JSON have no sums;
  `Option<Rec>` / `rec: null` / `rec == NULL` *is* the sum's spelling
  at a boundary whose job is to admit hostile states so the one
  validator can refuse them by name (parse, don't validate — the parse
  happens AT `validate`). Consequence: the 22 `reach-*.json` and 246
  other cases do **not** regenerate; the Lean decoder and the engine
  emitter stay aligned with the existing corpus.
* **Everything after a parse** — the Lean `Query`, the engine witness,
  the engine prepared object, and every SDK builder's *recorded* state
  — is a **sum with two arms**: rec-absent and rec-present. Interiors
  are ordinary data (a possibly-empty prefix) in **both** arms —
  Dijkstra's half-open interval, engine F37: the CQ is the empty
  prefix, never a third type and never a sidecar beside a body enum.

## C2. Rec identity: dense id, stored once, structural where it is self

`InteriorId` stays a dense index (`u32` engine / `Nat` Lean): interior
`i` is id `i`; the rec, when present, is id `interiors.len()`. That is
the boundary numbering and it does not change. What dies is every
**recomputation and dual coordinate** over it:

* Engine: the witness stores `rec_id: Option<InteriorId>` /
  `derived_count` once at validate (F28); prepare/execute/render read
  the stored value or match the witness sum — no site re-derives
  `len() + usize::from(is_some())`, and no `expect("rec present")`
  survives (F16).
* Lean: `Query.recId`, `Query.derivedCount`-as-flag, and the
  per-evaluator `self := ⟨q.interiors.length⟩` reconstruction die with
  the sum; the reach constructor is the ONE site that knows the rec's
  id. Inside a rec arm, self is **structural** (C4), not an id
  comparison.

## C3. Engine sums (witness and prepared)

* **Witness:** `ValidatedQuery` carries query-global param tables plus
  a shape sum: `Cq { interiors, main }` | `Reach { interiors, rec:
  ValidatedRec, main }`. `ValidatedRec` arms each carry the proof
  validation found: `self_occ` (the unique positive self-atom's
  occurrence) per rec arm (F5). Emptiness/linearity/negation stay
  roster errors at the boundary (names unchanged); the witness cannot
  spell them.
* **Prepared:** `PreparedBody` + sidecar `interiors` die. One pipeline
  sum, interiors inside each arm (F1):
  `PreparedPipeline::Cq { interiors, rules }` |
  `Reach { interiors, driver, main, rounds_budget }`.
  Statically-dead main is `rules: []` — `Empty` is not a variant
  (F23); the empty fast path is the zero-iteration loop. The direct
  key-probe lane is parsed ONCE at build (its own arm or a
  build-computed property consumed by the one protocol), never
  re-detected per call (F8/F31), and `execute`/`profile` are one
  protocol parameterized by counters (F8).
* **Rules:** `PreparedRule = FreeJoin | KeyProbe` only. A rec arm is
  `RecArm { delta: OccId, rule: FreeJoinRule }`, inhabitable only in
  `ReachDriver.rec` (F2). `RecursiveRule`/`DeltaVariant` and the
  variant vocabulary die (F7). `ReachDriver` owns base + rec arms +
  rec sink/scratch; main does NOT live in the driver (F15).
* **Budgets:** `tuples_budget` on the prepared query (universal axis);
  `rounds_budget` on the Reach arm (F14). `set_derived_budget` keeps
  its name, signature, and observable behavior.
* **Binds/scratch:** one derived-images layout (working
  `TransientImage` per derived id + published `Arc` after close); rec
  ping-pong is a `PingPong { a, b, flip }` of that (F13/F40). The rec
  bind is one sum (`DerivedBind::Finished | Rec { delta, acc }`), not
  three `Option`s (F10); dead `variant_delta`/`rec_delta` parameters
  are deleted; `idb_*` renames to `derived_*`.
* **Sealing:** interiors type against `&[Predicate]` already sealed in
  declaration order; rec base against that slice; rec arms against
  slice + rec predicate. No `Option<Predicate>` holes, no second
  screen inside `column` (F6).
* **Stats/introspection:** stats mirror the pipeline sum; ghost
  always-empty `rules` tables die; `unit_labels`-emptiness stops being
  a mode bit (F12/F29). Vocabulary: `ground_main`, `derived_images`,
  `interior {id}` / `rec` in display and render — no `predicate p{}`,
  no `strata`, no `program` on live data (F11/F33/F34/F35).
* **Signature naming (amended during issue explosion):** the sealed
  signature type `ir/validate::Predicate` and its `predicate()`
  accessors are Datalog-predicate vocabulary on the main/derived
  signatures. They rename mechanically (`Signature` / `signature()`),
  engine + bench internal (F41; docs F2/F17 teach the corrected word).
  The C ABI, the locked names, and the boundary IR are untouched.

## C4. The Lean sum and the typed rec

```
inductive Query
  | cq    (interiors : List Interior) (arity : Nat) (rules : List Rule)
  | reach (interiors : List Interior) (r : LinearRec) (arity : Nat) (rules : List Rule)
```

(Constructor names `cq`/`reach`; `rec` is unavailable as a Lean field
or constructor name — the recorded recursor collision. Accessors
`Query.interiors` / `Query.arity` / `Query.rules` are total by match.)

* **`LinearRec` is typed** (H2): base arms and step arms are their own
  structures — `RecRule { finds, atoms, conditions }` (no `negated`
  field: negation in the rec is **unrepresentable**, matching
  `NegationInRec`) and `RecStep { finds, selfBindings, atoms,
  conditions }` where `selfBindings` IS the unique positive self-atom
  (linearity structural — `selfCount`, `hasNegatedSelf`, `recLinear`,
  and the `oddRec` syntax inhabitant die; `odd_not_monotone` stays as
  the operator-level wall). `base`/`step` are nonempty by type.
  Boundary spelling unchanged: the decoder parses the JSON `rec`
  object (self-atom = the `interior` atom whose id equals
  `interiors.length`) into this type; reach cases are all legal recs.
* **Arity:** `Interior.arity` and `Rec.arity` die (H6) — a derived
  head's width is its rules' `finds.length` (uniform under acceptance,
  which is what the evaluator already trusted). The main `arity` stays
  only if a denotation genuinely reads it; otherwise it dies too.
* **Denotation:** `evalQuery` is one function by constructor cases.
  Interiors evaluate as a structural **fold in declaration order** (no
  `Nat` stage, no `none => False` arm — M1); a later or out-of-range
  read is empty by construction of the fold, which is the same recorded
  phantom semantics as today. `Query.Plain`, `evalQuery_plain`,
  `Query.allRules` + the three `mem_allRules_*` inversions,
  `Query.recId`, `recLinear`, `Rule.edbOnly`, `plain_wellFormed`, and
  the `WellFormed` ∧-bundle (`sourcesInRange`, `interiorsDag`) die
  (H1/H3/H4/M3/M6/M8). `naiveIter`/`semiNaiveIter` leave the meaning
  module; `reachDen = lfpS` is the one meaning (M4). `recDom` speaks
  derived tables, not `idb` (M5).
* **One rule-list theory:** theorems about a rule list (Dedup,
  Rewrites, `disjoint_witness`, union regime) are stated over
  `List Rule` + an environment, not over a `Query` wrapper;
  `RewriteStep : List Rule → List Rule` (H5/L2). Theorem 9
  (`snapshot_single`) is restated over `evalQuery` (H5).
* **One decoder:** `Main.lean`/`Conformance.lean` decode ONE `Query`
  and ONE atom grammar; `relation` (seeded spelling) and
  `edb`/`interior` (reach spelling) are two JSON spellings of the one
  `AtomSource` (M2). `plainQuery : CQuery → Query` and the second
  `evalQuery` die. The corpus files do not change.

## C5. Ruling R-DENSE (the one deliberate partial)

Identities stay dense (`InteriorId : Nat`, `FieldId : Nat`) and
environments stay total functions at the Lean spec level; the spec
models the boundary object the corpus feeds and keeps every denotation
total, so theorems carry named premises instead of dependent indices.
The Fin-telescope/`Vector` halves of Lean H3/H4/H6/M7/M8 are **refused
under this ruling** (Insight 15/16: the index bookkeeping across a
24k-line proof tree costs more than the branches it deletes, and the
recorded phantom-read semantics — exact agreement with or without the
screen — is a boundary behavior the model deliberately keeps
expressible). Every *dual coordinate* those findings identify (flags,
bundles, recomputed ids, dead screens) dies per C4. The audit ledger
annotates the split per finding.

## C6. SDK contract (lower to the unchanged boundary IR)

* **Rust `query!`:** `ParsedRule` is a sum (`Bare`/`Interior`/
  `Recursive`, name inside the carrying constructors — #14); param
  style is a sum, not two bools (#15).
* **TS:** the builder carries phase in the type — `interior`/
  `recursive` exist only while `Rec extends null`; `.recursive()`
  moves the type parameter, so second-rec / interior-after-rec are
  `never` like after-main (#5, #18). `collectRec` builds the sealed
  `RecData` in one assignment — no readonly-cast mutation (#7).
  `QueryIr` stops being an open public constructor: `dbPrepare`
  accepts a branded `ParsedQuery` that only `lowerQuery` / an exported
  `parseQueryIr` (structural parse: rec/main nonempty, aggregate finds
  split) inhabit (#6). Aggregate finds are a sum — Count carries no
  `over`; folds require it — in `FindTermIr`, `CmpData` mask moves
  into the op (#17), `isQueryValue` stops type-predicating what it
  did not check (#16).
* **C++:** `query_value` becomes a phase machine in the template —
  `query_value<S, NI, HasRec, NR>`: `.recursive` exists only when
  `!HasRec`, `.interior` only before rec/main, `prepare` only when
  `NR >= 1`; `rec_ir` exists only when `HasRec`; `has_rec` the runtime
  bool dies (#1, #2, #19). `wire_atom` carries one tagged source, not
  relation+bool+interior_id (#3). `find_form` gains `measure` — four
  cases mirroring `FindTerm`; `has_over` and dummy `op` fillers die
  (#4, #20). Polarity is a sum for interior atoms too (#10);
  builder-IR discriminator-plus-all-payloads collapse to variants or
  per-alternative arrays (#11). SDK-invented caps die: the engine's
  `MAX_RULES = 16` is the one number; `rec_ir` is one pooled array +
  `base_count` (#12). `wire_condition` is a tree (#13). Wildcard is
  absence from recorded bindings, not `term_form::absent` (#9).
  `array<T, 0>` replaces the dummy-slot dance (#21). Compile-fail
  fixtures pin the phase machine (#18).
* **C ABI:** flat tagged structs stay (essential C). `has_over` dies:
  `bdb_find_term_kind` distinguishes the nullary Count as its own kind
  (or equivalently the aggregate kinds always read `over`); bridge and
  `query_view` move together (#8). Beyond tag-selection, the bridge
  stays a spelling transform: the engine validator is the ONE refusal
  authority (C1) — the *dialects* are what can no longer mint the
  illegal states.

## C7. Docs speak the present tense of THIS contract

Query, interiors, one linear rec, main signature. No `program`, no
`SCC`/`Tarjan`, no `stratum`, no `Idb`, no `CTE`-as-our-noun, no
`CQuery`, no deleted cap names taught by negation, no "today's query
plus two empty fields" embeddings (docs F1–F28). The cookbook's
`Program` example relation renames. `feature-register` speaks
`AggregateInInterior` and `recLinear`-successor names as they now
exist. `ForeignPreparedQuery` is documented as essential runtime
identity (process-distinct instance) with the horizon representation
named (docs F18).

## C8. Census and corpus stay gates

`scripts/lean.sh` (build + battery + spec census + 268-case
conformance + three-way comparator) and `scripts/check.sh` define
green. Bridge.lean mechanism/instrument tokens move WITH engine
renames; docs' `lean/…` citations move WITH Lean renames. Assertions
are never weakened to pass.

---

## C9. Sealed schema sums

C1 does **not** freeze these trees. After `SchemaDescriptor::validate`,
every trusted schema layer is a sum. The hostile descriptor stays a
product: `RelationDescriptor.extension: Option` is the boundary spelling
(schema-010) so validate can refuse `EmptyExtension` / `StrOnClosedRelation`
/ … by name. The witness parses:

* **Relation:** `RelationBody::Ordinary { fresh: Option<KeyId> }` |
  `Closed { extension }` — closedness is not an Option beside the
  ordinary fields. Closed relations are not writable. Shared layout
  fields may sit outside the sum; `fresh` vs `extension` must not.
* **Keys:** `KeyForm::FreshRow | Scalar | Pointwise` — FreshRow cannot
  be pointwise; `DisjointDeterminantProof` lives on the Pointwise arm.
  The ordinary relation's mint is that FreshRow key (or `None`), not a
  second `fresh_row` bool / `fresh_row_field` that must agree.
* **Capacity measure/window:** `SealedWeight` / `SealedBound` carry
  Duration tails **in-arm**. `Unit` and `Unbounded` are cases, not
  absences (descriptor `hi: Option<Bound>` may keep the hostile `*`
  spelling).
* **Capacity enforcement:** `CapacityEnforcement::{ScalarProbe, Closed}`
  — containments keep three-arm `Enforcement`; `IntervalCoverage` is
  unrepresentable on a capacity. Containment `IntervalCoverage` carries
  `source_tail` (schema-011).

Do **not** number a corruption-variant clause C10: `capacity-laws` C10
is ray-Duration refusal (`Error::CapacityRayMeasure`). Named
`CorruptionError` arms (err-004) land under C1–C8.
