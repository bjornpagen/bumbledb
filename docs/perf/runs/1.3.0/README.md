# BumbleDB 1.3.0 benchmark data

The 13 JSON reports here are the original outputs of the complete local
suite used by the [benchmark report](../../results.md). All report-producing
lanes, the pre-timing verifier, and chart generation finished successfully.

Measured source: `a022bfacdff3dba6903284767e302ed6eb9f5a3f`.
Frozen executable SHA-256:
`1953305d464b4d0cc43fc54fc9333e1caabd44ded90905c33452326b874378b7`.
Completed: `2026-09-11T18:09:36.973808+00:00`.

[MANIFEST.json](MANIFEST.json) records commands, timing boundaries, scheduling
policy, report paths, and verified SHA-256 hashes. [SOURCE.json](SOURCE.json)
records the clean build's source inventory, toolchain, host, and power state.
The [verification log](verify.log) records all 2,879 pre-timing oracle cases.
Commands retain the original host's paths; report paths are relative here.
The index's `chart_generation` section records the renderer and all 21 SVG
hashes, with paths relative to the repository root.
The complete local logs and frozen executable remain in
`bench-out/release-1.3.0-20260911.pcxa6o0b/` on the measurement host.

The M2 Max measured one lane at a time with user-interactive QoS. This is a
shared-host run with macOS scheduling steering, not hard P-core affinity.
Retain clock flags, incomplete comparisons, and original source identities.
A SQLite cap is not a completed timing. External qualification and native
profiling are separate from this run.

From the repository root, regenerate all 21 measured SVGs:

```sh
python3 scripts/bench_viz.py --night docs/perf/runs/1.3.0 --out assets --note '1.3.0 a022bfac | M2 Max | 2026-09-11 | serial/shared-host | macOS QoS; no hard affinity'
```

See the [measurement guide](../../measurement-plan.md) for rerunning the suite.
