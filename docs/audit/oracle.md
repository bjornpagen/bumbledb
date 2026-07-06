# Oracle correctness audit

## Scope (files and docs read, with line counts)

Mandatory reading, in order:

- Free Join paper `docs/free-join-paper/arXiv-2301.10841v2/`: `main.tex` (162),
  `tex/00-abstract.tex` (15), `tex/01-intro.tex` (243), `tex/02-background.tex` (510),
  `tex/03-free-join.tex` (608), `tex/04-optimizations.tex` (478), `tex/05-eval.tex` (337),
  `tex/06-discussion.tex` (85); `tex/025-tale.tex` (358) read though not input by
  `main.tex`; `tex/07-relatedworks.tex` / `tex/08-conclusion.tex` are empty (0).
- Architecture docs: `README.md` (71), `00-product.md` (186), `10-data-model.md` (227),
  `20-query-ir.md` (178), `30-execution.md` (295), `40-storage.md` (205),
  `50-validation.md` (179, normative for the oracle), `60-api.md` (120).

Audit targets, read in full: `crates/bumbledb-bench/src/translate.rs` (540),
`sqlmap.rs` (273), `compare.rs` (259), `verify.rs` (391), `querygen.rs` (941),
`gen.rs` (520), `families.rs` (719). Supporting context: `schema.rs` (283),
`corpus.rs` (256), `sqlite_run.rs` (467), `cli.rs` (378), `driver.rs` (759, partial),
`scripts/bench.sh`; engine spot-checks in `crates/bumbledb/src/api/prepared.rs`,
`storage/dict.rs`, `exec/dispatch.rs`, `plan/fj.rs`, `ir/validate.rs`.

Behavioral claims about SQLite (SUM overflow, i64::MIN literals, HAVING-without-GROUP-BY,
NUL-in-literal, alias GROUP BY, DISTINCT collapse) were confirmed by executing SQL
against SQLite 3.53.2 (bundled oracle is 3.46.0 via libsqlite3-sys 0.30.1; the tested
behaviors are stable across both).

## Verdict

The comparison core is sound: the value mapping is typed and total both directions, the
projection and aggregate translations are semantically equal to the engine's set/fold
semantics on everything the suite can generate (verified adversarially, including
duplicate-amount collapse, miss-sentinel Ne, empty-input global aggregates, and grouping
by strings/enums), the multiset diff cannot mask an inequality, and a mismatch cannot be
swallowed on any verify path — errors panic loudly and the stamp lands only on a clean
run. No CRITICAL wrong-accept in the comparison machinery was found. The weaknesses are
in what the oracle never looks at and in what the stamp fails to invalidate: the verify
stamp does not track the code identity of the engine, translator, or generator (the one
version ingredient is pinned at 0.1.0), so `bench` will brand a changed, never-verified
engine as VERIFIED from a stale stamp; and the query generator plus family set have
enumerable coverage holes — no cross-atom residual comparison, no cyclic join, no
never-satisfied gate/empty relation, no U64 aggregate, no Bytes predicate — several of
which are coverage the validation doc explicitly promises. Engine bugs in exactly those
subsystems (residual placement/evaluation, dynamic cover choice on cycles, gate
execution, u128 sum accumulation) are currently invisible to the oracle.

## Findings

### [HIGH] The verify stamp does not invalidate on code changes — bench brands unverified engines VERIFIED

`crates/bumbledb-bench/src/verify.rs:68-76` (`stamp_value`), `driver.rs:174-187`
(`stamp_is_fresh`), `driver.rs:406-408`. Invariant at stake: 00-product success
criterion 1 / 50-validation — "exact result-set equality … before any timing claim;
`bench` refuses to time without the stamp." The stamp hashes: crate version, corpus
digest, family digest, case count, seed. The corpus digest covers generated *content*
(so `gen.rs` data changes re-baseline, good) and the family digest covers family IR +
golden SQL — but nothing covers the engine, the translator, the comparison code, the
randomized query generator, or the family *param* functions, except
`CARGO_PKG_VERSION`, which is pinned at `0.1.0` with no history of ever being bumped
(`crates/bumbledb-bench/Cargo.toml:3`). Concrete scenario: run `verify` (stamp lands in
the digest-keyed corpus dir), then change the engine's join executor arbitrarily,
rebuild, run `bumbledb-bench bench` — `stamp_is_fresh` recomputes the identical stamp
value, the gate passes, and the report carries the stamp as its verification evidence
(`driver.rs:505-507`) for a binary that was never compared against SQLite.
`scripts/bench.sh` mitigates by always running verify first, but the stamp mechanism is
supposed to be the gate, and nothing forces the script path. Fix direction: fold a build
identity into the stamp — hash of the compiled binary, `git describe --dirty`, or at
minimum a build-time content hash of the bench + engine crate sources; alternatively
make the stamp process-local (verify and bench in one invocation) instead of a file.

