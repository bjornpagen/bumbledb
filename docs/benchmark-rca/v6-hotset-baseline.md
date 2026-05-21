# V6 Hotset Baseline

## Purpose

This document records the first untraced mechanics-counter baseline for the v6 mechanical performance pass.

The goal is to explain hot loops without relying on multi-gigabyte trace files.

## Artifact Paths

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-counters-nonjob.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-counters-job-10k.json
```

## Commands

```sh
cargo run -p bumbledb-bench --release -- --preset nonjob --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-counters-nonjob.json
```

```sh
cargo run -p bumbledb-bench --release -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --open-limit 10000 \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-counters-job-10k.json
```

## Non-JOB Mechanics Table

| Query | Runtime | Sink emits | Project seen | Project dupes | LFTJ next | LFTJ seek | LFTJ keys | Direct step rows | Direct output rows | Direct storage rows | Gate |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| ledger/postings_for_holder_range | Lftj | 3 | 3 | 0 | 10 | 1 | 16 | 0 | 0 | 0 | pass |
| ledger/balances_by_instrument | Lftj | 3 | 0 | 0 | 13 | 1 | 19 | 0 | 0 | 0 | pass |
| ledger/tag_lookup_join | IndexNestedLoop | 10000 | 10000 | 0 | 0 | 0 | 0 | 20000 | 10000 | 0 | pass |
| sailors/red_boat_sailors | Lftj | 16660 | 16660 | 6660 | 34153 | 17491 | 105789 | 0 | 0 | 0 | pass |
| sailors/sailor_range_reserves | DirectKernel | 5 | 5 | 0 | 0 | 0 | 0 | 0 | 0 | 5 | pass |
| sailors/high_rating_red_boats | Lftj | 6660 | 6660 | 0 | 34153 | 17493 | 105793 | 0 | 0 | 0 | pass |
| joinstress/chain4_from_a | IndexNestedLoop | 1 | 1 | 0 | 0 | 0 | 0 | 3 | 1 | 0 | pass |
| joinstress/triangle_count | Lftj | 0 | 0 | 0 | 90000 | 119995 | 589992 | 0 | 0 | 0 | pass |
| tpch/revenue_by_customer_range | Lftj | 8000 | 0 | 0 | 20000 | 4000 | 40002 | 0 | 0 | 0 | pass |
| tpch/supplier_nation_orders | Lftj | 5716 | 5716 | 0 | 18577 | 7143 | 50013 | 0 | 0 | 0 | pass |

## JOB Mechanics Table

| Query | Runtime | Sink emits | Project seen | Project dupes | LFTJ next | LFTJ seek | LFTJ keys | Direct step rows | Direct output rows | Direct storage rows | Gate |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| job_broad_cast_keyword_company | DirectKernel | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | pass |
| job_broad_movie_info_star | DirectKernel | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | pass |
| job_q01_top_production | StaticEmpty | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | pass |
| job_q09_voice_us_actor | DirectKernel | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | pass |
| job_q16_character_title_us | StaticEmpty | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | pass |
| job_q24_voice_keyword_actor | StaticEmpty | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | pass |
| job_movie_link_bridge | Lftj | 0 | 0 | 0 | 62 | 80 | 432 | 0 | 0 | 0 | pass |
| job_q33_linked_series_companies | StaticEmpty | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | pass |

## Top 5 Queries By Sink Emits

| Query | Sink emits |
|---|---:|
| sailors/red_boat_sailors | 16660 |
| ledger/tag_lookup_join | 10000 |
| tpch/revenue_by_customer_range | 8000 |
| sailors/high_rating_red_boats | 6660 |
| tpch/supplier_nation_orders | 5716 |

## Top 5 Queries By LFTJ Operations

LFTJ operations here are `lftj_next_calls + lftj_seek_calls + lftj_key_reads`.

| Query | LFTJ operations |
|---|---:|
| joinstress/triangle_count | 799987 |
| sailors/high_rating_red_boats | 157439 |
| sailors/red_boat_sailors | 157433 |
| tpch/supplier_nation_orders | 75733 |
| tpch/revenue_by_customer_range | 64002 |

## Top Queries By Direct Chain Rows

| Query | Direct chain step rows |
|---|---:|
| ledger/tag_lookup_join | 20000 |
| joinstress/chain4_from_a | 3 |

## Top Queries By Encoded Project Duplicates

| Query | Duplicate encoded project rows |
|---|---:|
| sailors/red_boat_sailors | 6660 |

## Initial Hypotheses

### PRD 03: Batched Encoded Projection Sink

The strongest projection-sink target is `red_boat_sailors`:

```text
sink emits: 16660
encoded project rows seen: 16660
duplicate rows: 6660
```

The current sink inserts into a dedup structure during emit. The next PRD should replace per-emit set insertion with append-first encoded row buffering, sort/dedup at finish, and exact distinctness fast paths where provable.

Expected secondary beneficiaries:

```text
tag_lookup_join
revenue_by_customer_range
high_rating_red_boats
supplier_nation_orders
```

### PRD 04: Batched Direct Materialization

`tag_lookup_join` is the direct-chain hot target:

```text
sink emits: 10000
direct chain step rows: 20000
direct chain output rows: 10000
```

The direct-chain path should batch encoded output rows directly into the projection sink, avoid generic `TupleSink::emit` per output where possible, and reuse binding storage.

### PRD 05: LFTJ Iterator Mechanics

The LFTJ hot targets are:

```text
triangle_count: 799987 LFTJ operations
high_rating_red_boats: 157439 LFTJ operations
red_boat_sailors: 157433 LFTJ operations
supplier_nation_orders: 75733 LFTJ operations
revenue_by_customer_range: 64002 LFTJ operations
```

`triangle_count` is count-like and has no sink emits, so it is the best pure iterator/intersection target. The high-output materialized queries combine LFTJ iterator work with projection emission work.

### PRD 06: Width-Specialized Encoded Operations

Width-specialized comparisons should be evaluated after projection/direct batching. The strongest candidates are:

```text
width 1 enums in static proof and filters
width 8 serial/int/timestamp in LFTJ and direct predicates
```

ARM NEON-only SIMD is specified in PRD 06. No x86 SIMD path is allowed.

## Notes

JOB query mechanics are already sparse for sink/LFTJ operations. JOB query wins depend primarily on direct count, static proof, and query-image behavior. JOB ingest remains a separate dictionary/index-write problem for PRD 08.
