# 30 — Dependencies

This chapter owns every invariant the engine enforces on committed states. There are
exactly **three statement forms** — the two original judgments (functionality,
containment) and the one extension form (the cardinality window) — all
statements *about queries*, and nothing else:
no constraint kinds, no modes, no triggers, no deferral. The words *unique key*,
*referential constraint*, *primary key*, *check constraint*, *exclusion constraint*, *cascade*,
and *restrict* name nothing here; where one of them used to name something, this
chapter derives that something as an instance and the word is retired.

## The two judgments

Both are parameterized by **single-atom queries** in the ordinary query IR
(`20-query-ir.md`): a relation, a **selection** φ (a set of (field, literal-set)
bindings read conjunctively — each binding a disjunction over its spelled set,
any type's literals; a singleton set is the equality binding, exactly
(`lean/Bumbledb/Schema.lean: Selection.singleton_satisfies_iff`), and the sets
are first-class rather than per-literal sugar because counts over a union do not
decompose (`lean/Bumbledb/Countermodels.lean:
disjunctive_window_not_literal_conjunction`)), and a **projection** X (an ordered
field list). Write such a query `R(X | φ)`; an empty selection is written `R(X)`;
a set binding is written `field == {A, B}` (canonical form sorted and
duplicate-free — validation sorts and rejects the degenerate spellings).
Dependencies and queries share one representation; a dependency is not a new kind of
thing, it is a required property of an old kind of thing.

**Functionality (FD).** `R(X) -> R`: at most one fact per determinant tuple
(`lean/Bumbledb/Dependencies.lean: Functionality`, `functionality_unique_witness`
— a key proves uniqueness, never existence). X is ordered (the order defines the determinant
key, `50-storage.md`), non-empty, duplicate-free. The general form `R(X) -> R(Y)`
with Y ⊊ fields exists in the theory (dependency theory's equality-generating
dependencies); **only the key form — Y = the whole relation — is accepted**: a
relation satisfying a non-key FD is mis-designed (BCNF says X should have been a key
of its own relation), and the engine refuses to be the crutch that makes the
mis-design comfortable. Selections on FDs are likewise rejected — a "conditional
key" is a relation split waiting to happen (`10-data-model.md` modeling discipline).
**Decision.** **Alternative:** accept general/conditional FDs (the machinery is the
same determinants). **Why it lost:** every instance surveyed is better said as a schema
shape; accepting them sells normalization back as a runtime feature. **Reverses if:**
a real invariant appears that no relation split can express.

**The arrow is canon — RATIFIED, owner ruling 2026-07-18.** `R(X) -> R` is
the dependency-theoretic utterance, not ceremony: the key projection
DETERMINES the tuple, and the arrow closing over its own relation is what
makes a key a key — a functional dependency `X -> R` over R's own
attributes. The spelling is never respelled, in either host's renderer
(the TS render golden byte-pins it), and the macro teaches the reading
rather than merely asserting it: a right side naming a foreign relation is
a spanned compile error
(`crates/bumbledb/tests/schema-compile-fail/key_arrow_foreign_relation.rs`).

**The taxonomy is checked — owner ruling 2026-07-18 ("option 1").** The
macro's notation stays exactly as written (`as NewType`, `fresh`, every
statement form — macro-side `ref`, projection sorts, and signature blocks
were all rejected); what changed is that the hand taxonomy stopped being an
unchecked claim. The ONE shared lowering (`SchemaSpec::descriptor`, the
macro's own expansion path) verifies newtype coherence across every
statement's paired faces, positionwise over the two projections —
containment, `==`, and cardinality-window target pairs alike, ψ-selected
faces included (selection never changes the pairing). The rule's three
cases: labeled pairs only with the SAME label; bare pairs only with bare
(so a deliberately-bare pointer in no paired-face statement stays legal);
a labeled↔bare pairing is an error too — the same strictness the TS SDK's
computed classes enforce, so the two hosts judge identically from opposite
directions (Rust declares sorts and checks them against the laws; TS
declares nothing and computes the sorts FROM the laws). Surfaced per host
idiom: a `compile_error!` spanned at both offending faces
(`tests/schema-compile-fail/statement_newtype_mismatch.rs`,
`statement_newtype_half_labeled.rs`) and the typed
`SpecIssue::StatementNewtypeMismatch` on the spec path (the SDK's
`newtypeMismatch` rejection). Authoring-time only: newtypes are dropped at
lowering, so descriptors, fingerprints, and stores never carry this law.

**Containment (IND).** `A(X | φ) <= B(Y | ψ)`: subset inclusion of the selected
projected views (`lean/Bumbledb/Dependencies.lean: Containment`,
`contains_iff_view_subset`). |X| = |Y| with positional type equality — exact
structural equality at scalar positions (`10-data-model.md`), **element-domain
equality at interval positions (Q1)**: two interval-shaped types of one element
domain match whatever their widths, so `interval<u64, 1>` meets `interval<u64>`
positionally while `interval<u64, w>` against `interval<i64>` (or any scalar)
still mismatches. The pointwise judgments quantify over points, which carry an
element domain and not a width (`lean/Bumbledb/Schema.lean:
Value.points_one_tag_u64`), and the coverage walk is width-blind by
construction (`storage/commit/judgment.rs::check_coverage`).

**Decision: element-domain typing at interval positions.** **Alternative:**
exact-encoding equality (widths included, as at scalar positions). **Why it
lost:** the pointwise judgments quantify over points, which carry an element
domain, not a width — the denotation never distinguished widths, so strict
equality would refuse statements the spec already judges; concretely, it
rejects the purge's own named replacement recipe (the ordering triple's
exact-partition `==` between a general playlist span and its `interval<u64, 1>`
slots, the cookbook recipe the order-mark funeral points at). **Reverses if:**
never — the denotation decides.

