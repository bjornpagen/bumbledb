# 30 — Dependencies

This chapter owns every invariant the engine enforces on committed states. There are
exactly **two judgment forms**, both statements *about queries*, and nothing else:
no constraint kinds, no modes, no triggers, no deferral. The words *unique key*,
*foreign key*, *primary key*, *check constraint*, *exclusion constraint*, *cascade*,
and *restrict* name nothing here; where one of them used to name something, this
chapter derives that something as an instance and the word is retired.

## The two judgments

Both are parameterized by **single-atom queries** in the ordinary query IR
(`20-query-ir.md`): a relation, a **selection** φ (a set of (field, literal)
equality bindings — any type's literal), and a **projection** X (an ordered field
list). Write such a query `R(X | φ)`; an empty selection is written `R(X)`.
Dependencies and queries share one representation; a dependency is not a new kind of
thing, it is a required property of an old kind of thing.

**Functionality (FD).** `R(X) -> R` asserts that the projection πX is injective on
R: no two distinct facts of R agree on X. X is ordered (the order defines the guard
key, `50-storage.md`), non-empty, duplicate-free. The general form `R(X) -> R(Y)`
with Y ⊊ fields exists in the theory (dependency theory's equality-generating
dependencies); **only the key form — Y = the whole relation — is accepted**: a
relation satisfying a non-key FD is mis-designed (BCNF says X should have been a key
of its own relation), and the engine refuses to be the crutch that makes the
mis-design comfortable. Selections on FDs are likewise rejected — a "conditional
key" is a relation split waiting to happen (`10-data-model.md` modeling discipline).
**Decision.** **Alternative:** accept general/conditional FDs (the machinery is the
same guards). **Why it lost:** every instance surveyed is better said as a schema
shape; accepting them sells normalization back as a runtime feature. **Reverses if:**
a real invariant appears that no relation split can express.

**Containment (IND).** `A(X | φ) <= B(Y | ψ)` asserts πX(σφ(A)) ⊆ πY(σψ(B)) as
sets of tuples: every projected tuple of A's selected facts occurs among the
projected tuples of B's selected facts. |X| = |Y| with positional structural type
equality (`10-data-model.md`). Unselected, this is dependency theory's inclusion
dependency; with selections it is the conditional inclusion dependency (CIND) of the
data-quality literature. The bidirectional statement `A(X | φ) == B(Y | ψ)` is
exactly the two containments, each judged independently.

**Judged on final states, only.** A dependency is a property of *committed*
databases: it is checked once at commit, against the transaction's final state; a
violation aborts the whole transaction with a typed error carrying the statement id
and the offending fact's bytes (never storage row ids — `10-data-model.md`). Since
point reads inside write transactions see the same final-state view the checker sees
(`70-api.md`) and full queries there are forbidden, no observable state ever
violates any statement, with no way out — stricter than SQL's opt-in deferrable
constraints, and the reason the modes died: *deleting a whole dependency-linked
cluster in one transaction is legal because the final state is clean*, which is what
cascade was a workaround for, and *dangling references never commit*, which is what
restrict was a weak spelling of. Operation order inside the transaction remains
semantically irrelevant (`50-storage.md` delta write path); cyclic references insert
without any staging concept.
**Decision.** **Alternative:** per-operation checking with staged visibility.
**Why it lost:** it enforces invariants on states nobody can observe, pushes
ordering obligations onto the caller, and fights the accumulate-then-commit write
path. **Reverses if:** never — semantics.

## Statements: the schema surface

Dependencies are declared as standalone statements between relation blocks — the
macro surface is the algebra with ASCII operator images (`⊆` is not a Rust token):
`->` for FD, `<=` for ⊆ (the subset order *is* an order), `==` for set equality,
`(fields)` for projection, `| field == literal` for selection.

```rust
bumbledb::schema! {
    relation Holder  { id: u64 as HolderId, fresh, name: str }
    relation Account {
        id: u64 as AccountId, fresh,
        holder: u64 as HolderId,
        kind: enum Kind { Checking, Savings },
        active: interval<i64>,
    }
    relation SavingsTerms { account: u64 as AccountId, rate_bps: i64 }

    Account(holder) <= Holder(id);
    Account(id | kind == Savings) == SavingsTerms(account);
    SavingsTerms(account) -> SavingsTerms;
    Account(holder, active) <= Employment(holder, during);
}
```

There is **no sugar and no field-level constraint syntax**: no `unique` modifier, no
`fk(...)`, no `union` block. A field carries its type, optional `as NewType`, and
optional `fresh` (which auto-materializes `R(field) -> R`, `10-data-model.md`) —
everything relational is a statement. Statements are anonymous; their identity is
their materialized-order id, pinned by the fingerprint, and errors cite the
statement rendered back in this notation.
**Decision: raw statements only.** **Alternative:** blessed sugar keywords lowering
to statements (`key`, `in`, `union`). **Why it lost:** owner ruling —
the surface must *be* the mental model; three keywords re-import three SQL concepts
and hide that they were one. The derivations below are documentation, not syntax.
**Reverses if:** never — and a future text frontend would lower to statements, not
around them.

