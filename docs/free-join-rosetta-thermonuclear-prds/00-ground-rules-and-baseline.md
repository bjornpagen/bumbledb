# PRD 00: Ground Rules And Baseline

## Purpose

Freeze the contract for the remaining work. This PRD exists so every later optimization is judged against Rosetta first, then the Free Join paper, then trace data.

## Current Baseline

Baseline command:

```bash
cargo run --release -p bumbledb-bench --features query-tracing -- --preset job-sample --job-dir data/job --open-limit 100000 --format json --repeats 1 --warmup 1 --trace-output file --profile-query-label current-full-trace --alloc on
```

Baseline trace files:

```text
data/traces/current-full-trace-7934-0.json
data/traces/current-full-trace-7934-1.json
data/traces/current-full-trace-7934-2.json
data/traces/current-full-trace-7934-3.json
data/traces/current-full-trace-7934-4.json
data/traces/current-full-trace-7934-5.json
data/traces/current-full-trace-7934-6.json
data/traces/current-full-trace-7934-7.json
```

Baseline summary:

| query | bumbledb ms | sqlite ms | result rows | alloc calls | allocated MB |
| --- | ---: | ---: | ---: | ---: | ---: |
| `job_broad_cast_keyword_company` | 60.235 | 4.606 | 3 | 230,085 | 130.85 |
| `job_broad_movie_info_star` | 39.980 | 5.869 | 3 | 7,011 | 16.59 |
| `job_q01_top_production` | 11.409 | 4.724 | 0 | 9,077 | 7.17 |
| `job_q09_voice_us_actor` | 62.101 | 4.509 | 0 | 164,361 | 87.23 |
| `job_q16_character_title_us` | 61.950 | 4.372 | 0 | 163,905 | 84.47 |
| `job_q24_voice_keyword_actor` | 74.335 | 4.922 | 0 | 165,052 | 88.90 |
| `job_movie_link_bridge` | 21.110 | 4.567 | 0 | 2,580 | 11.87 |
| `job_q33_linked_series_companies` | 42.986 | 4.383 | 0 | 10,500 | 15.13 |

Root traced phase totals:

| phase | total ms | allocated MB |
| --- | ---: | ---: |
| `ExecuteNode` | 291.91 | 208.15 |
| `PlanSelect` | 14.49 | 25.97 |
| `Normalize` | 0.09 | 0.11 |
| `SinkFinish` | 0.02 | 0.00 |

Execution child totals:

| phase | total ms | allocated MB |
| --- | ---: | ---: |
| `BaseImageLoad` | 235.06 | 22.78 |
| recursive runtime | about 55.24 | about 183.80 |
| `ColtBuild` | 1.56 | 0.67 |
| filter encode/cache lookup | about 0.06 | 0.00 |

## Required Reading For Every PRD

- `docs/ROSETTA_STONE.md`
- Free Join paper `tex/02-background.tex`, especially selection pushdown assumptions.
- Free Join paper `tex/03-free-join.tex`, especially GHT and plan validity.
- Free Join paper `tex/04-optimizations.tex`, especially COLT laziness and dynamic covers.
- Free Join paper `tex/06-discussion.tex`, especially the warning about disk-backed COLT and random access.

## Forbidden Shortcuts

- Do not add a second query engine.
- Do not materialize all intermediate result rows as an escape hatch.
- Do not bypass LMDB with an in-memory mirror.
- Do not depend on SQLite, DuckDB, or SQL for planning or execution.
- Do not introduce bag behavior to match paper examples.
- Do not make accelerators correctness-critical.
- Do not make benchmark-only code paths.
- Do not hide work by turning off tracing counters.
- Do not accept same-count correctness checks. Only exact values count.

## Required Trace Vocabulary

Later PRDs must preserve or extend these phases with real counters:

- `BaseImageLoad`
- `SourceFilterEncode`
- `EmptySourceShortCircuit`
- `ColtBuild`
- `ColtForce`
- `ColtIter`
- `ColtGet`
- `CoverChoice`
- `ProbeSibling`
- `BindingExtend`
- `SinkConsume`
- `SinkFinish`

If a PRD moves work from one phase to another, it must update counters so the movement is visible.

## Passing Criteria

- This PRD suite exists under `docs/free-join-rosetta-thermonuclear-prds/`.
- Every later PRD cites the Rosetta/paper alignment requirement it preserves.
- The baseline command above runs and all exact SQLite comparisons pass before any optimization work begins.
- No later PRD is allowed to claim success without a before/after trace comparison against this baseline or an explicitly ratcheted replacement baseline.