Unselected, this is dependency theory's inclusion
dependency; with selections it is the conditional inclusion dependency (CIND) of the
data-quality literature. The bidirectional statement `A(X | φ) == B(Y | ψ)` is
exactly the two containments, each judged independently
(`lean/Bumbledb/Dependencies.lean: containsEq_iff_view_ext`).

**Accepted equality is a keyed bijection.** Each containment direction must
independently resolve its target projection to a declared key; with both keys,
mutual inclusion is a one-to-one correspondence between the selected A- and
B-facts, on whole projected products
(`lean/Bumbledb/Dependencies.lean: keyed_eq_unique_correspondence`; the bare
mutual-inclusion form without the key premises has the countermodel
`lean/Bumbledb/Countermodels.lean: bare_eq_not_unique`). This is not whole-fact
equality — unprojected payloads may differ — and it makes no claim about facts
outside φ or ψ; `key_permutation` only reorders fields for the target key and
weakens nothing.

**Judged on final states, only.** A dependency is a property of *committed*
databases: one judgment per commit, of the transaction's final state — operation
order inside the transaction is not representable in the judge's input
(`lean/Bumbledb/Txn.lean: final_state_judgment_order_free`; the per-operation
alternative judges states nobody can observe —
`lean/Bumbledb/Countermodels.lean: per_op_judgment_wrong`) — and every committed
state models its theory (`lean/Bumbledb/Txn.lean: committed_states_model`). A
violation aborts the whole transaction with a typed error whose payload is the
failing phase's **complete violation set**
(`lean/Bumbledb/Txn.lean: rejection_is_complete`) —
every violated statement of that phase, cited exactly once (per direction for a containment:
source before target), in materialized statement order, each citation carrying
the statement id and the offending fact's bytes (never storage row ids —
`10-data-model.md`); the set is sealed — nonempty, sorted, deduplicated — by
its only constructor (`Violations::seal` in `crates/bumbledb/src/error.rs`,
stable-sorting by the citation key `Violation::citation`: statement id, then
direction), so an under-reported rejection is unrepresentable. The citation
ORDER is contract, not accident: the conformance verdicts compare the list
whole, and at the statement-index surface (`lean/Main.lean: RVerdict`) the two
directions of one containment collapse to a single cited index — ascending
citation order keeps the direction pair adjacent, so the collapse
(`lane_verdict` in `crates/bumbledb-bench/src/conformance/judgment.rs`) is
total: ascending statement indices, a both-directions containment cited once. One
preemption, from the enforcement structure
itself: the containment probes are defined over the *keyed* final state
(determinants are the probe index), which exists only when every key statement
holds — so key violations preempt the statement phase and no rejection ever
mixes the two
(`lean/Bumbledb/Txn.lean: rejection_never_mixes`). Within the containment set, the two
directions partition the final state's source facts: a fact inserted this commit
is judged source-side only, a pre-existing survivor target-side — one statement is
never convicted twice through one fact. Since
point reads inside write transactions see the same final-state view the checker sees
(`70-api.md`) and full queries there are forbidden, no observable state ever
violates any statement, with no way out — stricter than SQL's opt-in deferrable
constraints, and the reason the modes died: *deleting a whole dependency-linked
cluster in one transaction is legal because the final state is clean*, which is what
cascade was a workaround for, and *dangling references never commit*, which is what
restrict was a weak spelling of. Cyclic references insert
without any staging concept (`50-storage.md` delta write path).
**Decision.** **Alternative:** per-operation checking with staged visibility.
**Why it lost:** it enforces invariants on states nobody can observe, pushes
ordering obligations onto the caller, and fights the accumulate-then-commit write
path. **Reverses if:** never — semantics.

## Rendering the rejection (normative)

The rejection is the repair diagnostic, so it must be fully consumable as plain
data by a host that has no typed fact structs — a bindings layer, an LLM repair
prompt. Two mechanisms, both public:

- **Decoded cited facts ride the set.** Alongside each citation's canonical
  fact bytes, `Violations` carries the same facts decoded to owned values
  (`CitedFact { relation, values }` via `Violations::cited_facts` /
  `Violations::citations`): one `Value` per sealed field, `str` fields resolved
  to owned strings. The decode happens AT the commit boundary's one rejection
  exit (`storage/commit/write.rs`), and only there, because only there is it
  possible: an inserted fact's `str` fields may carry provisional intern ids
  minted by the very transaction the rejection aborts — the abort flushes
  nothing, so a post-hoc decode would misread a genuine rejection as a dangling
  id. The cited relation is derived from the violated statement: a key's own
  relation, a containment's SOURCE (the judgment speaks about sources), a
  window's TARGET (the convicted parent). The allocation is acceptable at
  rejection time; the accept path allocates nothing new.
