# PRD 17: Benchmark Gates And Rollout

## Status

Proposed.

## Motivation

The PRDs in this directory deliberately break internal representations. We need a strict rollout and gate plan so future agents can make ambitious changes without losing correctness or hiding regressions.

This PRD defines:

- PRD implementation order.
- Required test commands.
- Per-PRD benchmark gates.
- Trace/allocation rerun points.
- Success criteria for the whole campaign.

## Baseline Trace Run

Baseline traced run artifacts:

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-full-trace-latest/job-results.json
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-full-trace-latest/job-trace-summary.txt
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-full-trace-latest/job-trace.jsonl
```

Important baseline facts:

| Query | Baseline issue |
|---|---|
| `job_broad_cast_keyword_company` | 899.1 ms image build, 32.72M query-image alloc calls |
| `job_broad_movie_info_star` | 98.53% sample time in direct dispatch, 35,728 execute alloc calls first run |
| `job_q01_top_production` | 3.27 ms cold static proof, cached samples still 83 us |
| `job_q09_voice_us_actor` | 682.8 ms cold LFTJ build, 16.58M build alloc calls, 296.7 ms sample avg |
| `job_q16_character_title_us` | cached samples still 82 us for zero rows |
| `job_q24_voice_keyword_actor` | 33.6 ms cold LFTJ build, 920k build alloc calls |
| `job_movie_link_bridge` | 42.7% first-run planning, 4,080 prefix count probes/sample |
| `job_q33_linked_series_companies` | Bumbledb loses to SQLite: 91 us vs 65 us |

## Rollout Order

Implement exactly in this order unless a PRD discovers a hard dependency correction:

1. `01-measurement-and-allocation-contract.md`
2. `02-encoded-column-builder-substrate.md`
3. `03-query-image-flat-segment-decoding.md`
4. `04-lftj-atom-column-builders.md`
5. `05-lftj-indexed-prefix-streaming.md`
6. `06-relation-index-prefix-count-api.md`
7. `07-direct-count-kernels-before-planning.md`
8. `08-structural-query-cache-keys.md`
9. `09-static-empty-zero-row-fast-path.md`
10. `10-prepared-normalized-query-reuse.md`
11. `11-compact-direct-and-static-plans.md`
12. `12-query-image-scoped-loading.md`
13. `13-lftj-durable-index-trie-source.md`
14. `14-lftj-zero-alloc-traversal.md`
15. `15-sink-specialization.md`
16. `16-planner-compact-ids-and-lazy-candidates.md`

PRD 17 is the gate plan and should be updated as results come in.

## Global Test Commands

Run these after every PRD that changes code:

```sh
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --manifest-path fuzz/Cargo.toml
```

For small localized PRDs, run targeted tests first, then full gates before commit.

## Targeted Test Matrix

| PRD | Targeted tests before full gates |
|---|---|
| 01 | `cargo test -p bumbledb-bench` |
| 02 | `cargo test -p bumbledb-lmdb query_image` |
| 03 | `cargo test -p bumbledb-lmdb query_image`, `cargo test -p bumbledb-lmdb storage` |
| 04 | `cargo test -p bumbledb-lmdb query` |
| 05 | `cargo test -p bumbledb-lmdb query` |
| 06 | `cargo test -p bumbledb-lmdb query_image`, `cargo test -p bumbledb-lmdb query` |
| 07 | `cargo test -p bumbledb-lmdb query`, `cargo test -p bumbledb-test-support sqlite_comparison` |
| 08 | `cargo test -p bumbledb-lmdb query` |
| 09 | `cargo test -p bumbledb-lmdb query`, `cargo test -p bumbledb-test-support sqlite_comparison` |
| 10 | `cargo test -p bumbledb-core`, `cargo test -p bumbledb-lmdb query` |
| 11 | `cargo test -p bumbledb-lmdb query` |
| 12 | `cargo test -p bumbledb-lmdb query_image`, `cargo test -p bumbledb-lmdb query` |
| 13 | `cargo test -p bumbledb-lmdb sorted_trie`, `cargo test -p bumbledb-lmdb query` |
| 14 | `cargo test -p bumbledb-lmdb sorted_trie`, `cargo test -p bumbledb-lmdb query` |
| 15 | `cargo test -p bumbledb-lmdb query`, `cargo test -p bumbledb-test-support sqlite_comparison` |
| 16 | `cargo test -p bumbledb-lmdb query`, `cargo test -p bumbledb-test-support sqlite_comparison` |

## Benchmark Commands

Use the practical JOB dataset path:

```text
/var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb
```

### Quick Single Query Pattern

```sh
cargo run -p bumbledb-bench --release -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --query QUERY_NAME
```

### Allocation Trace Pattern

```sh
RUST_LOG="bumbledb_lmdb=debug" \
cargo run -p bumbledb-bench --release --features alloc-profile -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb \
  --query QUERY_NAME \
  --trace --trace-format json \
  --trace-output /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/QUERY_NAME-trace.jsonl
