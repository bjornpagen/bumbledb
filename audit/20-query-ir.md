# Audit: 20-query-ir.md

Auditor scope: `docs/architecture/20-query-ir.md`, cross-checked against 00/10/30/40/50,
the Free Join paper (`tex/02-background.tex`, `tex/03-free-join.tex`), and the v5 IR
source (`1b65ae8^:crates/bumbledb-core/src/query_ir.rs`). Findings ordered most-severe
first.

## 1. **Aggregation is ambiguous over which variables form the folded binding set — and both natural readings break the ledger thesis** [blocker]

The doc says "The logical solution of a query is a **set of variable bindings**",
"Existential variables never multiply output", and "each group folds the set of
satisfying bindings" — but never says which variables the folded binding set ranges
over. Reading A (bindings over *all* query variables, including existentials): an
existential that takes 3 values for a group multiplies Count and Sum by 3, contradicting
"existential variables never multiply output". Reading B (bindings projected to find
variables only): `Sum(amount)` grouped by account collapses two distinct postings with
equal amounts into one binding — a ledger balance of {100, 100} sums to 100, failing
00-product's "balance-style aggregates" success family and the SQLite oracle (SQL `SUM`
is bag-semantic). The obvious workaround — bind the posting's serial id — either lands
it in `finds` (group key becomes per-posting, no aggregation) or leaves it existential
(collapsed again under Reading B). 30-execution's D2 subtree-skip ("can only multiply
witnesses of an already-emitted projection") is only sound for plain projection under
Reading B and is wrong for aggregate sinks under Reading A, so the two docs jointly
force this ambiguity to a head.
Question: Over exactly which variable set is the binding set formed per group, and how
does `Sum(amount) by account` produce 200 from two distinct postings of amount 100
without bag semantics?

## 2. **`Literal(Value)` — `Value` is never defined; this recreates the exact conditions of the v5 Bytes-literal hole** [blocker]

