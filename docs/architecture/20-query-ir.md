# 20 — Query IR

## Decision: the query surface is the IR, permanently — pure data

Queries are **plain Rust data structures** — encodable in principle (plain owned
data, no borrows, no behavior), inspectable. The IR is the engine's public contract,
and it is also the language dependencies are written in (`30-dependencies.md`): one
representation for "what holds" and "what do you want".

**The ruling (owner-ruled 2026-07-10), permanent:** no builder API, no typed query
variables, no text language, no ergonomic layer in the engine — ever. Any
convenience syntax lives in a downstream package (in any language) and lowers to IR
data; the engine never knows it exists. The code/data boundary is logic's own — a
schema is the *theory*, a query is a *sentence in* the theory (`70-api.md` § the two
surfaces) — and the pure-data doctrine, recorded above for testability, has a second
reader: **a foreign-function boundary can only carry data**, and the IR already is
data. Two earlier decisions are vindicated by a requirement that did not exist when
they were made: the memoized one-copy result heap (a snapshot-lifetime borrow cannot
cross a language boundary; the copy can) and the dyn write surface's typed-error
discipline (the portable half of the API, not ETL plumbing).

**Alternative 1 — a Logica text frontend**: investigated deeply (findings
summarized here). Logica's syntax and rule model are excellent; its semantics are
SQL-shaped at the core — explicit bag semantics ("conjunction multiplies
multiplicities"), null in every type, negation defined as an is-null test over an
aggregate, 269 untyped SQL-passthrough functions plus a raw-SQL escape hatch, recursion
as observable depth-8 unrolling, `Count=` compiling to *approximate* count on one
engine. A "strict subset" could keep the syntax but not the semantics. **Why it lost:**
one user, calling from Rust; the parser layer served nobody. What we keep as IR design:
named-argument atoms, group-key-from-output aggregation, membership as a binding form
(Logica's `x in R`), and rules-as-views reborn as **host-language composition** — a
"derived predicate" is a Rust function returning an IR fragment; the engine has no
view/rule/module concept.

**Alternative 2 — typed builder/generics as the contract, and its stronger 2026-07-10
form, a typed builder beside the contract.** **Why both lost:** owner ruling —
builders bake a Rust calling convention into the surface (closures and generics are
exactly what a foreign host cannot invoke) and translate poorly to other-language
bindings; data structures translate to anything; and the builder's
compile-time-checking dividend is re-provided by the validation roster's typed
errors, which foreign callers need anyway. Sugar *producing* the IR is downstream-
package territory, in any language, permanently. (Host newtypes still give
compile-time nominal safety at the app layer — see `10-data-model.md`.)

## The query shape: one head, a set of rules

A query is a **program**: one head and a non-empty list of conjunctive
**rules** — which is precisely a **non-recursive Datalog program**. The head
owns the find shape (arity, aggregate ops, and the output typing — the
predicate below, sealed at validation); each rule is a conjunct
(positive atoms, negated atoms, conditions) whose find terms align against
the head position by position. The single-rule query is the degenerate case
and embeds the conjunctive query unchanged (`Query::single`); every
pre-rules query is a one-rule program.

- **A query defines one anonymous predicate; rules derive it.** The head is
  its definition, and its typed **signature** is the result-type row: one
  column per head position, each carrying the type that lands in the buffer
  (`Count`/`CountDistinct` are U64 whatever they counted; the measure is
  U64; `Sum`/`Min`/`Max` carry their input's type; `Pack` its interval
  type; the Arg forms the carried payload's type) together with the fold
  producing it. It is derived **once**, at validation, and sealed in the
  witness (`ir/validate`'s `Predicate`); sink construction, result-buffer
  typing, finalize's all-words decision, and EXPLAIN's header all read that
  one object — no second derivation of the output row exists anywhere.
  The fence: the predicate is anonymous and engine-internal, **referenced
  by nothing** (names live in the host, exactly like relations pre-`as`).
  The moment something REFERENCES a predicate — a head usable as a body
  atom — that is the recursion trigger firing: go through the recursion
  design's ledger, not around it.

- **Denotation: the query denotes the set union of its rules' denotations.**
  Set semantics means there is exactly one union — no bag distinction exists
  or is representable (there is no UNION ALL to refuse).
- **Variables are strictly rule-scoped**: the same `VarId` in two rules
  names two unrelated variables (they may resolve to different types). A
  rule is its own typing scope and its own plan.
- **Params are query-global**: one binding surface; any rule may reference
  any param, and every rule's anchors must resolve one type per param.
- Rules are deliberately **one step short of the fixpoint**: a rule's head
  is never a body atom, so no recursion is expressible. The recursion `OPEN`
  item (below) gains its landing pad here and is not entered.

## Semantics

- The logical solution of a **rule** is the **set of distinct bindings of
  the rule's variables** that satisfy every positive atom, every membership
  binding, every condition, and **no negated atom** (below); projection
  returns the **set** of projected facts, and the query's solution is the
  union of its rules' projections.
- **Existential variables never multiply projection output.** (Scoped to projection —
  see aggregation below.)
- Distinctness is the default and only behavior; there is no DISTINCT concept.
- No ordering or limit in the IR: results are sets; the host sorts (`70-api.md`).

### Negation (normative)

A rule carries a list of **negated atoms**. A binding satisfies a negated atom iff
**no fact** of its relation matches the atom's bindings under that variable
assignment — plain anti-join over sets; no null trick, no three-valued logic.
**Safety rule:** every variable occurring in a negated atom must also occur in a
positive atom (a negated atom binds nothing; it only rejects). Literals, params,
param sets, and membership bindings are all legal inside negated atoms. There is no
stratification concern because there is no recursion. Negated atoms contribute no
find variables and never multiply anything — they are filters with a relation's
worth of vocabulary.

**The outer-join idiom, documented (the sanctioned decomposition):** "A with its B,
if any" is two queries — `A ⋈ B`, and `A` with a negated `B` atom — merged by the
host. Results are sets; the merge is a concatenation. An outer-join concept will
never enter the IR; a row that is half-binding, half-absence is a null wearing a
join costume.

### Aggregation (normative)

- **The fold domain of every aggregate is the group's set of distinct full bindings
  over all the rule's variables.** Group key = the values of the non-aggregated find
  variables. **Across rules**, aggregates read the head: the fold domain is
  the union of the rules' binding sets projected to the head (the executor's
  spanning seen-set keys exactly that head projection —
  `40-execution.md` § the rule loop; provably disjoint rules elide it,
  § set semantics). Two postings of amount 100 to one account are two distinct bindings (their
  fresh ids differ): `Sum(amount) by account` = 200.
- **The footgun, stated loudly:** joining a multiplicity-adding relation into an
  aggregate multiplies the binding set — `Posting ⋈ PostingTag` with 3 tags per posting
  triples the sum, exactly as in SQL. Don't write that query; aggregate first (in v0:
  aggregate in one query, join its result in the host) or bind nothing from the
  multiplying relation... which still adds its variables. This is inherent to
  joins+aggregation, not to set semantics.
- `Sum` accumulates in **i128** and range-checks the final value once: Sum(I64)→I64,
  Sum(U64)→U64 (accumulator u128), out-of-range = a runtime query error. Deterministic
  under any fold order — set folds have none.
- `Count` is **nullary**: |the group's binding set|, result type U64.
- `CountDistinct(x)`: |the set of distinct values of x across the group's binding
  set|, result type U64; legal over every type (equality is all it needs).
- `Min`/`Max` accept U64 and I64 only (the orderable types — intervals and
  `bytes<N>` excluded, `10-data-model.md`); result type = input type;
  deterministic (a set has one minimum).
- **Arg-restriction (`ArgMax`/`ArgMin`), semantics before shape:** when a find list
  contains Arg terms, the group's binding set is first **restricted to the bindings
  attaining the extreme of the key variable** (max for ArgMax, min for ArgMin), and
  the group's output rows are projected from that restricted set. This definition
  makes multi-carry coherent by construction (all carried values come from the same
  surviving bindings) and makes ties honest: **a tie yields every attaining row** —
  the answer is a set; with fresh keys ties cannot occur. Validation: all Arg terms
  in one query share one key variable and one direction; the key must be orderable
  (U64/I64); the key variable may itself be projected. Arg terms and fold aggregates
  (Sum/Min/Max/Count/CountDistinct) may not mix in one query in v0 — "sum of the
  latest" is two queries, and the composed form waits for a real need.
  **Arg-restriction is single-rule only** (a typed validation error on 2+-rule
  programs, DNF-lowered rules included): the restriction key is a rule-scoped
  variable outside the head's vocabulary — rules need not even agree on its type —
  so "the extreme over the union" is undefined. Modeling answer: one Arg query per
  disjunct, host-merged. *Trigger* for defining a cross-rule restriction: a real
  query.
- **`Pack` (the coalescing fold — Snodgrass's coalesce), semantics:** over an
  interval-typed variable, per group the result is the set of **maximal disjoint
  half-open segments** of the union of the group's interval point sets. **`Pack`
  is relation-shaped: one result row per (group, maximal segment)** — the
  one-row-per-group convention was never a law (`ArgMax`'s tie sets were the
  precedent), and a set-semantic query's result is already a set of rows, which
  is exactly what dissolved the old OPEN item's "a set per group" blocker. Head
  shape: the group variables plus **one interval-typed result position** (the
  packed segment shares its input's element type); at most one `Pack` per head —
  the multi-`Pack` product has no sighting and is refused with the trigger "a
  real query needing two coalesced columns in one row". `Pack` mixes with **no
  other aggregate** (the Arg/fold mixing rule, extended: a fold column repeated
  per segment row is a join in aggregate costume, and two relation-shaped
  aggregates do not compose in one head); its companions are group-key positions
  only. Adjacency merges (`end == next.start` — the half-open law), identical
  claims collapse in the coalesce (set-semantic dedup upstream is unchanged),
  and a ray absorbs everything after its start — **the packed ray is a ray** (no
  measure is taken, so no `MeasureOfRay` interaction). **Across rules `Pack`
  folds the union** (unlike Arg-restriction): the head projection carries the
  raw claim, so the spanning seen-set keys (group, claim) pairs and the coalesce
  runs over ∪. Composition refusals, recorded: coalesced-time accounting
  (`Sum∘Duration∘Pack`) is **two prepared queries or a host fold over packed
  rows** — aggregates of aggregates stay refused (no nesting; the standing
  aggregate law) — with the trigger "a measured two-pass budget violation"; free
  time (`Gaps`) stays a two-line host walk over sorted packed output (README
  refusals ledger).
- **All-aggregate finds are legal** (empty group key, one global group). Over empty
  input the result is the **empty set** — not a 0 or NULL row. "The balance of an
  account with no postings is an absent row, not 0." This is a documented divergence
  from SQL's ungrouped-aggregate behavior; the oracle rule lives in
  `60-validation.md`.
- Aggregates over illegal input types, an aggregate whose variable is also a group
  key, and duplicate find terms are validation errors.

## IR shape (normative)

```rust
Query {
    head:       Vec<HeadTerm>,        // ≥1; the find shape every rule aligns to
    rules:      Vec<Rule>,            // ≥1, ≤ MAX_RULES (16)
}
Rule {
    finds:      Vec<FindTerm>,        // one per head position; duplicates rejected
    atoms:      Vec<Atom>,            // ≥1; conjunctive, positive
    negated:    Vec<Atom>,            // anti-join atoms (safety rule above)
    conditions: Vec<ConditionTree>,   // the list is a conjunction; trees are
                                      //   the INPUT grammar — validation
                                      //   distributes them away (below)
}
ConditionTree = Leaf(Comparison)      // the input condition grammar: any
              | And(Vec<ConditionTree>)  // boolean combination of positive
              | Or(Vec<ConditionTree>)   // comparisons — lowered at validation
HeadTerm   = Var | Aggregate(HeadOp)  // var-free: variables are rule-scoped,
                                      //   so the head names shapes and the
                                      //   rules supply the variables (a
                                      //   Duration find is a Var position:
                                      //   a u64 value per binding)
HeadOp     = Sum | Min | Max | Count | CountDistinct | ArgMax | ArgMin | Pack
Atom {
    relation:   RelationId,
    bindings:   Vec<(FieldId, Term)>, // named-field; absence of a field IS the wildcard
}
Term       = Var(VarId) | Param(ParamId) | ParamSet(ParamId) | Literal(Value)
           | Duration(VarId)          // the measure — comparison side only
                                      //   (§ the measure; a binding position
                                      //   is a typed rejection)
Value      = Bool(bool) | U64(u64) | I64(i64)
           | IntervalU64(u64, u64)    // start < end enforced at the boundary
           | IntervalI64(i64, i64)
           | String(Box<[u8]>)        // raw UTF-8 bytes; interning is the engine's job
           | FixedBytes(Box<[u8]>)    // a bytes<N> value: exactly N raw bytes — the
                                      //   length is the type; inline, never interned
           | AllenMask(AllenMask)     // the mask value shape — a param payload,
                                      //   never a field type (10-data-model.md)
FindTerm   = Var(VarId)
           | Aggregate { op: AggOp, over: Option<VarId> }   // over: None for Count
           | Duration(VarId)                                // the measure, projected
           | AggregateDuration { op: AggOp, over: VarId }   // Sum/Min/Max of the
                                                            //   measure (only those
                                                            //   three; typed rejection
                                                            //   otherwise)
AggOp      = Sum | Min | Max | Count | CountDistinct
           | ArgMax { key: VarId } | ArgMin { key: VarId }  // over = the carried var
           | Pack                                           // over = the packed interval var;
                                                            //   relation-shaped (§ aggregation)
Comparison { op: CmpOp, lhs: Term, rhs: Term }
CmpOp      = Eq | Ne | Lt | Le | Gt | Ge
           | Allen { mask: MaskTerm }  // THE interval-pair comparison (below)
           | Contains                  // point membership as a predicate — the
                                       //   point form only; ⊇ is Allen(COVERS)
MaskTerm   = Literal(AllenMask) | Param(ParamId)  // a variable or set mask is
                                                  //   unrepresentable, not rejected
```

Representation notes (the branch-removal decisions): no `union`/`or` node
exists — disjunction at the top **is** the rule list, so an OR execution
node is unwritable, and the head's var-free shape makes "a head variable
shared across rules" equally unwritable (rules align positionally instead).
No wildcard variant — an unbound
field is absent from `bindings`, so "wildcard bound to something" is unwritable.
Variables carry dense ids only; names are a debugging sidecar. `Value` has exactly one
variant per data-model type — no universal-integer variant: U64 and I64 literals are
exact-typed, out-of-range is unrepresentable rather than truncated, and an interval
literal with `start ≥ end` is rejected at the validation boundary like a non-UTF-8
string. Negated atoms reuse `Atom` unchanged — negation is a *position* in the query,
not a kind of atom.

**Membership is a typing rule, not a node.** A binding `(field, term)` where the
field is `Interval(E)` and the term's type is `E` means **point membership**:
the binding satisfies iff `start ≤ t < end`. A term of type `Interval(E)` in the
same position means interval **value equality** (identity, `10-data-model.md`).
Var, Param, ParamSet, and Literal all participate under the same rule. The point
domain is `MIN ..= MAX−1` (`10-data-model.md`'s point-domain law — `end == MAX`
denotes the ray `[s, ∞)`): an element-typed literal equal to the domain ceiling
is a validation error wherever it meets an interval position (membership bindings
and `Contains` operands), and a point-position param bound to the ceiling is the
matching bind-time error — `MAX` is the ray's ∞, never a point, so the mistake is
typed out instead of silently matching nothing. One
consequence, enforced by validation: a variable bound *only* by membership bindings
has no enumerable domain — every point variable must be bound by at least one
non-membership occurrence (a scalar field binding). Interval-vs-interval
comparison needs no shared point variable: that is the `Allen` predicate.

**Comparison rules, complete:** both sides must have the same structural type except
where stated (no U64-vs-I64, no silent coercion). `Eq`/`Ne` are legal for all six
types; `Lt/Le/Gt/Ge` only for U64/U64 and I64/I64 — **never intervals, never
`bytes<N>`** (`10-data-model.md` orderability; each refusal named in its own
diagnostic). `Allen { mask }` requires two interval terms of
one element type — **the** interval-pair comparison (next section). `Contains`
requires an interval left side and an **element-typed** right side (point
membership as a predicate — the predicate form of the binding rule, for terms
already bound elsewhere); its old interval⊇interval form is not an operator —
that predicate is `Allen(COVERS)`. `Eq` between
two variables is unification and obeys identical type rules. `Duration(t)` is
a u64-valued variable side legal under the order operators only (§ the
measure). Any comparison without
a variable side (literal-vs-literal, param-vs-literal, param-vs-param) is a
validation error, and so is a variable compared with itself — both are
constant-valued: write the query you mean.

## The Allen operator (normative — the interval-pair coordinate system)

The 13 Allen basic relations are jointly exhaustive and pairwise disjoint over
nonempty half-open intervals (the type's preconditions, `10-data-model.md`):
every configuration of two intervals is **exactly one** of them. The set of all
interval-pair predicates is therefore the powerset 2¹³, and the IR carries it as
exactly that: `Allen { mask }` between two interval terms of one element type,
satisfied iff `classify(lhs, rhs) ∈ mask`. One operator parameterized by a
13-bit mask replaces an operator vocabulary permanently — the vocabulary can
never grow again, because nothing exists outside the coordinate system.

- **The bit order is a specified representation, not an implementation
  detail**: bit *i* = basic *i* in the **palindromic order** — before, meets,
  overlaps, starts, during, finishes, **equals**, finished-by, contains,
  started-by, overlapped-by, met-by, after. Each basic's converse sits at the
  mirrored position, so `converse(mask)` — the involution with
  `Allen(a, b, m) ≡ Allen(b, a, converse(m))` — is the 13-bit reversal: one
  `rbit` plus a shift, scalar or vector. The bits are laid out as the
  algebra's symmetry.
- **Named constants, not sugar** (they are values of the algebra): the 13
  singletons under Allen's names, plus the workload composites — `INTERSECTS`
  (9 bits: the point-sets share a point; under half-open intervals *meets*
  shares none), `COVERS` (equals ∪ contains ∪ started-by ∪ finished-by),
  `COVERED_BY` (its converse), `DISJOINT` (before ∪ meets ∪ met-by ∪ after,
  `INTERSECTS`' complement).
- **Vacuity is typed out**: validation rejects the empty mask ("never" —
  write no query) and the full mask ("always" — write no condition) with
  distinct errors; a mask *param* gets the same two rejections at bind, where
  the value exists.
- **The mask is paramable**: `MaskTerm::Param` makes the temporal relation a
  bind-time argument (`Value::AllenMask` / `BindValue::AllenMask`) — one
  prepared query answers any of the 2¹³ − 2 questions per execution.
- **Interval `Eq`/`Ne` are derived facts**: normalization canonicalizes them
  to `Allen(EQUALS)` / `Allen(¬EQUALS)`, so exactly one interval-pair form
  reaches the planner. (Bindings are untouched: an interval term in an
  interval field position is value equality and still probes as a selection.)
- **Point membership is untouched**: Allen is a pair-of-intervals algebra;
  the membership typing rule above is a different judgment.

**The three-confinement disjunction law** (the set's organizing rule):
OR is never an execution node — disjunction is
data in exactly three confinements. *Inside a condition*: an Allen mask is a
disjunction of basics, evaluated as one classify-and-test. *Inside a
position*: a `ParamSet` is a disjunction of values, evaluated as one probe
set. *At the top*: rules (the query shape above) are a disjunction of
conjunctive queries, evaluated as a set union. The tangled middle — a
cross-atom OR inside one rule — is refused representation downstream; DNF
lowering (§ the input condition grammar, below) recovers it as rules at
the validation boundary.

Constraint-side unification (no semantics change): the pointwise key
judgment's meaning — per-group pairwise disjointness — is the statement
"every pair satisfies `DISJOINT`" (`30-dependencies.md`); the checker's
neighbor probe is its O(log n) enforcement plan. One vocabulary, both sides
of the engine.

## The measure (normative — the denotation's one arithmetic)

`Duration(t)` over an interval-typed rule variable is the measure of its
point set, `|[s, e)| = e − s`, type u64 (`10-data-model.md`: the one
arithmetic the denotation defines; everything else that looks like interval
arithmetic is endpoint math and stays refused). **Legal positions,
exhaustively:** a find term (`FindTerm::Duration` — a group-key position
under aggregation, exactly like a plain variable find); the aggregated
input of `Sum`/`Min`/`Max` (`FindTerm::AggregateDuration` — `Sum` in the
wide accumulator with the single finalize range check, like every Sum); and
one side of an **order comparison** (`Lt`/`Le`/`Gt`/`Ge`) against a
u64-typed term or literal — "meetings longer than an hour". Every other
position is a typed validation rejection: a binding position (the measure
is a computation, not a bindable value), a non-order operator, both sides
of one comparison, a non-interval variable, and any fold but the three.

- **Exactness, recorded:** the engine evaluates the measure as one
  subtraction over the encoded column words, exact for both element types —
  the encodings are unit-spaced order-preserving maps onto u64 words (u64
  the identity, I64 the +2⁶³ bias, which cancels in the difference), and
  the constructor invariant `end > start` keeps the difference positive and
  below 2⁶⁴. No overflow, no decode.
- **The ray error:** a ray has no finite measure (`10-data-model.md`, the
  point-domain law), and boundedness is not provable at validation, so the
  subtraction path tests `end == MAX` and raises the typed execution error
  `MeasureOfRay`, carrying the offending interval's two encoded words —
  **the engine's one runtime type error**. Hosts exclude rays first: an
  `Allen(DISJOINT)` guard against the ray probe `[MAX−1, MAX)` (an interval
  intersects it iff its end is MAX — exactly the rays), or a bounded-end
  filter (`Allen(COVERED_BY)` a bounded window) on the measured atom.
- **The filter-order law:** a measure comparison lowered to an atom's
  filter list evaluates only on facts surviving the atom's *other* filters
  — a same-atom guard always runs before the subtraction, so a guarded
  fact never reaches it. Cross-atom measure comparisons are residuals
  (evaluated where whole-value residuals attach), and the measure in finds
  and folds evaluates at emit — after every condition — so guards protect
  those positions unconditionally.
- **Lowering:** normalization lowers the measure to a two-slot read +
  subtraction feeding the existing word machinery — a constant or same-atom
  comparison becomes an occurrence filter, a cross-atom comparison a
  measure residual, and the sink positions a derived word in the sink's row
  representation. At sink construction, symbolic measure finds parse into a
  measure-free execution vocabulary; no sink consumer re-checks whether that
  lowering happened. The one new executor shape is the fused gather+subtract
  scan (dense case NEON per the port-topology law — subtraction is not
  flag-bound; strided/gathered shapes stay scalar until measured, per the
  standing rule).
- **Selectivity:** a measure comparison is a range condition over the
  derived duration word; the existing range keep-fraction floor applies
  unmodified.
- The measure position weakens no proof silently: rule-disjointness treats
  a measure head position as non-witnessing
  (`end − start` is a non-injective map of its variable, so distinct
  bindings may project equal head rows).

**Params:** a param's type is inferred from its anchors — the fields it binds and the
typed terms it compares against. `ir::Value` stays owned by decision: IR literals are
long-lived query data; only the bind surface borrows (`BindValue<'a>` carries str/bytes
payloads by reference — `70-api.md`). An *unanchored* param is unwritable by construction
(every param position is itself an anchor; param-only comparisons are already constant
comparisons) — the roster discharges that item by representation. Conflicting anchors
and non-dense param ids (a gap would be a positional slot whose supplied value is never
type-checked) are validation errors. At execution bind time the supplied values are
checked for count and structural type; String params (and literals) resolve to
intern ids by read-only dictionary lookup **per execution** — never an insert, never an
error, and a value interned by a later write is picked up on the next execution (no
stale-resolution trap; the lookup is one LMDB get, allocation-free). A `bytes<N>`
param or literal is self-encoding — its padded column words, zero dictionary
traffic, no miss to speak of. **Miss semantics
are per operator:** a missed value resolves to the never-minted sentinel intern id
(u64::MAX — the mint path asserts it is never issued), so an `Eq` use matches nothing
(and may short-circuit the query to empty, the only case where that is sound) while an
`Ne` use matches every stored value — the complement, per the semantics section above.

**Param sets (`ParamSet`):** a param id used as a set — bound at execution to a slice
of values of the anchored type. Semantics: the term denotes *any element* — a
binding position matches iff the field value is **in** the set; membership bindings
accept point sets (t ∈ set, t ∈ interval — any element satisfying both). Legal in
atom bindings (positive and negated) and as one side of `Eq`; **illegal under every
other operator** — `Ne(x, set)` reads as ambiguous quantification, and "not in set"
is a negated atom or the host's complement, written explicitly. A ParamId is scalar
or set, never both (validation); the empty set is legal and matches nothing
(short-circuit rules as for `Eq` misses); duplicate elements are deduplicated at
bind (sets are sets). Intern-miss semantics apply per element. This is the `IN` of
the surveyed workloads (the second-most-used operator in both — 150 and 3 sites),
admitted as a term because the alternative is N point queries per batch fetch.

## The input condition grammar and DNF lowering (owned here; runs inside validation)

The rule's condition list admits trees: `ConditionTree = Leaf(Comparison) |
And(Vec) | Or(Vec)`, the list itself conjunctive — the one place the surface
accepts a nested OR. The engine never sees it: **DNF of a query is a set of
rules**, so validation distributes every rule's trees to disjunctive normal
form and **each disjunct becomes a rule** — atoms and finds cloned, the
rule's conditions that disjunct's leaves — before any per-rule check runs.
This is the outer-join precedent applied to disjunction: a documented
decomposition, never a node. The refusal it recovers (README refusals, "OR
tangled mid-rule across atoms"): a cross-atom disjunction poisons filter
pushdown and selectivity as an *execution* concept, so it is refused
representation downstream and recovered as rules at the boundary — **OR is
data or it is nothing.** Negated atoms and membership stay leaf-level; atoms
disjoin by writing rules, which is what rules are for.

- **The cap:** the distributed program validates under the ordinary roster,
  `MAX_RULES` included. The blowup is judged on the *structural* term count,
  before a single disjunct materializes; past the cap it is the typed
  `DnfExceedsRules { produced, cap }` — the exponential case is rejected at
  declaration, exactly like guard-width overflow. (A program *written* with
  more than `MAX_RULES` rules is still `TooManyRules`, judged first.)
- **The nesting cap:** trees deeper than `MAX_CONDITION_DEPTH` (64) are the
  typed `ConditionNestingTooDeep`, judged **iteratively** (an explicit work
  list) before the count or the distribution runs — those walks recurse by
  depth, so an unguarded hostile depth would be a stack exhaustion, not an
  error (the trust-boundary law, § validation boundary). The cap is generous:
  a meaningful tree's depth is bounded by its leaf count, and the blowup cap
  already limits leaves.
- **Duplicate rules after distribution collapse** — set semantics at the
  representation level, the duplicate-statement machinery's sibling:
  identical normalized bodies (finds, atoms, negated verbatim; condition
  lists as sets — conjunction is idempotent and commutative) keep their
  first occurrence.
- **The empty combinations keep their algebraic readings**: `And([])` is
  true (no leaves), `Or([])` is false — its rule lowers to zero rules,
  accepted exactly as statically contradictory conditions are (the semantics
  are exact); a program whose *every* rule vanishes is the empty union,
  rejected as the empty rule set.
- **The validated artifact contains no `Or`** — grep-provable: everything
  downstream of validation carries flat comparison lists (`LoweredRule`),
  and the planner and executor never learn disjunction existed. Rule
  indices in diagnostics and in the witness are lowered-rule indices.

## Normalization (owned here; runs inside validation)

Normalization runs **per rule** — a rule lowers exactly as the conjunctive
query did, and the normalized artifact is a **list**, one entry per rule,
because the query is a program. The paper's formalism (§2) assumes atoms
with all-distinct variables, no self-joins
(renamed apart), and selections pushed to base tables. The IR deliberately permits all
three; **normalization lowers IR form to paper form**:

1. Atom occurrences are numbered — self-joins are supported; everything downstream
   (plan validity, trie schemas) quantifies over **atom occurrences**, never relation
   names.
2. A repeated variable within one atom lowers to a per-atom same-fact equality filter;
   literal, param, and param-set bindings lower to per-atom filters; membership
   bindings lower to per-atom range filters over the interval field's two encoded
   words.
3. Same-atom var-vs-var comparisons lower to per-atom field-vs-field filters:
   membership and point containment as word compositions over start/end, and
   `Allen` as the mask-carrying shape (two interval fields + mask —
   classify-then-test; a comparison written constant-first keeps the field on
   the left and converses the mask, so no operand-order flag exists).
4. **Negated atoms** are numbered as occurrences but join no plan node: each lowers
   to an **anti-probe filter** attached to the earliest point where all its
   variables are bound (`40-execution.md`), exactly as residual comparisons attach.
5. Output: distinct-variable positive atoms + per-atom filter lists + a **residual
   list** (cross-atom comparisons — scalar whole-value, decomposed
   point-containment words, and `Allen` residuals carried whole as four
   endpoint slots + mask — and anti-probe filters; nothing single-atom
   survives to the residual list).
6. **The statically-empty fold** (last, per rule — the comptime-unreachable
   analog): per (participating occurrence, slot), a conjunction of constant
   order filters on one u64/i64 slot folds into a single `[lo, hi]` summary
   over **encoded words** (the sign-flip I64 encoding gives both integer
   types one unsigned comparison domain), and the summary replaces its
   constituents — emitted back as at most two order filters + one Eq per
   slot, existing filter shapes, no new kind, no new kernel. Contradictions
   among constants — an empty summary; `Eq` to two distinct constants on
   one slot; an `Eq` constant outside the summary; a membership set empty
   after sentinel-trim, or refuting an `Eq` constant; an `Allen`
   literal-vs-literal condition `classify` refutes; a failed
   constant-point-in-constant-interval membership — are a **statically
   empty verdict for the rule**: the rule is marked dead carrying the
   rendered killing condition (EXPLAIN prints it), a dead rule inside a
   live program is deleted at prepare and never runs, and a program of
   only dead rules prepares to the `Empty` plan (`40-execution.md`,
   § access paths). `Ne` and param-bearing conditions never fold (params
   are stage-3; `Ne` prunes nothing statically); interval variables fold
   via their two slot summaries independently — no cross-slot reasoning in
   v0 (the constructor invariant `start < end` is data, not plan
   knowledge); a negated occurrence's contradiction is no verdict (its
   anti-probe just never rejects). Estimator note: a folded summary is ONE
   range condition — its keep fraction applies once per slot, never per
   constituent (`plan/selectivity.rs`; the fold is also the
   double-counted-range selectivity fix).

**Deviation (paper §2):** the paper assumes selections pre-pushed and per-atom variables
distinct; we accept the richer surface and own the lowering, because there is no
external optimizer to have done it. Execution placement of filters and residuals is
specified in `40-execution.md`. **Reverses if:** never — the paper's assumption is a
WLOG, not a design.

Degenerate shapes, ruled: a rule with no positive atoms is invalid (negated atoms
alone bind nothing); an atom with zero bindings is legal and means a nonemptiness
gate on that relation (Cartesian with the rest, well-defined under the plan
formalism) — a zero-binding *negated* atom is an emptiness gate, equally legal;
every find variable must appear in some positive atom (Datalog safety); variables
appearing only in comparisons are invalid; binding the same FieldId twice in one
atom is invalid (use one variable twice across fields for equality, or a filter).

## Validation boundary (the roster is exhaustive)

Malformed IR is rejected once, at the boundary, yielding a `ValidatedQuery` witness that
everything downstream trusts — no inner layer re-validates.

**The trust-boundary law.** Queries arrive as data — eventually foreign data — so
every panic reachable from an `ir::Query` value is a crash a caller can trigger.
The law, extended from the dyn write surface's ("ETL input is data, not code",
`70-api.md`): **no panic is reachable from IR data** — validation, DNF lowering,
normalization, and prepare return `Ok` or a typed error on *arbitrary* input:
out-of-range ids, duplicate bindings, vacuous masks, MAX-point literals,
cap-exceeders, hostile nesting. The caps (`MAX_RULES`, the DNF blowup cap,
`MAX_CONDITION_DEPTH`, `MAX_OCCURRENCES`, the 128-variable cap) are **boundary
guards**, not planner hygiene — the nesting cap in particular exists because the
tree walks recurse by depth, and its own judge is iterative so the guard is total.
Enforced mechanically: the adversarial sweep (a property test in the engine's
integration suite) drives 10⁴+ structurally random malformed queries through
validate → normalize → prepare and reddens on any panic; `unreachable!` arms
*downstream* of validation are exempt — they are guarded by it, and the sweep's
job is proving the guard total.

The program shape first, each with a distinct typed error: an **empty rule
set** (the empty union is no query); more than **`MAX_RULES` (16) rules**
(the roster cap, documented at the definition and counted independently of
the per-rule occurrence cap — rules are planned one at a time, so the
program's breadth is bounded here and each rule's width there); and **head
misalignment** — a rule whose find-term count differs from the head's arity,
whose term shape (variable vs aggregate-op kind) differs at a position, or
whose resolved positional type differs from the pinned row (rule 0's
resolved input types pin the head's positional row; every later rule must
agree position by position — that alignment is *how* every rule derives
the one predicate, whose signature the witness then seals from rule 0). Between the program shape and
the per-rule roster, the **nesting boundary guard** (trees deeper than
`MAX_CONDITION_DEPTH` are the typed `ConditionNestingTooDeep`, judged
iteratively before any recursive walk — the trust-boundary law above), then
**DNF distribution** (§ the input condition grammar):
the blowup past `MAX_RULES` is the typed `DnfExceedsRules { produced, cap }`
on the structural term count, duplicates collapse, and a program whose every
disjunction is empty is the empty union. Rules then validate **one at a
time** under the per-rule roster below — a rule validates exactly as a
conjunctive query did, with its own bivalent-anchor typing fixpoint — and
every rule-local diagnostic names a position inside the first failing
**lowered** rule.
Params, being query-global, unify after the rules' own fixpoints: type,
scalar-vs-set role, and value-vs-mask role must agree across rules, and id
density is judged jointly across the whole program.

Per-rule rejections: unknown
relation/field ids; duplicate FieldId in one atom's bindings; variable type conflicts
(structural — membership bindings anchor the *element* type); literal-vs-field and
param-anchor type mismatches (non-UTF-8 String literals and `start ≥ end` interval
literals included); element-typed point literals at the domain ceiling in
membership bindings and `Contains` operands (the point-domain law — point params
get the same rejection at bind, where the value exists); comparisons violating the
type rules above (order operators on intervals and on `bytes<N>` each named in
their own diagnostic — the predictable mistake gets the good error); the Allen vacuity rules (the ∅
and full literal masks, distinct typed errors; mask params get the same two at
bind); constant comparisons;
self-comparisons; a ParamId used both scalar and set, or a ParamSet under any
operator but `Eq`; a mask param with any value anchor (a mask is not a
data-model type); non-dense param ids — dense across value and mask params
jointly; point variables bound only by membership;
negated-atom variables not bound by any positive atom; unbound find variables;
comparison-only variables; empty finds; duplicate find terms; no positive atoms;
aggregate input-type violations; aggregate-over-group-key; mixed Arg and fold
aggregates, Arg terms with differing keys or directions, or a non-orderable Arg
key; the `Pack` roster (a second `Pack` term, `Pack` beside a fold or an Arg
term, `Pack` over a non-interval variable — each its own typed error); the
measure's position roster (§ the measure — a `Duration` in a binding,
over a non-interval variable, under a non-order operator, on both sides of one
comparison, or folded by anything but `Sum`/`Min`/`Max`, each with its own
typed error); and the planner caps (more atom occurrences than the DP accepts — negated
occurrences counted, they consume plan-time work — more than 128 distinct
variables) — enforced here so downstream id widths and bitset sizes are true
invariants.

## The renderer — `ir::render`, the read-side syntax

The statement renderer's sibling (`schema/render.rs`): `ir::render::render` prints
a query in the **rule notation** — one clause per rule, set-builder shaped,
`;`-terminated —

```text
(v0, v1) | Busy(person: v0, during: v1), Allen(v1, INTERSECTS, ?0);
(v0, v1) | Ooo(person: v0, during: v1), Allen(v1, INTERSECTS, ?0);
```

— the schema grammar's own query side, promoted: atoms as statements write them,
in-atom selections `field == literal` (params admitted as `?N`), `!` negation,
membership as `in`, `Allen(term, MASK, term)` with masks as named basics joined by
`|` or the workload composites, clause-level `|` reading *such that*. (The
notation's normative grammar block is § the query notation, below; the renderer
emits it.) When the write-side surface is data, the renderer **is** the pretty
syntax — ergonomics on the side that costs nothing and crosses every boundary.

**Handles print as handles.** A literal word at a closed-reference position — a
binding on a field whose declared containment targets a closed relation's id, or
the closed relation's own id field (the table is one schema walk at renderer
construction, the manifest's own inference) — renders as its **handle** (`kind ==
DirectPass`), the vocabulary's name resolved through the sealed extension; an
out-of-range word renders visibly wrong as `Kind(7?)` (the relation's name — the
engine never learns host newtype names), because rendering hides nothing. The
statement renderer (`schema/render.rs`) prints selection literals through the same
convention, and EXPLAIN's fold lines print surviving sets as handle sets
(`{DirectPass, JudgedPass}`) — one vocabulary of names on every surface a row id
reaches. Comparison terms carry no field position, so a literal there renders by
value; the selection form is the handle's home.

Deterministic, golden-pinned (the calendar union query, the Pack/Duration heads,
and the closed-reference handles, byte-exact), and **total on plain data**:
variables render as `v{id}` and params as
`?{id}` (ids are all the IR carries), unresolvable ids as `relation#N`/`field#N`
placeholders, and a nested condition tree functionally (`and(..)`/`or(..)`, depth-
budgeted at `MAX_CONDITION_DEPTH`) — malformed queries must render, because the
renderer's consumers are diagnostics: roster errors print the offending query
(`Db::render_query` — prepare rejected it, so no prepared handle exists), EXPLAIN's
report opens with the query it explains (`PreparedQuery::rendered_query` is the
same string), and the oracle's arbitration bundles carry the notation beside the
raw IR. Rendering allocates; it runs on no warm path.

## The query notation (normative — the render grammar)

**The notation is derived, not designed.** The schema grammar already contains a
query notation: the two judgments are parameterized by single-atom queries written
`R(X | φ)`, and `|` already reads *such that* — set-builder's own bar. The query
notation is that form **promoted**: multi-atom, given a head, terminated by the
statement's `;`. Borrowed grammar is refused (owner ruling 2026-07-10; the refusals
ledger): Datalog's `head :- body` was considered and rejected — this engine's
statements are already statements *about* queries, so the query surface must be the
statement surface's query side, not an import. One notational family, schema to
query.

```text
query   := clause+                     // two or more clauses denote set union
clause  := '(' head ')' '|' body ';'
head    := headterm (',' headterm)*
headterm:= var | [name ':'] agg        // named positions become result columns
agg     := Sum(t) | Min(t) | Max(t) | Count | CountDistinct(v) | Pack(v)
           where t := v | Duration(v)
body    := item (',' item)*
item    := atom                        // positive occurrence
         | '!' atom                    // negation (anti-probe; safety per roster)
         | term 'in' term              // membership: point ∈ interval, value ∈ ?set
         | Allen '(' term ',' mask ',' term ')'
         | term cmp term               // ==  !=  <  <=  >  >=
atom    := Relation '(' binding (',' binding)* ')'
binding := field                       // punning: binds a var named after the field
         | field ':' var               // explicit variable — the join spelling
         | field '==' value            // selection, schema-grammar-verbatim
mask    := MASK ('|' MASK)* | ?param   // masks are sets of basics; '|' is set union
term    := var | ?param | literal
```

Every token is either the schema grammar's own or Rust's: atoms are `Relation(...)`
as statements write them; in-atom selections are the schema's selections with
params admitted (and a set-bound param is the binding's membership spelling, `field
in ?N`); membership is the Rust keyword `in` (∈ is not a lexable token); negation
is `!`; params keep `?`; `;` terminates clauses as it terminates statements.
Selection values admit closed-relation **handles** exactly as statement selections
do — bare (`kind == DirectPass`, resolving through the field-named host enum) or
qualified (`arm == ClaimKind::Busy`); the renderer prints handles bare, so the
rendered text is its own fixed point wherever the closed relation is named
`UpperCamel` of its referencing field (the naming convention's dividend — a theory
named otherwise reparses through the qualified spelling). The
two bars are the two the audit already upheld: clause-level `|` is *such that*;
mask-level `|` is set union over the 13 basics — set-builder and set-union,
context-separated exactly as the two levels of `==` are.

**The punning law (B, decided; alternative (A) is in the refusals ledger).** A bare
field name binds a **clause-local variable named after the field** — Rust's
struct-shorthand instinct, used for projection. The same punned name appearing in
two atoms of one clause is a macro error, spanned at the second occurrence
("ambiguous punning — rename explicitly"); joins are always written `field: v` on
both ends. Under the refused alternative — same-name-same-variable across the
clause — every relation naming its key `id` makes a forgotten rename a *silent*
join the roster cannot object to (structurally both u64); under (B) the wrong query
is unwritable at the call site.

**The macro and its placement.** `query!(Theory { ... })` lives in the downstream
crate `crates/bumbledb-query` — the bench-crate quarantine verbatim: hosts may
depend on it, the engine never depends back, and the surface ruling's
no-query-grammar shape holds unamended. The parser is hand-rolled over the token
stream, `syn`-free, on `bumbledb-macros`' precedent. Name checking rides the id
constants (`70-api.md`): proc macros cannot see each other's output, so expansion
emits paths to `Theory::BUSY` / `Theory::BUSY_PERSON` (and mask names to
`AllenMask`'s constants) and ordinary rustc name resolution makes a typo a compile
error at the query literal; variable *type* consistency stays the validation
roster's, the same split the foreign surfaces have. **Constant text only**: the
macro consumes a literal token tree and expansion constructs the `ir::Query` value
— compile-time lowering, no runtime parser; dynamic composition stays on the raw
IR layer, which exists regardless.

**One notation, everywhere — the anti-drift discipline.** `ir::render` emits this
grammar; the cookbook writes its queries in it; any foreign sugar (the anticipated
TS binding) parses it. The renderer is the spec, and every implementation is pinned
by **round-trip goldens**: `render(lower(text))` equals the normalized text,
byte-exactly (`bumbledb-query/tests/notation.rs`). Three consumers, one grammar,
zero drift by construction.

## Prepared queries

A `ValidatedQuery` is planned once into a `PreparedQuery` — the reusable object the
zero-allocation contract is written against (`40-execution.md`). The plan
pipeline (statistics → DP → lowering → plan validation) runs **per rule**:
the prepared query holds one validated plan per rule and **one** sink
configuration, owned by the head — execution is the rule loop driving
every rule's plan into that sink, whose seen-set spanning rules is the
union (`40-execution.md` § the rule loop). **Plans pin the
statistics read at prepare time and are never invalidated by writes**; stale plans are
accepted at this scale, and re-preparation is explicit. The compensating control is
`PreparedQuery::staleness` (`70-api.md`): the pinned per-occurrence row counts survive
on the prepared query as a cold record, and the host — never the engine — can pull the
drift against a snapshot's live `S` counters and decide to re-prepare; no engine-side
threshold, trigger, or background anything exists. (Literal values are part of the
query, hence of the prepared plan; queries meant for reuse across values use Params.)
**Param sets and the pinned plan:** the plan assumes a *small* set (it plans the
set-bound position like a selective equality; the documented assumption is |set| ≤
a few hundred). A host that binds pathological set sizes owns the consequences the
same way it owns stale plans — re-prepare, or restructure the query. **Decision:**
pin-at-prepare, extended to set cardinality. **Alternative:** re-plan on bind-size
drift. **Why it lost:** same reason as stats-driven invalidation — an honest trigger
re-plans constantly and allocates on the hot path. **Reverses if:** the benchmark
shows a stale-plan regression a re-prepare wouldn't have.

## `OPEN` extensions (designed-for, not built)

**Recursion** = an explicit fixpoint construct, semi-naive, if a real need appears —
the surveyed workloads precompute their closures and the modeling discipline blesses
that (`10-data-model.md`). The rules shape is its landing pad, deliberately
not entered: a query is already a non-recursive Datalog program, one step
short of the fixpoint — a rule's head is never a body atom. It arrives as a
new IR node kind; nothing above assumes it never comes.
