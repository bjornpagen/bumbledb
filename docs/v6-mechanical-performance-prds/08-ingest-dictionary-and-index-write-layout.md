# PRD 08: Ingest Dictionary And Index Write Layout

## Goal

Optimize bulk ingest mechanics for JOB-like datasets by reducing dictionary intern overhead and index-entry write amplification.

This PRD targets load time, not query latency.

## Background

JOB 10k heavy trace top ingest spans:

```text
write_txn: 75.7s traced
insert: 792400 events, 57.8s busy traced
dict_intern: 2850668 events, 17.6s busy traced
put current index entry: 2357403 events
```

The database currently pays substantial cost per inserted row and per dictionary lookup. If ingest matters, this path needs a dedicated mechanical pass.

## Explicit Non-Goals

- No backwards compatibility.
- No migrations.
- No old storage readers.
- No changing logical row semantics.
- No weakening uniqueness or foreign key enforcement.
- No query algorithm work.
- No relation-name-specific ingest hacks.

## Code Anchors

Expected areas:

```text
storage.rs
bulk_load
insert
dictionary intern path
index entry write path
storage_schema.rs
Row encoding
LMDB write transaction batching
```

## Required Measurement

Before implementation, produce ingest-specific artifacts:
```sh
RUST_LOG="bumbledb_lmdb=debug" cargo run -p bumbledb-bench --release -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --open-limit 10000 \
  --query job_q09_voice_us_actor \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-ingest-baseline-job-q09.json
```

The query filter still loads the dataset, so this can be used as a load benchmark.

Record:

```text
rows loaded
dictionary intern calls
dictionary hit/miss counts
index entries written
LMDB put calls if available
load wall time
```

## Candidate Improvements

### 1. Bulk Dictionary Interning

Instead of interning strings/bytes one value at a time during row insert:

- collect string/bytes values per relation/field during bulk load
- sort/dedup in memory
- assign dictionary IDs in batches
- rewrite row encoding with intern IDs

This is allowed to be a bulk-load-only path.

### 2. Per-Field Dictionary Caches

During one bulk load, maintain local maps:

```rust
HashMap<Vec<u8>, InternId>
```

or more efficient borrowed/interned alternatives to avoid repeated LMDB dictionary lookups for repeated values.

The trace shows many `dictionary value already interned` events, so repeated lookup avoidance is likely valuable.

### 3. Batched Index Entry Construction

Build encoded index entries into contiguous buffers per relation/index, sort, and write sequentially where LMDB permits.

Current event volume suggests write amplification from per-row per-index entry construction.

### 4. Foreign Key / Unique Validation In Bulk

When loading a new database or empty target, validate constraints in sorted batches instead of row-by-row lookup if correctness permits.

Do not weaken constraints.

### 5. Write Transaction Chunking

Measure whether one massive write transaction or chunked writes are better for LMDB and current durability semantics.

No compatibility requirement. Choose the fastest correct design.

## Required Tests

- Bulk load result matches row-by-row insert for representative fixtures.
- Duplicate rows remain set-semantic no-ops.
- Unique violations fail correctly.
- Foreign key violations fail correctly.
- Dictionary IDs remain stable within a loaded database.
- Reopen after bulk load returns same rows.
- Snapshot isolation tests still pass.
- Crash/failpoint tests still pass if relevant to modified paths.

## Required Benchmarks

Run:

```sh
cargo run -p bumbledb-bench --release -- \
  --preset job \
  --query job_q09_voice_us_actor \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --open-limit 10000 \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-ingest-job-q09.json
```

Run full JOB 10k:

```sh
cargo run -p bumbledb-bench --release -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --open-limit 10000 \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-ingest-job-10k.json
```

Run non-JOB to ensure query/load behavior is not broken:

```sh
cargo run -p bumbledb-bench --release -- --preset nonjob --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-ingest-nonjob.json
```

## Performance Targets

Hard gates:

- all existing query gates pass

Ingest targets:

- JOB 10k Bumbledb load time improves by at least 25%, or RCA explains why not
- dictionary intern calls or LMDB dictionary lookups drop by at least 50%, or RCA explains why not
- index-entry write construction is measurably reduced or made sequential/batched

## Passing Requirements

- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- failpoint/crash tests still pass or are explicitly documented if platform-disabled
- non-JOB gates pass
- JOB 10k gates pass
- ingest RCA documents load-time delta and dictionary/index counters

## Completion Criteria

- Bulk ingest has a measured, cleaner mechanical design.
- Query performance remains intact.
- This PRD is deleted and committed after passing.