- **`render_rejection(descriptor, violations)`** (`schema::render`) lowers the
  whole set to `[{statement id, kind tag, canonical spelling, direction?,
  count?, offending facts as (relation name, [(field name, value)])}]` — pure
  over the descriptor (a foreign host renders with its cached manifest-side
  descriptor, no database handle), spelling through the ONE canonical renderer
  (`render_declared`; the renderer is a bijection on legal statements, so every
  spelling pastes back). The manifest carries the same spellings per statement
  id (`70-api.md` § the manifest), so a host can also cite statements without
  the violations value in hand.

Pinned by `crates/bumbledb/tests/dyn_surface.rs`: one commit violating a
containment and a window renders both citations (the statement phase is
scan-complete), a second violating an FD renders the key form (key violations
preempt the statement phase — `rejection_never_mixes` — so no single commit can
exhibit all three), and the provisional-intern case decodes.

## The extension form

**Cardinality window.** `B(Y | ψ) <={lo..hi} A(X | φ)` — B-family, target-left
(the left side is the window's target): per selected target fact, the count of
selected source facts sharing its projected tuple lies in the
window (`lean/Bumbledb/Cardinality.lean: CardinalityWindow`;
`lean/Bumbledb/Schema.lean: Statement.cardinality`). The window vocabulary is
closed under the **canonical-utterance law** (`70-api.md` records the law): the
legal spellings are `{n}` — THE exact-count spelling, `{0}` the exclusion —
`{lo..hi}` with lo < hi, `{lo..*}` floors (lo ≥ 2), and `{0..hi}` ceilings;
each survivor is otherwise unrepresentable. `*` is the only spelling of "no
upper bound". Every other spelling is an error naming the canonical form —
grammar-side at expansion, descriptor-side at validation (`{n..n}`/`{0..0}`
are the exact count respelled; `{hi..lo}` inverted is unsatisfiable,
`CardinalityInvertedWindow`; `{0..*}` says nothing,
`lean/Bumbledb/Cardinality.lean: cardinality_zero_star`,
`CardinalityVacuousWindow` — the default posture is unspelled, so a spelled
statement always strengthens it; `{1..*}` is the bare containment respelled,
`CardinalityContainmentWindow`). Windows never manufacture parents —
existence obligations are containments' alone
(`lean/Bumbledb/Cardinality.lean: cardinality_of_empty_parent`). The form
extends, never contradicts, the original vocabulary: a floored window implies the
reverse containment (`lean/Bumbledb/Subsumption.lean: window_floor_containment` —
which is why `{1..*}` is banned as a duplicate spelling of `<=`),
and keyed `==` is exactly the `{1}` window
(`lean/Bumbledb/Subsumption.lean: keyed_eq_unit_window`,
`unit_window_containsEq` — the reconstruction returns bare mutual inclusion, the
key premises staying acceptance's). The upper bound is load-bearing, not
decorative (`lean/Bumbledb/Countermodels.lean: unit_window_two_children`).

The `{0}` exclusion deserves its footnote: it is **denial-flavored but
satisfaction-only** — "no ψ-selected parent has a φ-selected child" reads like
a negative constraint, but the judgment is the same count-in-window
satisfaction check as every window (count = 0, both ends inclusive), over the
final state, through the same touched-parent plan. No negation enters the
statement language, no denial constraint class opens, and the decidability
firewall below is untouched: `{0}` is a window that happens to be small, not a
new kind of statement.

The form's sides are **single atoms, permanently** (the E1 refusal: a join
inside the judge breaks the linear per-statement cost model — the shape is
proved uninhabitable at the gate type,
`lean/Bumbledb/Countermodels.lean: joined_window_form_uninhabitable`).

**The form is accepted at declaration and judged per commit.** Acceptance
seals the statement's plan handle (the window's resolved target key), and the
commit pipeline runs exactly the plan the calculus prices
(`lean/Bumbledb/Oracle.lean: cardinality_plan_decides`):
per touched parent one keyed parent probe and one child-group walk. The
touched set is the delta-restriction theorem's
(`lean/Bumbledb/Txn/DeltaRestriction.lean: touchedParents`),
and the form's violations join the statement phase's complete citation set
(§ judged on final states). `Db::verify_store` re-verifies the form
globally — the sweeper's half of the division of authority.

**Refused: order marks (plain and ranked).** The `order R(pos) per R(grp)
[by …]` statement form — per parent group, positions exactly `1..k`, the
ranked variant monotone with key-chased ranks — shipped for one day and left
the vocabulary whole. **Alternative:** keep them (they shipped for one day).
**Why it lost:** no censused workload; O(k) renumbering is intrinsic to the
contiguity invariant, not to any checker — fractional indexing is the
industry answer and needs only a keyed position; the keyword-led syntax broke
the operator-algebra surface; and the admission calculus had already
structurally refused the ranked form's chase (`touched_delta_bounded` fails
under the every-group escalation — the E1 exclusion's sibling), with the
one-day shipped form's only adversarial finding living exactly there (the
ranked-on-closed blocker, 5b45b87b's closeout). Replacements, both named:
fractional indexing over `R(group, pos) -> R` (host-side; cookbook), and the
exact-partition interval recipe for tiling contiguity (cookbook, the ordering
triple). **Reverses if:** a censused workload demands enforced unit-step
sequences — and then it returns as the interval recipe plus the fixed-width
type, never as a statement form.

