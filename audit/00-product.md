# Audit: 00-product.md

Auditor scope: `docs/architecture/00-product.md`, cross-checked against the other five
architecture docs, `docs/reference/apple-silicon-performance.md`, the Free Join paper
(§3, §4), and the post-mortem record (note: the `todo/` dump cited at `1b65ae8^`
does not exist in git history — `git ls-tree '1b65ae8^'` shows no `todo/`; post-mortem
claims below are sourced from doc-internal history statements).

---

## 1. **"Beats SQLite on the ledger benchmark" is not a measurable criterion** [blocker]

Success criterion 2 (lines 62–64) is the project's central performance claim and, as
written, is unfalsifiable. Unspecified: (a) the metric — per-query median latency,
geometric mean across families, or total suite wall time; (b) whether every query family
must win or only the aggregate; (c) warm vs cold — bumbledb's first query after open or
after any commit pays a full columnar image rebuild (`40-storage.md`), so cold/warm
choice decides the outcome; (d) SQLite's configuration — file vs `:memory:`, WAL,
`cache_size`, `synchronous`, whether SQLite gets indexes matching each query family,
whether prepared statements are reused; (e) whether the timed SQLite query includes the
`DISTINCT` that the oracle requires — `SELECT DISTINCT` materially slows SQLite, so
timing with it flatters bumbledb and timing without it compares different semantics;
(f) the machine — "Apple Silicon M-series" spans M1→M4 with ~15% memory-bandwidth
differences per the hardware notes. `50-validation.md` defines query families and data
scale but pins none of these either. The old repo's lesson was that a benchmark quietly
becomes the product thesis; an unpinned ratchet can be quietly re-pinned to whatever wins.
Question: Define criterion 2 exactly — metric, per-family or aggregate, warm/cold, the
canonical SQLite configuration and index set, DISTINCT-in-timing or not, and the
canonical machine.

## 2. **The LMDB pillar has no recorded decision, and its strongest alternative would dissolve half the architecture** [blocker]

The thesis names LMDB as one of three load-bearing bets (lines 10–12), and `40-storage.md`
opens with "LMDB is the only durable backend" — but no doc anywhere records the decision
per README rule 1 ("Every decision records its strongest alternative and why it lost...
If we can't articulate the alternative, the decision isn't made yet"). Given the scale
axiom (fits in RAM, single writer, writes rare), the strongest alternative is obvious and
strong: plain in-memory tables plus an append-only log or snapshot file for durability.
That alternative makes the durable representation *identical* to the paper's execution
environment, deleting Deviation D1, the columnar image cache, its generation-keyed
invalidation, and the entire class of v5 image-rebuild failures the cache exists to
prevent. LMDB may still win (crash-safety for free, multi-process readers, mmap paging)
— but by the project's own rule this decision "isn't made yet," and it is the one the
most machinery hangs from.
Question: Record the LMDB decision: what is the strongest alternative (in-memory +
WAL/snapshot given the RAM axiom), why does it lose, and what evidence would reverse it?

## 3. **"Writes are... comparatively rare" is unquantified, and a global-invalidation design hangs on it** [design-gap]

Line 32 says writes are "comparatively rare" — rare relative to what? The image cache in
`40-storage.md` is keyed on a *global* `storage_tx_id`, so any commit to any relation
invalidates every cached image; the first query after each commit rebuilds every image it
touches. Deviation D1's justification ("after warmup execution is exactly the paper's
environment") is only true if reads-per-write-generation is large. A live ledger
recording postings as they occur (say, ~1–10 commits/sec interleaved with queries) never
warms up and reproduces exactly v5's rebuild-every-query pathology that `40-storage.md`
calls "the old repo's quietest failure." The product doc is the only place that can
settle this, and "comparatively rare" cannot.
Question: What is the design-point write rate and read:write-generation ratio (order of
magnitude) that downstream docs may assume — e.g., "≥100 queries per committed write
generation, bursts of writes are batched into one transaction"?

## 4. **"Latency: interactive" carries no number** [design-gap]

Line 36 says "Latency: interactive. Queries serve application logic, not batch
pipelines." No target is given — 1ms? 100ms? This is not pedantry: the doc must
adjudicate real arguments, e.g. whether a post-commit full image rebuild of a 10⁷-fact
relation (tens of milliseconds of scan+decode) inside a query is acceptable, whether
first-execution COLT forcing spikes are acceptable, and whether an O(n) filtered scan for
a time-range query at the top of the scale envelope is "interactive." Every one of those
is currently arguable both ways because the budget doesn't exist.
Question: Give a latency budget — e.g. "p99 ≤ X ms per prepared-query execution at the
10⁷-fact scale point, including any image rebuild the query triggers" (or explicitly
exclude rebuild spikes and say why).