### [MEDIUM] No oracle query ever contains a cross-atom residual comparison

`crates/bumbledb-bench/src/querygen.rs:361-489` (`dress`/`dress_posting`),
`families.rs` (all eight). Invariant: 20-query-ir normalization sends *only*
comparisons whose sides come from different atoms to the residual list;
30-execution gives residuals their own machinery (earliest-node placement,
per-node evaluation, vectorized survivor compaction; `plan/fj.rs` has
`UnplacedResidual`, `PlacedComparison`). Every comparison the generator emits is
single-atom: `i64_range`, `enum_eq`, memo Eq/Ne, bool Eq are var-vs-constant on one
atom's fields; the one var-vs-var case (`dress_posting` arm 4) compares `amount` vs
`at` of the *same* Posting atom. The families likewise: chain's `at >= ?0` and skew's
`label = ?0` are single-atom. So every generated comparison lowers to a per-atom
filter, and the residual path — placement at the earliest node with both sides bound,
evaluation interleaved with probes, batch compaction — is never cross-checked against
SQLite. Concrete scenario a residual bug would need: `Q(x, y) :- Posting₁(amount = x,
transfer = t), Posting₂(amount = y, transfer = t), x < y` (self-join already generated;
only the cross-atom `x < y` is missing) — an engine that evaluated the residual at the
wrong node or with swapped sides would sail through verify. Fix direction: extend the
self-join shape with a cross-atom amount comparison (both sides typed i64 by
construction) and add one family with a cross-atom residual.

### [MEDIUM] Gates are only ever tested in the always-true direction; no relation is ever empty

`querygen.rs:510-519` (Gated always gates `TAG`), `gen.rs:89-127` (every relation
nonempty at every scale: tags = 256). Invariant: a zero-binding atom is a nonemptiness
gate — empty relation ⇒ empty result (20-query-ir). Since the corpus always has 256
tags, `EXISTS (SELECT 1 FROM "Tag")` and the engine's gate both evaluate true in every
verify case ever run; an engine that *dropped gate atoms entirely* (planner elides the
occurrence, never checks emptiness) would produce identical results on every case and
pass verify. 50-validation's generator coverage contract explicitly lists "empty
relations" as required coverage; the oracle suite has none (the differential/property
families in the engine crate may, but the SQLite oracle does not). Fix direction: give
the corpus an always-empty relation (or a scale variant with `tag_notes = 0`) and gate
on it in some fraction of Gated queries; assert at least one gate-false case per run.

### [MEDIUM] No cyclic join anywhere in the oracle, though 50-validation promises one

`families.rs:370-437` (eight families: point/fk_walk/chain/range/balance/stats/
string/skew — all acyclic star/chain shapes), `querygen.rs` (guard, star, chain,
self-join, gated, aggregate — all acyclic). 50-validation names "a cyclic-ish join for
WCOJ honesty" in the family list; it does not exist. This matters more than a generic
hole: 30-execution records that the paper's cover definition was *wrong on skewed data,
demonstrated with a triangle query* (the rebinding bug found by audit) — i.e., the
dynamic-cover/GJ-style machinery's known-dangerous class lives exactly on cyclic
shapes, and the engine-side regression test is the only thing watching it. The oracle,
whose entire purpose is catching what same-author tests share blindness to, never
executes a cycle. Concrete scenario: a triangle over the existing schema is writable
today — `Q :- AccountTag(account = a, tag = t), Posting(account = a, transfer = w),
Transfer(id = w, …)` extended to close a cycle, or a two-relation cycle via
Posting/AccountTag joined on account *and* a second shared variable. Fix direction: add
a triangle family (three atoms, three shared variables, cyclic hypergraph) with a
hand-written golden, and/or a Triangle shape in querygen.

### [MEDIUM] Aggregates over U64 are never generated, and the U64 rule has a hole for sums

