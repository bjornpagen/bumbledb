# Development autoresearch notes

Autoresearch is paused at the user's request. No further engine changes are
authorized until the user approves them. The hardware target is a Raspberry Pi
Zero 2 with 512 MB RAM; allocation and memory ownership need a plan before
implementation resumes. No Pi performance or memory qualification is claimed.

These are development experiments after the [published 1.0.1 results](results.md),
not replacement release benchmarks. Priorities are hardening, correctness,
performance, then simplicity. Existing native traces supplied the leads below;
no new traces were collected for this round.

## Shared scratch-owner hardening — September 7, 2026

Five negative-control tests failed before the accounting fixes, exposing three
defect classes:

- RAM upsert growth was counted twice.
- Disk writes could be refused despite fitting already-reserved capacity.
- Rounded reservations and spill overlap could exceed a narrower scratch policy.

Growth is now counted once, paid disk capacity is reusable, and reservations
atomically enforce the smaller of the caller's cap and the operation cap.
Tests cover rollback, spill failure, cancellation, concurrent reservations,
retained capacity and exact refunds. Durable formats and public APIs did not
change; no unsafe code was added.

The same change retains the earlier shared insertion optimization: a fresh
inline staged key uses LMDB's insert-if-absent operation instead of separate
lookup and insertion traversals. Existing-key replacement, wide-key collisions,
insertion order and transaction retry semantics remain covered.

### Local source validation

The combined source passed strict workspace all-target/all-feature Clippy,
2,545 tests (32 skipped), 37 doctests (one ignored), three focused Miri tests,
and the release warm-allocation contract. One nextest cleanup warning occurred
in a generated enum roundtrip test; three isolated reruns passed cleanly. Its
cause is unknown, and the original warning is retained.

The full local suite completed at **22:38:50 UTC**: 15 lanes, 32 read families,
34 scenario queries, and both 10,000-cycle churn workloads. Source/binary
identity and report digests were checked. Churn includes checkpoint probe
oracles and final model/engine/SQLite state agreement.

This does not qualify the manifest's six separate prerequisites, turn the
informational read latency budget green, or replace cross-platform CI.
Recorded clock contamination and capped comparisons remain limitations.

### Matched large-result timing

The test constructs and delivers 100,000 rows per operation through the real
completed-result and paged-delivery path. Each of 12 fresh processes measured
32 operations (3.2 million delivered rows), with a fresh canonical-row oracle.
The order was A/A, ABBA, BAAB, A/A; A is the baseline and B the candidate.

| Comparison | Mean ratio | Median ratio | Execution median ratio | Delivery median ratio |
| --- | ---: | ---: | ---: | ---: |
| Initial A/A | 0.9943 | 0.9888 | 1.0036 | 0.9119 |
| ABBA B/A | 0.8198 | 0.8224 | 0.7911 | 1.0346 |
| BAAB B/A | 0.7985 | 0.8036 | 0.7758 | 0.9788 |
| Final A/A | 1.0064 | 0.9961 | 0.9931 | 1.0278 |

These controls support approximately **18–20% lower end-to-end time for this
large-result workload**, with execution medians 21–22% lower. Delivery has no
established improvement. This is shared-host, scheduler-boosted timing, not an
isolated-host guarantee, confidence interval, or database-wide speedup claim.

### Broader results remain unqualified

The full run showed historical slowdowns in several reads and scenarios.
Those runs were not alternated, and several scaling cells were clock-flagged.
A follow-up four-family read experiment was stopped after its initial
identical-binary control showed **6.59× mean / 7.08× median drift** in the
24-byte displaced-probe lane, with severe clock contamination even after retry.

Both fresh dataset verifications and three baseline processes completed; the
first candidate process was deliberately terminated. All raw evidence is
retained as **inconclusive**, not passed. No completed candidate comparison
exists from that aborted experiment. It establishes neither broad neutrality
nor a candidate regression. The separate completed large-result controls above
are not replaced by these observations.

### Measurement identity

- Base source: `df737a6f290edf306a9806aacfa796abdd91c085`.
- Combined engine patch SHA-256: `b0c8f7d44cf2eaa0150d7b3e73b73de64d84f350d74716191c65b012bc8b9a19`.
- Baseline executable: `f25c5bfcb2fc89bb24c4eef351c2914a89f32353f8247810499311acc1ae9bf0` (byte-identical to the published 1.0.1 measurement executable).
- Candidate executable: `462143bb1f3f0450fd7fc83932c11bac7f1953529291676c1d448867dd383ce3`.
- Host: Apple M2 Max; pinned `nightly-2026-08-15`; ordinary optimized release build.

Raw patches, reports, logs and experiment dispositions are retained locally
under `bench-out/autoresearch-90-scratch-budget`. All heavy work was serialized
through the repository measurement lock. No release, tag or npm publication
is part of this development round.

## Unfinished resident-memory hardening

The scratch changes above are committed and pushed as
`bca930fb95a4713222324e3693e337a750941b14`. A subsequent source audit found
resident query buffers that can grow without working-byte admission. Twelve
regression tests compiled and failed at their expected assertions on that
commit. They cover projections, aggregate groups, exact-float and Pack
storage, shared allowances, retained charges, and scan gathering. These are
related accounting gaps, not twelve independent defects or evidence of public
data corruption. No production fix has been implemented.

The tests are parked outside the active suite while implementation is paused.
Their exact [source](experiments/resident-budget/resident_budget.rs.txt),
[module binding patch](experiments/resident-budget/binding.patch),
[run identity and failure roster](experiments/resident-budget/run.json), and
[original test output](experiments/resident-budget/tests.log) are preserved.
The run finished September 7, 2026 at 22:50:10 UTC with nextest exit 100.
That is an expected negative-control failure, not a passing check or a fix.

To restore the experiment after approval, put the saved source at
`crates/bumbledb/src/exec/sink/tests/resident_budget.rs`, apply the binding
patch, then run under the measurement lock:

```sh
scripts/measure.sh cargo nextest run -p bumbledb --lib --no-fail-fast -E 'test(resident_budget::)'
```

The broad target is growing maps, group banks, and retained Pack storage.
Production scan gathering already uses windows of at most 256 rows, and
aggregate batch survivors are bounded by the configured batch size (128 by
default). Initial preparation allocations and LMDB's resident page cache are
separate audit boundaries; a working-byte ledger is not a process RSS limit.
Any future fix must preserve the existing 24 KiB same-ledger fallback test,
charge retained allocations until they are actually released, and admit group
storage before publishing an index into it. Do not increase test allowances
or refund live storage to conceal a refusal.