## 5. **The concurrency and deployment model is missing from the product doc** [design-gap]

Lines 28–29 say "embedded in his Rust applications" (plural) and non-goals exclude
"Multiple writers" and "Async API," but the doc never says: (a) whether query execution
is single-threaded — `30-execution.md` D4 sizes batches to fill "~28 MLP lanes," a
*per-core* figure, silently implying single-core execution, and no doc rules
multi-threaded joins in or out; (b) whether multiple *processes* (several of the owner's
applications) may open the same database concurrently — LMDB supports it, but the
environment-scope image cache, the in-memory counter flushing, and the schema-fingerprint
check all have cross-process implications nobody has examined. An implementer is forced
to guess the threading model of the engine and the multi-process story of the file.
Question: Is query execution single-threaded by design (and is intra-query parallelism a
non-goal), and is concurrent multi-process access to one database in or out of the
envelope?

## 6. **Durability policy is unstated — for a ledger** [design-gap]

The workload is "ledger-like" (line 31) and the storage doc says "LMDB atomicity is the
whole crash-consistency story," but nothing anywhere states the durability contract:
synchronous fsync per commit (LMDB default) vs `NOSYNC`-style relaxation. For ledger
data this is a product decision, not a tuning knob — can a committed posting be lost on
power failure or not? It also directly affects criterion 2's fairness (SQLite's write
performance varies ~100× with `synchronous`/WAL settings) and the write-path design in
`40-storage.md`.
Question: What durability does a committed write transaction guarantee (fsync-per-commit,
or relaxed with a stated loss window), and what SQLite sync setting is the fair
comparison?

## 7. **The scale axiom's units are ambiguous** [design-gap]

Line 33–35: "Design envelope is up to ~100s of MB." Of what? Raw encoded fact bytes, the
LMDB file (which adds `M`/`U`/`R` guard entries and the dictionary — plausibly 2–4× the
fact bytes), or the *working set* (LMDB pages + decoded columnar images + per-query COLT
arenas — the images are a second full copy of touched columns, COLT a partial third)?
At `50-validation.md`'s top scale of 10⁷ facts, these readings differ by roughly an
order of magnitude, and "data fits in RAM" (RAM of which machine — an 8GB M1 or a 128GB
M4 Max?) settles arguments differently under each reading. The axiom is the doc's most
load-bearing sentence; it should be exact. Relatedly, the reassurance that "LMDB's mmap
keeps us from falling off a cliff" (line 34) is unsupported for this architecture: the
hot representation is decoded anonymous-memory images, not mmap pages, so beyond-RAM
behavior cliffs regardless — fine, since it's a non-goal, but the sentence implies a
graceful degradation that the design does not actually provide.
Question: State the envelope as numbers: max fact count, max raw fact bytes, expected
LMDB file multiplier, expected peak working set (file cache + images + arenas), and the
minimum RAM configuration assumed.

## 8. **"The ratchet" contradicts 50-validation's gate philosophy** [design-gap]