## Statements: the schema surface

Dependencies are declared as standalone statements between relation blocks — the
macro surface is the algebra with ASCII operator images (`⊆` is not a Rust token):
`->` for FD, `<=` for ⊆ (the subset order *is* an order), `==` for set equality,
`<={lo..hi}` for the cardinality window (B-family, target-left; `{n}` the
exact-count spelling, `{lo..*}` the no-ceiling floor),
`(fields)` for projection, `| field == literal` for selection
with `| field == {A, B}` the literal-set binding (two or more literals — a
one-element set is the bare literal, by the canonical-utterance law).

```rust
bumbledb::schema! {
    closed relation Kind as KindId = { Checking, Savings };

    relation Holder  { id: u64 as HolderId, fresh, name: str }
    relation Account {
        id: u64 as AccountId, fresh,
        holder: u64 as HolderId,
        kind: u64 as KindId,
        active: interval<i64>,
    }
    relation SavingsTerms { account: u64 as AccountId, rate_bps: i64 }

    Account(holder) <= Holder(id);
    Account(id | kind == Savings) == SavingsTerms(account);
    SavingsTerms(account) -> SavingsTerms;
    Account(holder, active) <= Employment(holder, during);
    Holder(id) <={1..4} Account(holder | kind == {Checking, Savings});
}
```