`querygen.rs:267-307` (`aggregate`: Sum(amount:i64), Min(at:i64), Max(amount:i64),
Count only), `families.rs` (balance/stats: same types). Invariants: 20-query-ir defines
`Sum(U64)→U64` with a u128 accumulator; the 50-validation U64 rule constrains *stored
values* to `< 2^63`. Two consequences: (a) the engine's entire u128 Sum path and
U64 Min/Max fold are never cross-checked against SQLite; (b) the mapping axiom does not
extend to sums — a legal `Sum(U64)` whose true value lands in `[2^63, 2^64)` is
*unrepresentable* on the oracle side: confirmed empirically, SQLite raises `integer
overflow` (`SELECT SUM(x)` over `9223372036854775806 + 10`), which
`compare::from_sqlite` returns as `Err` and `verify.rs:129` turns into a panic
(`expect("oracle executes")`) — no bundle, and a human may misread it as a tool bug
rather than a semantics divergence. Related, confirmed: SQLite SUM raises on
*transient* running-sum overflow (`i64::MAX, 1, -2` errors) where the engine's i128
accumulator returns the in-range final value — a real semantic divergence, currently
unreachable only because corpus amounts are bounded (±5×10⁶ × 10⁷ rows ≈ 5×10¹³ max).
Fix direction: generate U64 aggregates over id-typed fields (bounded domains keep sums
< 2^63); document the Sum-range corollary of the U64 rule in 50-validation
("generator must also bound reachable sums below 2^63"); convert the oracle-executes
expect into a comparison failure carrying a bundle so error-vs-value divergences are
arbitration artifacts, not panics.

### [MEDIUM] The generator's asserted coverage contract is weaker than the documented one

`querygen.rs:543-643` (`Coverage` counts ops/aggregates/shapes in aggregate),
50-validation ("every comparison op on every legal type … the cyclic query … empty
relations"). What is provably never generated, enumerated (each verified against the
shape/dressing code): Eq/Ne on Bytes (no dressing touches `Transfer.extref`; the
`param_value` Bytes arm and the enum/bool param arms are dead code — params anchor only
to U64/I64/String fields); Ne on Enum and Bool (only Eq literals); Lt/Le/Gt/Ge on U64
(ordered dressing is i64-only, so the engine's order-preserving u64 word order is
never oracle-checked); Ne against an i64 literal/param (Ne appears only on memo and on
the same-atom var-var pair); U64 param misses in randomized queries (`SetKind::Miss`
for U64 draws *in-domain* — `querygen.rs:748-754` — so only the point/fk_walk families
probe nonexistent ids); more than two occurrences of one relation; multiple aggregates
in one randomized find list (stats family only); Bool group keys; gate+aggregate
combinations (`Gated` draws from the four non-aggregate shapes only). The coverage test
asserts per-op *totals* > 0, so `cmp_lt > 0` is satisfied entirely by i64 — the
per-(op, type) matrix the doc describes is not what is asserted. Each hole is a place
an engine bug class (per-type comparison kernels, sentinel handling for Bytes, u64
ordered probes) is unobservable. Fix direction: either extend the generator to the
documented matrix (Bytes Eq/Ne via extref with hit/miss values, u64 ordered ranges over
id domains, enum/bool Ne, U64 param misses) or amend 50-validation to the contract the
code actually keeps — README rule 5 requires one or the other.

### [LOW] A NUL byte in a String literal silently truncates the SQL statement

`translate.rs:33-36` (`sql_string_literal`). Invariant: the translator claims totality
over valid IR ("total, mechanical"). `Value::String` may legally contain 0x00 (valid
UTF-8; the engine interns and matches it correctly), but the translator splices it raw
into the SQL text, and SQLite's tokenizer treats NUL as end-of-input even when given an
explicit length — the statement truncates inside an unterminated string literal and
fails to prepare, which `verify.rs:127` escalates to a panic. Confirmed empirically
(via the Python sqlite3 driver, which pre-rejects; SQLite core truncates). Not
generator-reachable today (all generated strings are `m{n}` / `missing-{n}` /
vocabulary formats), so this is fragility, not a live bug. Fix direction: reject NUL in
`sql_string_literal` with a named error, or emit such literals as
`CAST(X'…' AS TEXT)`.

### [LOW] Divergence-by-error is a panic, not a mismatch bundle