Criterion 2 and the decision block call the ledger benchmark "the ratchet" (lines 63,
73), a word that implies an enforced never-regress mechanism. `50-validation.md` says the
*only* numeric gate the project keeps is the allocation boolean and explicitly rejects
"budget tables" and ratcheting files. So what is criterion 2 operationally — a CI gate
(contradicting 50), a one-time milestone ("beat SQLite once, then done"), or a manually
re-run report? Each has different failure modes; the old repo's disease was both gate
theater *and* silent regressions, so the choice deserves an explicit sentence rather
than a metaphor.
Question: Is "beats SQLite" an enforced regression gate, a milestone to be declared once,
or a periodically re-run manual check — and where is that mechanism specified?

## 9. **Criterion 2 depends on aggregate execution, which is explicitly unscheduled** [design-gap]

The workload's headline queries are "balance-style aggregates" (line 32), and
`20-query-ir.md` states "a ledger database that cannot compute a balance fails its own
thesis." Yet README marks aggregate *execution* phasing OPEN ("when its execution lands
relative to plain joins is unscheduled") and `50-validation.md` lists the aggregate
family "(when aggregate execution lands)". So the success criterion's benchmark suite has
a hole exactly where the thesis lives: criterion 2 can be "passed" on a suite missing its
most thesis-relevant family. Nothing in 00-product says whether beating SQLite
pre-aggregates counts.
Question: Does criterion 2 only become evaluable once the aggregate family is in the
suite, or is there an interim criterion — and if the latter, what stops the interim
number from becoming the de facto ratchet?

## 10. **The SELECT DISTINCT oracle in criterion 1 is undefined for aggregates** [design-gap]

