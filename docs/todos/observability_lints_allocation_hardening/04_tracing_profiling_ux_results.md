# 04 Tracing And Profiling UX Results

**Completed CLI UX**
- Added repeatable `--query NAME` filtering after dataset selection.
- Added `--warmup N`; warmup samples are reported separately from measured samples.
- Preserved `--repeats N` as the measured sample count.
- Added `--format json` and `--json` machine-readable output.
- Added `--trace-output PATH`.
- Added `--trace-format fmt|json|chrome|flame`.
- Implemented `fmt` and `json` trace formats with `tracing-subscriber` only.
- `chrome` and `flame` return typed errors until optional profiler dependencies are explicitly added.

**Completed Benchmark Reporting**
- Main result rows now keep average timings for continuity.
- Added `## Distribution` with prepare timing, warmup samples/average, measured samples, min, p50, p95, and max for Bumbledb and SQLite.
- Kept runtime kind in result and phase timing tables.
- Kept phase timing and allocation summary sections from PRD 03.
- Added interpretation notes for high image, planning, index build, execution, sink finish, and allocation signals.

**Completed Structured Output**
- JSON output includes dataset, query, rows, chosen plan, runtime, Bumbledb distribution, SQLite distribution, prepare timing, warmup timing, phase timing, allocation summary, selected counters, and gate notes.
- JSON output suppresses the human benchmark banner so it can be consumed by tools.

**Completed Trace UX**
- Default benchmark runs do not initialize a subscriber.
- `--trace`, `--trace-output`, or `--trace-format` initialize benchmark-binary tracing only.
- `RUST_LOG` is respected when set.
- Default trace filter remains `bumbledb_lmdb=debug` when `RUST_LOG` is absent.
- Trace output can go to stdout/stderr or to a file path.
- Added benchmark-level profile summary events for query and node summaries without row payloads or raw values.

**Smoke Commands**
```sh
cargo run -p bumbledb-bench --release -- --dataset sailors --query sailor_range_reserves --scale 10000 --repeats 3 --warmup 1 --format markdown
cargo run -p bumbledb-bench --release -- --dataset joinstress --query chain4_from_a --scale 10000 --repeats 3 --format json
RUST_LOG=bumbledb_lmdb=debug cargo run -p bumbledb-bench --release -- --trace-output /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-trace-prd04.jsonl --trace-format json --dataset joinstress --query chain4_from_a --scale 100 --repeats 1 --format json
```

**Verification**
- `cargo fmt --all --check`
- `cargo check --workspace --all-targets --all-features`
- `cargo clippy --workspace --all-targets --all-features -- -D warnings`
- `cargo test --workspace --all-features`
- `cargo check --manifest-path fuzz/Cargo.toml`
- `scripts/check-cutover.sh`
- `scripts/check-prd-map.sh`
- `scripts/check-performance-kill-list.sh`
- `cargo run -p bumbledb-bench --release -- --scale 10000 --repeats 3 --format markdown`

**Next PRD**
- `docs/todos/observability_lints_allocation_hardening/05_allocation_recording_and_heap_observability.md`
