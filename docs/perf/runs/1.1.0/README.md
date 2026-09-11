# Benchmark data

The 13 JSON reports here are the original measurement outputs used by the
[historical 1.1.0 report](https://github.com/bjornpagen/bumbledb/blob/v1.1.0/docs/perf/results.md)
and the comparison in the [current report](../../results.md).
`MANIFEST.json` indexes their commands,
timing boundaries, scheduling policy and SHA-256 hashes. Paths in commands
identify the original host's inputs; report paths are relative to this directory.

Measured source: `548193d46f645ff4a4f007517733deb4bd389569`.
Frozen executable SHA-256:
`84b920460fe27b49181a8e8a444e6557971f559e96972fb5e5780f305d47cc74`.

The M2 Max run measured one lane at a time with verified user-interactive QoS.
macOS does not guarantee hard P-core affinity. Both this run and the published
baseline use serial lanes. Results remain shared-host observations, not
controlled code speedups. A capped comparison is not a completed timing.
The report index is not a cross-platform correctness certificate.

See the [measurement guide](../../measurement-plan.md) for the current runner.
The GitHub release includes the report data, per-lane logs and checksums.
