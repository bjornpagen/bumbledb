# Cross-Doc Seams Audit — Architecture as a Whole

Scope: every doc in `docs/architecture/`, the hardware reference, the full Free Join
paper source, and git history. Findings are ordered most-severe first. Each names the
docs whose seam breaks and asks the concrete question the owner must answer.

---

## 1. **Aggregation's fold domain is undefined, and the two candidate readings are respectively wrong-for-ledgers and self-contradictory** — [blocker]

`20-query-ir.md` says "The logical solution of a query is a **set of variable
bindings** ... Existential variables never multiply output" and, two bullets later,
"each group folds the set of satisfying bindings." `30-execution.md` D3 says aggregate
sinks fold "grouped by the non-aggregated finds." Neither doc says whether the folded
set is (a) the set of *full* bindings over all query variables, or (b) the set of
bindings *projected to the find terms*. Under (b), two postings of amount 100 to the
same account collapse to one element and `Sum` returns 100 — silently wrong balances,
the exact query family `00-product.md` stakes the thesis on. Under (a), `Sum`/`Count`
results change depending on which existential variables an atom happens to bind, and
the sentence "existential variables never multiply output" is false for aggregates;
additionally D2's optimization ("when the remaining plan suffix can only multiply
witnesses ... the executor skips the subtree") must be explicitly scoped to
projection sinks or it drops multiplicity that reading (a) requires.
**Question:** For `finds: [account, Sum(amount)]` over `Posting(id, account, amount)`
joined with `PostingTag`, what exactly is the multiset of values Sum receives, and
which doc will state the rule and its interaction with D2's subtree skip?

## 2. **Database-generated Serial values break `insert(fact)`'s set-semantic contract** — [blocker]

`10-data-model.md` defines `insert(fact)` as "an idempotent no-op if the fact exists,"
and separately defines Serial as "database-generated monotonic u64 sequences per
declared serial field" (app-supplied only during ETL). For a serial-bearing relation
the app cannot construct the full fact — the DB assigns the serial at insert — so two
identical `insert` calls produce two *different* facts and insert is not idempotent;
`40-storage.md`'s `M | relation_id | fact_hash` membership check (blake3 of
`fact_bytes`, which includes the serial field) can never match a second insert. No doc
defines the write-side representation of a to-be-inserted fact with a generated field,
whether insert returns the assigned serial, or what `delete(fact)` takes for such
relations. **Question:** What is the signature and idempotence semantics of insert for
a relation with a Serial field — and which doc owns the public write-side fact
representation?

## 3. **Residual predicate evaluation has no owner — 30 and 40 each tell a partial story that leaves the common case uncovered** — [blocker]

`20-query-ir.md` puts `predicates: Vec<Comparison>` in the IR (including var-vs-var
comparisons across atoms). `40-storage.md` handles only one case: "Filters push down
into the image build *when an image is being built anyway*" — filtered views are
query-local, only unfiltered images cached. `30-execution.md`'s execution loop
(iterate cover, probe siblings, recurse) has no predicate-evaluation step at all; its
only mention of predicates is D4's "NEON kernels for fixed-width predicate scans and
survivor compaction," which implies executor-side batch filtering without ever
specifying when it runs. Unspecified: (a) the warm-cache case — image already cached
unfiltered, so no build to push into; where is `amount > 0` evaluated and from whose
arena does the survivor list come; (b) cross-atom comparisons (`x < y` from two
relations), which can only run after both are bound — no doc places them in the plan;
(c) how COLT's "vector of offsets into the base columns" and "root iterates the base
table directly" compose with a filtered view rather than the cached image. The paper
(§2) assumes selections are already pushed to base tables — bumbledb has no doc
performing that lowering. **Question:** Which doc owns the normative predicate
placement rule (per-atom filters → filtered view at node entry; cross-atom residuals →
earliest node where both sides are bound?), and what does the executor's node loop
look like with that hook in it?