There is **no sugar and no field-level constraint syntax**: no `unique` modifier, no
`fk(...)`, no `union` block. A field carries its type, optional `as NewType`, and
optional `fresh` (which auto-materializes `R(field) -> R`, `10-data-model.md`) —
everything relational is a statement. A closed relation likewise auto-materializes
`R(id) -> R` on its synthetic id field (`10-data-model.md` § closed relations);
**materialized order is: every fresh auto-key (relation-then-field declaration
order), then every closed auto-key (relation declaration order), then the declared
statements in declaration order** — the order is a fingerprint input, pinned once
and never revisited. The materialization judgment is pure theory and lives
engine-free with the descriptor vocabulary
(`SchemaDescriptor::materialized_statements` in
`crates/bumbledb-theory/src/schema.rs` — the zero-dependency theory crate,
`00-product.md` § Dependencies (crates), reached only through the `bumbledb`
re-exports per `70-api.md`'s facade ruling); the admission boundary
(`SchemaDescriptor::validate`) stays engine-side. Statements are anonymous; their identity is
their materialized-order id, pinned by the fingerprint, and errors cite the
statement rendered back in this notation.
**Decision: raw statements only.** **Alternative:** blessed sugar keywords lowering
to statements (`key`, `in`, `union`). **Why it lost:** owner ruling —
the surface must *be* the mental model; three keywords re-import three SQL concepts
and hide that they were one. The derivations below are documentation, not syntax.
**Reverses if:** never — and a future text frontend would lower to statements, not
around them. (The window's `<={lo..hi}` annotation is not that rejected sugar: it
is a first-class judgment form with its own denotation
(`lean/Bumbledb/Cardinality.lean: CardinalityWindow`), not a keyword lowering to
other statements — and its keyword-led ancestor spelling, `in lo..hi per`, is
deleted vocabulary: the grammar rejects it at expansion naming the canonical
form, the same funeral the `order` keyword received.)

## The acceptance gate

**The representation is general; the accepted vocabulary is closed.** A statement is
accepted only if the checker has an enforcement plan costing **O(log n) per
delta-touched fact** (amortized; coverage walks below add the touched-window term).
Each accepted form's plan — and its consultation count — is a theorem of the
order-oracle plan calculus (`lean/Bumbledb/Oracle.lean: acceptance_gate`), and the
gate itself is one inhabited type: each single-key statement form's case for
acceptance is its `lean/Bumbledb/Admission.lean: AdmissibleForm` term — denotation,
executable judge, delta restriction, oracle plan, and creation-quarantine
compliance, the checklist as a type. Five forms inhabit it (scalar and pointwise
FD, scalar and coverage IND, the window — every one
accepted at declaration and judged per commit); a future form enters the vocabulary by
inhabiting it first, and the E1 joined-window shape is proved uninhabitable on
the plan field (`lean/Bumbledb/Countermodels.lean: joined_window_form_uninhabitable`).
An answer-dependent (chase-shaped) read or an every-group escalation is
structurally refused by the same fences — which is part of why the order-mark
forms left the vocabulary (§ refused: order marks above).
Concretely, validation demands:

- **FD:** key form, no selection; at most **one** interval-typed field, and it must
  be the **final** projection position (the neighbor probe needs the scalar prefix
  as its group — one point probe per touched fact scalar
  (`lean/Bumbledb/Oracle.lean: fd_plan_decides`), two neighbor probes pointwise
  (`lean/Bumbledb/Oracle.lean: neighbor_probe_decides`); two interval positions
  would be 2-D exclusion, which the ordered
  determinant index cannot answer; SQL:2011 imposes the same last-position rule for the same
  reason). Determinant key width must fit `MAX_DETERMINANT_WIDTH` (`50-storage.md`) — rejected
  at declaration, never discovered at write time.
- **IND:** the target projection Y must be a permutation of some declared key of B
  (probe-ability: one determinant get answers "is this tuple present" — the probe
  arms decide the delta-restricted check,
  `lean/Bumbledb/Oracle.lean: containment_plan_decides`, and the key premise is
  what prices the unit probe,
  `lean/Bumbledb/Oracle.lean: accepted_target_key_prices_the_probe`); if any position is
  interval-typed, that key must carry the interval (pointwise — coverage needs the
  target's intervals disjoint and ordered, which its own key provides as a theorem,
  not a requirement on the user). Validation seals that theorem as a
  `DisjointDeterminantProof`; interval enforcement and the coverage checker require the
  token, so the forward sweep cannot be selected by an unchecked flag (the entry
  seek + prefix walk verdict is the denotation under it —
  `lean/Bumbledb/Oracle.lean: coverage_walk_decides`). Each
  direction of `==` passes the gate
  independently. Selections may appear on either side; a selected field may not also
  be projected (a constant column — write the statement you mean).
- **IND into a closed target:** the target side is stage-1-known, so there is no
  key search and no probe strategy — the enforcement plan is **the answer set
  itself** (`lean/Bumbledb/Oracle.lean: member_test_decides` — zero oracle
  consultations). Y must be exactly the synthetic id (the handle is the one probe-able
  identity of a closed relation); ψ is applied to the sealed extension at validate
  and the surviving declaration ids compile to a 256-bit member set (the ≤256 roster cap
  exists exactly to fix this width). The ψ-selected form gives sub-vocabularies —
  `Escalation(severity) <= Severity(id | pages == true)` — the same O(1) plan.
  Interval positions on a containment with a closed side (either side) are
  **refused v0**: a pointwise judgment against a virtual extension would mix the
  coverage walk with virtual storage, and a constant source's coverage demand has
  no delete-time re-judgment path (*trigger* for lifting: a census sighting).
- **Cardinality window:** first the **canonical window vocabulary** (the
  canonical-utterance law's descriptor face — an inverted window is
  unsatisfiable, `CardinalityInvertedWindow`; `0..*` says nothing,
  `CardinalityVacuousWindow`; `1..*` is the containment respelled,
  `CardinalityContainmentWindow` — so a sealed schema holds canonical windows
  only and the renderer never faces a banned spelling), then the shared side
  shapes (arity, positional structural
  type equality, the σ rules), then the containment target-key rule **reused
  verbatim** — Y must resolve a declared key of B (`resolve_target_key`, the
  same probe-ability demand; the promised plan is per touched parent one keyed
  point probe plus one child-group walk,
  `lean/Bumbledb/Oracle.lean: cardinality_plan_decides`,
  `window_plan_consultations`) — and the **v0 interval refusal**: window
  projections carry no interval-typed position, either side
  (`CardinalityIntervalPosition` — a window counts FACTS per parent, and an
  interval position would make the count ambiguous between facts and points;
  *trigger* for lifting: a sighted counting-over-denotation workload —
  `lean/Bumbledb/Cardinality.lean` § v0 refusals). Closed-side rules mirror
  containment's: a closed target compiles the member-set plan through the same
  key rule (projection = the synthetic id), and a window between constants is
  decided at validate outright.
- **Statements between constants** (both sides closed) are decided at validate
  outright: a declaration the ground axioms refute — a source axiom outside the
  member set, a declared key two axioms collide under, or a parent axiom whose
  child count falls outside its window — is a schema error, not
  a latent judgment, because a theory whose axioms refute its own statement has no
  model to commit (`lean/Bumbledb/Schema.lean: den_closed_constant` — a closed
  relation denotes the same sealed fact set at every instance).

A statement failing the gate is a schema-declaration error naming the missing plan.
The exact-field-set rule explains itself: `NoMatchingTargetKey` and
`NoPointwiseTargetKey` own the target relation, requested projection, and every
available key id plus field set. Their `Display` lists that evidence; the interval
form ends by pointing at the executable repair—declare the exact pointwise key
`R(prefix…, interval) -> R`. The owned payload is assembled by
`schema/validate.rs::target_key_candidates`, so the rejection outlives its
descriptor without borrowing schema internals.
This is the simplicity doctrine applied to invariants: generality of representation,
discipline of acceptance — an accepted statement is a *measured promise*, exactly
like an accepted optimization (`00-product.md`).

The sealed representation is a sum with homogeneous typed arenas — keys,
containments, windows.
`FieldSet` gives each projection canonical set identity (sorted and
duplicate-free), while `Projection` retains statement order beside that set so
validation compares identity and execution derives the target-key permutation.
Validation is the only mint for `KeyId`, `ContainmentId`, and `WindowId`:
a key witness resolves
totally through `Schema::key`, a containment witness resolves totally through
`Schema::containment` (windows through `Schema::window`), and
`Schema::dependents` carries containment witnesses indexed
by a key witness. The global `StatementId` order survives as a separate sum-typed
spine; `Schema::statement` parses it into the corresponding borrowed typed arm for
fingerprint identity, storage, diagnostics, and rendering. Downstream code consumes
that arm directly — the witness carries the proof, so no descriptor/enforcement
variant agreement remains to assert.

A strict-superkey FD is accepted and enforced, but sealing records the non-fatal
`SchemaWarning::RedundantSuperkey { relation, key, implied_by }`: the smaller
determinant already implies it, so the larger determinant is write amplification.
`Schema::warnings()` is diagnostics only; it changes neither the statement spine
nor enforcement, and therefore does not enter the fingerprint.

**Decision: the engine judges satisfaction, never implication — the decidability
firewall.** The engine decides only judgments about finite, present data: a
commit's final state, a sealed extension (the both-sides-closed case above).
Consequence *among statements* appears in exactly three sanctioned forms — a
specific theorem compiled into a witness type (`DisjointDeterminantProof`), a
conservative optimization that is sound, may answer "unknown", and always has a
semantics-preserving fallback (`provably_disjoint`/`provably_distinct`, grounding
elimination), or diagnostics (`RedundantSuperkey`) — and no code path's
correctness may ever require deciding whether one statement follows from the
rest. The presumption behind the law is a design input, not a survey: for this
statement class (composite, cyclic, key-based INDs with selections), implication
is presumed undecidable, per the classical FD+IND result. Four tripwires name
the law's edges: acceptance never resolves an implied key — the exact-field-set
rule above is this law's acceptance face, and the entailment-vs-acceptance gap
is formal (`lean/Bumbledb/Dependencies.lean: no_closure_superkey_implication` —
proved, deliberately unspent); enforcement never skips a check as implied;
schema evolution re-judges instances (`Db::verify_store`, ETL), never proves
theory-to-theory entailment; statement selections stay literal-SET-shaped —
the 2026-07-14 decision executed: today's accepted surface is the
(field, literal-set) disjunctive binding above, exactly the spec's fragment
(`lean/Bumbledb/Schema.lean: Selection`; a singleton set is today's equality,
`lean/Bumbledb/Schema.lean: Selection.singleton_satisfies_iff`), and the
tripwire's edge is now the set form itself — nothing richer enters (richer σ
than literal sets moves the class toward denial constraints, where even
satisfiability stops being trivial). **Alternative:** a decidable-fragment implication engine
(unary INDs, acyclic reference graphs) powering redundancy elimination and
migration proofs. **Why it lost:** it restricts the schema language to buy a
feature nothing needs — the delta-restricted checker already makes enforcement
cheap without it, and an incomplete implication procedure on the enforcement
path silently accepts or rejects wrong. **Reverses if:** a censused workload
where provably-redundant re-checking measurably dominates commit cost — and
even then the feature lands diagnostics-side first, never on enforcement.