`verify.rs:120-129`: `prepare`, engine `execute`, oracle `prepare_cached`, and oracle
execute are all `expect`ed. Invariant: "the run fails loudly with arbitration bundles."
If either engine errors where the other returns rows — engine `Overflow` vs SQLite
value, SQLite `integer overflow` vs engine value (see the MEDIUM Sum finding), engine
`Corruption` — verify aborts with a panic and *no bundle*: loud (no stamp is written,
so nothing is wrongly accepted), but the arbitration artifact the protocol promises is
lost, and the failure is indistinguishable from a tool defect. Fix direction: treat
one-side-errors as a mismatch case (bundle with the error text on the erring side).

### [LOW] Boundary param sets probe only domain minima

`querygen.rs:750-795` (`SetKind::Boundary`: U64 → 0, I64 → window `lo`, Enum → 0,
Bool → false). Maxima (`domain - 1`, window `hi`, last enum ordinal), i64 extremes, and
off-by-one values just outside the window are never bound as params, so `Lt/Le/Gt/Ge`
edge behavior at the top of each domain is exercised only when a random draw happens to
land there. i64::MIN/MAX never flow through the oracle at all (delegated to non-oracle
property tests by the mapping table — a documented decision for storage, but the
*comparison* path near extremes is oracle-blind too; for what it is worth,
`x = -9223372036854775808` was confirmed exact in SQLite, typeof integer). Fix
direction: add a fourth boundary flavor drawing maxima and window edges.

### [NOTE] The balance family's Sum is not a balance — duplicate amounts collapse (both engines agree)

`families.rs:199-237`. The query binds only `(account, amount)`; per 20-query-ir the
fold domain is the distinct binding set, so two postings of equal amount on one account
fold *once*. The docs' own example ("Sum(amount) by account = 200") holds only when the
serial id is bound, which this family does not do. The translation is faithful (inner
`SELECT DISTINCT account, amount` collapses identically — verified empirically), so the
oracle is correct; but at S-scale the hot account (~25k postings over ~10⁷ distinct
amounts) has a nonzero expected collision count, meaning the family verifies and times
a distinct-amount fold, not a ledger balance. If the benchmark's intent is a true
balance, bind `id` in the Posting atom (which also lets the elision flag engage). Worth
an explicit sentence in 50-validation either way.

### [NOTE] A stamp can be earned with zero randomized cases

`cli.rs:188`, `driver.rs:127-149`: `verify --cases 0` runs families only, writes the
sidecar `verify.cases = 0`, and `bench` accepts the resulting stamp. The stamp encodes
the case count, so it is honest about *what* was verified, but no minimum evidence
level is enforced anywhere. Consider a floor (or branding the report with the case
count next to the stamp).

### [NOTE] Family param functions are outside the family digest

`families.rs:487-507`: `digest()` covers name + query IR debug + golden SQL. The param
*policies* (which values, which misses) are code; changing `point_params` to drop the
miss set alters verify evidence and bench workload without moving the stamp or any
report key. Subsumed by the HIGH stamp finding but worth its own line since
`families.rs` advertises the digest as the family identity.

## Checked and sound

- **Projection translation.** `SELECT DISTINCT` over find columns equals the engine's
  distinct-projected-binding set for every query shape: projection of a distinct set
  equals distinct of the projected bag; both sides produce sets, and multiset
  comparison of two sets is set equality. Var equating (first-binding column + equality
  predicates for later bindings, cross-atom and in-atom) is correct, including
  self-joins and the repeated-var fusion.
- **The aggregate template.** Inner `SELECT DISTINCT` over *all* bound variables is
  exactly the engine's fold domain (validation guarantees every query variable is
  bound); `GROUP BY` over the non-aggregated finds matches the engine's group key
  (verified for U64/I64/String/Enum keys — TEXT grouping is BINARY-collation byte
  equality ≡ intern-id equality); `COUNT(*)` = |binding set|; Min/Max are
  duplicate-insensitive; duplicate amounts across distinct bindings fold identically on
  both sides (confirmed by direct SQL experiment). `HAVING COUNT(*) > 0` without
  GROUP BY is legal SQLite, returns zero rows over empty input, and passes nonempty
  groups through untouched (confirmed empirically); grouped queries need no rule since
  SQL emits no empty groups and the engine has none. Output alias resolution in
  GROUP BY confirmed.