## The acceptance gate

**The representation is general; the accepted vocabulary is closed.** A statement is
accepted only if the checker has an enforcement plan costing **O(log n) per
delta-touched fact** (amortized; coverage walks below add the touched-window term).
Concretely, validation demands:

- **FD:** key form, no selection; at most **one** interval-typed field, and it must
  be the **final** projection position (the neighbor probe needs the scalar prefix
  as its group — two interval positions would be 2-D exclusion, which the ordered
  guard cannot answer; SQL:2011 imposes the same last-position rule for the same
  reason). Guard key width must fit `MAX_GUARD_WIDTH` (`50-storage.md`) — rejected
  at declaration, never discovered at write time.
- **IND:** the target projection Y must be a permutation of some declared key of B
  (probe-ability: one guard get answers "is this tuple present"); if any position is
  interval-typed, that key must carry the interval (pointwise — coverage needs the
  target's intervals disjoint and ordered, which its own key provides as a theorem,
  not a requirement on the user). Each direction of `==` passes the gate
  independently. Selections may appear on either side; a selected field may not also
  be projected (a constant column — write the statement you mean).

A statement failing the gate is a schema-declaration error naming the missing plan.
This is the simplicity doctrine applied to invariants: generality of representation,
discipline of acceptance — an accepted statement is a *measured promise*, exactly
like an accepted optimization (`00-product.md`).

## Pointwise lifting (the interval semantics, derived)

Both judgments read interval positions through the denotation
(`10-data-model.md`): a fact stands for its point-family, and the judgment holds
iff it holds of the point-families.

- **FD, pointwise:** `R(room, during) -> R` with `during: interval` means no two
  facts share `room` and any point of `during` — i.e. **every per-room pair
  satisfies `DISJOINT`**, the Allen composite (before ∪ meets ∪ met-by ∪ after
  — `20-query-ir.md` § the Allen operator; one vocabulary, both sides of the
  engine). The "exclusion constraint" is not a feature of this system; it is this
  judgment on this type. Enforcement is two ordered-neighbor probes per touched
  fact (`50-storage.md`) — the O(log n) plan for the pairwise statement. Rays (`end == MAX` = `[s, ∞)`, the point-domain law —
  `10-data-model.md`) need no case of their own: two rays in one group share every
  point past the later start and always conflict — "at most one ongoing booking
  per room" is this judgment on this value; a bounded interval abutting a ray's
  start is legal, exactly as between bounded intervals.
- **IND, pointwise:** `A(who, span) <= B(who, span)` means every point of every A
  fact's span is covered by B facts for the same `who` — B's intervals need not
  match A's bounds, only jointly cover them. Checkable in O(log n + segments)
  because B's key keeps its intervals per-group disjoint and start-ordered: walk
  adjacent guard entries from the span's start, require no gap before its end. A
  **source ray requires coverage to ∞**: only a target chain reaching a ray
  satisfies it — bounded targets always leave a gap — while a target ray covers
  any bounded source above its start; both fall out of the same gap check, since
  ∞ = MAX is just the largest end word.
- Scalar positions in the same statement are unaffected — lifting is per-position,
  and a statement with no interval positions is the classical judgment unchanged.

## The derivations (where the old words went)

**Foreign key** = `A(x⃗) <= B(y⃗)`, unselected, one direction, y⃗ a key of B. All 99
FKs in the surveyed Postgres workload and all 4 in the surveyed SQLite workload are
this statement. *Restrict* is subsumed by final-state judgment; *cascade* is the
host deleting the cluster in one transaction (2 uses in 99 surveyed — the mode never
earned its semantics).

**Discriminated union** (sum-typed entity, the class-table-inheritance pattern) =
one bidirectional conditional containment per variant arm, plus the parent's key:

```rust
relation Grading {
    id: u64 as GradingId, fresh,
    kind: enum GraderKind { Deterministic, CustomOperator },
}
Grading(id | kind == Deterministic)  == DeterministicGrading(grading);
Grading(id | kind == CustomOperator) == CustomOperatorGrading(grading);
```

Three theorems fall out, each of which SQL either cannot state or states with
triggers:

1. **Totality** (`==`, left-to-right): a Deterministic grading *has* its child row —
   in the same commit, always. Row-at-a-time engines cannot check parent-implies-
   child at insert; deferrable mutual FKs recover a fixed two-table case and nothing
   conditional.
