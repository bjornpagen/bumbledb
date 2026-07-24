## Bridge binary self-reports 0.1.0 while the shipped package is 0.6.0 — the one runtime-readable version string is exempt from the lockstep gate

category: incoherence | severity: low | verdict: CONFIRMED | finder: ts:bridge

### Summary

`engine_version()` is the bridge's proof-of-life export and the only version string readable out of the loaded binary at runtime. It formats `CARGO_PKG_VERSION`, and `ts/crate/Cargo.toml` is frozen at `0.1.0` while the published main and platform npm packages are `0.6.0`. The build's version-lockstep gate — whose stated purpose is that "the FFI ABI is not semver-stable — a main package may only ever resolve its own-version binary" — checks exactly three npm manifest positions and never the crate manifest, so a 0.6.0 install's binary answers `bumbledb-node 0.1.0 (bumbledb storage format v…)`. The smoke gate cannot catch this either: it asserts only that the string is non-empty.

### Evidence (all verified in the working tree)

- `ts/crate/Cargo.toml:7` — `version = "0.1.0"` (crate is `publish = false`).
- `ts/crate/src/lib.rs:74-80` — `format!("bumbledb-node {} (bumbledb storage format v{})", env!("CARGO_PKG_VERSION"), bumbledb::STORAGE_FORMAT_VERSION)`.
- `ts/package.json:3` and `ts/npm/darwin-arm64/package.json:3` — `"version": "0.6.0"`.
- `ts/scripts/build.ts:91-118` (`assertVersionLockstep`) — reads only main `package.json`, the platform `package.json`, and the `optionalDependencies` pin; the sole `Cargo.toml` reference in the script (build.ts:40) is the build path, not a version read.
- `ts/scripts/build.ts:198-206` (`smokeLoad`) — asserts `engineVersion()` returns a non-empty string only, so drift passes every build.

Corrections to the original finding, from verification:

- **Exposure is narrower than claimed.** `engineVersion` is declared only on the internal `Native` interface (`ts/src/native.ts:329`) and is not re-exported from the SDK's public surface; its only in-repo consumer is the build smoke test. A user sees the stale string only by requiring the platform package directly (which support/diagnostic tooling plausibly would — the doc on the export calls it exactly that kind of evidence string).
- **0.1.0 is convention, not a missed bump.** Every engine crate in the workspace (`crates/bumbledb`, `-query`, `-theory`, `-macros`, `-query-macros`, `-bench`) is also `0.1.0`: the repo deliberately versions releases in npm only. The incoherence is therefore not "someone forgot an edit" but "a permanently-frozen crate version leaks into a runtime-visible identity string," which the lockstep gate's own doc comment ("a release bump is one edit that this gate then enforces") contradicts — there is a fourth version-bearing position, and it is the only one visible after install.

### Failure scenario

A user on the published 0.6.0 package (or a maintainer triaging a load problem) requires `@bjornpagen/bumbledb-darwin-arm64` and calls `engineVersion()` to identify the loaded binary. It reports `bumbledb-node 0.1.0 (…)` — for this release and every future one — while the install manifest says 0.6.0. Any support bundle, smoke log, or bug report quoting the string misidentifies the binary, in exactly the ABI-mismatch triage situation the lockstep gate exists to make unambiguous.

### Suggested fix

Two coherent options; the finder's original (fold `crate/Cargo.toml` into `assertVersionLockstep`) is the weaker one because it forces version bumps on a `publish = false` crate against the repo-wide frozen-0.1.0 convention. Cleaner, representation-first cure: make the string carry only identities that are actually maintained — drop `CARGO_PKG_VERSION` from `engine_version()` and report `STORAGE_FORMAT_VERSION` alone (the lib.rs doc already concedes the storage format version is what makes the string proof rather than decoration), or inject the npm version at build time (e.g. `env!` from a build-set variable) so the runtime string and the lockstep gate share one source. Either way, the smoke assertion in `build.ts` can then check the string against the lockstep version instead of mere non-emptiness, closing the loop.
