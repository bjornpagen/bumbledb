# 60 — Validation

Correctness discipline without gate theater. The validation system: two oracles
plus the offline store sweeper, a seeded differential suite with an asserted
coverage contract, a small golden anchor set, one allocation boolean, and the
ledger benchmark protocol. Every count and
every performance claim is derived and earned on this engine — a claim without a
current run behind it does not exist.

## The two oracles

**SQLite is the external oracle for query results** — never infrastructure. Every
benchmark and golden query *expressible in SQLite* is executed against it and
bumbledb's result set must equal SQLite's **exactly, by value**, before any timing
claim.
**Decision.** **Alternative:** reference-engine-only validation. **Why it lost:** an
independent, battle-tested implementation catches whole bug classes a same-author
reference shares. **Reverses if:** never.

**The naive model is the oracle for dependency semantics** — an in-memory reference
engine (naive loops + BTreeSets, obviously correct), required infrastructure
because SQLite cannot express the judgments
(`30-dependencies.md`): pointwise keys, conditional containments, totality. The
naive model implements chapter 30 literally — after every commit in a differential
run it evaluates **every statement by brute force over the full final state** and
must agree with the engine's accept/abort verdict *and*, on abort, the violating
statement id and the judgment `Direction` — verdicts compare whole. It also executes every query IR (negation, membership, param sets, and
Arg-restriction included) by nested loops, closing the expressibility gaps in the
SQLite lane. The naive model is the executable form of the semantics chapters: when
engine, model, and docs disagree, the docs arbitrate and the loser is fixed in the
same change.
**Decision.** **Alternative:** emulate judgments in SQLite via triggers. **Why it
lost:** trigger emulation is a second nontrivial implementation of the semantics
*in a language nobody here trusts for it*, validated by nothing; the naive model is
smaller than its triggers would be and doubles as the query-gap oracle.
**Reverses if:** never — three-way beats two-way.

**The store sweeper is the third leg: the oracles judge semantics; the sweeper
judges the store.** `Db::verify_store` takes one read snapshot and sweeps
O(store): every namespace pairing re-verified against the schema (F↔M↔U↔R plus
the `S` counters against the `F` scan — including the `R`-delete class the
commit path defers, `50-storage.md`), and **both judgment forms re-verified
globally** over the full committed state through the commit path's own probe
and coverage walk — the class no incremental check can see: an incremental form
wrong once, long ago, preserved by every commit since. Findings are report
data, never errors; CLI-wrapped as `bumbledb-bench verify-store` (nonzero exit
iff findings are non-empty).

**Durability parity under `synchronous=FULL`.** Both engines flush **to media** on
the timing machine: LMDB does unconditionally on macOS (`lmdb-master-sys`
`mdb.c:171` — `MDB_FDATASYNC(fd)` is `fcntl(fd, F_FULLFSYNC)` under `__APPLE__`),
while SQLite's default `fullfsync=OFF` issues a plain `fsync(2)` that macOS does not
propagate through the drive cache. The bench session therefore pins
`PRAGMA fullfsync=ON` and `PRAGMA checkpoint_fullfsync=ON`, and `FairnessCheck`
asserts both — an unpinned fullfsync manufactures order-of-magnitude phantom
write-latency gaps that are drive-cache policy, not engine work.

**The value mapping is normative.** Comparison uses
the **typed rusqlite API**, never CLI text — text parsing with fallback defaults
silently coerces everything and is the oracle-corrupting bug class:

| bumbledb | SQLite | note |
|---|---|---|
| Bool | INTEGER 0/1 | |
| U64 | INTEGER | generator constrains oracle-checked data to `< 2^63`; full-range U64 is covered by non-oracle property tests (encode/decode, guards) |
| I64 | INTEGER | |
| Interval | two INTEGER columns (start, end) | value equality = pairwise; an `Allen` mask translates to its basics' endpoint formulas OR'd (under the query's SELECT DISTINCT); membership is the endpoint pair — fully expressible in SQL; the *judgments* over intervals are the naive model's lane |
| String | TEXT | intern ids decoded to bytes **before** comparison, outside any timed region |
| Bytes(N) | BLOB (fixed-length content) | never TEXT — DISTINCT distinguishes `X'41'` from `'A'`; the N raw bytes, unpadded (the word pad is bumbledb encoding, not data) |
| closed relation (references and payload) | an ordinary mirrored table: INTEGER `id` (PRIMARY KEY — the row's declaration index) plus payload columns per this table, INSERTed from the sealed extension at mirror-build time | the extension rides with the schema DDL, never the corpus — a closed relation is never empty, so empty-store pairs carry the axioms too; closed atoms are then ordinary tables on the `SQLite` side (what makes the differential meaningful for the folds), while the ψ-subset WRITE judgments stay naive-only (the division of labor below) |

**Projection queries:** `SELECT DISTINCT` over the join with all find variables.
**Rules:** one `SELECT DISTINCT` per rule joined by `UNION` — SQLite's `UNION` is
exactly ∪ under `DISTINCT` discipline; a multi-rule *aggregate* head folds over the
`UNION` of the rules' head-projected distinct rows (the union-fold template,
mirroring the rules-IR definition: per-rule dedup at head granularity, one set
union, then the fold), while the single-rule fold domain stays the distinct full
binding set. **The measure:** `Duration` = `(end − start)` on the two stored
interval columns — exact in SQLite's INTEGER for every generated corpus (the U64
lane's sentinel end sits below 2⁶³).
**Negation:** `NOT EXISTS` correlated subqueries — the translator owns the
correlation variable mapping. **Param sets:** SQL `IN` lists expanded per execution
(the translator re-renders; prepared-statement parity is not claimed for set-bound
families, stated). **Aggregate queries (normative template):** the aggregate applied
over a `SELECT DISTINCT <all bound query variables>` subquery — never a bare
`GROUP BY` over the joined bag (which folds witness multiplicity) and never
`SUM(DISTINCT x)` (which folds distinct values). `Count` = `COUNT(*)` over that
subquery; `CountDistinct(x)` = `COUNT(DISTINCT x)` over it. **Arg-restriction:** the
subquery joined back against its per-group extreme (`WHERE (group, key) IN (SELECT
group, MAX(key) ...)`) — ties survive on both sides by construction, matching the
set-honest semantics. **Empty-input global aggregates:** bumbledb yields the empty
set; SQLite yields one NULL/0 row; the harness rule is that the oracle SQL wraps
ungrouped aggregates to drop the empty-input row — a documented translation rule,
not an ad-hoc comparison patch.

**`Pack` is naive-only by decision.** SQLite has no coalescing aggregate — a
relation-shaped fold (one row per (group, maximal segment)) is not a SQL fold, and
a recursive-CTE emulation would test the emulation, not the engine. The
expressibility gate is the enumerated `Inexpressible` set (`translate::
sqlite_expressible`): `Pack` heads plus the two dependency judgments, consumed by
the harness so nothing is ever *silently* skipped — the verify run counts and
reports its naive-only cases. The naive model packs **from the point-set
definition** (union of point sets → maximal segments, sort-and-merge over logical
endpoints), independent of the engine's word sweep. **The compiled subsets share
the division of labor**: ψ-subset write judgments (containments into a closed
target, `Escalation(severity) <= Severity(id | pages == true)`-shaped) are the
naive model's alone — `SQLite` does not express commit-time CINDs, and the
judgment kinds are already inside the enumerated set — while the naive check is
**definitional**: σψ applied to the extension rows by value comparison on the
shared `Value` sum, never the engine's compiled member set (the independence
law). Closed *reads* are fully three-way: mirrored extension tables on the
`SQLite` side, the seeded extension on the naive side.

**Error parity is typed identity, not agreement-in-kind** (the PRD-02
direction-divergence lesson, generalized): wherever both oracles reject, the
verdicts compare *whole* — the judgment verdict with statement id and `Direction`,
`MeasureOfRay` and aggregate overflow as query verdicts through the differential
runner, and the roster rejections against the naive model's own
from-the-definition computation: a cap-exceeding predicate tree must be
`DnfExceedsRules` with `produced` equal to the naive DNF width (leaf = 1, `And` =
product, `Or` = sum), a program whose every disjunct vanishes is the empty union,
and the vacuous masks (EMPTY and FULL) are the mask-cardinality rejections. A
case where both sides error *unexpectedly* stays a bundle — agreement-in-error
must not impersonate verification.

