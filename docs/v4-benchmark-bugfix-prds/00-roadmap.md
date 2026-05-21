# V4 Benchmark Bugfix PRD Roadmap

## Purpose

This suite fixes the bugs exposed by the latest v3 benchmark trace and simplifies the query engine around basic, explainable rules.

The v3 pass recovered JOB 10k q09/q16/q24, deleted UUID, made enums byte-sized, collapsed identities to serials, and added generic compound FK support. However, the latest benchmark evidence shows two serious issues:

- Non-JOB materialized workloads now blow up because expensive static semijoin proof work runs before normal execution and then fails to prove emptiness.
- JOB looks suspiciously fast because repeated benchmark samples often measure prepared result/cache hits rather than recomputing the join/count work each sample.

The next pass must make measurement honest and make the query execution path simpler. The priority is correctness, observability, and deterministic basic rules. Do not add another pile of clever heuristics.

## Explicit Non-Goals

- No backwards compatibility.
- No migration logic.
- No compatibility shims.
- No v1/v2 benchmark semantic modes.
- No old storage readers.
- No restoring deleted schema concepts.
- No Datalog or SQL frontend work.
- No preserving confusing internal names if they obscure the fix.

## Current Evidence

Latest artifacts:

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v3-final-nonjob.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v3-final-job-10k.json
```

Non-JOB failures:

| Query | BDB avg | SQLite avg | Symptom |
|---|---:|---:|---|
| `ledger/tag_lookup_join` | `745592us` | `1275us` | massive unaccounted pre-exec overhead |
| `sailors/red_boat_sailors` | `630938us` | `4901us` | massive unaccounted pre-exec overhead |
| `tpch/supplier_nation_orders` | `270909us` | `1527us` | massive unaccounted pre-exec overhead |

JOB suspicious-fast sample results:

| Query | BDB avg | SQLite avg | Why suspicious |
|---|---:|---:|---|
| `job_q09_voice_us_actor` | `54us` | `3812us` | prepared count cache after expensive first run |
| `job_q16_character_title_us` | `13us` | `4052us` | static-empty cache after proof |
| `job_q24_voice_keyword_actor` | `16us` | `10592us` | static-empty cache after proof |

The engine may be allowed to cache immutable-snapshot results, but benchmark reporting must make that explicit and must also provide recomputation-oriented measurements.

## Engineering Principles For This Pass

- Prefer small deterministic rules over broad heuristic search.
- Prefer exact preconditions over estimated planner guesses.
- Prefer skipping an optimization over doing expensive failed proof work.
- Make every expensive optimization phase visible in timings.
- Cache negative proof attempts if they are allowed to run more than once.
- Do not hide result-cache hits in benchmark numbers labeled as execution.
- Do not add JOB-specific relation-name logic.
- Do not add backwards compatibility, migrations, or compatibility shims.
- Do not reintroduce Datalog, primary keys, UUIDs, identity allocation, or old enum encoding.

## Ordered PRDs

Implement these in order.

1. `01-baseline-and-rca-artifacts.md`
2. `02-instrument-hidden-query-phases.md`
3. `03-contain-static-semijoin-proof.md`
4. `04-cache-negative-static-proof-results.md`
5. `05-direct-paths-before-expensive-proofs.md`
6. `06-benchmark-cache-mode-separation.md`
7. `07-prepared-result-cache-policy.md`
8. `08-recover-nonjob-materialized-paths.md`
9. `09-job-performance-honesty-gates.md`
10. `10-simplify-query-optimizer-surface.md`
11. `11-final-validation-and-cleanup.md`

## Required Final State

- Non-JOB materialized workloads no longer pay failed static semijoin proof costs.
- Static semijoin proof is either cheap and successful or skipped.
- Failed static proof attempts are cached or gated so repeated samples do not repeat the same failure.
- Benchmark JSON clearly distinguishes recompute, prepared-plan, and prepared-result-cache modes.
- JOB q09/q16/q24 remain fast, but the benchmark explains whether the number is recompute or cache-hit time.
- No relation-name-specific JOB hacks return.
- All existing code quality gates pass.

## Final Gates

```sh
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --manifest-path fuzz/Cargo.toml
cargo run -p bumbledb-bench --release -- --preset nonjob --format json
cargo run -p bumbledb-bench --release -- --preset job --job-dir /var/folders/fj/10pmb37j1m1cyd1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb --open-limit 10000 --format json
```

If the JOB path typo above is copied by mistake, use the real path:

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb
```

## Final Rejection List

Active Rust code must still not contain deleted architecture concepts:

```text
datalog
Datalog
PrimaryKeyDescriptor
GeneratedIdDescriptor
RelationKind
ValueType::Uuid
Value::Uuid
UuidBytes
IdentityAllocation
IdentityValue
ValueType::Identity
Value::Identity
ValueType::Code
Value::Code
IndexKind::Primary
IndexKind::Ref
ComponentRole::Identity
KeyValues
NS_CURRENT_ROW
NS_UNIQUE_GUARD
```

Historical docs may mention these only if explicitly marked historical.
