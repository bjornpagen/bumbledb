# Benchmark data

The 13 JSON reports here are the original measurement outputs used by the
[benchmark report](../../results.md). `MANIFEST.json` indexes their commands,
timing boundaries, scheduling policy and SHA-256 hashes. Paths in commands
identify the original host's inputs; report paths are relative to this directory.

Measured source: `e5d4e4e3d4ba8b865a2dd1805230e2cc19b90335`.
Frozen executable SHA-256:
`ee3d52b4278b5dfa379e1997abeb76e83bfec5f9bc2350789fc27c40ef09018d`.

The M2 Max run used up to eight concurrent lane processes with
user-interactive QoS. macOS does not guarantee hard P-core affinity.
Results are shared-host observations, not isolated latencies or controlled
speedups over the earlier serial suite. A capped comparison is not a completed
timing. The report index is not a cross-platform correctness certificate.

See the [measurement guide](../../measurement-plan.md) for the current runner.
The GitHub release includes the report data, per-lane logs and checksums.