**The IR→SQL translator is named infrastructure** with its own tests: hand-written SQL
goldens pin its output for known queries. Arbitration for 3-way disagreements
(engine vs model vs SQLite): the hand-verified golden answers decide; a disagreement
on a non-golden query becomes a minimized golden before it is "fixed."

**Negative validation** has no oracle (SQLite accepts what we reject): a corpus of
invalid IR *and invalid dependency statements* with pinned error kinds asserts the
validation rosters in `20-query-ir.md` and `30-dependencies.md`.

## The primary benchmark: ledger

Owned here (00-product describes shape; this doc owns the schema — restated in the
statement notation, with the redesign's temporal surface added):

```
closed relation Currency = { Usd, Eur, Gbp }
closed relation Source   = { Manual, Import, System }
closed relation Tag      = { Fee, Rebate, Adjustment }

relation Holder       { id: u64, fresh, name: str }
relation Account      { id: u64, fresh, holder: u64, currency: u64 }
relation Instrument   { id: u64, fresh, symbol: str }
relation JournalEntry { id: u64, fresh, source: u64, created_at: i64 }
relation Posting      { id: u64, fresh, entry: u64, account: u64,
                        instrument: u64, amount: i64, at: i64 }
relation PostingTag   { posting: u64, tag: u64 }
relation Org          { id: u64, fresh, name: str }
relation OrgParent    { child: u64, parent: u64 }
relation Mandate      { account: u64, org: u64, active: interval<i64> }

Account(holder)      <= Holder(id);
Account(currency)    <= Currency(id);
JournalEntry(source) <= Source(id);
PostingTag(tag)      <= Tag(id);
Posting(entry)       <= JournalEntry(id);
Posting(account)     <= Account(id);
Posting(instrument)  <= Instrument(id);
PostingTag(posting)  <= Posting(id);
OrgParent(child)     <= Org(id);   OrgParent(parent) <= Org(id);
Mandate(account)     <= Account(id);   Mandate(org) <= Org(id);
Mandate(account, active) -> Mandate;                    // pointwise key: one mandate per account per instant
```

The 15 gated ledger families are: `point`, `containment_walk`, `chain`, `range`,
`balance`, `stats`, `string`, `skew`, `spread`, `triangle`,
`entries_for_account_set`, `postings_without_tag`,
`latest_posting_per_account`, `mandate_at_instant`, and `mandate_overlap`. They
cover key point lookups; postings for a holder/account over a time range; entries
touching an account set (**param-set family** — the host-side union convention is
retired with `ParamSet`); multi-hop joins across holders/accounts/postings/
instruments/entries; balance-style aggregates; interned strings; skew; summary
statistics; **latest-posting-per-account (Arg-restriction family)**;
**postings-with-no-tag (negation family)**; **mandate-at-instant and
mandate-overlap (interval families — membership probe and Allen-mask join)**; a
cyclic join for WCOJ honesty; and a duplicate-witness projection. Data: seeded,
reproducible, generated at 10⁵–10⁷ facts; the mandate generator emits both disjoint
and adjacent intervals (the neighbor-probe boundary is a data case, not just a unit
test).

**Protocol (success criterion 2 is measured exactly this way):** SQLite file-backed,
WAL, `synchronous=FULL`, **fully indexed per family** (the honest opponent — interval
families get `(account, start, end)` composite indexes, the best SQL can do),
prepared statements reused, `ANALYZE` run; `SELECT DISTINCT` (or the aggregate
template) included in the timed SQL — same semantics both sides; timed region =
execution + result materialization on both sides, decode excluded per the mapping
table; warmup then repeats; statistic = per-family **median**; **every family must
win**; warm timing gates, cold-after-commit reported alongside; canonical machine =
the owner's. The suite is an explicit versioned query list in-repo; **the claim is
void until re-earned on the new format**. The 10 ms warm-p99 budget binds only at
scale L; because no L corpus exists, S reports it as informational. The "ratchet" is a manually re-run report
per meaningful change — not a CI gate. JOB and friends may be run for curiosity; they
gate nothing.

## The calendar benchmark: the second theory

The algebra's earning (ledger-adjacent scheduling from the workload census —
`00-product.md`): a second schema/corpus/family world under the **exact same
protocol** (fully-indexed SQLite mirror, fullfsync parity, prepared statements,
`ANALYZE`, `SELECT DISTINCT`, warm medians, verify-before-time), sharing the
digest directory and the stamp with the ledger — one corpus identity, one
stamp, both theories inside. The calendar families join the ALL-WIN set.
Owned here, restated in the statement notation:

