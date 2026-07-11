# PRD set — the algebra pass: one logic, three confinements

This directory was the complete, ordered work plan for the algebra phase: the
interval algebra cutover, the rules-shaped query IR, the temporal completions,
and the borrowed/typed surface. It **followed** the correctness-and-elegance
campaign (`docs/prd/`). **The set is closed** — every content PRD landed and
retired; this file remains as the record (the principle, the policy, the phase
ledger, the refusals). When this record and an architecture chapter disagree,
**the chapter wins**.

## The organizing principle, applied to the letter

The house axiom (`00-product.md`, Brooks → Pike → Raymond → Torvalds): **the
biggest lever is the shape of the data, not the cleverness of the code.** When a
case shows up that wants a branch, a flag, or a mode, the first question is what
representation would make the case inexpressible. This set is that principle run
at the engine's own vocabulary, and every PRD names its representation move:

- **Choose the coordinates** (Dijkstra EWD831, homogeneous coordinates): the
  Allen mask replaces a growing operator vocabulary with a coordinate system in
  which every interval-pair predicate that will ever exist is one value (PRDs
  03–04); the blessed ray makes "unbounded" a point of the representation, not a
  sentinel hack (PRD 02).
- **Make illegal states unrepresentable** (Minsky): the schema-as-type and
  `Db<S>` make cross-schema fact confusion a compile error (PRD 14); `fresh`
  stays u64-only and writable because both are theorems of the update idiom, now
  recorded (PRD 01).
- **Parse, don't validate** (King): declaration errors surface as the typed
  `SchemaError` at open, never as a runtime panic in a memoized initializer
  (PRD 14); a point literal at the domain's ceiling is rejected at validation,
  never silently unmatched (PRD 02).
- **Reify control flow as data** (SICP ch. 4): OR is never an execution node —
  a query is a *set of rules* and disjunction is data three ways: a mask inside a
  predicate, a set inside a position, rules at the top (PRDs 05–08). The tangled
  middle is refused; DNF lowering recovers it as rules.