The IR sketch says `Term = Var(VarId) | Param(ParamId) | Literal(Value)` and no doc
defines the `Value` enum. v5's IR (`query_ir.rs` at `1b65ae8^`) had
`Literal { Bool, Integer(i128), String }` — no `Bytes`, and nobody noticed until
post-mortem. Walking 10-data-model's roster: **Bytes** — unstated (the historical hole).
**Enum** — is a literal written by variant name or ordinal, and is an unknown variant a
validation error? Unstated. **Serial** — a point lookup by `AccountId` (a headline
workload item in 00-product) needs a serial literal or param, but a bare `u64` literal
cannot carry the nominal type that the doc's own validation rule ("`AccountId` never
unifies with `InstrumentId`") requires; where does the nominal type ride on a literal?
**U64 vs I64** — distinct exact-typed variants, or a v5-style universal `Integer(i128)`
with range checks? (Post-mortem 35 — silent capacity truncation — says exact-typed with
hard range errors, but the doc doesn't say it.) **String** — raw string presumably, but
see finding 4.
Question: What is the exact variant list of `Value`, one per 10-data-model type, and
what carries a Serial literal's nominal type?

## 3. **"Sum uses checked overflow → error" specifies a nondeterministic semantics** [blocker]

A fold over a *set* has no defined order, and under Free Join the enumeration order
varies with plan and hash iteration. With an i64 accumulator and per-step checked
addition, summing {i64::MAX, 1, -2} errors or succeeds depending on visit order — the
same query on the same data is nondeterministically an error, which breaks 50-validation's
"exact result-set equality with SQLite... always" and makes plan changes
observable in results. The deterministic alternative — accumulate in a wider integer
(i128/u128) and range-check the *final* value against the result type — is one sentence,
but it is a different decision (and i128 is not in the value roster, so the result type
question is real). Also unstated: which value types `Sum` accepts at all (Sum over
Bool/Enum/String/Serial must be a validation error, and the validation roster doesn't
list aggregate type restrictions).
Question: Is Sum defined as the mathematical-integer sum with a final range check
(deterministic), or per-step checked ops (order-dependent) — and what is Sum's result
type for I64 and U64 inputs?

## 4. **String/Bytes literal intern resolution timing collides with "statistics changes invalidate plans, not validation"** [design-gap]

Facts store intern ids (10-data-model), so a String/Bytes literal must be resolved
through the dictionary at some point. If resolution happens at validation time and the
literal is not yet interned, the query is provably empty — but a later insert interns
that string, and the doc's own rule "statistics changes invalidate plans, **not
validation**" means the stale resolved-to-empty literal is never re-resolved: a prepared
query silently returns wrong (empty) results forever after the data changes. If
resolution happens per-execution instead, it must fit the zero-allocation contract.
Neither choice is written down, and the "miss = validation error vs miss = empty result"
question is also open.
Question: When is a String/Bytes literal resolved to an intern id, what happens on a
dictionary miss, and how does a prepared query observe a literal that gets interned
after preparation?

## 5. **Comparison type rules are undefined: no orderable-type roster, no mixed-type rule, and Eq/Ne conflated with order ops** [design-gap]

Validation rejects "comparisons over non-orderable types", but no doc defines which
types are orderable. 10-data-model only rules out String/Bytes ordering ("string
ordering... not supported"); Bool, Enum (ordered by declaration order?), and Serial
(monotonic but nominal — is `Lt` over `AccountId` meaningful?) are all unspecified.
Read literally, the rule also bans `Eq`/`Ne` over String — but string *equality* is
explicitly cheap and must be legal, so the roster needs an equality-types /
orderable-types split (`Eq|Ne` vs `Lt|Le|Gt|Ge`). Mixed-width comparisons (U64 field vs
I64 literal) are neither permitted nor rejected anywhere — silent coercion here is
exactly the post-mortem-35 failure class. Relatedly, `Comparison { op: Eq, lhs: x, rhs: y }`
is unification by another name: do predicates obey the identical type rules as
repeated-variable atoms (including the cross-serial ban)?
Question: For each of Bool/U64/I64/Enum/String/Bytes/Serial, which of Eq/Ne and
Lt/Le/Gt/Ge are legal, and is any cross-type comparison ever legal?

## 6. **Param typing has no source: no declarations, no inference rule, no bind-time check specified** [design-gap]

`Param(ParamId)` is a bare id. v5 had `TypedInput { id, name, value_type }`; the new
sketch drops the declaration list without saying what replaces it. Presumably a param's
type is inferred from the field positions it binds — but that is never stated, and a
param appearing *only* in a `Comparison` against another param or a literal has no field
anchor at all. Also unstated: whether one ParamId used at two conflicting types is a
validation error (the roster says "variable type conflicts", not param conflicts), when
supplied param *values* are checked (execution-time bind step?), what happens on
missing/extra params, and how a caller supplies a nominally-typed Serial param such that
`AccountId`-vs-`InstrumentId` confusion is caught.
Question: Where do param types come from, and at which boundary (validate vs execute)
are param count, types, and serial nominality checked?

## 7. **Nobody owns the normalization from IR form to the paper's CQ form (self-join renaming, repeated variables, selection pushdown)** [design-gap]

The paper's formalism assumes: no self-joins ("if two atoms have the same relation name,
then we simply rename one of them"), "all variables in the atom $R_i(\bm x_i)$ are
distinct", and "selections are pushed down to the base tables". The IR deliberately
permits all three violations: repeated variables in one atom ("legal and means same-fact
equality"), literal/param bindings inside atoms (selections), and — required by
50-validation's multi-hop `OrgParent` family — two atoms over the same relation. Plan
validity ("no two subatoms share a relation") and trie construction only make sense
after atom occurrences are renamed apart, and repeated-variable/literal bindings must be
desugared into pushed-down filters (40-storage's image-build filter pushdown is the
mechanism, but nothing connects it to this desugaring). No doc names the normalization
pass, its output invariants, or which layer (validation? planner?) owns it — this is
where the paper's WLOG assumptions become somebody's silent code decision, the exact
failure mode README rule 3 warns about.
Question: Which component normalizes IR atoms into paper-form CQ (distinct-variable,
renamed, selection-free atoms), and which doc specifies its invariants?

## 8. **The "entries touching an account set" benchmark family is inexpressible: no IN, no disjunction, no set-valued params** [design-gap]

50-validation lists "entries touching an account set" as a ledger query family, but the
IR is purely conjunctive: atoms plus single-value comparisons. A membership-in-set
predicate needs disjunction, an `In` comparison op, a set-valued Param, or a
host-side "run one query per account and union" convention — the doc chooses none and
records no decision (README rule 1: decision + strongest alternative + why it lost).
Host-side union is workable under set semantics but must be stated, because it changes
what the benchmark measures and what the SQLite oracle compares against. The absence of
any decision block for the predicate sublanguage's boundaries (no arithmetic, no IN, no
OR) is a gap in its own right.
Question: How is "postings for a set of accounts" expressed — new IR construct, or a
documented host-side union convention — and where is that decision recorded?

## 9. **Global aggregates: legality unstated, and the empty-group rule diverges from the SQLite oracle** [design-gap]

"Group key = the non-aggregated output fields" is well-formed when `finds` contains only
aggregates (empty group key, one global group), but the doc never says whether that
query is legal — "empty `finds`" is a validation error and all-aggregate `finds` is
adjacent enough that an implementer will guess. If legal, "Empty groups do not appear"
means a global `Count` over an empty relation returns the *empty set*, while SQLite's
`SELECT COUNT(*)` returns one row containing 0 — so 50-validation's "exact result-set
equality" needs a documented oracle mapping for this case, and the application-visible
rule "balance of an account with no postings is an absent row, not 0" deserves explicit
text since every ledger caller will hit it.
Question: Is an all-aggregate `finds` legal, and is "global aggregate over empty input
yields the empty set (not 0)" the intended committed semantics against the oracle?

## 10. **Prepared-query invalidation is underspecified and, as written, invalidates every plan on every write** [design-gap]

"Statistics changes invalidate plans" combined with 30-execution's "per-relation row
counts (maintained on write, exact)" means every committed write changes statistics and
therefore invalidates every cached plan — replanning per write defeats the plan cache
and reintroduces planner allocations adjacent to the zero-steady-state-allocation hot
path (is a replan "steady state"?). Presumably a threshold or generation policy is
intended, but none is written. Also unstated: what "plan caching keys on the IR
structure" includes — if literal *values* are in the key, two point lookups differing
only in the literal never share a plan (push users to Params — say so); if they are not,
literal selectivity can't inform the plan.
Question: What is the actual invalidation trigger (every stats change? a drift
threshold?), when does replanning run relative to execution, and are literal values part
of the plan-cache key?

## 11. **The validation roster is incomplete, and the sketch makes at least one illegal state representable that the roster doesn't catch** [design-gap]

The doc's list — "unknown relation/field ids, variable type conflicts..., comparisons
over non-orderable types, aggregates over unbound variables, empty `finds`" — reads as
exhaustive ("parse, don't validate" makes this roster the de-facto definition of
`ValidatedQuery`), but it is missing at least: an unbound *find* variable (a `finds` var
appearing in no atom — the Datalog safety condition; only *aggregate*-over-unbound is
listed); variables appearing only in comparisons; duplicate `FieldId` within one atom's
`bindings` (the `Vec<(FieldId, Term)>` representation happily encodes binding one field
twice — note the doc boasts the wildcard illegal state "cannot be written" while
introducing this representable one); aggregate type restrictions (finding 3); unknown
enum variants; literal-vs-field type mismatches (literals are not variables, so
"variable type conflicts" doesn't obviously cover them); and constant comparisons
(literal-vs-literal, param-vs-param). Each omission is a place v6 code will decide
silently.
Question: Is the roster intended to be exhaustive, and what is the ruling on duplicate
FieldId in `bindings` (error, or equality constraint between the two terms) and on
unbound find variables?

## 12. **Count's `over: VarId` is semantically inert per the doc's own text, and Count-of-bindings is otherwise inexpressible** [clarification]

The sketch requires `Aggregate { op: Count, over: VarId }`, but the semantics section
says "Count counts the group's binding set, exactly" — under that definition the `over`
variable does not affect the result, so two Counts differing only in `over` are the same
aggregate written two ways (an illegal-state-representable smell), while under a
Count-distinct-`over` reading the choice of variable changes the answer. This is the
same undecided scope as finding 1, surfaced in the type shape: either `over` is
meaningful (define it) or Count should not carry it. There is also no way to write
"count the group's bindings" without picking an arbitrary variable.
Question: Does `Count { over: x }` mean |group's binding set| (make `over` optional/
absent) or |distinct values of x in the group| (then say so and define Sum consistently)?

## 13. **Interaction of aggregated variables with the group key is undefined** [clarification]

Nothing forbids `finds = [Var(x), Aggregate { op: Sum, over: x }]` (the aggregated
variable is also a group key) or `finds = [Var(x), Var(x)]` (duplicate find). The first
is well-defined but degenerate under "group key = non-aggregated output fields" (every
group's Sum(x) folds a singleton {x}); the second produces a projected fact with two
identical positions. Both should be either explicitly legal-and-defined or in the
validation roster; silence invites divergent guesses between the engine, the reference
engine, and the SQLite oracle translation.
Question: Are duplicate find variables and aggregates over group-key variables legal,
and with what results?

## 14. **Degenerate query shapes are undefined: no atoms, zero-binding atoms, comparison-only constraints** [clarification]

The empty-query cases are never pinned down: a query with `atoms: []` (only literals/
params in comparisons — presumably an error since every find var is unbound, but the
roster doesn't say); an atom with `bindings: []` (a pure nonemptiness gate on a relation
— meaningful under the semantics, Cartesian with the rest, and the paper's plan
partitioning handles empty variable sets, but is it *intended*?); and a satisfiable/
unsatisfiable constant comparison gating the whole query. The reference engine
(50-validation) must implement exactly the same answers, so these need one ruling each.
Question: Which of {no-atom queries, zero-binding atoms, variable-free comparisons} are
valid, and what do the valid ones mean?