```
relation Account    { id: u64, fresh, name: str }
relation Person     { id: u64, fresh, account: u64, name: str }
relation Calendar   { id: u64, fresh, owner: u64 }
relation Event      { id: u64, fresh, calendar: u64, span: interval<i64>,
                      created_at: i64, hash: bytes<32> }
relation Attendance { id: u64, fresh, event: u64, person: u64, rsvp: u64 }
relation Claim      { source: u64, person: u64, arm: u64, span: interval<i64> }

closed relation Rsvp = { Accepted, Tentative, Declined }
closed relation Arm  = { Busy, Ooo }
relation Room       { id: u64, fresh, name: str }
relation Booking    { room: u64, event: u64, span: interval<i64> }
relation WorkHours  { person: u64, hours: interval<i64> }

Person(account)     <= Account(id);     Calendar(owner)   <= Person(id);
Event(calendar)     <= Calendar(id);    Attendance(event) <= Event(id);
Attendance(person)  <= Person(id);      Claim(person)     <= Person(id);
Attendance(rsvp)    <= Rsvp(id);        Claim(arm)        <= Arm(id);
Attendance(event, person) -> Attendance;
Claim(source)       -> Claim;           Claim(person, span) -> Claim;
Attendance(id | rsvp == Accepted) == Claim(source | arm == Busy);  // the DU
Claim(person, span | arm == Busy) <= WorkHours(person, hours);     // coverage
Booking(room)       <= Room(id);        Booking(event)    <= Event(id);
Booking(room, span) -> Booking;                     // room exclusion, pointwise
WorkHours(person)   <= Person(id);      WorkHours(person, hours) -> WorkHours;
```

`Event.hash` is the `bytes<32>` content-hash column (identity-shaped digests —
the census's byte-shaped ruling). Corpus: seeded and streaming, stratified over
persons × meeting density × ray fraction — a hand-rolled Zipfian density
envelope (`max_segments >> ⌊log₂(rank + 1)⌋`, the 1/rank curve in closed form,
no crate, no floats — the dependency quarantine), per-person segment chains
valid under the pointwise keys **by construction** (sequential, every third
boundary abutting), every fourth person's chain ending in a ray (`[s, ∞)`
recurrence horizons — ray events, ray claims, and coverage-to-∞ exist
structurally), busy segments spawning one event + accepted attendance + busy
claim (the `==` holds by construction; the engine loads the Attendance/Claim
cluster through joint chunked writes — either relation alone violates one
direction), and exact-abutment working-hour chains from the epoch to ∞.

**The families — each one times a named representation:**

| family | representation timed |
|---|---|
| `busy_scan` | the Allen mask against a param window over an O(n) scan (03/04); the range-accelerator trigger's evidence |
| `meets_chain` | named-relation probes: singleton `MEETS` chain join + `DURING` filter — singleton cost = composite cost (03) |
| `rsvp_union` | the DU whole-read: three rules, one per RSVP arm, through the spanning union seen-set (05/07/08) |
| `conflict_pairs` | the Allen-mask self-join, `INTERSECTS` across one account's persons (04) |
| `conflict_free` | the anti-probe: ¬Claim with a point-membership binding at an event-creation instant (04 + negation) |
| `free_busy` | `Pack`, the coalescing fold, per person per window (11/12); free time is the host's gap walk (the `Gaps` refusal) |
| `claim_hours` | the measure: `Sum(Duration)` by claim arm under the `Allen(DISJOINT)` ray guard (10) |

