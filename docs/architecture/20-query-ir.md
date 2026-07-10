# 20 — Query IR

## Decision: no text query language; the IR is pure data

Queries are **plain Rust data structures** — encodable in principle (plain owned
data, no borrows, no behavior), inspectable. The IR is the engine's public contract,
and it is also the language dependencies are written in (`30-dependencies.md`): one
representation for "what holds" and "what do you want".

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

**Alternative 2 — typed builder/generics as the contract.** **Why it lost:** owner
ruling — builders bake a Rust calling convention into the contract and translate poorly
to other-language bindings; data structures translate to anything. Builders/macros may
appear later as sugar *producing* the IR. (Host newtypes still give compile-time nominal
safety at the app layer — see `10-data-model.md`.)

## The query shape: one head, a set of rules

A query is a **program**: one head and a non-empty list of conjunctive
**rules** — which is precisely a **non-recursive Datalog program**. The head
owns the find shape (arity, aggregate ops, and the positional type row —
computed at validation and pinned in the witness); each rule is a conjunct
(positive atoms, negated atoms, predicates) whose find terms align against
the head position by position. The single-rule query is the degenerate case
and embeds the conjunctive query unchanged (`Query::single`); every
pre-rules query is a one-rule program.

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
  binding, every predicate, and **no negated atom** (below); projection
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
  the union of the rules' binding sets projected to the head (dedup
  semantics are owned by the rule-execution PRD, ALG-07; the elision
  theorem by ALG-08). Two postings of amount 100 to one account are two distinct bindings (their
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
- `Min`/`Max` accept U64 and I64 only (the orderable types — intervals excluded,
  `10-data-model.md`); result type = input type; deterministic (a set has one
  minimum).
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
    predicates: Vec<PredicateTree>,   // the list is a conjunction; trees are
                                      //   the INPUT grammar — validation
                                      //   distributes them away (below)
}
PredicateTree = Leaf(Comparison)      // the input predicate grammar: any
              | And(Vec<PredicateTree>)  // boolean combination of positive
              | Or(Vec<PredicateTree>)   // comparisons — lowered at validation
HeadTerm   = Var | Aggregate(HeadOp)  // var-free: variables are rule-scoped,
                                      //   so the head names shapes and the
                                      //   rules supply the variables
HeadOp     = Sum | Min | Max | Count | CountDistinct | ArgMax | ArgMin
Atom {
    relation:   RelationId,
    bindings:   Vec<(FieldId, Term)>, // named-field; absence of a field IS the wildcard
}
Term       = Var(VarId) | Param(ParamId) | ParamSet(ParamId) | Literal(Value)
Value      = Bool(bool) | U64(u64) | I64(i64)
           | Enum(u8)                 // declaration-order ordinal, range-checked
           | IntervalU64(u64, u64)    // start < end enforced at the boundary
           | IntervalI64(i64, i64)
           | String(Box<[u8]>)        // raw UTF-8 bytes; interning is the engine's job
           | Bytes(Box<[u8]>)
           | AllenMask(AllenMask)     // the mask value shape — a param payload,
                                      //   never a field type (10-data-model.md)
FindTerm   = Var(VarId)
           | Aggregate { op: AggOp, over: Option<VarId> }   // over: None for Count
AggOp      = Sum | Min | Max | Count | CountDistinct
           | ArgMax { key: VarId } | ArgMin { key: VarId }  // over = the carried var
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
where stated (no U64-vs-I64, no silent coercion). `Eq`/`Ne` are legal for all seven
types; `Lt/Le/Gt/Ge` only for U64/U64 and I64/I64 — **never intervals**
(`10-data-model.md` orderability). `Allen { mask }` requires two interval terms of
one element type — **the** interval-pair comparison (next section). `Contains`
requires an interval left side and an **element-typed** right side (point
membership as a predicate — the predicate form of the binding rule, for terms
already bound elsewhere); its old interval⊇interval form is not an operator —
that predicate is `Allen(COVERS)`. `Eq` between
two variables is unification and obeys identical type rules. Any comparison without
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
  write no query) and the full mask ("always" — write no predicate) with
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
data in exactly three confinements. *Inside a predicate*: an Allen mask is a
disjunction of basics, evaluated as one classify-and-test. *Inside a
position*: a `ParamSet` is a disjunction of values, evaluated as one probe
set. *At the top*: rules (the query shape above) are a disjunction of
conjunctive queries, evaluated as a set union. The tangled middle — a
cross-atom OR inside one rule — is refused representation downstream; DNF
lowering (§ the input predicate grammar, below) recovers it as rules at
the validation boundary.

