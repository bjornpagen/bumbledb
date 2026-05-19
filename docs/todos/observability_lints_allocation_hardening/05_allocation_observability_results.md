# 05 Allocation Recording And Heap Observability Results

**Completed Allocation Telemetry**
- Added `bumbledb-lmdb::allocation` snapshot/delta APIs.
- Added feature-gated `allocation-telemetry` support in `bumbledb-lmdb`.
- Added `alloc-profile` feature in `bumbledb-bench`.
- Added a benchmark-only `#[global_allocator]` wrapper behind `alloc-profile`.
- Normal library users and default benchmark runs do not install a custom allocator.

**Recorded Counters**
- Allocation calls.
- Deallocation calls.
- Reallocation calls.
- Bytes allocated.
- Bytes deallocated.
- Net bytes.
- Current live byte delta.
- Peak live byte delta.
- Allocation size-class histogram.

**Phase Attribution**
- Total query.
- Validate inputs.
- Normalize.
- Encode inputs.
- QueryImage acquisition.
- Planning.
- LFTJ build.
- Hash index build/lookup.
- Execute.
- LFTJ execute.
- Hash execute.
- Sink finish.

**Benchmark Output**
- Markdown `## Allocation Summary` now includes current live bytes and peak live bytes.
- Markdown now includes `## Allocation Phase Detail` with per-phase allocation calls, bytes allocated, net bytes, current live bytes, and peak live bytes.
- JSON output includes allocation phases and size-class histograms.

**Profile-Enabled Smoke**
Command:

```sh
cargo run -p bumbledb-bench --features alloc-profile --release -- --dataset joinstress --query chain4_from_a --scale 10000 --repeats 3 --format markdown
```

Observed result:

| Dataset | Query | Alloc Calls | Bytes Allocated | Net Bytes | Peak Live Bytes | Gate |
|---|---|---:|---:|---:|---:|---|
| joinstress | chain4_from_a | 103377 | 46288043 | 17943451 | 17943451 | pass |

Key phase deltas:

| Phase | Alloc Calls | Bytes Allocated | Net Bytes |
|---|---:|---:|---:|
| normalize | 46 | 2375 | 2375 |
| encode_inputs | 2 | 76 | 68 |
| query_image | 1 | 1394 | 0 |
| plan | 13148 | 16313027 | 16235 |
| hash_index | 90123 | 29967820 | 17924948 |
| execute | 90175 | 29970912 | 17925128 |
| sink_finish | 2 | 224 | -390 |

**Deep Profiling Guidance**
- Use allocation counters first to identify which query phase needs callsite attribution.
- Use `dhat` or jemalloc profiling separately from normal gates when callsites are needed.
- Do not capture backtraces in the cheap allocator hook because that would distort benchmark results.

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
- `cargo run -p bumbledb-bench --features alloc-profile --release -- --dataset joinstress --query chain4_from_a --scale 10000 --repeats 3 --format markdown`

**Next PRD**
- `docs/todos/observability_lints_allocation_hardening/06_stack_gat_and_hot_path_allocation_cleanup.md`
