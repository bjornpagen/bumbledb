# V6 Ingest Dictionary And Index Write Decision

## Purpose

Document the ingest optimization investigation for JOB-like datasets.

This PRD attempted a low-risk transaction-local dictionary intern cache, measured it, rejected it, and reverted it. No ingest code change was kept.

## Artifacts

Current measured artifacts:

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-ingest-job-q09.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-ingest-job-q09.stderr.log
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-ingest-job-10k.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-ingest-job-10k.stderr.log
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-ingest-nonjob.json
```

Prior trace evidence:

```text
docs/benchmark-rca/current-heavy-trace-analysis.md
```

## Commands

Focused JOB q09 load benchmark:

```sh
RUST_LOG="bumbledb_lmdb=debug" cargo run -p bumbledb-bench --release -- \
  --preset job \
  --query job_q09_voice_us_actor \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --open-limit 10000 \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-ingest-job-q09.json \
  2> /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-ingest-job-q09.stderr.log
```

Full JOB 10k:

```sh
cargo run -p bumbledb-bench --release -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --open-limit 10000 \
  --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-ingest-job-10k.json \
  2> /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-ingest-job-10k.stderr.log
```

Non-JOB validation:

```sh
cargo run -p bumbledb-bench --release -- --preset nonjob --format json \
  > /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/v6-ingest-nonjob.json
```

## Load Results

Measured after the attempted dictionary cache, before it was rejected and reverted:

```text
q09 focused Bumbledb load: 19.237204333s
q09 focused SQLite load: 15.510065667s
full JOB 10k Bumbledb load: 19.648486084s
full JOB 10k SQLite load: 14.882800584s
```

Recent pre-attempt Bumbledb JOB 10k load times from v6 runs were roughly 18.4s to 19.3s. The attempted transaction-local dictionary cache did not improve load time and may have added overhead from cache-key allocation.

## Attempted Change

Attempted but reverted:

```text
transaction-local BTreeMap<(kind, raw bytes), intern id>
```

Why it was rejected:

- It did not improve JOB 10k load time.
- It required allocating `raw.to_vec()` for dictionary cache keys on every intern attempt.
- It added storage-path complexity without a measured win.
- It did not address index-entry write amplification.

## Remaining Ingest Evidence

Heavy trace still shows ingest hot spots:

```text
insert: 792400 events, 57.8s traced busy
dict_intern: 2850668 events, 17.6s traced busy
put current index entry: 2357403 events
```

The right future ingest work is not a naive per-transaction dictionary cache. The likely useful design is a true bulk-load pipeline:

1. Collect dictionary candidates per field while streaming rows.
2. Sort/dedup dictionary values in memory.
3. Assign intern IDs in batches.
4. Encode rows from intern-ID maps.
5. Build per-relation/per-index key slabs and write them sequentially.
6. Validate uniques/FKs in sorted batches where correctness permits.

That is a larger architectural PRD and should be done as a dedicated ingest pipeline redesign, not as a small cache patch.

## Gate Results

- non-JOB gates: pass
- JOB 10k gates: pass
- query behavior unchanged after reverting the attempted cache

## Recommendation

Defer ingest optimization until after query-side v6 work completes, unless load time becomes a product priority.

When reopened, do not implement another lookup cache first. Implement a real bulk dictionary/index build pipeline.

## Compatibility Statement

No backwards compatibility. No migrations. No ingest code change was kept in this PRD.
