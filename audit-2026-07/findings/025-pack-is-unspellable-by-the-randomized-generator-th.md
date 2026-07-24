## The random verify lane's type caps the grammar at SQL-expressible; Pack query shapes are structurally unreachable by any randomized lane

category: missing-free-feature | severity: medium | verdict: CONFIRMED | finder: r2:differential-apparatus-soundness

### Summary

The randomized verify lane translates every generated query to SQLite and panics if translation fails (`.expect("generated queries translate")`), so the query generator's grammar is structurally restricted to the SQL-expressible subset of the IR. `AggOp::Pack` — the coalescing fold, the one head SQLite cannot express and the most algorithmically complex aggregate in the IR — therefore has no row in the generator's `Shape` enum and is unspellable by any randomized query lane. Its query-shape coverage is exactly three fixed shapes in the verify naive lane plus fixed shapes in the differential tests. Randomized *data* coverage for Pack is real and substantial (see Evidence — this corrects the original finding's "only coverage" claim), but randomized *query-shape* composition (Pack under conditions, param sets, param-selected arms, multi-atom joins, varied grouping arities) is zero, and the exclusion is representational: the harness treats inexpressibility as a panic instead of as routing data, so every future naive-only construct inherits the same silent exclusion.

### Evidence

All citations verified against the working tree at HEAD (89086d4f).

The structural cap:
- `crates/bumbledb-bench/src/verify/run.rs:147-148` — `let translated = translate(&query, target::schema(), &draw.sets).expect("generated queries translate");` in `random_lane`. Any grammar draw whose translation returns `Err` panics the harness, so the grammar must stay ⊆ SQL-expressible by construction.
- `crates/bumbledb-bench/src/querygen.rs:92-139` — the `Shape` enum (KeyProbe … GroundFold, 17 rows): no Pack. `grep Pack` under `querygen/` returns only the skip arm.
- `crates/bumbledb-bench/src/querygen/coverage.rs:489-493` — `AggOp::Pack => {}` with the comment "Pack heads are naive-only by the expressibility gate; their oracle rows live in the verify naive lane's own generator". The naive lane has no query generator: its Pack rows are hand-built.
- `crates/bumbledb-bench/src/verify/run_algebra.rs:235-292` (`pack_and_measure_ops`) — exactly three fixed Pack query shapes (grouped, global, two-org-arm multi-rule), each asserted `Err(Inexpressible::PackAggregate)` at run_algebra.rs:284-290 and routed naive-only.
- `crates/bumbledb-bench/src/translate.rs:197-211, 250-258` — the `Inexpressible` enum and `sqlite_expressible` exist precisely as typed routing data; the random lane never calls them.

The spec: `docs/architecture/60-validation.md` § "**`Pack` is naive-only by decision**" (lines ~173-182) and the algebra-oracle bullet (~line 855: "**`Pack`** answers (grouped, global, and the multi-rule union fold) naive-only per the expressibility gate") — the code follows the doc exactly, so this is not a code-vs-spec divergence. But the same doc's deletion record (~line 908: "Executor differential fuzzing is subsumed by the seeded generator above") is falsified for the naive-only subset: the seeded generator cannot spell Pack, so nothing randomized replaced whatever shape-space the deleted coverage-guided campaign could have reached there.

Correction to the original finding — the fixed rows are NOT the only Pack coverage:
- `crates/bumbledb-bench/src/differential/tests/pack.rs:162-185` (`randomized_claim_sets_agree_with_the_naive_model`) — 40 seeded rounds of randomized data corpora per run: overlaps, containments, exact-adjacency (`end == next.start`) and gap-1 boundaries, duplicate claims, rays, over BOTH element types including i64 with negative starts, engine vs a naive model that packs from the point-set definition (independent of the engine's word sweep, per the doc). The originally cited scenario "wrong adjacency merging for encoded i64 endpoints in a grouped Pack" is covered here.
- `crates/bumbledb/src/interval/sweep.rs:238-260` — randomized kernel property tests (`packed_output_matches_the_naive_point_set`, `coverage_verdict_matches_the_naive_subset_check`) vs the naive point set.
- `crates/bumbledb/src/api/prepared/tests/pack.rs:151-360` — four engine tests including ray absorption and the multi-rule union fold; `differential/tests/pack.rs:274-360` — the multi-rule union fold differentially (fixed 3-row corpus).
- `run_algebra`'s three queries run inside every verify run against the seeded generated mandate corpus (`verify/run_naive.rs:236-262`), so their data varies with the corpus seed.

The genuinely uncovered space: Pack composed with any of the grammar's other dimensions — dressing filters, DNF condition trees, param sets and param-set-selected arms, multi-atom joins, closed atoms, varied grouping arities, negation. No lane, fixed or randomized, exercises these, and none *can* until the random lane stops panicking on inexpressibility.

### Failure scenario

An engine bug in the Pack path that only manifests under shape composition — e.g. a seen-set interaction between the rule-union dedup and Pack's per-group claim collection when arms are selected by a param set, or grouping-key handling when the group is multi-variable or filtered through a lowered DNF tree — passes every verify run and every differential test: the randomized-data tests all use the same single-atom, condition-free (or fixed-condition) shapes. The apparatus stays green while the shape space stays at n=3 (+3 fixed differential-test shapes). Since the fuzzer was hard-deleted (2026-07-20, deletion record in 60-validation.md), no lane can reach this space even by accident, and the exclusion silently extends to every future SQLite-inexpressible construct.

### Suggested fix

Make inexpressibility routing data instead of a panic — this is the repo's own doctrine (representation over control flow; the `Inexpressible` enum was built as an enumerated set "so nothing is ever silently skipped"). Concretely: add a Pack row to the grammar (Pack composes with the existing dressing, param, and multi-rule machinery), and in `random_lane` route each draw by `sqlite_expressible(&LaneCase::Query(&query))` — `Ok` keeps the current SQLite differential; `Err(Inexpressible::PackAggregate)` routes the draw through the naive-model differential leg that `run_naive_slice`/`differential::run` already implement. Count and report the naive-only draws exactly as the algebra rows are counted today, preserving the never-silently-skipped invariant. This also repairs the deletion record's subsumption claim for whatever naive-only construct lands next.