# 20 — Query IR

## Decision: no text query language; the IR is pure data

Queries are **plain Rust data structures** — serializable in principle (plain owned
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

## Semantics

- The logical solution of a query is the **set of distinct bindings of all query
  variables** that satisfy every positive atom, every membership binding, every
  predicate, and **no negated atom** (below); projection returns the **set** of
  projected facts.
- **Existential variables never multiply projection output.** (Scoped to projection —
  see aggregation below.)
- Distinctness is the default and only behavior; there is no DISTINCT concept.
- No ordering or limit in the IR: results are sets; the host sorts (`70-api.md`).

### Negation (normative)

A query carries a list of **negated atoms**. A binding satisfies a negated atom iff
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
  over all query variables.** Group key = the values of the non-aggregated find
  variables. Two postings of amount 100 to one account are two distinct bindings (their
  serial ids differ): `Sum(amount) by account` = 200.
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
  the answer is a set; with serial keys ties cannot occur. Validation: all Arg terms
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
    finds:      Vec<FindTerm>,        // ≥1; duplicates rejected
    atoms:      Vec<Atom>,            // ≥1; conjunctive, positive
    negated:    Vec<Atom>,            // anti-join atoms (safety rule above)
    predicates: Vec<Comparison>,
}
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
FindTerm   = Var(VarId)
           | Aggregate { op: AggOp, over: Option<VarId> }   // over: None for Count
AggOp      = Sum | Min | Max | Count | CountDistinct
           | ArgMax { key: VarId } | ArgMin { key: VarId }  // over = the carried var
Comparison { op: Eq|Ne|Lt|Le|Gt|Ge|Overlaps|Contains, lhs: Term, rhs: Term }
```

Representation notes (the branch-removal decisions): no wildcard variant — an unbound
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
Var, Param, ParamSet, and Literal all participate under the same rule. One
consequence, enforced by validation: a variable bound *only* by membership bindings
has no enumerable domain — every point variable must be bound by at least one
non-membership occurrence (a scalar field binding). Interval-vs-interval overlap
needs no shared point variable: that is the `Overlaps` predicate.

**Comparison rules, complete:** both sides must have the same structural type except
where stated (no U64-vs-I64, no silent coercion). `Eq`/`Ne` are legal for all seven
types; `Lt/Le/Gt/Ge` only for U64/U64 and I64/I64 — **never intervals**
(`10-data-model.md` orderability). `Overlaps` requires two interval terms of one
element type: satisfied iff the point-sets intersect. `Contains` requires an
interval left side and either an interval of the same element type (⊇ of
point-sets) or an element-typed right side (point membership as a predicate — the
predicate form of the binding rule, for terms already bound elsewhere). `Eq` between
two variables is unification and obeys identical type rules. Any comparison without
a variable side (literal-vs-literal, param-vs-literal, param-vs-param) is a
validation error, and so is a variable compared with itself — both are
constant-valued: write the query you mean.

**Params:** a param's type is inferred from its anchors — the fields it binds and the
typed terms it compares against. An *unanchored* param is unwritable by construction
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

## Normalization (owned here; runs inside validation)

The paper's formalism (§2) assumes atoms with all-distinct variables, no self-joins
(renamed apart), and selections pushed to base tables. The IR deliberately permits all
three; **normalization lowers IR form to paper form**:

1. Atom occurrences are numbered — self-joins are supported; everything downstream
   (plan validity, trie schemas) quantifies over **atom occurrences**, never relation
   names.
2. A repeated variable within one atom lowers to a per-atom same-fact equality filter;
   literal, param, and param-set bindings lower to per-atom filters; membership
   bindings lower to per-atom range filters over the interval field's two encoded
   words.
3. Same-atom var-vs-var comparisons lower to per-atom field-vs-field filters
   (membership and `Overlaps`/`Contains` included — all decompose into word
   comparisons over start/end).
4. **Negated atoms** are numbered as occurrences but join no plan node: each lowers
   to an **anti-probe filter** attached to the earliest point where all its
   variables are bound (`40-execution.md`), exactly as residual comparisons attach.
5. Output: distinct-variable positive atoms + per-atom filter lists + a **residual
   list** (cross-atom comparisons and anti-probe filters — exactly those; nothing
   single-atom survives to the residual list).

**Deviation (paper §2):** the paper assumes selections pre-pushed and per-atom variables
distinct; we accept the richer surface and own the lowering, because there is no
external optimizer to have done it. Execution placement of filters and residuals is
specified in `40-execution.md`. **Reverses if:** never — the paper's assumption is a
WLOG, not a design.

Degenerate shapes, ruled: a query with no positive atoms is invalid (negated atoms
alone bind nothing); an atom with zero bindings is legal and means a nonemptiness
gate on that relation (Cartesian with the rest, well-defined under the plan
formalism) — a zero-binding *negated* atom is an emptiness gate, equally legal;
every find variable must appear in some positive atom (Datalog safety); variables
appearing only in comparisons are invalid; binding the same FieldId twice in one
atom is invalid (use one variable twice across fields for equality, or a filter).

## Validation boundary (the roster is exhaustive)

Malformed IR is rejected once, at the boundary, yielding a `ValidatedQuery` witness that
everything downstream trusts — no inner layer re-validates. Rejections: unknown
relation/field ids; duplicate FieldId in one atom's bindings; variable type conflicts
(structural — membership bindings anchor the *element* type); literal-vs-field and
param-anchor type mismatches (non-UTF-8 String literals and `start ≥ end` interval
literals included); enum ordinal out of range for the field's variant list (in
bindings and in comparisons, each precisely diagnosed); comparisons violating the
type rules above (order operators on intervals named in their own diagnostic —
the predictable mistake gets the good error); constant comparisons;
self-comparisons; a ParamId used both scalar and set, or a ParamSet under any
operator but `Eq`; non-dense param ids; point variables bound only by membership;
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
zero-allocation contract is written against (`40-execution.md`). **Plans pin the
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
that (`10-data-model.md`). **`Pack`** = the coalescing aggregate over interval
variables (maximal disjoint intervals per group — Snodgrass's coalesce, Postgres's
`range_agg`); its result is a *set* of intervals per group, which breaks the
one-row-per-group aggregate shape, so it waits for both a real need and a shape
decision. Both arrive as new IR node kinds; nothing above assumes they never come.