Criterion 1 (lines 60–61) promises "Exact result-set equality with SQLite (`SELECT
DISTINCT` oracle) on the full validation suite, always" — but `SELECT DISTINCT` only
defines the oracle for plain conjunctive queries. For aggregates, bumbledb folds "sets of
bindings" (`20-query-ir.md`), so the SQLite equivalent is an aggregate over a `SELECT
DISTINCT <all bound variables>` subquery — a nontrivial mapping neither this doc nor
`50-validation.md` writes down, and one that is easy to get subtly wrong (which variables
are in the DISTINCT set determines the sum). Overflow must also be aligned: bumbledb's
Sum is checked-overflow→error, SQLite's integer SUM also errors but TOTAL() silently
floats. Since criterion 1 gates every timing claim, its aggregate mapping cannot be left
implicit.
Question: Specify the oracle construction for aggregate queries (the exact SQL shape,
including which variables appear in the inner DISTINCT) — in this doc's criterion or by
explicit delegation to 50-validation.

## 11. **Criterion 3's "zero heap memory in steady state" conflicts with 30-execution and leaves "steady state" undefined** [design-gap]

Line 64 says prepared-query execution "allocates zero heap memory in steady state,
enforced in CI." `30-execution.md` says "The result buffer is the single sanctioned
allocation site (and callers can provide one)" — so the truthful criterion is "zero
allocations *except/unless* the caller provides the result buffer," and the CI gate must
pick one. "Steady state" is also undefined against three known allocation events: first
execution of a prepared query (COLT forcing "allocates within the arena" — do arenas
grow on first use?), the first execution after any commit (image rebuild allocates whole
new images), and arena growth when data grows between generations. A boolean gate with an
undefined predicate is exactly how v5's allocation discipline eroded.
Question: Define steady state operationally — which execution of a prepared query is
measured (Nth after warmup, no intervening commit?), and is the gate zero-including-
result-buffer (caller-provided) or zero-excluding-it?

## 12. **The workload promises point lookups and time-range scans; the storage doc forbids the access paths that serve them** [design-gap]

Line 30–32 names "point lookups by unique key, FK walks, time-range scans" as primary
workload shapes. Point lookups and FK walks are served by the `U`/`R` guard namespaces in
`40-storage.md` — fine. Time-range scans are not: encodings are order-preserving, but
there is no ordered secondary access path over a timestamp field, and `40-storage.md`
explicitly bans always-on accelerators ("may return later as declared, opt-in
accelerators, only with a benchmark that demands them"). So a time-range query is a full
image scan-and-filter of the relation — O(total facts), not O(matching facts) — at every
scale point. Nothing correlates `row_id` order with time order in the docs, and even if
insertion order roughly matches time, no doc licenses relying on that. The product doc
should either accept O(n) time ranges inside its latency budget or put declared range
accelerators in the v0 envelope; right now the workload sentence and the storage
non-existence list are pulling in opposite directions with 00-product as a party.
Question: Are O(full-relation-scan) time-range queries acceptable at 10⁷ facts within the
interactive-latency budget, or is a declared ordered accelerator part of the design
envelope from day one?

## 13. **"BCNF... with no way out of it" is an unenforced convention stated as a representation guarantee** [clarification]

Line 8 claims "BCNF-normalized typed relations with no way out of it," but nothing makes
non-BCNF schemas unrepresentable: `10-data-model.md` calls it "the modeling discipline"
and its forbidden list (JSON blobs, EAV) is convention, not construction — the schema API
will happily accept a denormalized relation with a redundant functional dependency. By
the project's own philosophy ("make illegal states unrepresentable"), this is exactly the
pattern the docs elsewhere ban: a rule enforced by vigilance rather than representation.
Either the claim should soften to "by discipline" or the schema layer should state what,
if anything, it checks.
Question: Is BCNF a checked property (e.g., declared FDs validated at schema build) or an
owner discipline — and should line 8's "no way out of it" be scoped accordingly?

## 14. **The 128-byte cache-line claim rests on a self-contradictory reference** [clarification]

Line 47 asserts "128-byte cache lines: columnar data is SoA, 128-byte aligned," citing
the hardware notes as authority. Those notes contradict themselves: Category 1 says
"Cache lines are 128 bytes across levels," while Category 5 (three separate sources)
repeatedly states 64-byte L1D lines ("L1D 128KB, 8-way, 64B lines, 256 sets"). The
alignment decision and any conflict-set/stride reasoning downstream depend on which is
true at which level. 00-product silently picks 128 without flagging the discrepancy.
Question: Pin the line size per cache level (with the source you trust) in the reference
doc, and state in 00-product which number the 128-byte alignment decision is derived
from and why it is still right if L1D lines are 64B.

## 15. **Success criterion 4 is unfalsifiable as written** [clarification]

Line 65: "These documents still describe the actual system six months from now." Worthy
goal, but there is no observer, no test, and no definition of "describe" — the previous
repo's docs also "described" the system in the loose sense while the image cache silently
didn't exist (`40-storage.md`'s own account). If this is to be a criterion rather than a
vibe, it needs a mechanism: e.g., "every `Deviation:`/`Decision:` block has a pointer to
the code that implements it, checked at each doc touch," or downgrade it to stated intent.
Question: What concrete check (if any) operationalizes criterion 4, or is it accepted as
aspiration rather than criterion?

## 16. **The doc's remaining pillar decisions (Rust, Apple-Silicon-only) lack alternative records** [clarification]

README rule 1 applies to every decision, yet 00-product contains exactly one `Decision:`
block (the benchmark). "Rust, specifically for allocation control" (lines 23–26) gives a
rationale but no strongest alternative (Zig and C++ both offer stricter or equal
allocation control; the real winning reason is presumably "the owner's applications are
Rust," which is stronger and should be stated). "Apple Silicon M-series is the only
performance target" (line 40) records no alternative (portable-performance posture) and
no why-it-lost. These are cheap to record and this doc is where they live; leaving them
implicit is the exact failure mode README rule 1 exists to prevent.
Question: Add one-paragraph alternative records for the Rust and Apple-Silicon-only
decisions (and fold the LMDB record from finding 2 wherever it belongs) — or state where
else these decisions are owned.
