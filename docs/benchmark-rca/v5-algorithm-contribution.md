# V5 Algorithm Contribution Baseline

## Artifact Paths

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v5-baseline-nonjob.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v5-baseline-job-10k.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v5-baseline-job-q09-prepared-result.json
```

## Commands

```sh
cargo run -p bumbledb-bench --release -- --preset nonjob --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v5-baseline-nonjob.json
```

```sh
cargo run -p bumbledb-bench --release -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --open-limit 10000 \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v5-baseline-job-10k.json
```

```sh
cargo run -p bumbledb-bench --release -- \
  --preset job \
  --query job_q09_voice_us_actor \
  --cache-mode prepared-result \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --open-limit 10000 \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v5-baseline-job-q09-prepared-result.json
```

## Summary

| Suite | Queries | BDB wins | BDB losses | Gate failures |
|---|---:|---:|---:|---:|
| Non-JOB | 10 | 2 | 8 | 0 |
| JOB 10k | 8 | 8 | 0 | 0 |

## Non-JOB Runtime Families

| Dataset | Query | BDB us | SQLite us | Plan family | Runtime | Chosen plan | Gate |
|---|---|---:|---:|---|---|---|---|
| ledger | postings_for_holder_range | 54 | 5 | FreeJoinLftj | Lftj | pure_lftj | pass |
| ledger | balances_by_instrument | 53 | 6 | FreeJoinLftj | Lftj | pure_lftj | pass |
| ledger | tag_lookup_join | 7279 | 1329 | IndexNestedLoop | IndexNestedLoop | direct_materialized | pass |
| sailors | red_boat_sailors | 7210 | 5158 | FreeJoinLftj | Lftj | pure_lftj | pass |
| sailors | sailor_range_reserves | 9 | 2 | Direct | DirectKernel | direct_storage | pass |
| sailors | high_rating_red_boats | 5537 | 3937 | FreeJoinLftj | Lftj | pure_lftj | pass |
| joinstress | chain4_from_a | 17 | 4 | IndexNestedLoop | IndexNestedLoop | direct_materialized | pass |
| joinstress | triangle_count | 10538 | 14906 | FreeJoinLftj | Lftj | pure_lftj | pass |
| tpch | revenue_by_customer_range | 2962 | 3890 | FreeJoinLftj | Lftj | pure_lftj | pass |
| tpch | supplier_nation_orders | 3331 | 1567 | FreeJoinLftj | Lftj | pure_lftj | pass |

## JOB Runtime Families

| Query | BDB us | SQLite us | Plan family | Runtime | Chosen plan | Cache mode | Prepared result hits | Static empty hits | Gate |
|---|---:|---:|---|---|---|---|---:|---:|---|
| job_broad_cast_keyword_company | 371 | 5638 | Direct | DirectKernel | direct_count | prepared-plan | 0 | 0 | pass |
| job_broad_movie_info_star | 437 | 57657 | Direct | DirectKernel | direct_count | prepared-plan | 0 | 0 | pass |
| job_q01_top_production | 187 | 844 | StaticEmpty | StaticEmpty | static_empty | prepared-plan | 0 | 0 | pass |
| job_q09_voice_us_actor | 908 | 3791 | Direct | DirectKernel | direct_count | prepared-plan | 0 | 0 | pass |
| job_q16_character_title_us | 584 | 3923 | StaticEmpty | StaticEmpty | static_empty | prepared-plan | 0 | 0 | pass |
| job_q24_voice_keyword_actor | 607 | 10362 | StaticEmpty | StaticEmpty | static_empty | prepared-plan | 0 | 0 | pass |
| job_movie_link_bridge | 130 | 140 | FreeJoinLftj | Lftj | pure_lftj | prepared-plan | 0 | 0 | pass |
| job_q33_linked_series_companies | 56 | 63 | StaticEmpty | StaticEmpty | static_empty | prepared-plan | 0 | 0 | pass |

## Algorithm Contribution

### Mixed Hash/LFTJ

No benchmark query in the v5 baseline selected `Mixed` or a mixed/hybrid plan family.

This makes mixed hash/LFTJ the first deletion candidate. Its value must come from correctness tests or unbenchmarked shapes, not from the full benchmark suite.

### Hash Probe

No benchmark query in the v5 baseline selected `HashProbe`.

This makes hash probe the second deletion candidate. Its value must come from correctness tests or unbenchmarked shapes, not from the full benchmark suite.

### StaticEmpty

Used by:

```text
job_q01_top_production
job_q16_character_title_us
job_q24_voice_keyword_actor
job_q33_linked_series_companies
```

Static proof remains valuable. q16 and q24 are core recovered JOB gates.

### DirectKernel / IndexNestedLoop

Used by:

```text
ledger/tag_lookup_join
sailors/sailor_range_reserves
joinstress/chain4_from_a
job_broad_cast_keyword_company
job_broad_movie_info_star
job_q09_voice_us_actor
```

Direct kernels and direct index-nested-loop are benchmark-proven and should not be deletion candidates in this pass.

### LFTJ / FreeJoinLftj

Used by most non-JOB materialized joins and `job_movie_link_bridge`.

Pure LFTJ is the general join fallback and should not be deleted in this pass.

## q09/q16/q24 Cache Behavior

Prepared-plan mode:

```text
q09: 908us, prepared_result_cache_hits=0
q16: 584us, static_empty_cache_hits=0
q24: 607us, static_empty_cache_hits=0
```

Prepared-result q09 targeted artifact:

```text
q09: 56us, prepared_result_cache_hits=30
```

The benchmark output is honest: prepared-plan numbers are not result-cache hits, and prepared-result q09 explicitly reports result-cache hits.

## Deletion Decision Criteria

An algorithm may be hard-deleted only if all are true after deletion:

- `cargo test --workspace --all-features` passes.
- full non-JOB benchmark has zero gate failures.
- JOB 10k benchmark has zero gate failures.
- q09/q16/q24 still pass their cache-mode honesty gates.
- performance regressions are documented and accepted by the PRD if they are below existing gates.
- no permanent disable switch remains.

If deletion fails these criteria, revert only that deletion attempt, document the reason, and keep the algorithm with measured evidence.
