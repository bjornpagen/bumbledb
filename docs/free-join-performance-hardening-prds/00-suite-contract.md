# PRD 00: Suite Contract

## Purpose

Lock the implementation contract for the performance hardening suite. This suite exists to make Bumbledb measurably fast without compromising the Rosetta set engine or the Free Join paper architecture.

## Scope

This PRD applies to all later PRDs in `docs/free-join-performance-hardening-prds/`.

## Required Direction

- Treat correctness and measurement as prerequisites to optimization.
- Prefer breaking internal and public APIs over preserving weak or misleading seams.
- Keep exactly one query execution architecture: formal Free Join over snapshot-local base images, GHT/COLT sources, dynamic covers, vectorized execution, and private sinks.
- Preserve exact duplicate-free `QueryResultSet` semantics until Rosetta explicitly changes the public query model.
- Make all diagnostic paths explicit and structured.
- Make all performance claims reproducible with benchmark commands.
- Keep LMDB as the only durable storage layer.

## Forbidden Work

- No SQL frontend.
- No bag output.
- No nulls.
- No floats in persistent values.
- No runtime DDL.
- No async API.
- No server mode.
- No alternate durable backend.
- No old storage readers.
- No compatibility aliases.
- No public aggregate API.
- No x86 vectorization of any kind.
- No synthetic counters.
- No benchmark count-only correctness checks.

## Breaking-Change Policy

Breaking changes are encouraged when they simplify the engine or deepen paper alignment.

Required breaking changes are allowed for:

- storage format version bumps;
- query diagnostics API replacement;
- benchmark JSON field replacement;
- internal query model restructuring;
- removal of test-only escape hatches that no longer match the engine;
- replacing `Vec<Vec<u8>>` column storage;
- replacing eager `Vec` GHT iteration;
- removing stale names from the LFTJ era;
- making NEON-only vectorized execution the only SIMD path.

## Global Acceptance

Run these after each implementation PRD unless the PRD explicitly narrows the gate for an intermediate mechanical split:

```bash
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --manifest-path fuzz/Cargo.toml
bash scripts/check-line-counts.sh
```

## SIMD Acceptance

The following search must return no matches under `crates/` after every PRD that touches execution, tuple comparison, batching, or vectorization:

```bash
rg "std::arch::x86|std::arch::x86_64|target_feature.*(sse|avx)|\b_mm_|\bavx\b|\bsse\b" crates
```

The following search must find every explicit SIMD implementation behind AArch64-only code:

```bash
rg "std::arch::aarch64|target_feature.*neon|cfg\(target_arch = \"aarch64\"\)" crates
```

## Documentation Acceptance

- Each completed PRD must be deleted or moved to an explicit completed archive only after its acceptance evidence is committed.
- The README ordered table must stay accurate.
- The Rosetta Stone remains the product authority when the paper assumes bag semantics, SQL, aggregation, or main-memory-only execution.
