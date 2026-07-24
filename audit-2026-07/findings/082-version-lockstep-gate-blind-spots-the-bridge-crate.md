## Version lockstep gate blind spots: the bridge crate version is ungated and the smoke assertion that could witness it checks only non-emptiness

category: unification | severity: medium | verdict: CONFIRMED | finder: r2:scripts-ci-packaging
outcome: fixed 88f3cf93

### Summary

The build's version-lockstep gate enforces version identity across exactly three spellings, all package.json values. A fourth spelling — the napi bridge crate's own version in `ts/crate/Cargo.toml` — is ungated and has sat at `0.1.0` through every release since; the shipped 0.6.0 packages carry a binary whose proof-of-life export `engineVersion()` answers `"bumbledb-node 0.1.0 (bumbledb storage format vN)"`. The one place the binary's actual version string is observed on every build (`smokeLoad`) asserts merely non-emptiness, so the drift sails through the very check written under the gate's version-identity rationale. `PUBLISHING.md`'s census ("The version lives in ONE place… Three values must match") is false by omission.

### Evidence (all verified against the tree)

- `ts/scripts/build.ts:91-117` — `assertVersionLockstep` reads only `ts/package.json` `version`, its `optionalDependencies` pin, and `ts/npm/<publish>/package.json` `version`. No Cargo.toml read anywhere in the gate.
- `ts/scripts/build.ts:85-86` — the gate's stated rationale: "the FFI ABI is not semver-stable — a main package may only ever resolve its own-version binary."
- `ts/scripts/build.ts:203-204` — the smoke assertion in full: `if (typeof version.data !== "string" || version.data === "") { throw errors.new("smoke assertion failed: engineVersion() must return a non-empty string") }`.
- `ts/crate/Cargo.toml:7` — `version = "0.1.0"`; `ts/crate/Cargo.lock:58-59` follows (`bumbledb-node` / `version = "0.1.0"`). Meanwhile `ts/package.json:3` and `ts/npm/darwin-arm64/package.json:3` are both `0.6.0`.
- `ts/crate/src/lib.rs:72-80` — `engine_version()` formats `env!("CARGO_PKG_VERSION")` into its return, so the drift is embedded in the shipped artifact, not just the manifest.
- `ts/test/ffi.test.ts:185-187` and `ts/test/native-loader.test.ts:49-51` — the only other callers of `engineVersion()` in the repo; both assert only string-typed and non-empty. No test anywhere compares the binary's self-report to the release version.
- `ts/PUBLISHING.md:59-65` — "The version lives in ONE place: `ts/package.json` `version`. Three values must match exactly" — a census that omits the crate manifest.
- `.github/workflows/ci.yml:240-242` — CI clippy/tests the crate but performs no version comparison; the lockfile freeze gates only the JS side.

### Two mitigations that bound severity (checked; they soften, not refute)

1. The `0.1.0` self-report is documented as intentional: the `engine_version` doc (`ts/crate/src/lib.rs:66-71`) and `ts/src/native.ts:325-329` both say the string names "the bridge crate version." The code follows its own docs — the defect is that no release process or gate ever touches that version, so "the bridge crate version" is a number frozen at scaffold time masquerading as identity.
2. The its-own-version-binary ABI invariant is genuinely enforced today — but at the package-resolution layer (the EXACT `optionalDependencies` pin, gated at `build.ts:103-107`), not by anything the binary itself says. So there is no live misresolution bug; the impact is a misleading proof string in every shipped binary, a doc census that is wrong, and a trap for any future consumer, support triage, or gate that reads `engineVersion()` expecting the release version.

### Failure scenario

Exactly what shipped: 0.6.0 packages whose binary self-reports 0.1.0. Anyone diagnosing a version-skew bug in the field (`node -e "…engineVersion()"`) reads a number no release bump has ever moved and concludes the wrong thing; any future gate that upgrades the smoke check to compare against the release version discovers the crate has been frozen at 0.1.0 for six releases.

### Suggested fix

Fold the fourth spelling into the existing gate: `assertVersionLockstep` also reads `ts/crate/Cargo.toml`'s `version` (one line-anchored regex over a file the build already names at `build.ts:40`), and `smokeLoad` tightens its assertion to `version.data.includes(version)` — the gate then covers every spelling that crosses the FFI, the smoke check finally witnesses identity rather than mere life, and `PUBLISHING.md:59-65`'s census becomes true by listing four values. A release bump stays one conceptual edit, now with the crate manifest included and machine-enforced.