- **One mechanism, N callers** (the repo's own anti-probe precedent): the
  coverage judgment's sweep and `Pack`'s finalize become one primitive (PRDs
  11–12); the exclusivity theorem the checker enforces is the same fact the
  executor uses to elide cross-rule dedup (PRD 08).
- **The limit** (Brooks, essential vs accidental): the Refusals section below
  records every boundary where a representation would cost more than it saves.
  Each refusal names its modeling answer; none of them is a gap.

## Vocabulary discipline (binds every PRD)

Dependency-theory and type-theory names only. The register: *statement*,
*functionality/key (FD)*, *containment (IND)*, *judgment*, *guard*, *reverse
edge*, *rule*, *head*, *fresh*, *measure*, *denotation*, *arm*, *theory*,
*model*. Banned as identifiers or concepts: *serial* (dead — PRD 01 landed), *unique*,
*foreign key*, *primary key*, *constraint*, *cascade*, *IN* (the op is
membership, ∈), *UNION ALL* (there is one union; it is set union). The 13 Allen
basics keep Allen's names; `Pack` keeps Snodgrass's.

## Policy (read before executing any PRD)

1. **A PRD is a work-organizational unit, not an atomic passing-code state.**
   Never write a transitional shim, a compatibility alias, or a feature flag.
   Rip the old thing out and cut directly to the end state; downstream breakage
   is the next PRD's job. Zero backwards compatibility is an axiom
   (`00-product.md`), not a risk.
2. **Passing criteria are typed.** `[shape]` — checkable by reading or grep the
   moment the PRD lands. `[test]` — unit tests written in this PRD. `[gate]` —
   holds when the campaign closes: `cargo fmt --all --check`, `clippy --workspace
   --all-targets -- -D warnings`, `cargo test --workspace`, `scripts/check.sh`.
3. **No migrations, ever.** Stores are regenerated or ETL'd; no PRD writes
   conversion code.
4. **Every measured claim waits for the bench.** New operators exist unearned
   until PRD 16's family runs green under the two-oracle stamp; no performance
   number is cited before then.
5. **Conflict protocol:** if executing a PRD reveals the architecture docs are
   wrong or silent, stop and record the conflict in the PRD file.
6. **Doc amendments land in the same change** (architecture README rule 5).

## The PRDs

Phase A — the atom — landed whole and retired (01 — `fresh` —, 02 — the
ray —, 03 — the Allen mask —, and 04 — the configuration kernel); its
rulings live in `10-data-model.md`, `20-query-ir.md` § the Allen
operator, and `40-execution.md` § vectorized execution (the sanctioned
kernel shapes).

Phase B — the logic — landed whole and retired (05 — the rules-shaped
IR —, 06 — DNF lowering —, 07 — rule execution —, 08 — the exclusivity
elision —, and 09 — the chase, per rule); its rulings live in
`20-query-ir.md` § the query shape and § the input predicate grammar,
and `40-execution.md` § the rule loop — one head, one sink, the spanning
seen-set as ∪ —, § set semantics — the rule-disjointness elision, whose
witness form and consumers are recorded there and in
`30-dependencies.md`'s third-consumer line —, and § planner — the
per-rule chase and the rule-subsumption witness, with the refused
NP-hard general form.

Phase C — the temporal completions — landed whole and retired (10 — the
measure —, 11 — the sweep: one walk, two callers —, and 12 — `Pack`, the
coalescing fold); its rulings live in `20-query-ir.md` § the measure and
§ aggregation (`Pack`'s relation shape, head shape, and the multi-`Pack`/
nesting/`Gaps` refusals), `10-data-model.md`'s one-arithmetic sentence,
`40-execution.md` § set semantics and § the rule loop (the `Pack` sink
and its union fold), and — for the shared segment sweep,
`interval/sweep.rs`, whose two continuations are the coverage judgment
and `Pack`'s finalize — `30-dependencies.md` § enforcement and
`50-storage.md` § commit step 3.

Phase D — the surface (**landed early**: implemented as `docs/prd/22` before
this set began execution; the 13/14 reconciliation records are retired — done
items leave the ledger, and 14's residual, the `Theory` rename, landed with 01).

Phase E — the earning — landed whole and retired (15 — oracles and the
generator: rules→UNION in the translator, the naive model's direct
rules/tree/mask/measure/`Pack` evaluations, the converse-property and
error-parity lanes, `Pack`'s naive-only routing through the enumerated
inexpressible set, the boundary-shape ladder on every interval draw,
and the str-extrema roster check; and 16 — the calendar family: the
second theory joins the protocol whole — schema, valid-by-construction
Zipfian corpus, the six representation-named families plus the
anti-probe variant, the hand-coalesce mirror for `Pack` reported
translator-unpaired, the elision-delta sub-measurement, the
`bytes<32>` content-hash column, the witnessed-write row, and the
calendar verify lanes under the stamp); the rulings live in
`60-validation.md` — § the calendar benchmark for 16.

Phase F — the write side, the type ledger, and the surface ruling (17 —
identity bytes — landed whole and retired: `bytes<N>` replaced variable
`bytes`, the dictionary went str-only and untagged, and the rulings live in
`10-data-model.md` § the type layer and § interning, `50-storage.md`, and
`00-product.md`'s census sentence; 18 — the generation witness — landed
whole and retired: `Db::write_from` takes the snapshot as the witness and
aborts with `GenerationMoved` on a moved state-changing generation, and the
rulings live in `70-api.md` § conditional writes, `00-product.md`'s
deleted-vocabulary rows and concurrency sentence, and `30-dependencies.md`'s
runs-before-judgment cross-reference; 19 — derived relations, the doc unit —
landed whole and retired: the view story is canonized in `10-data-model.md`
§ derived relations — composition, materialization under statements, the
arithmetic-agreement limit, both refusals — with the vocabulary rows in
`00-product.md` and the composition idiom named in `70-api.md` § host-side
sugar; 20 — the data surface, ruled — landed whole and retired: schemas are
code (the open-ended theory grammar with its categorical boundary), queries
are data (the permanent surface ruling), the adversarial panic sweep
enforcing the trust-boundary law — whose founding find, hostile predicate
nesting exhausting the stack, closed as the iterative `MAX_PREDICATE_DEPTH`
boundary guard — the declaration-order id constants and the `Theory`
manifest, and `ir::render`, the read-side syntax; the rulings live in
`20-query-ir.md` — the surface ruling, § validation boundary (the
trust-boundary law), § the renderer — and `70-api.md` § the two surfaces,
§ id constants and the manifest, § anticipated bindings (the punt record),
with `00-product.md`'s superseded OPEN item and non-goals note; and 23 —
the query notation, set-builder promoted from the schema grammar — landed
whole and retired: `query!(Theory { (head) | body; ... })` in the new
quarantined downstream crate `crates/bumbledb-query` (hand-rolled parser,
`syn`-free; hosts depend on it, the engine never depends back), punning
law B with the ambiguity error spanned at the second occurrence, name
checking through the id constants (a typo'd relation or field is a compile
error at the query literal), compile-time lowering to the `ir::Query`
value, the round-trip goldens pinning `render(lower(text))` byte-exact
against the normalized text, and the compile-fail roster (typo'd
relation/field, ambiguous punning, `?param` in a head, `:-` anywhere);
the rulings live in `20-query-ir.md` § the query notation and
`70-api.md` § host-side sugar, with `00-product.md`'s dependency-law
sentence).

Phase G — the intuition — landed whole and retired (21 — the cookbook:
`docs/cookbook.md`, the 20-recipe roster written against the whole set's
surface — foundations, structure, time and tilings, the write side, and
the five-gravestone anti-recipe block, each recipe one `schema!` block
with theorem-naming comments and queries in the 23 notation — rot-proofed
by `crates/bumbledb-query/tests/cookbook.rs`: every schema compiled and
validated as a token-pinned duplicate, the roster enumerated with a count
assertion, the teachable queries round-tripped through `query!` + prepare
against `ir::render`; the architecture README's document table carries its
row, reader: the owner and any agent writing a theory).

Dependency spine, discharged: Phases A (01–04), B (05–09), C (10–12), and
E (15–16) landed whole; 13/14 landed (residual landed with 01); 17 landed
(its adversarial digest rows are in the generator's target ledger,
inherited by 15's families); 18 landed; 19 landed; 20 landed (its sweep
and renderer target the rules-shaped IR); 23 landed (its notation is the
render grammar, round-trip-pinned); 21 landed last, written against the
whole set's surface. E closed the measured half of the set (16 carries the
`bytes<32>` content-hash column 17 owed it and the witnessed-write row 18
owed it); G closed the set itself. **No content PRDs remain in this
directory** — what stays is the record: the organizing principle, the
vocabulary discipline, the policy, the phase ledger above, and the
refusals below (cited by the cookbook's anti-recipes and by the
architecture chapters).

## Refusals (recorded with derivations — do not re-litigate)

- **`Intersect` as an operator.** Intersection of two rules over one head *is*
  conjunction — write the join. An operator would be a name for something the
  IR already is.
- **General difference (subtracting a whole subquery).** Atom-negation
  (anti-probes) covers every sighted case. *Trigger:* a real query no
  anti-probe can express.
- **OR tangled mid-rule across atoms.** A cross-atom disjunction poisons filter
  pushdown and selectivity. It is not refused expressiveness — DNF lowering
  (landed; `20-query-ir.md` § the input predicate grammar) recovers it as
  rules, capped. OR is data or it is nothing.
- **Enum order comparisons.** Declaration order is an encoding, not a
  semantics; an order op would make variant reordering a silent meaning change.
  Modeling answer: an explicit rank field, or a relation split.
- **Str/bytes order, prefix, substring.** Intern ids are identity, not order;
  order ops would demand an ordered dictionary — a subsystem. The host sorts;
  search engines are a different product.
- **Endpoint accessors on intervals.** The denotation owns intervals; exposed
  endpoints invite user-space arithmetic and half-open off-by-ones. The mask
  says everything endpoints could.
- **Arithmetic beyond the measure.** `Duration` is the one operation the
  point-set denotation defines (its measure). Everything else is computation
  and belongs to the host.
- **A `Gaps` operator.** Free time is a two-line host walk over sorted `Pack`
  output. *Trigger:* a measured need `Pack` + host cannot meet.
- **Interval `Min`/`Max`** (no total order — standing ruling) and **`Min`/`Max`
  over str/bytes** (intern words are not order-preserving; the roster rejection
  holds — `AggregateInputType`, pinned for str and bytes alike).
- **Floats and embeddings.** Permanently out; fixed-point i64 is the modeling
  answer for scores and money; vector search belongs to other engines.
- **A large-object storage class.** Facts are fixed-width; big payloads are
  refs to external storage. Content churn is recorded on the dictionary-GC OPEN
  item as its trigger profile.
- **Recursion, still.** Rules make the IR a non-recursive Datalog program —
  deliberately one step short of the fixpoint. The OPEN item stands; rules are
  its landing pad, not its arrival.
- **Order operations on `bytes<N>`.** A digest's lexicographic order is an
  encoding artifact; admitting it makes hash-function choice semantically
  visible. Identity only (Eq/Ne, membership). The guard B-tree still sorts
  them — sortedness is the index's need, not a query semantics.
- **A raw-integer witness API.** `write_from` takes the `Snapshot`, never a
  generation number: a snapshot is evidence, an integer is a claim (parse,
  don't validate). Landed; recorded in `70-api.md` § conditional writes.
- **A named-view registry in the engine.** A view is a host function
  returning atoms; a registry would be a second schema with none of the
  theory's guarantees. Landed; recorded in `10-data-model.md` § derived
  relations.
- **Arithmetic-agreement statements** (a derived column equaling a
  computation over its sources). Outside the ∀∃ vocabulary by the acceptance
  gate; host discipline plus offline re-derivation, with the trigger
  recorded in `10-data-model.md` § derived relations.
- **A typed query builder, and any engine-side query ergonomics.** Queries
  are data (PRD 20's ruling): builders bind construction to Rust closures
  and generics — exactly what a foreign host cannot invoke — and the
  roster's typed errors re-provide the checking for every caller equally.
  Sugar is downstream-package territory, in any language, lowering to IR.
- **JS/N-API bindings, now.** Pure anticipation, recorded with their
  quarantine shape in PRD 20; zero deliverable in this set, and no engine
  decision may lean on their existence.
- **Borrowed query grammar (Datalog's `head :- body`).** Considered and
  rejected (owner ruling 2026-07-10): this engine's statements are already
  statements *about* queries, so the query surface must be the statement
  surface's query side, promoted — `(head) | body;`, one notational family,
  schema to query. `:-` does not parse, anywhere; the refusal is pinned by
  compile-fail (`20-query-ir.md` § the query notation).
- **Punning alternative (A) — same-name-same-variable across the clause.**
  Set-builder-honest, and refused for exactly what it reads: it makes joins
  silent. Every relation names its key `id`, so a forgotten rename silently
  unifies a `HolderId` with an `AccountId` and the roster cannot object
  (structurally both u64) — under (A) a *wrong* query compiles, validates,
  and runs. Law (B) stands: a pun is clause-local projection; the same
  punned name in two atoms is a macro error at the second occurrence; joins
  are explicit `field: v` on both ends. Illegal states unrepresentable
  outranks notational purism. *Reverses if:* ambiguity errors prove noisy on
  real theories AND a schema-aware lint becomes possible (it is not today:
  the macro cannot see the theory).
