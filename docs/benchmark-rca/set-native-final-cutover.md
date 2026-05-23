# Set-Native Final Cutover

## Purpose

Final artifact record for the set-native v4 rewrite cutover.

## Validation Artifacts

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/set-native-final-nonjob.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/set-native-final-job-10k.json
```

## Validation Commands

```text
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --manifest-path fuzz/Cargo.toml
```

## Cutover Summary

- Storage format is bumped and old pre-v4 formats are rejected.
- Query output is `QueryResultSet`, not a multiplicity carrier.
- Query cardinality uses `QueryResultCardinality` and is not an aggregate count value.
- Covering unique constraints are no longer the physical schema contract.
- Canonical fact membership is stored separately from access entries.
- Write paths are exact insert/delete set deltas.
- Segment publishing and history records are removed from the current write/query path.
- Factorized/product-of-fanout count kernels and scalar prepared count caches are deleted.
- Benchmark correctness compares exact SQLite values before timing samples are accepted.
- Golden examples and property/failpoint/fuzz validation cover the new set-native contract.