## 4. **Point lookups by unique key — a headline workload item — have no sub-linear execution path anywhere in the architecture** — [blocker]

`00-product.md` lists "point lookups by unique key" as core workload and success
criterion 2 requires beating SQLite on the ledger suite; `50-validation.md` includes
"membership point-lookups via unique keys" as a benchmark family at 10⁵–10⁷ facts.
But the only read path any doc specifies is Free Join over columnar images: a
fully-bound single-atom query executes as a full relation scan (COLT root "iterates
the base table directly") plus filter. `40-storage.md`'s `U` and `M` namespaces could
answer point lookups in O(log n) LMDB gets, but `30-execution.md`'s planner has no
access-path concept and 40 explicitly deleted "always-on per-field value accelerators"
without exempting the guards that already exist. SQLite answers the same lookup with
one B-tree descent; an O(n) scan per lookup loses that family by orders of magnitude.
**Question:** Is scan-and-filter the accepted execution for unique-key point lookups
(then say so and defend it against criterion 2), or does the planner get a
guard-probe access path — and which doc owns that decision? (Same question, weaker
form, for time-range scans over unsorted images.)

## 5. **The result contract and the host-facing API surface have no owner document at all** — [design-gap]

A query's life has no specified ending: no doc defines what the caller receives —
result schema/ordering of find terms, whether interned strings come back as ids or
decoded bytes, the return type of an aggregate query, or how `Param` values are
supplied and type-checked at execution (Params carry only `ParamId`; validation must
infer their types from usage — unstated). `30-execution.md` says "the result buffer
is the single sanctioned allocation site (and callers can provide one)" and
`40-storage.md` says "the dictionary decode path allocates only in the caller's
result buffer," but neither defines the buffer's format or who decodes intern ids
(and when a string *literal/param* is translated to an intern id — validation or
per-execution dict lookup). Likewise the environment lifecycle (create vs open, LMDB
map size at "fits in RAM" scale, close semantics) appears in no doc; `10-data-model.md`
only says open compares fingerprints. **Question:** Which new or existing doc owns
(a) env lifecycle, (b) the write surface, (c) params, and (d) the result
representation and decode story?

## 6. **The IR legalizes atom shapes the paper explicitly assumes away, and no doc owns the lowering** — [design-gap]

`20-query-ir.md`: "Repeating a variable within one atom is legal and means same-fact
equality," and atom bindings may be `Literal(Value)`. The paper (§2) states the
opposite precondition: "we assume that the selections are pushed down to the base
tables ... in particular, all variables in the atom R_i(x_i) are distinct." Free
Join's plan validity, GHT schema derivation, and probe-key construction all assume
distinct variables per atom. So repeated-var atoms and literal bindings must be
lowered to selections over a filtered view *before* planning — a step no doc
specifies (`40-storage.md`'s pushdown paragraph is about image building, not about
who rewrites the atom). **Question:** Which layer normalizes an atom with repeated
variables / literal bindings into (filtered-source, distinct-vars) form, and is that
form what the planner's statistics and `binary2fj` consume?

## 7. **Plan shape: `binary2fj` needs a left-deep input, bushy plans need materialization, and D3 forbids materialization — the DP's output space is never constrained** — [design-gap]

`30-execution.md` adopts "a cost-based **binary** left-deep plan → binary2fj →
factor()" but describes the planner only as "exhaustive DP (Selinger-style) over the
atoms." The paper handles bushy optimizer output by decomposing it into multiple
left-deep plans with materialized intermediates (§2), and reports materialization as
its main bottleneck (§5, §6) — but bumbledb's D3 ("Aggregation never materializes the
join") plus the zero-allocation contract leave no sanctioned home for intermediate
materialization. If the DP is restricted to left-deep plans the problem vanishes, but
no doc says so. **Question:** Is the DP left-deep-only by design (record it and its
lost alternative), or do bushy plans exist — and if so, which doc specifies
intermediate materialization and its allocation story?

## 8. **Build-phase GHT schema derivation and cover-set enumeration are unowned, and dynamic covers conflict with the build-time "drop the trailing []" rule** — [design-gap]

The paper's build phase (§3.3) computes each relation's trie schema from the plan's
subatom partitioning, including the optimization "if the last subatom is the cover of
its node, drop the last []" — a *build-time* designation of the cover. §4.4's dynamic
cover choice ("first find **all** covers for each node") makes the cover a *runtime*
choice, so which subatom gets the vector-forever last level is ambiguous when a node
has multiple covers. `30-execution.md` adopts both ("last trie level stays a vector
forever" and dynamic covers) without saying who computes trie schemas, who enumerates
the cover sets, or what `ValidatedPlan` actually contains (nodes, subatoms, cover
sets, trie schemas, field→column mappings?). **Question:** Which doc owns
`ValidatedPlan`'s contents — specifically per-relation trie schemas derived per §3.3
and per-node cover sets per §4.4 — and what is the rule for the trailing-[] drop when
a node has several covers?

## 9. **The image cache has no eviction story, which contradicts 40's own steady-state heap claim** — [design-gap]

`40-storage.md`: the cache is keyed by `(relation_id, field_scope, storage_tx_id)`
and "stale-generation images fall out by key" — but falling out *by key* only means
new lookups miss them; nothing ever removes old-generation entries from the
environment-scope map, and `Arc` clones held by long read transactions keep them
alive regardless. After N commits the map holds up to N generations per relation,
violating the same doc's "the steady-state process heap is images + LMDB's mmap +
a constant." Also unstated: two concurrent readers at the same tx-id can race to
build the same image (the stated invariant covers only *sequential* transactions),
and whether the winner's instance replaces or coexists with the loser's.
**Question:** What is the eviction rule (e.g. drop all keys with tx_id < newest on
insert? refcount?), and does the build race resolve to a single shared instance?

## 10. **COLT arenas owned by prepared queries scale with data size, contradicting the memory-discipline model** — [design-gap]

`30-execution.md`: "All scratch — ... COLT arenas, sink state — comes from per-query
reusable arenas owned by the prepared query or the transaction context," and COLT
forcing "allocates within the arena, proportional to distinct keys." Distinct keys
can be O(relation), so each prepared query's retained arena grows to data scale; an
app holding dozens of prepared queries retains dozens of data-sized arenas forever.
`40-storage.md`'s target — "steady-state process heap is images + LMDB's mmap + a
constant" — doesn't budget for this, and the phrase "prepared query *or* the
transaction context" names two different owners with different lifetimes without
choosing. This also decides whether one prepared query can execute concurrently on
two threads (arena implies exclusive `&mut`). **Question:** Who owns execution
scratch, what bounds its retained size, and is concurrent execution of one prepared
query supported?

## 11. **The image cache key story disagrees between 30 and 40, and `field_scope` undermines the "tiny key space" argument** — [design-gap]

`30-execution.md` D1: images are "built once per (relation, tx-generation)."
`40-storage.md`: the key is "(relation_id, field_scope, storage_tx_id)" — a third
component 30 omits, using a term ("tx-generation" vs "storage tx id") defined only in
40. With `field_scope` in the key, two queries touching different column subsets of
the same relation build and cache *disjoint* images (no sharing, double decode,
double memory), while 40 simultaneously argues "only unfiltered images are cached,
keeping the cache key space tiny and hit rates high" — field-subset keying makes the
key space per relation exponential in arity, not tiny. No doc says who computes a
query's field scope or whether scopes are widened (e.g. always all columns) to force
sharing. **Question:** Is an image always all-columns (then delete `field_scope` from
the key and fix D1's wording), or per-scope (then who canonicalizes scopes and what
is the sharing/hit-rate story)?

## 12. **Plan invalidation on statistics change makes "steady state" unreachable for any workload that ever writes** — [design-gap]

`20-query-ir.md`: "statistics changes invalidate plans, not validation."
`30-execution.md`: statistics include "per-relation row counts (maintained on write,
exact)" — so *every commit* changes statistics, invalidating every cached plan; the
next execution replans (DP + binary2fj + factor + validate), which allocates,
directly against "executing a prepared query performs zero heap allocations in
steady state" for the product's own read-after-write workload. No doc defines the
staleness policy (relative-change threshold? generation pinning? explicit re-prepare?).
Also unspecified: what inputs the "per-filter survivor estimates from cheap
heuristics" actually consume, given "a statistic that isn't real doesn't exist in the
struct" rules out histograms. **Question:** What is the plan-staleness rule, where
does replanning's allocation live relative to the CI allocation gate, and what are
the filter heuristics' real inputs?

## 13. **The delete path is unspecified in storage** — [design-gap]

`10-data-model.md` makes `delete(fact)` half of the entire mutation algebra, and FK
`Restrict` semantics exist specifically to guard deletes. But `40-storage.md`'s write
path section describes only inserts ("Per inserted fact: one `F` put, one `M` put,
guard puts..."). Never specified: how delete locates the row (fact hash → `M` → row_id),
which namespaces get deletes (`F`, `M`, `U`, `R` entries for the fact's *outgoing*
FKs), how the Restrict check probes the `R` prefix for *incoming* references, its
interaction with same-transaction deletes of referencing facts, and counter/stat
maintenance on delete. **Question:** What is the per-deleted-fact storage operation
list, and does a Restrict violation fail the operation or the transaction?

## 14. **ETL is the schema-change story in three docs and is specified in zero** — [design-gap]

`00-product.md` ("Migrations (ETL into a new database is the schema-change story)"),
`10-data-model.md` ("schema change = ETL into a new database with the new binary";
"ETL may supply explicit values" for serials), and `40-storage.md` ("Bulk load sorts
by key and uses LMDB append mode") all lean on ETL, but no doc specifies: how to
enumerate all facts of a relation out of the old database (there is no scan-all query
surface — results are sets with no ordering, through a result buffer of unspecified
format), how explicit-serial insertion interacts with constraint checks and the
high-water mark, or how bulk load is exposed. Backup/restore is absent from every
doc — at minimum a sentence declaring it out of scope (file copy of the LMDB env?)
is owed. **Question:** Which doc owns the export surface (full-relation scan) and
the bulk-import surface that the migration story presumes?

## 15. **There is no error model anywhere** — [design-gap]

Error handling appears only as fragments: `20-query-ir.md` "typed errors" at the
validation boundary; `20` "Sum uses checked overflow → error" (a *runtime* error —
what happens to the sink, the transaction, the result buffer?); `10-data-model.md`
fingerprint "mismatch is a hard failure"; FK/unique violations have unstated
reporting; `40-storage.md` covers only whole-transaction atomicity. No doc defines
the error taxonomy, whether a failed operation poisons the write transaction, or the
panic-vs-Result philosophy (relevant to the allocation contract: error paths that
format messages allocate). **Question:** Which doc will own the error model — at
minimum: validation errors, runtime query errors (overflow), constraint violations,
and environment/open errors — and the atomicity consequence of each?

## 16. **The SQLite oracle as specified cannot check aggregate queries** — [design-gap]

`50-validation.md`: "Every benchmark and golden query is executed against SQLite with
`SELECT DISTINCT`, and Bumbledb's result set must equal SQLite's exactly." That
methodology is well-defined only for projection queries. For the ledger's
balance-style aggregates — present in the IR "from day one" (`20-query-ir.md`) and in
50's own query families — the SQL translation depends on the answer to finding 1:
`SUM` over distinct full bindings requires `SUM` over a `SELECT DISTINCT <all bound
vars>` subquery, not `SELECT DISTINCT account, SUM(...)`. Until the fold-domain rule
exists, the oracle translation for aggregates is unwritable, and 50 doesn't
acknowledge the case. **Question:** What is the normative SQL template an aggregate
IR query translates to for oracle comparison (and does the same template define the
differential reference engine's fold)?

## 17. **Multi-process access is neither promised nor excluded** — [clarification]

LMDB (and `heed`) natively supports multiple *processes* — many readers plus one
writer across process boundaries. `00-product.md`'s non-goals exclude "Server mode"
and "Multiple writers" but say nothing about two app processes opening the same
environment; `40-storage.md` says "Single writer, many readers, inherited from LMDB,"
which a reader can take as multi-process support. The environment-scope image cache
is process-local, so a second process silently doubles image memory (correct but
unbudgeted), and single-writer enforcement across processes relies on LMDB's lock
semantics no doc discusses. **Question:** Is concurrent multi-process open of one
database supported, tolerated-but-unoptimized, or forbidden — and where is that
recorded?

## 18. **EXPLAIN's "actual cardinalities" require the hot-path counters that the same doc bans** — [clarification]

`30-execution.md` promises "`EXPLAIN` (plan + per-node estimated vs actual
cardinalities + cover choices) ... from day one" and, two paragraphs later, that
release builds have "no counters accumulating on hot paths." Per-node actuals and
per-node-entry cover choices *are* counters on the hot path. Presumably EXPLAIN is a
separate instrumented execution (compile-time feature? always-on but batch-granular
counters?), but no text says so, and "cover choices" can differ per parent binding —
what does EXPLAIN report then? **Question:** Is EXPLAIN a distinct instrumented run
of the same plan, and what granularity of actuals is it committed to?

## 19. **"The value types' total order" is asserted for Min/Max but never defined, and 10 removes it for strings** — [clarification]

`20-query-ir.md`: "Min/Max over the value types' total order" and validation rejects
"comparisons over non-orderable types" — but no doc enumerates which types are
orderable. `10-data-model.md` explicitly says string ordering "not supported" (so
`Min(string_var)` and `Lt` on strings must be rejected — unstated), and is silent on
whether Enum is ordered (by declaration order?), whether nominal Serials admit
`Lt`/`Min` (meaningful as insertion order, dubious nominally), and Bool. `40-storage.md`'s
order-preserving encodings answer the *storage* question, not the *semantic* one.
**Question:** Which doc owns the orderability table (type → orderable? / Min-Max-able?),
and are Enum and Serial in or out?

## 20. **Both README.md and 40-storage.md cite a post-mortem artifact that does not exist in git history** — [clarification]

`docs/architecture/README.md` ("the full post-mortem review is in `todo/` at
`1b65ae8^`") and `40-storage.md` ("post-mortem `todo/20`–`todo/24` at `1b65ae8^`")
point at `todo/`, but no commit reachable from any ref contains a `todo/` directory
(verified by sweeping `git rev-list --all` with `ls-tree`; `1b65ae8^` contains only
`crates/`, `docs/`, `fuzz/`, `scripts/`, and config files). The evidence base that
justifies the storage layout ("80% of traced execution", "values stored 4–6×") is
currently unrecoverable from the repo. **Question:** Where does the 34-file
post-mortem actually live (uncommitted local dump? another machine?), and can it be
committed or the citations corrected before the docs' load-bearing claims become
unfalsifiable?

## 21. **Success criterion 2 depends on a feature the README marks unscheduled** — [clarification]

`00-product.md` criterion 2: "Beats SQLite on the ledger benchmark — that suite, not
JOB, is the ratchet." `50-validation.md`'s ledger families include "balance-style
aggregates by account and instrument *(when aggregate execution lands)*," and
`README.md`'s OPEN list says aggregate execution phasing "is unscheduled." So the
ratchet's most thesis-central family has no scheduled existence, and nothing states
whether criterion 2 can be claimed on the aggregate-free subset in the interim — the
exact "benchmark quietly becomes the thesis" failure mode 00 warns about, inverted.
**Question:** Does the ledger ratchet gate anything before aggregate execution lands,
and is a partial-suite pass allowed to count?
