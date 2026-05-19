# Testing

The v0 engine is protected by layered correctness, differential, concurrency, failpoint, crash, and benchmark tests.

**Fast Checks**
- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo check --manifest-path fuzz/Cargo.toml`

**Expensive Checks**
- `cargo test --workspace --release`
- `cargo test --workspace -- --ignored`

**Focused Suites**
- `cargo test -p bumbledb-test-support --test property_and_differential`
- `cargo test -p bumbledb-test-support --test concurrency`
- `cargo test -p bumbledb-test-support --test failpoints`
- `cargo test -p bumbledb-test-support --test sqlite_comparison`
- `cargo test -p bumbledb-test-support --test trybuild`

**Crash Recovery**
- Crash tests are ignored by default because they intentionally spawn subprocesses that abort.
- Run them explicitly with `cargo test -p bumbledb-test-support --test crash_recovery -- --ignored`.

**Stress**
- Stress tests are ignored by default to keep normal development fast.
- Run them with `cargo test -p bumbledb-test-support --test stress -- --ignored`.

**Fuzzing**
- Fuzz targets live under `fuzz/` and are not part of normal `cargo test`.
- Install cargo-fuzz with `cargo install cargo-fuzz`.
- Quick local examples:
- `cargo fuzz run fuzz_encoding_decode -- -max_total_time=30`
- `cargo fuzz run fuzz_datalog_parser -- -max_total_time=30`

**Miri**
- Miri should target `bumbledb-core` first.
- LMDB FFI and mmap behavior are not expected to be Miri-friendly.
- Suggested command: `cargo +nightly miri test -p bumbledb-core`.

**Policy**
- Normal tests must remain fast enough for regular development.
- Correctness failures block progress.
- Performance measurements are observational until explicit performance gates are introduced.
- If a stronger test exposes an architecture bug, fix the engine instead of weakening the test.