**Mirror rules.** The calendar mirror follows the value-mapping and template
rules above; `free_busy` is the one family the IR→SQL translator cannot express
(`Pack` — the enumerated `Inexpressible` set), and it is **reported
translator-unpaired, never dropped**: its SQLite side is a hand-written
window-function coalesce (order each person's distinct claim windows, cut
islands where a start exceeds the running max end, fold each island to
`(MIN(start), MAX(end))`) — SQLite's honest best shot at Snodgrass coalescing
(measured faster than the recursive-CTE row walk, so the fairer opponent),
verified row-identical against the engine's `Pack` and the naive model before
any timing.

**Verify lanes.** The calendar corpus joins the verify pass before any timing:
every family × its fixed rotation plus a seeded randomized draw slice against
the mirror; the same families over an empty store pair; and a unit-scale naive
differential slice — the corpus stream replayed through joint `==`-cluster
chunks, four judgment-violating deltas (room exclusion, `==` totality, `==`
arm validity, working-hours coverage — each violating exactly one statement,
verdicts compared whole), and every family query against the brute-force
model. The stamp digests both theories' family lists and both corpora.

**The witnessed-write row** (`commit_witnessed`): `commit_single` through
`Db::write_from` with a fresh snapshot witness per sample — the delta against
`commit_single` prices the witness mechanism (a snapshot generation read plus
one integer compare). SQLite-unpaired by decision: SQLite has no
snapshot-witness surface, and a BEGIN-IMMEDIATE + user-version emulation would
time the emulation, not the engine.

## Differential and property tests

- The **naive model** (promoted above) executes the same IR and judges the same
  commits; randomized queries and randomized write sequences over randomized
  ledger-shaped data must agree three ways (engine, model, SQLite) where SQLite can
  express the case and two ways where it cannot — with the inexpressible set
  enumerated in the harness, never silently skipped.
- **The generator has a feature-coverage contract, itself asserted** (the exact
  form the coverage test pins at n = 1000): every shape within ±30% of its weight;
  every *legal* cell of the per-(operator, type) comparison matrix nonzero (`Eq`/`Ne`
  over all six types; order operators over the two integer types;
  `Allen` masks (composites and singleton basics) over both interval element types, including the
  adjacent-touching boundary `[a,b) [b,c)` in both polarities) and every illegal
  cell zero (order-on-interval prominently); repeated in-atom variables; self-joins
  with cross-atom ordered residuals; zero-binding gate atoms drawn from more than
  one relation (including under aggregates, including **negated** gates); negated
  atoms across the binding-shape space (key-covered and not, with params and sets);
  param sets across sizes {0, 1, 2, boundary-large} with per-type miss policies and
  duplicate elements (dedup asserted); membership bindings against literals,
  params, and variables (each anchoring the element type) — with the **cost-bound
  rule**: a var-point membership or cross-atom Allen/Contains construct is
  generated only on an equality-connected spine (the interval occurrence shares an
  equality join variable with the point's atom, or carries an equality
  selection) — the keyless form is a Cartesian (`40-execution.md`) and the
  generator bounding its cost is the same duty as bounding reachable sums;
  aggregates of every op
  over their legal types (u64 generators must bound reachable sums below 2⁶³);
  CountDistinct over every type; Arg-restriction with and without ties (tie data
  constructed, not hoped for) and with the key projected; multi-aggregate find
  lists; and **duplicate-witness data that exercises the D2 subtree skip and the
  aggregate-sink binding dedup** (the two places a set-semantics bug would hide).
  The algebra families extend the same contract: **multi-rule programs** at arm
  counts 2–4 — provably-disjoint arms (distinct closed-reference selections on one
  discriminant, with the proof visible diagnostically and the spanning union
  exercised against the oracles' plain union), overlapping arms with duplicate
  head rows across rules (the union's
  teeth), and the multi-rule aggregate union fold (`rules ∧ aggregate`, at least
  once per run); **the measure** in all three construct kinds — find position,
  order predicate, and `Sum`/`Min`/`Max` fold (`Sum` under a duration bound, the
  same Sum-range duty) — over the ray-free U64 window lane, with ray-bearing
  measure parity in the naive lane; **`Allen` masks** as named composites, all 13
  singletons, and random masks (every basic reachable through some literal mask
  per run, asserted cell by cell), plus the composites `mask ∧ negation` and
  `membership ∧ Allen` at least once per run; and **the boundary-shape ladder** —
  equal / adjacent-touching / strictly-nested / ray — systematized for *every*
  interval literal the generator draws (shape literals, dressing literals, and
  interval-typed param draws alike), each rung asserted per run.
  Empty relations are covered by the verify run's **empty-store pass**: every
  family plus a seeded randomized slice runs against a zero-row store pair each
  verify — every gate false, every scan empty, every aggregate folding nothing.
- **The entropy seam** (`corpus_gen::rng`): every generator draw goes through
  one closed sum — `Rng::Seeded` (the bench/differential arm, the seeded
  stream above) and `Rng::Bytes` (the fuzzer arm: draws consume a fuzzer's
  byte string; exhaustion falls back to a deterministic zero tail, never a
  panic) — two sources, one generation path, with the corpus digest pinning
  the seeded arm byte-identically across the seam. The fuzz lanes generate at
  `Scale::Tiny`, the scale ladder's fuzz-iteration point (ledger: 1 024
  postings / 32 instruments / 8 orgs; calendar: 32 persons with 16-segment
  max chains — everything else derives as at S/M/L), sized so a full
  build-store → ops → oracles iteration is milliseconds; Tiny is a
  first-class scale under the same by-construction invariants, not a
  special-cased path.