Constraint-side unification (no semantics change): the pointwise key
judgment's meaning — per-group pairwise disjointness — is the statement
"every pair satisfies `DISJOINT`" (`30-dependencies.md`); the checker's
neighbor probe is its O(log n) enforcement plan. One vocabulary, both sides
of the engine.

**Params:** a param's type is inferred from its anchors — the fields it binds and the
typed terms it compares against. `ir::Value` stays owned by decision: IR literals are
long-lived query data; only the bind surface borrows (`BindValue<'a>` carries str/bytes
payloads by reference — `70-api.md`). An *unanchored* param is unwritable by construction
(every param position is itself an anchor; param-only comparisons are already constant
comparisons) — the roster discharges that item by representation. Conflicting anchors
and non-dense param ids (a gap would be a positional slot whose supplied value is never
type-checked) are validation errors. At execution bind time the supplied values are
checked for count and structural type; String/Bytes params (and literals) resolve to
intern ids by read-only dictionary lookup **per execution** — never an insert, never an
error, and a value interned by a later write is picked up on the next execution (no
stale-resolution trap; the lookup is one LMDB get, allocation-free). **Miss semantics
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

## The input predicate grammar and DNF lowering (owned here; runs inside validation)

The rule's predicate list admits trees: `PredicateTree = Leaf(Comparison) |
And(Vec) | Or(Vec)`, the list itself conjunctive — the one place the surface
accepts a nested OR. The engine never sees it: **DNF of a query is a set of
rules**, so validation distributes every rule's trees to disjunctive normal
form and **each disjunct becomes a rule** — atoms and finds cloned, the
rule's predicates that disjunct's leaves — before any per-rule check runs.
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
- **Duplicate rules after distribution collapse** — set semantics at the
  representation level, the duplicate-statement machinery's sibling:
  identical normalized bodies (finds, atoms, negated verbatim; predicate
  lists as sets — conjunction is idempotent and commutative) keep their
  first occurrence.
- **The empty combinations keep their algebraic readings**: `And([])` is
  true (no leaves), `Or([])` is false — its rule lowers to zero rules,
  accepted exactly as statically contradictory predicates are (the semantics
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

The program shape first, each with a distinct typed error: an **empty rule
set** (the empty union is no query); more than **`MAX_RULES` (16) rules**
(the roster cap, documented at the definition and counted independently of
the per-rule occurrence cap — rules are planned one at a time, so the
program's breadth is bounded here and each rule's width there); and **head
misalignment** — a rule whose find-term count differs from the head's arity,
whose term shape (variable vs aggregate-op kind) differs at a position, or
whose resolved positional type differs from the pinned row (rule 0's
resolved types pin the head's positional type row in the witness; every
later rule must agree position by position). Between the program shape and
the per-rule roster, **DNF distribution** (§ the input predicate grammar):
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
get the same rejection at bind, where the value exists); enum ordinal out of range for the field's variant list (in
bindings and in comparisons, each precisely diagnosed); comparisons violating the
type rules above (order operators on intervals named in their own diagnostic —
the predictable mistake gets the good error); the Allen vacuity rules (the ∅
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
key; and the planner caps (more atom occurrences than the DP accepts — negated
occurrences counted, they consume plan-time work — more than 128 distinct
variables) — enforced here so downstream id widths and bitset sizes are true
invariants.

## Prepared queries

A `ValidatedQuery` is planned once into a `PreparedQuery` — the reusable object the
zero-allocation contract is written against (`40-execution.md`). The plan
pipeline (statistics → DP → lowering → plan validation) runs **per rule**:
the prepared query holds one validated plan per rule and **one** sink
configuration, owned by the head. (Execution of multi-rule programs — the
union loop driving every rule's plan into the one sink — is PRD ALG-07's;
until it lands a 2+-rule execution is the typed `MultiRuleExecution`
refusal, never a wrong answer, while single-rule programs execute in
full.) **Plans pin the
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
short of the fixpoint — a rule's head is never a body atom. **`Pack`** = the coalescing aggregate over interval
variables (maximal disjoint intervals per group — Snodgrass's coalesce, Postgres's
`range_agg`); its result is a *set* of intervals per group, which breaks the
one-row-per-group aggregate shape, so it waits for both a real need and a shape
decision. Both arrive as new IR node kinds; nothing above assumes they never come.
