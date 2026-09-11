# Structural query costs — 2026-09-11

Local measurements on Apple M2 Max (darwin/arm64), Node v26.4.0.
The source is the uncommitted worktree based on `82053535f7c0c0252dbfab70e20038f4371baeb5`;
the benchmark's source inventory SHA-256 is `b953cb95a0f328eb48e5e7252e8ddd69f757de668929abd31ee9635d31ce711e`.

Run `node ts/bench/structural-cost.ts` after building the SDK. The benchmark records
its inventory in the first raw record. Each cell has two warmups and nine measured
runs; the table reports median milliseconds. Every run collects the complete
answer and verifies its row count. These measurements include native query
preparation, stage materialization, bridge delivery, and JavaScript collection.
They are local observations, not cross-platform release qualification.

| Input rows | Interval width | Difference → measure (2N rows) | mulDiv (N rows) | Checked multiply → divide (N rows) |
|---:|---:|---:|---:|---:|
| 0 | 30 | 0.222 | 0.147 | 0.115 |
| 0 | 3,000,000,000,000 | 0.155 | 0.108 | 0.108 |
| 1,000 | 30 | 3.124 | 0.683 | 0.681 |
| 1,000 | 3,000,000,000,000 | 2.883 | 0.668 | 0.674 |
| 10,000 | 30 | 30.252 | 5.672 | 5.767 |
| 10,000 | 3,000,000,000,000 | 30.073 | 5.726 | 5.612 |

Each fact has an identity, a span `[0,W)`, and the excluded middle third
`[W/3,2W/3)`. The interval query emits and measures both surviving pieces.
The scalar queries calculate `id * 2 / 3`, using either `mulDiv` with truncation
or the existing checked operations. These small operands let both expressions
succeed, so the comparison measures equivalent answers. Wide products that fit
only after division are covered separately by correctness tests.

Changing the represented width by eleven orders of magnitude left the 10,000-row
interval workload near 30 ms in this sample. Cost follows input/output cardinality,
not the number of points an interval represents. Intersection emits at most one
piece and difference at most two per binding, using endpoint comparisons. Multiple
independent differences can still produce up to `2^k` combinations; joins, sorting,
deduplication, and collecting those results add their ordinary costs. This is not
a claim that every query runs in logarithmic time.

`alternatives` expands to existing containment and mirrors statements. Its native
schema and costs equal the corresponding manually written laws; it introduces
no additional admission pass. The admission implementation is unchanged.

Raw repetitions: [structural-algebra-20260911.jsonl](structural-algebra-20260911.jsonl).
