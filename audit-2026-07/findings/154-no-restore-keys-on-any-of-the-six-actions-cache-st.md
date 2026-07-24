## No restore-keys on any of the six cache steps — every lockfile or toolchain repin forces fully cold CI builds

category: perf | severity: low | verdict: CONFIRMED | finder: r2:scripts-ci-packaging
outcome: fixed e5a9ef39

### Summary

Every `actions/cache@v5` step in `.github/workflows/ci.yml` (the repo's only workflow) uses an exact key of the form `<lane>-${{ runner.os }}-${{ hashFiles(<pins>) }}` and none carries a `restore-keys` entry (`grep -r restore-keys .github/` returns nothing). Under `actions/cache` semantics an exact-key miss restores nothing, so any change to `Cargo.lock`, `rust-toolchain.toml`, `ts/crate/Cargo.lock`, `ts/pnpm-lock.yaml`, or `lean/lean-toolchain` makes the next run of each affected lane a fully cold build — toolchain deps, the `lmdb-master-sys` C compile (LMDB via heed, confirmed in `Cargo.lock:371`), and the whole workspace — instead of the seconds-scale incremental delta a prefix restore key would allow. The check lane pays this twice per repin, once per matrix OS.

### Evidence (all verified in the file)

- `.github/workflows/ci.yml:85` — `key: check-${{ runner.os }}-${{ hashFiles('rust-toolchain.toml', 'Cargo.lock') }}` (check lane, matrix `[macos-latest, ubuntu-latest]` at :70)
- `.github/workflows/ci.yml:107` — miri lane, same pattern
- `.github/workflows/ci.yml:133` — lean lane, `key: lean-${{ runner.os }}-${{ hashFiles('lean/lean-toolchain') }}` (caches `~/.elan` + `lean/.lake`)
- `.github/workflows/ci.yml:163` — lean-conformance cargo cache, same rust pin pattern
- `.github/workflows/ci.yml:215` — sdk-cargo, keyed on `rust-toolchain.toml` + `ts/crate/Cargo.lock`
- `.github/workflows/ci.yml:223` — sdk-pnpm, keyed on `ts/pnpm-lock.yaml`
- No `restore-keys` anywhere under `.github/`; no doc in `docs/` (checked `docs/architecture/60-validation.md`, which owns the measurement discipline the CI header cites) records a deliberate decision to omit them.
- The file's own header (`ci.yml:20-26`) tracks lane wall-time budgets to the second and concedes "the first run on a cold cache (toolchain download + full builds) will exceed it once" — but with exact-only keys that cold run recurs on every repin, not once.

### Bench impact

Bump one dependency in `Cargo.lock` (a routine event — recent history includes two lockfile-regeneration commits): the check lane's next run rebuilds the full dependency graph from nothing on BOTH macos and ubuntu runners, including the LMDB C build; miri and lean-conformance go cold on their next scheduled/dispatched runs; a `ts/pnpm-lock.yaml` or `ts/crate/Cargo.lock` change does the same to the sdk lane; a `lean/lean-toolchain` repin re-downloads the whole elan toolchain and rebuilds `lean/.lake`. With a `restore-keys` prefix, the runner would restore the previous near-identical cache and cargo/pnpm/lake would rebuild only the delta — the documented standard shape for these caches.

### Suggested fix

Add a prefix fallback under each cache step, e.g. for the check lane:

```yaml
key: check-${{ runner.os }}-${{ hashFiles('rust-toolchain.toml', 'Cargo.lock') }}
restore-keys: |
  check-${{ runner.os }}-
```

and the analogous `miri-`, `lean-`, `lean-conformance-`, `sdk-cargo-`, `sdk-pnpm-` prefixes at :107, :133, :163, :215, :223. One caveat worth a comment in the file: prefix-restored `target/` dirs accumulate stale artifacts across saves; if cache size ever matters, pair this with an occasional key rotation or `cargo clean` of dead artifacts — a maintenance nuance, not a reason to stay cold.