```

Use `RUST_LOG=bumbledb_lmdb=debug` for routine traced benchmarking. Use `trace` level only when investigating specific spans because full trace produced 28G.

### Full Practical JOB

```sh
cargo run -p bumbledb-bench --release -- \
  --preset job \
  --job-dir /var/folders/fj/10pmb37j1m1cy6d1lfclvvrw0000gn/T/opencode/bumbledb-job-imdb
```

### Non-JOB Guardrail

```sh
cargo run -p bumbledb-bench --release -- --preset nonjob
```

## Per-PRD Benchmark Gates

| PRD | Required benchmark gate |
|---|---|
| 01 | JSON/markdown clearly label cold/warm/sample/allocation scope; row counts unchanged |
| 02 | No benchmark gate alone; builder tests pass |
| 03 | `job_broad_cast_keyword_company` query-image alloc calls drop at least 95% from 32.72M |
| 04 | q09 `lftj_build` alloc calls drop at least 80%; q24 `lftj_build` alloc calls drop at least 80% |
| 05 | q09/q24 indexed-prefix path has no nested row materialization; no correctness regression |
| 06 | direct count queries use prefix count; `job_movie_link_bridge` and `job_broad_movie_info_star` do not regress |
| 07 | direct count first-run plan allocation collapses; direct results/counters unchanged |
| 08 | no `format!("{query:?}")` cache key remains; q33/q16 allocation modestly improves or does not regress |
| 09 | q33 Bumbledb avg beats SQLite avg |
| 10 | benchmark samples no longer pay runtime normalization for prepared queries |
| 11 | static/direct plans no longer clone generic free-join plan metadata |
| 12 | direct/static/generic queries build scoped images, not implicit full-schema images |
| 13 | eligible LFTJ atoms avoid sorted-trie builds; q09/q24 cold build improves |
| 14 | q09 `lftj_execute` allocation calls drop at least 80% from 645,250 |
| 15 | global count sink allocations collapse; q24 tiny projection overhead improves or does not regress |
| 16 | generic planner allocation drops; non-JOB correctness and performance do not materially regress |

## Success Metrics For Whole Campaign

Target outcomes after all PRDs:

| Query | Target |
|---|---|
| `job_broad_cast_keyword_company` | cold image build no longer dominates; alloc calls no longer tens of millions |
| `job_broad_movie_info_star` | direct count executes without heap central-key set and without generic planning |
| `job_q01_top_production` | static-empty cached sample approaches minimal read/return overhead |
| `job_q09_voice_us_actor` | cold LFTJ build allocations reduced by >80%; steady traversal allocations reduced by >80% |
| `job_q16_character_title_us` | static-empty cached sample faster than current 82 us |
| `job_q24_voice_keyword_actor` | cold LFTJ build allocations reduced by >80%; sample remains strongly faster than SQLite |
| `job_movie_link_bridge` | first-run planning removed; prefix count probes cheaper |
| `job_q33_linked_series_companies` | Bumbledb beats SQLite |

## Correctness Invariants

- Typed schema constraints remain enforced.
- No null semantics are introduced.
- Dictionary string/bytes literal encoding remains snapshot-safe.
- Query image cache keys include schema and storage tx id.
- Scoped images never treat missing required data as empty.
- Static-empty cache keys include schema and tx id.
- Direct count eligibility has no false positives.
- Prefix counts exactly match iterator counts.
- LFTJ durable index source exactly matches sorted-trie traversal semantics.
- Aggregate overflow behavior is unchanged.
- Materialized and count-only modes agree on row counts.

## Trace Hygiene

Full `RUST_LOG=trace` JOB runs can produce tens of gigabytes. Before running full traces:

- Create a fresh artifact directory.
- Delete only known trace artifact paths.
- Prefer query-specific traces.
- Use `debug` level unless row/index-level trace events are required.

Recommended trace summary flow:

```sh
scripts/summarize-trace-jsonl.sh TRACE.jsonl RESULTS.json > SUMMARY.txt
```

If the summarizer becomes too slow for large traces, create per-query trace extraction scripts rather than repeatedly scanning 28G raw traces.

## Documentation Updates During Rollout

After each PRD implementation:

- Update that PRD with actual benchmark result deltas.
- If implementation intentionally diverges from the PRD, document why.
- Update `00-roadmap.md` only if order or global strategy changes.
- Keep `docs/job-trace-analysis` as historical baseline unless rerunning the full trace suite.

## Commit Discipline

Each implementation PRD should be committed separately if the user requests commits for the implementation phase. Documentation-only planning does not require a commit unless explicitly requested.

## Definition Of Done

- Every PRD has targeted and global gates.
- Future agents can run one PRD at a time without needing conversational context.
- Final JOB benchmark shows the largest allocation cliffs eliminated.
- Non-JOB benchmark remains correct and informative.