**Decision: statements quantify over stored relations, permanently.** By
representation: a statement's atoms carry `RelationId`, and no predicate
vocabulary exists — or will exist — in the statement language, including under
the landed recursion cut (query-side `PredId` and `RelationId` are separate
identities that never pun, and the one-line `Idb` guard in both grounding
rewrites — `plan/ground.rs`, `plan/ground/evaluate.rs` — is this law's
mechanism, `20-query-ir.md` § engine recursion's consumer guards). A containment between derived predicates is Datalog query
containment — undecidable outright — and commit-time enforcement would require
materializing every constrained view per commit. **Alternative:**
deductive-database constraints over views. **Why it lost:** the undecidability
above, plus the acceptance gate's own rule — no O(log n) enforcement plan
exists for a fixpoint's blast radius (the join blast radius is the countermodel,
`lean/Bumbledb/Countermodels.lean: joined_window_blast`, composed into the gate
type's uninhabitability,
`lean/Bumbledb/Countermodels.lean: joined_window_form_uninhabitable`). **Reverses if:** never for recursive
predicates; a non-recursive-view variant re-opens only with its own theory
review, as a new decision.

## Pointwise lifting (the interval semantics, derived)

Both judgments read interval positions through the denotation
(`10-data-model.md`): the judgment is of the point-families, position by
position — a statement with no interval positions is the classical judgment
unchanged (`lean/Bumbledb/Schema.lean: Header.intervalSplit_scalar`). The
pointwise reading is of the field *set*, never the written projection order —
one interval field written anywhere is the pointwise judgment
(`lean/Bumbledb/Countermodels.lean: split_permuted_some`), exactly the
`FieldSet` canonicalization above.

- **FD, pointwise:** per scalar group, pairwise-disjoint point sets
  (`lean/Bumbledb/Dependencies.lean: pointwise_key_disjoint`) — the query
  surface's own `DISJOINT` vocabulary (`20-query-ir.md` § the Allen operator),
  one vocabulary on both sides of the engine. The "exclusion constraint" is not a
  feature of this system; it is this judgment on this type. Enforcement is two
  ordered-neighbor probes per touched fact (`50-storage.md`) — the O(log n) plan
  for the pairwise statement. Rays need no case of their own — "at most one
  ongoing booking per room" is this judgment on the ray value
  (`lean/Bumbledb/Values.lean: ray_is_unbounded_tail`).
