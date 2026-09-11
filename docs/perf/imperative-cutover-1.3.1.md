# Imperative cutover: focused 1.3.1 measurements

Measured September 11, 2026 on an Apple M2 Max, 96 GiB RAM, macOS ARM64,
Node 26.4.0, Effect 4.0.0-rc.112. These are focused SDK/transition measurements
on a shared development host, not a rerun of the comprehensive benchmark.
The [full benchmark report](results.md) still describes its measured 1.3.0 source.

The installed package family used immutable source tree
`162ca0ee903b8f04d6a59c87b9dfe106ac0c438e`, selected from base commit
`20314617d1ca0f14acae6fa7065d2a688b41ef6c` plus the pending 1.3.1 changes.
[Raw results](imperative-cutover-1.3.1.jsonl) retain the tree and exact package
hashes. Release notes, final test additions, and packaging checks followed this
measurement; the measured query and population implementations are retained.

Reproduce with a completed package family:

```sh
node scripts/measure-cutover.mjs /absolute/path/to/family
```

The runner installs the actual tarballs in a private consumer, typechecks the
measurement program without workspace aliases, and starts a fresh process for
each size and workload. The native build uses the release profile. No builds or
correctness battery ran concurrently with this recorded measurement. Host load,
filesystem synchronization, JIT warmup, and scheduling affect absolute timings;
these samples are not production throughput guarantees.

## Query heads

Each form uses the same two mutually exclusive `original` match conditions.
The direct form combines a variable projection with `amount - previous`; the
imported form puts the subtraction in an existing interior. The unmixed control
projects variables in both arms. Fixture `previous` values are zero, so all
three return exactly the same facts. Every result is checked by row identity,
value, count, and checksum. The control omits arithmetic work; it is not an
alternate implementation of arbitrary corrections.

Preparation and native execution are measured separately. Page delivery and
validation are outside both timers. Each cell is the median of seven samples,
after one warmup round, with form order rotated each round.

| Rows | Direct prepare / execute (ms) | Imported prepare / execute (ms) | Unmixed prepare / execute (ms) |
|---:|---:|---:|---:|
| 1,000 | 0.538 / 0.329 | 0.645 / 0.344 | 0.487 / 0.324 |
| 10,000 | 0.803 / 1.994 | 0.827 / 2.522 | 0.565 / 1.860 |
| 100,000 | 1.289 / 17.152 | 1.439 / 19.787 | 0.976 / 15.005 |

The direct mixed head removes the imported-stage workaround. At 100,000 rows
it took about 17.2 ms versus 19.8 ms for that workaround in this sample. The
unmixed control took 15.0 ms. This does not establish a release-wide speedup.
The repair aligns projection/fold validation and selects the existing computed
output adapter when any surviving arm needs it; it introduces no new query
execution algorithm.

## Bounded imperative population

The source has keyed submissions and one proof per submission. An ordinary
query joins them. Application TypeScript branches on the generated closed
method type and writes each submission plus its method-specific proof through
ordinary change builders. The target declares keys, selected containment, and
both mirror directions. Finalization hashes and judges the complete target,
then installs it frozen. The correctness fixture separately proves preservation,
activation, recovery, abort, and refusal of incomplete/incorrect arms.

Source initialization is outside the timers. Capture includes schema compilation
and native freeze. Join measures native execution, which constructs a complete
result; `pages()` bounds delivery, not query execution. Population includes page
delivery, JavaScript transformation, builders, native writes, and per-batch
scope cleanup. The reader delivered at most 256 joined rows per page. No
JavaScript array contained the complete source or target.

| Submissions | Facts written | Batches | Capture (ms) | Join (ms) | Populate (ms) | Finalize (ms) |
|---:|---:|---:|---:|---:|---:|---:|
| 1,000 | 2,000 | 4 | 19.1 | 2.5 | 104.9 | 64.7 |
| 10,000 | 20,000 | 40 | 15.6 | 13.8 | 611.7 | 242.3 |
| 100,000 | 200,000 | 391 | 15.4 | 71.3 | 3309.2 | 2031.6 |

| Submissions | First-quarter batch median (ms) | Last-quarter batch median (ms) | Process peak RSS (MiB) | Sampled transition JS heap peak (MiB) |
|---:|---:|---:|---:|---:|
| 1,000 | 22.76 | 22.06 | 177.0 | 37.8 |
| 10,000 | 16.47 | 11.63 | 213.4 | 51.0 |
| 100,000 | 8.13 | 6.87 | 320.8 | 57.2 |

Process peak RSS includes native allocations, V8, mapped database pages, and
source initialization. It is not a Rust allocator-only measurement. JS heap is
sampled every 5 ms and at batch/finalization boundaries after a pre-transition
GC; short peaks can escape sampling. Native memory is therefore included in
the process measure, not separately attributed. Result storage and database
indexes can grow with input size. These runs do not establish larger-than-memory
behavior or constant total process memory. All scoped runs returned native
handle and owner counts to zero.

At 100,000 submissions the measured population rate was about 60,400 written
facts/second, excluding finalization. Batch medians did not grow with the
already-populated prefix. The code supplies the stronger structural argument:
[`Population.apply`](../../crates/bumbledb-log/src/transition/local.rs) calls the
existing unjudged stage writer for the supplied changes; it performs no target
scan or complete judgment. [`admit_target`](../../crates/bumbledb-log/src/transition/local.rs)
runs the canonical disk-backed sort/hash and complete judgment at finish.
Ordered deletions remain sequential; command composition is not replay.
These are data-sized transition operations, not logarithmic-time migrations.

## Schema generation

The full Wagie Tools schema from commit
`04b3a1e48a1359857f7d9975d8edc456f169e7c8` was emitted independently from its
canonical `src/schema/current.json`: 190 relations, 1,032 expanded statements,
170,113 snapshot bytes, and 130,733 generated TypeScript bytes.

| Operation | Time (ms) |
|---|---:|
| Emit bindings | 14.0 |
| Strict TypeScript check | 1,806.6 |
| Recompile and compare canonical snapshot | 109.8 |

The exact canonical snapshot and native SchemaId were preserved:
`541917df479af2b99155b2de76e41f9e1541bd8c9b80513d609db53494d639a5`.
This is one full-consumer qualification sample. Generation/typechecking occurs
outside database operations. Field codecs reuse the existing interpreter;
outcome/diagnostic changes add no whole-row serialization or database scan.
