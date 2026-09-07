# Development autoresearch notes

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
