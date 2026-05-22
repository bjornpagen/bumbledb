# V4 Current Benchmark Bugs RCA

Historical note: this document predates the set-native v4 rewrite. References to prepared count caches and direct-count behavior describe removed legacy systems, not the current architecture.

## Artifact Paths

Fresh v4 baseline artifacts:

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v4-baseline-nonjob.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v4-baseline-job-10k.json
```

Prior v3 artifacts used to identify the regression pattern:

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v3-final-nonjob.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v3-final-job-10k.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/latest-v3-nonjob-scale10000-r30.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/latest-v3-job-openlimit10000-r30.json
```

## Benchmark Commands

Non-JOB baseline:

```sh
cargo run -p bumbledb-bench --release -- --preset nonjob --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v4-baseline-nonjob.json
```

JOB 10k baseline:

```sh
cargo run -p bumbledb-bench --release -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --open-limit 10000 \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v4-baseline-job-10k.json
```

## Non-JOB Failure Table

The three hard gate failures remain explicit:

| Dataset | Query | Bumbledb Sample Avg | SQLite Sample Avg | Phase Total | Execute Field | Symptom |
|---|---|---:|---:|---:|---:|---|
| `ledger` | `tag_lookup_join` | `741885us` | `1267us` | `780175us` | `6949us` | failed static/proof work dominates materialized execution |
| `sailors` | `red_boat_sailors` | `640407us` | `5185us` | `642106us` | `13516us` | failed static/proof work dominates materialized execution |
| `tpch` | `supplier_nation_orders` | `279641us` | `1513us` | `290676us` | `8829us` | failed static/proof work dominates materialized execution |

Other visible regressions with the same shape:

| Dataset | Query | Bumbledb Sample Avg | SQLite Sample Avg | Phase Total | Execute Field |
|---|---|---:|---:|---:|---:|
| `sailors` | `high_rating_red_boats` | `527678us` | `3875us` | `522765us` | `4658us` |
| `tpch` | `revenue_by_customer_range` | `274872us` | `3873us` | `282091us` | `6977us` |

## Timing Gap Analysis

Required non-JOB observation pattern:

```text
reported sample wall time is hundreds of milliseconds
engine subphase execute/sink/image/plan totals are only milliseconds
there is a large unaccounted gap inside execute_prepared_query
```

Examples from the regression trace pattern:

```text
tag_lookup_join: avg ~745ms, execute ~6.5ms
red_boat_sailors: avg ~631ms, execute ~13ms
revenue_by_customer_range: avg ~272ms, execute ~7ms
supplier_nation_orders: avg ~271ms, execute ~9ms
```

Fresh v4 baseline values show the broad phase total is now close to wall-clock time, but the expensive work is still not attributed to a specific named subphase. The `execute_us`, `image_us`, `plan_us`, and `lftj_build_us` fields remain small relative to total time. That means the likely culprit is still hidden inside the prepared execution path rather than the measured direct LFTJ execution fields.

## Current Hypotheses

Static semijoin proof is over-eager and under-instrumented. It appears to run before normal materialized execution on non-empty queries, spend hundreds of milliseconds trying to prove emptiness, fail, and then fall through to the real plan.

The engine should either prove emptiness cheaply and successfully, skip the proof, or cache the failed proof attempt so repeated samples do not repeat the same unsuccessful work.

## JOB Suspicious-Fast Table

| Query | Cold Correctness | Bumbledb Sample Avg | SQLite Sample Avg | Plan | Runtime | Why Suspicious |
|---|---:|---:|---:|---|---|---|
| `job_q09_voice_us_actor` | `141432us` | `56us` | `3681us` | `direct_count` | `DirectKernel` | repeated samples hit prepared count cache after expensive first run |
| `job_q16_character_title_us` | `26662us` | `14us` | `4035us` | `static_empty` | `StaticEmpty` | repeated samples hit static-empty cache after proof |
| `job_q24_voice_keyword_actor` | `52296us` | `15us` | `10773us` | `static_empty` | `StaticEmpty` | repeated samples hit static-empty cache after proof |

Required JOB observation pattern:

```text
q09/q16/q24 sample times are tiny because repeated samples hit prepared count/static caches
cold correctness execution still includes proof/precompute work
headline sample numbers must be labeled as cache-assisted
```

The JOB cache behavior may be valid for immutable prepared snapshots, but benchmark JSON and gates must label it honestly. Recompute, prepared-plan, and prepared-result-cache measurements need separate names and thresholds.

## Reproduction Notes

Use the two commands above from the repository root. The JOB command requires the extracted CWI JOB CSV directory at:

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb
```

If that directory is missing, recreate or extract the JOB data before running the JOB baseline. The non-JOB baseline does not depend on the external JOB CSV directory.
