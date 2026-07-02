# Consolidated Audit Worklist

135 findings across 8 audits, deduplicated into clusters. Each cluster lists: the
converging findings, the proposed resolution, and whether it needs an owner ruling or is
editorial (Claude drafts, owner reviews). Work top to bottom; A–D are the blockers.

Legend: `00#3` = finding 3 in `audit/00-product.md`, `seams#1` = `audit/cross-doc-seams.md`.

---

## A. Aggregation fold domain — THE semantic blocker [OWNER RULING]

Findings: 20#1, 20#3, 20#12, 20#13, 30#3, 30#13, 50#1, 50#5, seams#1, seams#16, readme#18.

The question: `Sum(amount) by account` — what multiset does Sum receive? Neither doc
defines the variable scope of the folded binding set, and the two readings are
respectively wrong-for-ledgers (project-to-finds collapses equal amounts: {100,100}→100)
and existential-multiplying (full bindings: joining PostingTag triples a sum).

**Proposed package:**
1. The fold domain is the **set of distinct full bindings over all query variables**.
   Two postings of 100 differ in their serial id → Sum = 200. Correct.
2. The footgun is documented loudly: joining a multiplicity-adding relation (PostingTag)
   into an aggregate multiplies the binding set — exactly as in SQL. Don't write that
   query; the docs show the wrong and right versions.
3. "Existential variables never multiply output" is scoped to **projection** output only.
4. **Sum accumulates in i128** with a single range check against the result type at the
   end — deterministic regardless of fold order (kills the order-dependent
   checked-overflow nondeterminism). Sum(I64)→I64, Sum(U64)→U64, range-error on overflow.
5. **Count is nullary** — `Aggregate::Count` carries no variable; it is |binding set of
   the group|. (Count-distinct-of-x is a later feature if ever wanted.)
6. D2's subtree skip is **legal only for the projection sink**, never under an aggregate
   sink. Stated in 30-execution with the reason.