- **Miss-sentinel semantics.** For every legal operator: Eq against a never-interned
  String/Bytes value matches nothing on both sides; Ne matches everything on both sides
  (engine sentinel u64::MAX is never minted — `storage/dict.rs:103` asserts;
  engine-side regression test at `api/prepared.rs:1384` exists). Ordered comparisons on
  String/Bytes are unrepresentable (validation), so the translator's "no special case"
  claim is complete.
- **Value mapping.** Both directions typed, total, and mutually inverse on the legal
  domain; `from_sql_value` rejects wrong storage classes, negative INTEGER for U64,
  out-of-range enum ordinals, non-UTF-8 TEXT, and (via the catch-all) NULL. STRICT
  tables + NOT NULL make storage-class drift impossible on the SQLite side. Bytes map
  to BLOB, never TEXT; `X'…'` literals (including empty) are correct. Bool renders as
  0/1 INTEGER. The `u64 < 2^63` axiom is enforced at all three sites: literal rendering
  errors (`translate.rs:43-45`), param binding panics (`sqlmap.rs:168`), corpus
  generation asserts (`gen.rs:166-169`); within the axiom, INTEGER order equals u64
  order, so u64 Eq/Ne through the mapping is exact.
- **Ordered i64 comparisons.** Generated windows cross the sign boundary (amounts
  ±5×10⁶), literals and params both exercised; `x = / >= -9223372036854775808` parses
  as an exact INTEGER in SQLite (confirmed), so even the extreme literal would
  translate correctly.
- **Comparison machinery.** `Owned` equality is exact per type with no cross-type
  coercion possible (column types fix the decode); sort + two-pointer diff is a correct
  multiset difference (duplicate-count differences surface — unit-tested); mismatch is
  decided by full `ours == theirs` *before* exemplar collection, so the 8-exemplar cap
  can never mask an inequality; engine-side and SQLite-side rows decode through the
  same expected-type vector from `PreparedQuery::column_types` (Count→U64,
  Sum(I64)→I64, Min/Max→input type — verified in `api/prepared.rs:481-510`).
- **Param plumbing.** Translator placeholder allocation (`param_ref`) dedups repeated
  `ParamId`s to one `?N` (fk_walk exercises reuse); `from_sqlite` binds
  `params[param_order[i]]` positionally — the ParamId→placeholder mapping is correct
  by construction and identical between verify and the timed runner.
- **Verify protocol.** No path swallows a mismatch: stale bundles and the stamp are
  deleted before the run; every comparison failure writes a full bundle (query IR, SQL,
  params, diff, golden when present — exercised by the deliberate-wrong-SQL test); the
  8-bundle cap stops *collecting* but still fails the run; the stamp is written only
  when zero bundles exist; tool errors panic (no stamp). The stamp is deterministic and
  ingredient-sensitive for the ingredients it has (seed, cases, corpus content, family
  IR+SQL, version) — the missing ingredients are the HIGH finding.
- **Corpus parity.** Both stores load the identical generated stream (bumbledb via
  `bulk_load`, SQLite via param-bound prepared inserts — no text-format coercion
  anywhere); serial ids and the AccountTag pair construction make duplicate facts
  impossible, so set-vs-bag storage cannot diverge; `corpus_digest` streams actual row
  content, so any generator change re-keys the cache directory and invalidates the
  stamp; `assert_loaded_equal` cross-checks counts and sampled values.
- **Family goldens.** All eight goldens semantically re-derived against their
  documented Datalog and IR by hand in this audit (join structure, literal ordinals,
  param positions, aggregate columns, group keys) — they agree; the
  translator==golden pin plus hand-written provenance gives the intended two-source
  arbitration.
- **Gate translation.** Zero-binding atoms become uncorrelated `EXISTS` conjuncts —
  no FROM entry, no multiplicity effect, exactly the engine's Cartesian nonemptiness
  semantics (including gate + bound occurrence of the same relation); the
  all-gates-only query is the one documented totality exception and is rejected with a
  named error.
- **TEXT comparison fidelity.** No collation is ever declared; SQLite BINARY collation
  is memcmp over the stored bytes (full length — embedded NULs stored via parameters
  compare correctly), no Unicode normalization on either side; TEXT and BLOB never mix
  (STRICT + distinct mapping), so DISTINCT/GROUP BY/Eq/Ne over strings and bytes is
  byte equality on both engines in all reachable cases.