- **The algebra oracle rows in every verify run** (the naive lane's extension):
  multi-rule programs replayed engine-vs-naive, the naive model evaluating rules
  **directly** — the union of per-rule binding sets from the definition, sharing
  no lowering, kernel, or sweep code with the engine (the independence law: the
  model imports the engine's *types* only); seeded random predicate **trees to
  depth 3**, the naive model evaluating the *input tree* while the engine
  evaluates the lowered rules — the differential is the DNF-lowering proof — with
  the cap-exceeders and vanished programs in the error-parity cases above;
  **`Pack`** rows (grouped, global, and the multi-rule union fold) naive-only per
  the expressibility gate; the **measure's rays** (`MeasureOfRay` on both sides,
  typed, and the `Allen(DISJOINT)` ray guard keeping the same query answering
  rows); and the **converse-property lane**: for every generated Allen-bearing
  query, the converse twin — operands swapped, mask conversed per leaf — must
  produce the identical result set on the engine (`Allen(a, b, m) ≡
  Allen(b, a, converse(m))`, the coordinate system's own theorem, quantified over
  the generator's whole mask distribution).
- **Dependency-judgment property family** (new, the redesign's write-side core):
  random statement sets over random schemas (within the acceptance gate), random
  write sequences; assert engine-vs-model verdict agreement; targeted subfamilies
  pin the theorems — union exclusivity (two arms fighting over one id must abort),
  totality (parent without child must abort; parent-with-child in one delta must
  commit), same-delta cluster demolition (must commit), pointwise-key
  adjacent-vs-overlapping boundaries, coverage with exact-abutment segment chains,
  the ray end (`MAX` = ∞, the point-domain law) at every boundary position, and **the net-disposition
  pattern class** — a redundant insert (plain, or a delete + re-insert netting to
  nothing) alongside a delete of its containment target must abort **target-side
  on both oracles, `Direction` compared as part of the verdict**: "source side"
  means facts the transaction actually added, the naive model is normative, and
  the delta's net dispositions (`50-storage.md`) make the engine agree by
  representation. The `==`/totality corner (no-op parent re-insert + child
  delete) is the same class, caught via the parent's standing reverse edge.
- Operation-sequence property tests for the write path: random insert/delete/alloc
  interleavings with judgment checks, asserting idempotence, guard consistency,
  reverse-edge consistency, and fresh monotonicity across commits and aborts —
  **plus WriteTx point reads asserted against the delta-overlaid view** (a read
  inside the transaction equals the post-commit read, on every interleaving).
- Scalar/vectorized (batch-size 1 vs 2/64/256/partial/empty) equality on every fixture.
- **Crash and reopen:** kill-during-commit (LMDB atomicity actually exercised) and
  reopen-after-commit asserting F/M/U/R/Q/S mutual consistency and counter truth —
  the deferred-counter-flush design (`50-storage.md`) makes reopen the only test that
  can catch a never-persisted high-water.
- **Concurrent reader/writer families:** long-lived reader pinned at generation T
  across commits T+1..T+n (its images survive; results stay at T); two readers racing
  to build one image (single shared instance or benign duplicate — per 50's rule);
  rapid write/read interleaving (a reader never sees a mismatched generation — the
  snapshot-sourced tx-id rule under test).
- **ETL family:** bulk-load ≡ sequential-insert equivalence (full-relation set
  equality); explicit-fresh/high-water property tests; chunk-boundary and mid-stream
  failure semantics (prior chunks committed, count carried on the error) — including
  the bidirectional-statement cluster-straddle case, which must fail loudly
  (`50-storage.md`); full round-trip (export → fresh database → import in
  dependency-cluster order → oracle-equal results). ETL is the migration story; an
  ETL bug is a data-loss bug.
- **Encoding round-trip fuzzing is retained** (decision: the one *in-tree* fuzz
  target — order-preserving encodings and composite guard keys are where a
  boundary bug corrupts sort order silently; i64::MIN, empty bytes, max-length
  values, and now interval starts/ends at element extremes and `start+1 == end`
  minimal intervals). Executor differential fuzzing is subsumed by the seeded
  generator above; the coverage-guided campaign over the public API lives in the
  detached `fuzz/` crate (the fuzzing charter, below).

## The fuzzing charter

The detached `fuzz/` crate (cargo-fuzz layout, its own workspace — the
workspace gates never build fuzz artifacts) is the adversary-first lane:
coverage-guided, structure-aware generation through the entropy seam
(`Rng::Bytes` steering the same `corpus_gen` generators the seeded lanes
run, at `Scale::Tiny`), driving the REAL public API against the oracles
this engine already owns. The harness owns no logic worth fuzzing
(refusal: we do not fuzz the harness — a fuzzer for the judge has no
judge); generation arms live in `bumbledb-bench`'s `corpus_gen`, and each
target in `fuzz/fuzz_targets/` is one thin call into one shared-harness
runner.

- **Targets** (one entry point each): `theory` — schema acceptance over
  the random-descriptor arm (`corpus_gen::theorygen`): structurally-free
  descriptors, deliberately-invalid shapes alongside valid ones, judged
  by `Db::create`. Later targets extend the roster: `ops` (the op-stream
  flagship), `query` + `rewrites`, `crash`.
- **Oracle discipline** (every iteration, all of them): *no-panic
  totality* — hostile input yields `Ok` or a typed error, any
  panic/abort is a finding by definition; *typed rejection* — every
  `Err` is a named variant, matched TOTALLY in the harness (zero `_ =>`
  arms over engine error enums, so a new variant is a compile error —
  the matcher is a census instrument); *judgment determinism* — the same
  input judged twice on fresh stores yields the identical verdict, and
  accepted schemas reopen cleanly with `verify_store` passing on the
  empty store.
- **Corpus policy**: `fuzz/corpus/<target>/` is a checked-in seed corpus
  from a small deterministic generator run; `fuzz/artifacts/` is
  gitignored. Every real counterexample is minimized (`cargo fuzz tmin`)
  and pinned as a permanent regression test in the crate that owns the
  bug.
- **Trophy ledger**: `fuzz/README.md` — one row per real finding (date,
  target, root cause, the pinning test).

## Golden set

Hand-written queries with hand-verified expected results over a fixed dataset — the
anchor when the 3-way differential disagrees. Must cover: duplicate witnesses (the
set-semantics signature), exact projection sets, duplicate insert no-ops, absent delete
no-ops, judgment violations of both forms (with the statement id pinned), the union
theorems by hand (exclusivity, totality, demolition), pointwise-key boundary cases
(abutting passes, one-point overlap aborts), aggregate folds with
collapsing-vs-distinguished bindings, Arg-restriction ties, negation against empty
and nonempty relations, and empty-input aggregates.

## The allocation gate

The one numeric gate: a counting allocator asserts the high-water allocation contract
under the exact protocol defined in `40-execution.md` — the steady-state window
(single-threaded, N warmups over a fixed param set, M measured runs asserting zero,
arena growth counted, caller-provided result buffer) plus the escalating high-water
window (allocations only on executions setting a new intermediate high-water; every
repeat of a seen parameter silent; at least one growth event observed, else the run
is vacuous). It is a boolean, not a budget file.

## EXPLAIN assertions

One small family: on constructed skew fixtures, EXPLAIN's counted execution asserts
the expected cover choice and that batching engaged — the cheap detector for
correct-but-slow regressions, the class no functional test can see.
Beyond this, the benchmark's timing is knowingly the only performance detector; stated.

## Observability: the trace seam

The one tracing mechanism is
`obs.rs`: nanosecond spans and point events recorded into a thread-local buffer
during explicit capture, drained by tooling (Chrome-trace export + terminal
flame summaries in `bumbledb-bench`). **Zero-cost when off**: under default
features every obs function is an inline empty body and the span guard is a
ZST — production timing paths carry no instrumentation. Under the `trace`
feature, spans check a capturing flag; capture is never enabled inside a
measured allocation window (trace and alloc are mutually exclusive run modes).

**Per-(node, phase) executor attribution.** The flame summaries bottom out at
one `join` span, so the executor exposes a phase seam:
`JoinPhase {iter, hash, probe, residual, descend, force}` boundaries reported
through `Counters::phase_start/phase_end` (default no-ops — the release path
monomorphizes `NoopCounters` and pays exactly nothing; the prepared-query
execute selects the timing implementation only under an active obs capture).
`PhaseTimers` accumulates per-(node, phase) ticks and flushes one
`Category::Phase` point event per touched cell (`a0` = total ns, `a1` = calls),
named from the `jp_*` registry (node-indexed, capped at 8 with an `nX`
overflow bucket). The bench renders these as a phase table with an `excl_us`
column — descend minus the next node's total = per-row bookkeeping + leaf
emits + the child node's un-phased entry setup. `WORDMAP_GROW` point events
surface sink-map rehashes inside measured executions.

**Measurement caveats, measured.** The raw `cntvct_el0` read costs 0.30 ns
(1 per cycle) — the instrument is free; the constraint is `cntfrq_el0`'s
24 MHz (41.67 ns granularity, unbiased across accumulation). The unfenced
closing-stamp slide is bounded by backend scheduler occupancy at ≤ ~50 ns —
NOT by the ~630-entry ROB — which is ≤ 2–3% on accumulated two-stamp phase
attribution and fatal only for single-shot timing of sub-500 ns regions.
Stamp policy, accordingly: `PhaseTimers` accumulates with raw stamps at both
ends (an `isb` fence costs more than the slop it removes — measured +164% at
10 ns phases); single-shot spans close with `CNTVCTSS_EL0` (self-synchronized,
4.6 ns worst case, slide-proof — half the price of `isb`'s 9.4 ns). Phase
totals carry the stamp overhead of deep small-batch nodes, and short phases
under-attribute up to ~7%. Therefore: **phase tables direct work; the
untraced timing tables decide gates.**

The sharper slide bound (measured): the unfenced closing stamp slides by
**min(remaining payload latency, scheduler drain)** — occupancy is only
the ceiling. On a latency-bound span (a pointer chase mid-flight at the
stamp) the slide reaches −99.6% of the span; on throughput-bound spans
it stays in the ≤ ~50 ns drain regime. `CNTVCTSS_EL0` closes hold ±7%
everywhere. The health rule: **attribution claims under ~1 µs require
CNTVCTSS closes AND repetition; a latency-bound span's raw-stamped
attribution is presumed wrong.** On the platform side, macOS's commpage
kind-3 conversion makes the libsystem clocks (`Instant`,
`mach_absolute_time`) slide-proof on M2. Traced samples are single warm
executions on the rotating param sets — for skewed families the sample may be
the hot parameter; gates cite p95 where that matters.

## What we deliberately do not have

Line-count gates. PRD-map checks. Banned-identifier greps. Coverage percentages.
Allocation budget *tables*. Failpoint matrices (the crash/reopen family above replaces
them with fewer, sharper tests). Trigger-emulated constraints in the oracle. The gate
surface is: `cargo fmt` / `clippy -D warnings` / `cargo test`, the two oracles, the
differential suite, the allocation boolean, and the EXPLAIN family. A gate earns its
place by catching a real bug class.
