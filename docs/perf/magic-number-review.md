# Performance constants to remeasure

These are current code locations, not a second configuration system. No tuning
values changed during documentation cleanup. Validate each hypothesis on the
actual target; an Apple Silicon result does not establish a portable optimum.

| Owner | Current choice | Experiment |
|---|---|---|
| `exec/run.rs` | Batch 128; prefetch width floor 4 | Full and partial batches; hit/miss mixes; cancellation latency; prefetch off control. |
| `exec/wordmap.rs` | Load denominator 3; sizing hint capped at 2^21 | Occupancy versus probes and retained bytes on real UUID/text/group keys. The hint is not a cardinality limit. |
| `exec/sink.rs` | Dense grouping cap 4096 | Dense/sparse crossover, including float accumulator memory. |
| `image.rs` | 16 KiB pitch model; 128-byte outer-transfer alignment; padding starts at 64 KiB | Preserve the measured tracker-band polarity: small nonzero residues are harmful, exact multiples are allowed. M2 Max L1 lines are 64 bytes, not 128. Test padding, cache pressure, and retained bytes; do not generalize to all ARM64. |
| `image/selection.rs` | Experimental indexed-row cost 8: count at most 1/8 of the relation before choosing a full scan | Compare small and skewed buckets, home and secondary indexes, median/mean/tail, and warm selection diversity. Index-only discovery still costs work; the ratio is not a semantic limit. |
| `api/prepared.rs` | Four memo slots | Parameter alternation, invalidation after writes, and tenant churn—not only repeat hits. |

Paths above are relative to `crates/bumbledb/src/`. Read the implementation
before changing a constant; these are review starting points, not automated
sweep controls.

Keep three categories separate:

- Representation-derived bounds need correctness arguments and boundary tests.
- LMDB/OS limits must come from the backend, not an invented application cap.
- Tuning and resource policies need units, measured workloads, and a safe
  behavior when the workload does not fit.

UUID width, canonical float behavior, hash domains, and persistent encodings
are format/semantic decisions, not knobs to sweep during a benchmark run.
Changes to them require their own correctness review.