- **IND, pointwise:** per group, the source's points are jointly covered by the
  target's intervals (`lean/Bumbledb/Dependencies.lean:
  coverage_is_support_inclusion`) — coverage, never bound matching. Checkable in
  O(log n + segments) because the target's own pointwise key keeps its intervals
  per-group disjoint and start-ordered — the premise validation seals as the
  `DisjointDeterminantProof` and the one-pass sweep spends
  (`lean/Bumbledb/Exec/Sweep.lean: sweep_covered_sound_complete`). A source ray
  is satisfied only by a target chain reaching a ray
  (`lean/Bumbledb/Exec/Sweep.lean: ray_needs_ray`); both directions of ∞ fall
  out of the same gap check.
- **Direction law:** one containment covers only the source support; target
  overhang is legal (`lean/Bumbledb/Countermodels.lean: one_way_overhang`).
  Exact partition is the conjunction of both coverage directions plus pointwise
  keys on both sides — five ordinary statements, no partition primitive
  (`lean/Bumbledb/Dependencies.lean: exact_partition_iff`); cookbook recipe 26
  spells them and locks gap rejection, overhang rejection, adjacency, and a
  two-scalar-prefix instance.

## The derivations (where the old words went)

**SQL referential-constraint special case** = `A(x⃗) <= B(y⃗)`, unselected, one
direction, y⃗ a key of B. All 99 references in the surveyed Postgres workload and
all 4 in the surveyed SQLite workload are this statement. *Restrict* is subsumed by
final-state judgment; *cascade* is the
host deleting the cluster in one transaction (2 uses in 99 surveyed — the mode never
earned its semantics).

**Discriminated union** (sum-typed entity, the class-table-inheritance pattern) =
one bidirectional conditional containment per arm, plus the parent's key:

```rust
closed relation GraderKind as GraderKindId = { Deterministic, CustomOperator };