7. SQLite oracle template: aggregate over a `SELECT DISTINCT <all bound variables>`
   subquery; `Count` = `COUNT(*)` over that subquery. Global aggregates (no group key):
   ruling needed on legality; if legal, empty input → empty result set (documented
   divergence from SQL's `[NULL]`/`[0]` row, with an oracle wrapper rule).

## B. Serial identity vs insert idempotence [OWNER RULING]

Findings: 10#1, 10#3, 10#14, 40#9, readme#11, seams#2.

`insert(fact)` takes a full fact; serials are DB-generated — contradiction. Also: is a
serial field implicitly unique? Is reinserting a deleted serial "reuse"?

**Proposed package** (the day-1 design had this right):
1. **Explicit allocation**: `tx.alloc::<AccountId>()` mints the serial inside the write
   txn; `insert` always takes complete facts and stays idempotent. One insert semantics,
   no generative variant — allocation is a separate operation (representation fix, not a
   branch).
2. Explicit serial supply is legal on the normal write path (it's how delete+reinsert
   mutation works), advancing the high-water mark. "Never reused" means the *generator*
   never re-issues a value observable in any committed state; reinserting a deleted
   value explicitly is legal.
3. A declared serial field carries an **implicit unique constraint** in its owning
   relation (two Accounts sharing an AccountId = unrepresentable).
4. Defining-occurrence rule: exactly one defining field (the declared serial field in
   `owning_relation`) owns the sequence; all other occurrences are references, no
   generator. Enforced at schema declaration.
5. `Q` counters join the in-memory-then-flush-at-commit set; mixed explicit/generated
   allocation in one txn tracks max.

## C. Point lookups and access paths [OWNER RULING]

Findings: seams#4, 30#12, 00#12.

Unique-key point lookups (headline workload) currently execute as O(n) scans; the `U`/`M`
guards that answer them in one B-tree get are write-path-only.

**Proposed:** the planner gets a **guard-probe access path**: a single-atom query whose
bindings cover a unique constraint (or the full fact) answers via `U`/`M` get + `F`
fetch — no images, no COLT, no plan search. This is an access path the planner selects,
not a mode. Time-range scans: accept O(n) image scan for v0, checked against the latency
budget (D below); declared opt-in range accelerators stay OPEN.

## D. Residual predicates and IR→CQ normalization [EDITORIAL after A/C rulings]

Findings: 30#1, 20#7, seams#3, seams#6, readme#5, readme#6, readme#7.

Cross-atom comparisons have no owner; repeated in-atom variables and literal bindings
violate the paper's CQ preconditions with no lowering pass specified; self-join plan
validity is stated over relations not atom occurrences.

**Proposed spec (write into 20 + 30 with Deviation blocks):**
1. A named **normalization pass** (owned by 20-query-ir, runs at validation): renames
   atom occurrences apart (self-joins legal), lowers repeated in-atom variables and
   literal/param bindings to per-atom filters, leaving distinct-variable atoms + a
   filter list per atom + a residual list.
2. Per-atom filters evaluate at the source (filtered view over the cached image —
   survivor-position vector, arena-backed, query-local).
3. Cross-atom residuals attach to the **earliest plan node where both sides are bound**;
   the executor's node loop gains one residual-evaluation step after probes, before
   recursion; vectorized batches compact survivors after residuals.
4. Plan validity quantifies over **atom occurrences**, not relation names.

## E. Image cache mechanics [OWNER RULING on 2 items, rest editorial]

Findings: 40#1, 40#4, 40#5, 40#6, 40#12, seams#9, seams#11, 30#11, readme#13.

1. **Generation source** (editorial, correctness-critical): a reader's generation T is
   the `_meta` storage-tx-id read **inside its own snapshot** — never an in-process
   counter. Kills the poisoning race.
2. **[RULING] Full-width images**: drop `field_scope` from the cache key; an image is
   always all columns of a relation (trivial at ≤100s of MB). Fixes the 30/40 key
   disagreement and the combinatorial-key contradiction.
3. **[RULING] Eviction**: on commit, the cache retains only the newest generation
   (old entries dropped from the map; pinned readers keep their Arcs alive until txn
   end; a long-lived reader at an old generation rebuilds query-locally — accepted,
   writes are rare). No memory-pressure eviction, ever — stated.
4. Tx-id bumps once per commit **that changed state**; all-no-op commits don't
   invalidate.
5. Cold build with a filter: one scan produces both the cached unfiltered image and the
   query-local survivor view.
6. Build race between same-generation readers: both build, first insert wins, loser
   adopts winner's Arc (or accepted double-build — pick one, say it).
7. COLT offsets are positions in the image's scan order; row_ids never appear in images.

## F. Planner reality [OWNER RULING on estimator]

Findings: 30#4, 30#10, 20#10, seams#7, seams#12, readme#14, 40#8.

1. **[RULING] The join-cardinality estimator** must be written down or the planner is
   v5's fake model again. Proposal: FK joins = |source| exactly; unique-key joins =
   min(|R|,|S|) bound; non-key equijoins = no estimate exists → planner treats as
   |R|×|S| worst case (honest pessimism pushes them last, which is correct behavior).
   No magic selectivity constants. S namespace = row counts only, stated.
2. Left-deep-only DP: yes, recorded as a decision (bushy + materialization loses to the
   sink/allocation contract).
3. **Plan invalidation**: plans pin their statistics at prepare time; they are NEVER
   invalidated by writes. Stale plans accepted (stats drift at this scale is benign);
   re-prepare is explicit. Kills the every-write-replans hole and protects the
   zero-alloc contract. 20's "statistics changes invalidate plans" sentence is deleted.

## G. Zero-allocation contract definition [EDITORIAL]

Findings: 30#7, 50#10, 00#11, seams#10.

Define the gate protocol: single-threaded harness; prepared query executed N warmup runs
(params drawn from a fixed set), then M measured runs with no intervening writes; arena
growth counted (steady state means arenas have reached fixpoint for that data); result
buffer caller-provided in the gate. Scratch is owned by the prepared query;
`PreparedQuery` is `!Sync`, single-threaded execution per query (matches the paper —
recorded); retained arena scale documented as O(touched data) per prepared query.

## H. Storage completeness [EDITORIAL]

Findings: 40#2, 40#3, 40#7, 40#10, 40#11, 40#14, 40#15, 10#9, readme#12, readme#19, seams#13.

Write the delete path (fact → M → row_id; F/M dels; U guard-key reconstruction from
fact_bytes; R prefix-scan Restrict check; counters). M key = full 32-byte blake3, no
verify, collisions accepted as documented axiom (same for `_dict`, whose "verify" claim
is dropped or given a real recovery rule — pick). Corrupt data at image build = hard
error, never skip. Single `_data` DB decision recorded (append mode caveat stated).
Open-time checks: format version then fingerprint, both hard fail. Key widths + id
assignment from canonical schema order, fingerprint-covered. Backup = file copy /
`mdb_copy`; compaction = ETL; one paragraph.

## I. Data-model precision [EDITORIAL after B ruling]

Findings: 10#2, 10#5, 10#6, 10#8, 10#10, 10#12, 10#13, 10#15, seams#19, 50#14.

Canonical fact encoding = fact identity, owned by 10 (Bool strictly 0/1, enum =
declaration-order ordinal, field concatenation in declaration order, value equality ≡
fact_bytes equality). Fingerprint inputs enumerated (including enum variant lists —
"adding a variant = ETL" stated as accepted). Orderability matrix: U64/I64 orderable
(Min/Max/range legal); Bool/Enum/String/Bytes/Serial equality-only. Dictionary: one
global dict, String and Bytes segregated by a type tag byte; UTF-8 validated at intern;
query literals resolve per-execution via read-only lookup, miss = empty result.
Constraint field lists ordered, non-empty, duplicate-free; names scoped per relation.
Nullary relations: legal (the empty fact falls out of the representation). Conventions
table (Date, etc.) added; nominal-domains OPEN stands.

## J. Oracle and validation mechanics [EDITORIAL]

Findings: 50#2–50#16, 00#10, readme#15, 20#9.

Value mapping table (each type → SQLite storage class; Bytes→BLOB, Enum→ordinal
INTEGER, Serial→INTEGER + harness-side type tag; typed rusqlite API, never CLI text;
decode outside the timed region). U64 rule: oracle-checked data constrained < 2^63,
enforced by the generator; full-range U64 covered by non-oracle property tests.
Aggregate template per cluster A. Fuzzing: encoding round-trip fuzzer retained
(decision recorded); crash/reopen + kill-during-commit family added; concurrent
reader/writer cache tests added; ETL family added (bulk≡sequential, serial high-water
properties, round-trip); golden set defined (hand-written queries, fixed dataset,
duplicate-witness coverage); random generator gets a feature-coverage contract;
IR→SQL translator named + pinned by SQL goldens + 3-way arbitration rule; negative
validation corpus with pinned error kinds.

## K. Benchmark honesty [OWNER RULING]

Findings: 00#1, 00#3, 00#4, 00#6, 00#8, 00#9, 50#4, 50#13, seams#21.

Criterion 2 needs: metric (proposal: per-family median, every family must win),
warm/cold protocol (proposal: warm, plus a reported-not-gated cold number), SQLite
config (file-backed, WAL, fully indexed for each family, prepared statements, ANALYZE
run — the honest opponent), DISTINCT included in timed SQLite queries (same semantics
both sides), machine pinned (owner's), suite membership versioned with the rule that
the claim is void until aggregate families land. Plus: write-rate design point
(proposal: bursty, ≥100 reads per write generation), latency budget (proposal: p99 ≤
10ms warm at 10⁷ facts), durability = fsync-per-commit (it's a ledger; SQLite compared
at synchronous=FULL). "Ratchet" defined as: re-run manually per meaningful change, not
a CI gate (consistent with 50's philosophy).

## L. Missing surfaces [OWNER RULING on scope]

Findings: readme#2, readme#9, readme#10, readme#16, seams#5, seams#14, seams#15, seams#17, 00#5.

A `60-api.md` is needed eventually: env lifecycle, write surface (pending B/FK
rulings), params, result representation + decode, error taxonomy + transactional
consequences, read-your-writes rule (proposal: queries inside a write txn are
forbidden in v0 — FK checks are internal; RMW flows use read-then-write txns),
process model (proposal: single process assumed; multi-process neither supported nor
guarded in v0 — stated), ETL export path (proposal: a full-relation scan API is the
export surface; old binary exports, new binary bulk-loads). Proposal: create
`60-api.md` now containing the decided fragments + a fat OPEN list, so the surface
has an owner even before it's fleshed out.

## M. Meta / repo hygiene [EDITORIAL]

Findings: 40#13, 30#15, readme#1, readme#17, readme#21, readme#22, seams#20, 00#14, 30#14.

Re-materialize the 34-file post-mortem from the session record and commit it (e.g.
`docs/history/post-mortem/`) so every load-bearing citation resolves; re-anchor
citations that can point at v5 source paths. Fix the 64B/128B L1D contradiction with a
correction note in the hardware reference (128B alignment stands either way — it
implies 64B alignment). README doc rules gain: reversal-evidence for decisions, the
every-mechanism-names-its-reader rule, and a doc-amendment procedure. OPEN items get
closure triggers. README OPEN list absorbs the 19-item sweep. Ledger schema ownership
moves to 50. Logica findings captured in-repo. Alternative paragraphs added for: LMDB,
Free Join, Rust, interning, M-table indirection, single-`_data`-DB, heed, aggregate op
set, blake3, SQLite-as-oracle.
