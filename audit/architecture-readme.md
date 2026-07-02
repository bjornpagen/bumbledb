# Audit: docs/architecture/README.md (doc-rules and OPEN-items index)

Scope: README as the contract about the doc set. Verified against 00-product.md,
10-data-model.md, 20-query-ir.md, 30-execution.md, 40-storage.md, 50-validation.md,
docs/reference/apple-silicon-performance.md, the Free Join paper tex sources, and git
history at and before `1b65ae8`.

## Findings (most severe first)

### 1. **The post-mortem citation is dead: `todo/` does not exist at `1b65ae8^` or anywhere in git history** [blocker]

README rule 4 states: "the full post-mortem review is in `todo/` at `1b65ae8^`."
`git show '1b65ae8^:todo/README.md'` fails ("path 'todo/README.md' does not exist"),
`git ls-tree '1b65ae8^'` shows no `todo/`, and `git log --all -- todo` is empty — the
directory was never tracked at any commit. 40-storage.md compounds this with a specific
citation ("post-mortem `todo/20`–`todo/24` at `1b65ae8^`") backing its central 80%-of-
traced-execution claim. The post-mortem is the evidentiary base for most "why it lost"
paragraphs in the whole doc set, and right now none of it is retrievable; the docs
violate their own referential standard on their single most-cited source.
Question: Where does the post-mortem review actually live (untracked local dir? another
machine?), and will it be committed so `1b65ae8^`-style citations resolve — or will the
citations be rewritten to point at something that exists?

### 2. **The doc set has no public API / embedding document, and the README's doc table doesn't notice the hole** [blocker]

The README calls these docs "the normative design," yet no document describes how an
application opens a database, declares its schema at runtime, begins/commits
transactions, executes a prepared query, or receives results (30-execution mentions "the
result buffer" and "callers can provide one" without ever defining the caller-facing
surface). The OPEN item "Write surface" covers only FK-enforcement timing and a
`replace` convenience — a tiny corner of the missing surface. For an *embedded* database
the API is the product boundary; leaving it undocumented is exactly the "code deciding
it silently" failure mode rule 3 warns against. The doc table (`00`–`50`) should either
gain an entry (e.g. `60-api.md`) or the README must carry an explicit OPEN item for the
entire embedding surface, not just FK timing.
Question: Will a public-API/embedding doc be added to the table, and until then, will
the OPEN list name the full missing surface (open/close, txn API, query execution,
result delivery, error returns)?

### 3. **LMDB — the most consequential decision in the system — records no alternative** [design-gap]