relation Grading {
    id: u64 as GradingId, fresh,
    kind: u64 as GraderKindId,
}
Grading(kind) <= GraderKind(id);
Grading(id | kind == Deterministic)  == DeterministicGrading(grading);
Grading(id | kind == CustomOperator) == CustomOperatorGrading(grading);
```

Three theorems fall out, each of which SQL either cannot state or states with
triggers:

1. **Totality** (`==`, left-to-right): a Deterministic grading *has* its child
   fact — in the same commit, always
   (`lean/Bumbledb/Dependencies.lean: keyed_eq_unique_correspondence`, the
   forward correspondence). Row-at-a-time engines cannot check parent-implies-
   child at insert; deferrable mutual FKs recover a fixed two-table case and nothing
   conditional.
2. **Arm validity** (`==`, right-to-left): a child fact's parent exists *with that
   kind* (the same correspondence, reversed) — one statement instead of the
   composite-FK-plus-CHECK-pin pair of mechanisms.
3. **Exclusivity** (derived): an id in two child relations would force the parent's
   `kind` to equal two handles; the parent's key on `id`
   (`lean/Bumbledb/Dependencies.lean: functionality_unique_witness`) makes that a
   contradiction, not a rule. Two consumers remain: the checker enforces it and the
   grounding spends it. Plan introspection can also report that rule heads are
   provably disjoint, but execution
   deliberately does not spend that knowledge; the measured refutation is in
   `40-execution.md` § set semantics.

A **parent-only handle** needs no statement (a kind with no child relation
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
because an untouched binding keeps its pre-state verdict
(`lean/Bumbledb/Txn/DeltaRestriction.lean: delta_restricted_commit_sound` — the
composition across the whole theory; its holds-before premise is the sweeper's
half of the division of authority,
`lean/Bumbledb/Countermodels.lean: incremental_verdict_needs_holds`). The generation
witness (`70-api.md` § conditional writes) runs before this pipeline entirely —
an aborted witnessed write never reaches judgment, and judgment semantics are
untouched by it. The phases:

- FD: determinant put conflicts (scalar) and ordered-neighbor probes (pointwise) during
  the insert phase.
- IND, source side: per **genuinely** inserted A-fact satisfying φ — true by
  representation: the delta's net insert set is exactly the facts the commit adds
  (`50-storage.md` net dispositions), so a redundant insert is never judged here —
  probe B's key determinant (plus the selection-literal check on the found fact, and the
  coverage walk for interval positions).
- IND, target side: per deleted-and-not-reestablished B key tuple
  (re-establishment ψ-qualified per statement — `50-storage.md`), probe the
  statement's reverse-edge namespace for surviving A-facts that still require it
  (interval positions: the touched window). Survivors inserted this commit are
  the source side's work (the direction partition, § judged on final states);
  the target side convicts through pre-existing survivors only.
- IND into a closed target: **O(1)** per inserted A-fact inside φ — one AND and
  one test against the compiled member set; an out-of-range word is simply a miss
  (the same violation as any dangling reference). No `R` reverse edges are ever
  written for the class (the target side is vacuous by construction — axioms don't
  delete), so the target-side phase emits nothing and the offline sweeper convicts
  any stored `R` entry naming one.
- IND from a closed source (**domain quantification** — the worked example below):
  the source side never fires (no closed inserts exist); the target side fires
  only on a B-key disestablishment, where the surviving sources ARE the sealed
  extension's selected ground axioms — an honest ≤256-element scan on the delete path replaces the
  `R`-prefix probe, since a constant source stored no edges.
- `==`: both directions, symmetric machinery.
- Cardinality window: per **touched parent** — every parent key tuple any
  delta child fact projects to (φ-blind, the recorded superset: a non-φ
  fact never changes a group, and wider touched only re-checks more), plus
  the delta's ψ-selected parents themselves
  (`lean/Bumbledb/Txn/DeltaRestriction.lean: touchedParents`) — one keyed
  probe resolves the parent's ψ-selected holder in the final state (no
  holder, no obligation: windows never manufacture parents), then one
  ordered walk of the statement's `R` bucket counts the child group
  (`lean/Bumbledb/Oracle.lean: cardinality_plan_decides` — the walk's
  length verdict IS the delta-restricted check; a closed child set scans
  its ≤256 axioms instead, exactly the domain-quantification move). A
  count outside the window convicts with the statement id and the parent
  fact's bytes. A floored window may share the containment's probe
  machinery (`lean/Bumbledb/Subsumption.lean: window_floor_containment`)
  but never skips its own check.

**Domain quantification, worked.** `Severity(id) <= Handler(severity)` with
`Severity` closed and `Handler(severity) -> Handler` declared says *every severity
has a handler*. Inserting handlers never fires it (the source is constant);
deleting the last `Handler` fact for severity 2 disestablishes the `(2)` key tuple,
the dependent statement scans the extension, finds the severity-2 axiom projecting
to the lost tuple, and aborts — while a delete whose tuple re-lands in the same
commit (a handler *replacement*) is dropped by the plain set difference before any
scan runs. The empty store violates the statement until the handlers land; commits
that never touch `Handler` cannot observe that, and the offline sweeper
(`60-validation.md`) re-verifies the class globally by walking the extension —
exactly the division of authority the delta-restricted judgment implies.

**The checker consumes constants** (the staging law): every σ literal whose
canonical bytes are a pure function of the value is sealed into the statement at
validate — the commit path byte-compares against sealed encodings (a set
binding compares membership among its sealed alternatives; a singleton stays
the one classic compare) and resolves
only interned text (dictionary state is per-database; a never-interned literal
still proves its side — for a set binding, that alternative — unsatisfiable). The pointwise/coverage judgment instead
consumes the `IntervalCoverage` variant's validator-minted
`DisjointDeterminantProof`; no boolean can license the sweep.
Two audited stays, recorded so the staging audit's lines are discharged rather
than forgotten: the `FactLayout` rebuild stays at open (open is rare, the
rebuild pure and cheap), and the fresh→FD materialization stays at validate
(the materialized ORDER is a fingerprint input, pinned there by contract).

Determinant namespaces (`U`, `R`) are **derived accelerators for these judgments, not
definitions** — the reframe is normative in `50-storage.md`. The checker shares its
anti-probe primitive with query-surface negation (`40-execution.md`): "no fact
matches" is one mechanism with two callers. The coverage walk's frontier loop is
the same move: one covered-frontier segment sweep (`interval/sweep.rs`) whose two
continuations are the checker's gap verdict and `Pack`'s coalescing fold
(`20-query-ir.md`) — the walk lives once, and each caller keeps only its own trust
checks and its own outcome.

Accepted statements also license planner rewrites: the grounding-based occurrence
elimination (`40-execution.md` § planner) deletes query joins a containment
already certifies.

## Validation roster (statements; exhaustive)

Rejected at schema validation, each with a distinct error: unknown relation/field
ids; empty or duplicate-carrying projections; arity mismatch between sides;
positional structural-type mismatch (element-domain at interval positions — Q1
above — exact everywhere else); selection literal type mismatch (including
non-UTF-8 string literals — every literal of a set binding checks); a selected field also
projected; a degenerate literal set (a `Many` of zero or one literals — the
empty set selects nothing and the singleton is the equality spelling); a
duplicate literal within one binding's set (the set is canonical — sorted,
duplicate-free); FD with >1 interval position, interval not in
final position, or determinant width overflow; IND whose target projection matches no key
of the target (or, with an interval position, no pointwise key carrying it) —
the same line covering a window's target projection, the rule being one;
an interval-typed position in a window projection (the v0 refusal above);
duplicate statements (identical normalized sides and form — write it once), where
two FDs over one field *set* are duplicates regardless of projection order (the
order shapes only the determinant, and key resolution is by set) and two
spellings of one literal set are one statement;
a statement referencing an interval position against a scalar position (that is the
type-mismatch case, called out because it is the one migration authors will hit);
an interval position on a containment with a closed side (the v0 refusal above);
a closed-target projection that is not the synthetic id (no key matches — the
handle is the one probe-able identity); a statement between constants that the
ground axioms refute (containment and window forms alike).
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

## Formal claims and runtime evidence

The obligation ledger replaced this chapter's prose theorem-to-evidence table: each Lean premise, its exact Rust discharge site, and the instrument that watches the seam is one machine-listable row of `Bumbledb.Bridge.ledger`.
It lives in `lean/Bumbledb/Bridge.lean`, whose every row carries a term-level theorem reference, so a renamed or deleted theorem fails `lake build`.
The Rust and docs half is grep-checked by `scripts/spec-census.sh` (mechanism and instrument tokens against `crates/` and `fuzz/`, `lean/` citations in these docs against the tree), run by `scripts/lean.sh` and CI's lean lane.