2. **Arm validity** (`==`, right-to-left): a child row's parent exists *with that
   kind* — this is the composite-FK-plus-CHECK-pin encoding, one statement instead
   of two mechanisms.
3. **Exclusivity** (derived): an id in two child relations would force the parent's
   `kind` to equal two variants; the parent's key on `id` makes that a contradiction,
   not a rule. The theorem's third consumer: the checker enforces it, the chase
   spends it, and the executor spends it again — rules selecting different `kind`
   values are provably disjoint, so the union's cross-rule dedup is elided at plan
   time (`40-execution.md` § set semantics, the rule-disjointness elision).

A **parent-only variant** needs no statement (a variant with no child relation
simply has no arm). A **0..1 optional attribute** — the no-nulls idiom
(`10-data-model.md`) — is the one-way form: `MailingAddress(business) <=
Business(id)` plus `MailingAddress(business) -> MailingAddress`; presence of the
child fact *is* the optionality, and the all-or-nothing column-group invariant that
bag-world schemas cannot state is unstatable *to violate* here.

**Partial/conditional keys** ("at most one active run per student") stay rejected as
FDs (above); the modeling answer is the relation split — an `ActiveRun` relation
whose ordinary key is the invariant, glued by containments — or, where the state is
temporal, an interval field under a pointwise key, which is usually what "active"
was.

**Temporal keys and temporal references** = the pointwise liftings above; SQL:2011's
`WITHOUT OVERLAPS` / `PERIOD` arrive as theorems of the denotation rather than
keyword features.

## Enforcement (summary; mechanics owned by 50-storage)

The commit pipeline evaluates every statement **restricted to delta-touched
bindings** against the final state — the incremental form of the judgment, sound
because an untouched binding cannot change a judgment's truth. The generation
witness (`70-api.md` § conditional writes) runs before this pipeline entirely —
an aborted witnessed write never reaches judgment, and judgment semantics are
untouched by it. The phases:

- FD: guard put conflicts (scalar) and ordered-neighbor probes (pointwise) during
  the insert phase.
- IND, source side: per **genuinely** inserted A-fact satisfying φ — true by
  representation: the delta's net insert set is exactly the facts the commit adds
  (`50-storage.md` net dispositions), so a redundant insert is never judged here —
  probe B's key guard (plus the selection-literal check on the found fact, and the
  coverage walk for interval positions).
- IND, target side: per deleted-and-not-reestablished B key tuple
  (re-establishment ψ-qualified per statement — `50-storage.md`), probe the
  statement's reverse-edge namespace for surviving A-facts that still require it
  (interval positions: the touched window).
- `==`: both directions, symmetric machinery.

Guard namespaces (`U`, `R`) are **derived accelerators for these judgments, not
definitions** — the reframe is normative in `50-storage.md`. The checker shares its
anti-probe primitive with query-surface negation (`40-execution.md`): "no fact
matches" is one mechanism with two callers. The coverage walk's frontier loop is
the same move: one covered-frontier segment sweep (`interval/sweep.rs`) whose two
continuations are the checker's gap verdict and `Pack`'s coalescing fold
(`20-query-ir.md`) — the walk lives once, and each caller keeps only its own trust
checks and its own outcome.

Accepted statements also license planner rewrites: the chase-based occurrence
elimination (`40-execution.md` § planner) deletes query joins a containment
already certifies.

## Validation roster (statements; exhaustive)

Rejected at schema validation, each with a distinct error: unknown relation/field
ids; empty or duplicate-carrying projections; arity mismatch between sides;
positional structural-type mismatch; selection literal type mismatch (including
out-of-range enum ordinals and non-UTF-8 string literals); a selected field also
projected; FD with >1 interval position, interval not in
final position, or guard width overflow; IND whose target projection matches no key
of the target (or, with an interval position, no pointwise key carrying it);
duplicate statements (identical normalized sides and form — write it once), where
two FDs over one field *set* are duplicates regardless of projection order (the
order shapes only the guard, and key resolution is by set);
a statement referencing an interval position against a scalar position (that is the
type-mismatch case, called out because it is the one migration authors will hit).
FD-with-selection and non-key FD forms are not rejected here — they are
**unrepresentable**: the descriptor cannot carry them, and the macro grammar
rejects the utterance (`70-api.md`).

## What this system says that SQL cannot (and refuses that SQL offers)

Says: totality of sum types; conditional reference targets (the arm's relation is
selected by a discriminator value); exclusivity as a theorem; pointwise keys and
coverage references without keyword special cases; whole-cluster atomic demolition
with no modes. Refuses: constraint modes, triggers, deferrability (all artifacts of
row-at-a-time checking over bags); CHECK constraints (host newtype constructors own
value validity — parse, don't validate); conditional and non-key FDs (schema shapes
own them). Every refusal names its replacement; none of them is a gap.