Rule 1: "Every decision records its strongest alternative and why it lost." 40-storage
opens "LMDB is the only durable backend" and 00-product bakes it into the thesis ("one
storage engine (LMDB)"), but no doc anywhere weighs an alternative (redb, sled, a custom
mmap'd file given the fits-in-RAM axiom, SQLite-as-KV, ...). This is especially glaring
because the post-mortem's headline storage finding — hundreds of thousands of LMDB
point-gets were 80% of traced execution — is an indictment of how v5 *used* LMDB, and
re-choosing LMDB deserves the strongest-alternative treatment the rules demand.
Question: What is the strongest alternative to LMDB under the fits-in-RAM,
single-writer axioms, and why does it lose?

### 4. **Free Join itself has no recorded alternative** [design-gap]

00-product declares "take one good algorithm (Free Join)" and 30-execution adopts the
paper wholesale, but no doc records the strongest alternative: plain cost-based binary
hash joins, which for a workload of "point lookups by unique key, FK walks" over
"rarely >10 atoms" (30-execution's own words) are the obvious contender — the paper
itself shows Free Join's wins concentrate on cyclic/skewed queries. 50-validation
quietly acknowledges the tension by adding "a cyclic-ish join for WCOJ honesty" to a
ledger benchmark that otherwise wouldn't exercise WCOJ at all, which is mildly circular.
Rule 1 says if we can't articulate the alternative, the decision isn't made yet.
Question: Why does Free Join beat a boring Selinger-planned binary hash join *on the
ledger workload specifically*, and where is that paragraph?

### 5. **Silent deviation from the paper's CQ definition: repeated variables within an atom** [design-gap]

Rule 2 requires a `Deviation:` block for every paper deviation. The paper's §2
(02-background.tex) states "all variables in the atom $R_i(\bm x_i)$ are distinct";
20-query-ir declares "Repeating a variable within one atom is legal and means same-fact
equality" with no Deviation block and no execution story in 30-execution (a COLT trie
built over base columns has no native same-fact-equality step — it must become a
build-time or probe-time filter). The divergence is fine as a product choice; it is
undocumented as a deviation, which is exactly what rule 2 exists to prevent.
Question: Will 20 (or 30) get a Deviation block stating what the paper assumes, what we
do, and how same-fact equality executes?

### 6. **Self-joins: the paper assumes them away; the IR permits them; no doc addresses them** [design-gap]

The paper (§2) says "we will assume that the query does not have self-joins... if two
atoms have the same relation name, then we simply rename one of them." Bumbledb's IR
freely allows two atoms with the same `RelationId`, and 30-execution's adopted validity
rule — "a node is valid if no two subatoms share a relation" — is stated in terms of
relations, not atoms, so it either wrongly rejects valid self-join plans or needs
atom-identity, and nobody has said which. Self-joins are natural in this schema
(`OrgParent(child, parent)` joined to itself for grandparents). No Deviation block, no
OPEN item.
Question: Are self-joins supported in v0, and does the plan-validity rule quantify over
atoms or relations?

### 7. **Comparison-predicate execution has no home** [design-gap]

The paper assumes "the selections are pushed down to the base tables" (§2). 20-query-ir
allows `Comparison { lhs, rhs }` over arbitrary terms — including two variables bound in
*different* atoms, which cannot be pushed down. 40-storage covers only single-relation
filter pushdown into image builds; 30-execution never says where cross-atom comparisons
evaluate (at first binding of both vars? at the sink?). This is an unmarked deviation
from the paper's assumptions and an unspecified piece of the executor.
Question: Where in the node recursion do cross-atom comparisons evaluate, and does that
deserve a Deviation block against the paper's pushed-down-selections assumption?

### 8. **Concurrency, threading, and multi-process model are unspecified** [design-gap]

40-storage says "single writer, many readers, inherited from LMDB" and puts the image
cache at "environment scope... correctness needs no locks beyond the map's" — but no doc
says whether the environment is shared across threads, whether readers run concurrently
with the writer in one process, or whether two *processes* may open the same database
(LMDB supports it; the in-process image cache and in-memory write-txn counters may or
may not — cross-process, another writer's commit bumps `_meta`'s tx id but nothing
states readers re-read it). Not in the OPEN list.
Question: What is the supported process/thread topology — single process assumed, or
multi-process readers permitted — and where is it written down?

### 9. **No error model anywhere** [design-gap]

Errors are gestured at in four places — "typed errors" at the IR validation boundary
(20), "checked overflow → error" for Sum (20), fingerprint "mismatch is a hard failure"
(10), and insert/delete "report whether they changed state" (10) — but no doc defines
the error taxonomy, whether a mid-query overflow poisons the read txn, whether a
constraint violation aborts the whole write transaction or just the operation, or what
the caller sees. Not in the OPEN list.
Question: Will an error-model section (or doc) enumerate the error categories and their
transactional consequences?

### 10. **Read-your-writes inside a write transaction is unaddressed** [design-gap]

The image cache is keyed by *committed* `storage_tx_id`, and FK `Restrict` checks plus
ordinary application logic (read, then insert) need reads inside an open write
transaction, whose uncommitted data no cached image can contain. Nothing says whether
queries are executable in a write txn (and if so, against what representation) or
forbidden (and if so, how FK checks and app read-modify-write flows work). This sits at
the seam of 40's cache design and the missing API doc.
Question: Can a write transaction execute queries, and what do they see?

### 11. **Serial generation contradicts the insert contract, unresolved** [design-gap]

10-data-model: facts are "full, typed facts," `insert(fact)` is idempotent, and there is
no update — yet `Serial` values are "database-generated monotonic u64 sequences." An
application inserting a new Account cannot supply the full fact (the serial doesn't
exist yet), so either insert returns generated values (a different signature than
`insert(fact)`), or the app pre-allocates ids (unspecified API), and idempotence is
vacuous for serial-bearing inserts (each call would mint a distinct fact). None of this
is specified or OPEN.
Question: What is the insert signature for relations with a serial field, and what does
idempotence mean for them?

### 12. **The delete path is unspecified in storage** [design-gap]

40-storage's "Write path" section specifies puts "per inserted fact" only. Deletion —
which `M`/`U`/`R` entries are removed, how reverse-FK guards are checked for `Restrict`,
whether `F` rows leave `row_id` holes and how the image builder and row counts handle
that — is never described, even though delete is half of the mutation algebra
(10-data-model: "Mutation is delete + insert"). Only the dictionary leak is documented.
Question: What exactly happens in LMDB on `delete(fact)`, and do row_id holes affect
image building or stats?

### 13. **Image-cache eviction and memory bound are hand-waved** [design-gap]

40-storage: "stale-generation images fall out by key" — falling out of a map keyed by
`(relation_id, field_scope, storage_tx_id)` requires an eviction mechanism nobody
specifies; without one the cache grows monotonically across generations, and with
long-lived readers pinning old generations (`Arc`), eager eviction is also wrong. The
memory-discipline target ("steady-state process heap is images + LMDB's mmap + a
constant") depends on this unstated policy. Also unstated: the `field_scope` policy
(per-scope images can duplicate columns across overlapping scopes vs. always building
full-width images).
Question: What evicts stale images (and when), and is `field_scope` per-query-scope or
always full-width?

### 14. **Planner statistics are declared "no fake fields" but the real ones are unspecified** [design-gap]

30-execution promises "per-filter survivor estimates from cheap heuristics" (which
heuristics?) and "Estimated cardinality is the entire cost function" — but with only
exact row counts and unique/FK knowledge, join-cardinality estimation (the actual cost
function) is never defined. This is the kind of hand-wave that becomes
decision-by-code; it is not in the OPEN list. Relatedly, 20's "statistics changes
invalidate plans" leaves invalidation granularity and its interaction with the
zero-alloc prepared-query contract (replanning allocates) unspecified.
Question: What is the join-cardinality estimation formula given the available
statistics, and when exactly do stats changes invalidate a prepared query?

### 15. **The SQLite oracle recipe is underspecified where it matters** [design-gap]

50-validation: results "must equal SQLite's exactly, by value" via `SELECT DISTINCT`.
But no doc defines the value mapping: `U64` does not fit SQLite's signed 64-bit INTEGER;
enums, serials, and interned strings/bytes need a declared encoding; and for aggregates,
`SELECT DISTINCT` does not replicate set-of-bindings folding (SQL aggregates over bags;
bumbledb folds the *set* of satisfying bindings — replicating that needs
aggregate-over-DISTINCT-subquery constructions, which is a different recipe than the
one written down). The oracle is success criterion #1; its construction can't be folklore.
Question: What is the exact bumbledb→SQLite value/type mapping and the SQL construction
used for aggregate queries?

### 16. **The ETL schema-change story has no export path** [design-gap]

10-data-model: fingerprint "mismatch is a hard failure. There is no migration, no ALTER,
no compatibility reader: schema change = ETL into a new database with the new binary."
But the new binary *cannot open* the old database (hard failure), so ETL requires the
old binary to export — in what format, via what API? The schema-change story, presented
as decided, is missing its load-bearing half and is not OPEN.
Question: How does data leave an old-schema database — old-binary export API, a
fingerprint-override read-only mode, or something else?

### 17. **The doc rules themselves don't block two of the post-mortem's failure modes** [design-gap]

Rule 2 gives Deviation blocks a "what evidence would reverse it" clause, but plain
decisions get no reversal criteria — so a wrong decision (rule 1) has no defined exit,
unlike a wrong deviation. Nothing encodes the v5 "namespace with no reader" lesson as a
rule (40 applies it ad hoc: "each was a v5 namespace with no reader"), i.e. no rule that
every specified mechanism must name its consumer — the precise guard against doc-driven
layout transcription. And rule 3 brands silent code decisions the failure mode but
defines no procedure for what happens when implementation discovers a doc is wrong
(success criterion 4 depends on one existing).
Question: Should the rules add (a) reversal evidence for decisions, (b) an
every-mechanism-names-its-reader rule, and (c) the amendment procedure when code and
docs disagree?

### 18. **Set-fold aggregation has a documented-nowhere footgun, and D2's skip is sink-dependent** [clarification]

20-query-ir: aggregation "folds over sets of bindings"; bindings range over *all* query
variables, so `Sum(amount)` is correct only when each contributing fact contributes a
distinct binding (e.g. the posting's serial is bound). A query that omits the
distinguishing variable silently collapses equal amounts — for a ledger database this
deserves an explicit callout. Relatedly, 30-execution D2's existential-subtree skip is
stated for "an already-emitted projection"; it is invalid for aggregate sinks unless the
skipped variables are existential w.r.t. the *binding set being folded*, and the
condition's sink-dependence is unstated.
Question: Will 20 state explicitly that binding sets range over all query variables
(with the Sum footgun example), and will D2 state its validity condition per sink?

### 19. **`M`-table hash collisions are unhandled by specification** [clarification]

40-storage's dictionary does forward lookup "keyed by hash of the raw bytes (with
equality verification on lookup)," but the membership table `M | relation_id |
fact_hash -> row_id` (blake3 of fact_bytes) has no stated verification step. Treating
blake3 collisions as impossible is a defensible engineering position — but it is a
decision, and the asymmetry with the dictionary's verify-on-lookup suggests it hasn't
been made deliberately.
Question: Is `M`-lookup verified against `fact_bytes`, or is blake3
collision-freeness an accepted axiom (write it down either way)?

### 20. **Durability level of commits is unstated** [clarification]

40-storage: "Durability, write atomicity, and reader snapshot isolation come from real
LMDB transactions." LMDB's durability depends on environment flags (`MDB_NOSYNC`,
`MDB_NOMETASYNC`, `WRITEMAP`); the doc never says whether commit means fsync'd-durable
or only crash-consistent. For a database whose "whole crash-consistency story" is LMDB
atomicity, the chosen flag set is part of the design.
Question: Which LMDB durability flags does bumbledb run with?

### 21. **50-validation attributes the ledger schema to 00-product, which contains no schema** [clarification]

50-validation: "The ratchet benchmark mirrors the product thesis (schema from
`00-product.md`'s workload)" — but 00-product describes workload *shape* only; the
schema (`Holder, Account, Instrument, ...`) is defined nowhere but 50 itself. Minor
referential slip; also `20-query-ir.md` cites evidence at `~/Documents/logica`, a
home-directory path outside the repo that future readers can't resolve.
Question: Should the ledger schema be owned by 50 outright (fixing the cross-reference),
and should the Logica findings be captured in-repo rather than by home-dir path?

### 22. **OPEN items have no closure triggers** [clarification]

Rule 3 says OPEN "is a real state, not a failure," and item 1 alone carries a trigger
("Deferred until engine internals are settled"); the other five have none. Without a
what-decides-this trigger per item, the OPEN list can persist unexamined until code
answers by accident — the exact failure rule 3 names. Cheap fix: one trigger clause per
item.
Question: Will each OPEN item state what event or measurement forces its decision?

## Missing OPEN items (complete sweep)

Every literal `OPEN` marker in the docs does appear in the README list (verified: 10's
FK-timing/replace and nominal-domains markers, 20's results-API-convenience and
negation/recursion markers all map to README entries). The list below is what is *open
in fact* but absent from the README:

1. Public API / embedding surface: open/close, schema declaration API, transaction API,
   query execution API, result delivery and buffers (finding 2).
2. Error model and taxonomy, including transactional consequences of errors (finding 9).
3. Threading model and multi-process story (finding 8).
4. Read-your-writes / queries inside a write transaction (finding 10).
5. Insert signature and idempotence semantics for serial-bearing relations (finding 11).
6. Delete-path storage operations and row_id-hole handling (finding 12).
7. Image-cache eviction policy and memory bound (finding 13).
8. `field_scope` image policy: per-scope vs full-width (finding 13).
9. Filter-survivor heuristics and the join-cardinality estimation formula (finding 14).
10. Plan-cache invalidation granularity and its zero-alloc interaction (finding 14).
11. Self-join support and the atom-vs-relation plan-validity rule (finding 6).
12. Execution strategy for repeated variables within an atom (finding 5).
13. Execution placement of cross-atom comparison predicates (finding 7).
14. Oracle value/type mapping to SQLite and the aggregate-comparison construction
    (finding 15).
15. Old-database export path for the ETL schema-change story (finding 16).
16. Vectorization batch size — explicitly deferred to measurement in D4; a legitimate
    defer, but it is an undecided value and belongs on the honest list.
17. LMDB durability flags (finding 20).
18. EXPLAIN output format/surface ("exists from day one" in 30, shape unspecified).
19. Result types of aggregates (e.g. Count's type; Sum over U64 vs I64 domains).

## Decisions without a recorded strongest alternative (rule 1 sweep)

Compliant examples exist in every doc (ledger-vs-JOB, no-PK, type roster,
schema-in-Rust, no-text-language, DP-planner, row-major facts). Missing:

1. LMDB as the storage engine (00, 40) — finding 3.
2. Free Join as the execution algorithm (00, 30) — finding 4.
3. Rust as the implementation language (00) — possibly an owner axiom; if so, say so.
4. Dictionary interning for String/Bytes (10) — vs inline fixed-prefix encoding or
   ordered value storage; the no-ordering consequence is documented, the alternative is
   not.
5. `M`-table (blake3 fact-hash → row_id) membership + row_id indirection (40) — vs
   keying facts by their own bytes with no indirection.
6. Single `_data` database with first-byte namespaces (40) — vs one LMDB database per
   namespace.
7. `heed` as the LMDB binding (40) — minor, but it is stated as decided.
8. Day-one aggregate op set {Sum, Min, Max, Count} (20) — why exactly these four.
9. blake3 for both fingerprint and fact hashing (10, 40) — minor.
10. SQLite as the oracle and the naive reference engine (50) — near-self-evident, but
    the rule says every decision.

## Rule-adequacy verdict (task item 4)

The three rules attack the right failure modes but leave gaps (finding 17): decisions
lack reversal criteria, no rule requires each mechanism to name its reader (the
anti-transcription guard), and there is no doc-amendment procedure when implementation
contradicts a doc. Rule 2 is well-honored where applied (D1–D5 are exemplary), but
rule 2's *coverage* fails silently at the IR layer (findings 5–7) because nobody owns
checking IR semantics against the paper's §2 assumptions — the rules police the
execution doc's deviations, not the query model's.
