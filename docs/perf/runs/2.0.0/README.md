# BumbleDB 2.0 benchmark data

The 12 JSON reports here are the original outputs of the complete local suite used by
the [benchmark report](../../results.md). All twelve timed lanes and the pre-timing
verifier (2,879 oracle cases, [verify.log](verify.log)) passed.

Measured source: `7135a9a18f091eabc3f4d345b6e87fb6c9bbcbb9`.
Executable SHA-256: `2b7a606251c79936cb9fd00f411ca73402c29089d728692868888b3927cc542c`.
Run on 2026-10-10 on an Apple M2 Max, one lane at a time with user-interactive QoS: a
shared-host run with macOS scheduling steering, not hard P-core affinity.

[MANIFEST.json](MANIFEST.json) records commands, scheduling policy and report paths.
Its `status` reads `INCOMPLETE` because the runner's chart step failed on a missing
Python dependency; no lane failed, and the charts were rendered afterwards from these
reports with the command below. Commands and paths in the manifest retain the original
host's locations.

From the repository root, regenerate all 21 SVGs:

```sh
uv run --no-project --with matplotlib python scripts/bench_viz.py --night docs/perf/runs/2.0.0 --out assets --note '2.0.0 7135a9a1 | M2 Max | 2026-10-10 | serial/shared-host | macOS QoS; no hard affinity'
```

See the [measurement guide](../../measurement-plan.md) for rerunning the suite.
