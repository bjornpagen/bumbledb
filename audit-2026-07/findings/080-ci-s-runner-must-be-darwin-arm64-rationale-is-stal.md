## CI's "runner must BE darwin-arm64" rationale is stale — build.ts stopped hardcoding darwin a day after the comment was written

category: incoherence | severity: medium | verdict: CONFIRMED | finder: r2:scripts-ci-packaging

### Summary

The sdk lane in `.github/workflows/ci.yml` carries a comment asserting that `macos-latest` "is REQUIRED, not just house style — ts/scripts/build.ts hardcodes the darwin-arm64 platform package and smoke-loads the .node it just built, so the runner arch must BE darwin-arm64." That was true when the lane landed (`ca579ac5`, 2026-07-18 15:29), but roughly ten hours later `caba6ffc` (2026-07-19 01:24, "ts: the build follows its host — local platform derived, never assumed darwin") replaced the hardcoding with host derivation and a synthesized dev-twin platform manifest, built explicitly so "a linux host builds, links, and verifies its own `.so` under its own name instead of misfiling it under the darwin one." The CI comment was never updated. Beyond the documentation incoherence, the consequence is real coverage left on the table: the linux branches of the build (dev-twin synthesis, `.so` artifact naming, the linux loader path) have zero CI execution, and the "cross-host fingerprint lock" runs both of its halves on a single macos runner.

### Evidence

All verified directly in the working tree at HEAD (`89086d4f`):

- `.github/workflows/ci.yml:178-181` — the stale comment: "Runner: macos-latest (M-series arm64) is REQUIRED, not just house style — ts/scripts/build.ts hardcodes the darwin-arm64 platform package and smoke-loads the .node it just built, so the runner arch must BE darwin-arm64." The lane itself: `runs-on: macos-latest` at ci.yml:190.
- `ts/scripts/build.ts:35` — `const LOCAL_PLATFORM = localPlatformTarget(process.platform, process.arch)`: the local platform is derived, not assumed. build.ts:19-21 states the design intent verbatim: "the LOCAL platform (this host, derived from `process.platform`/`process.arch` in `platform.ts`) owns artifact placement, the by-name link, the smoke-load, and the platform tarball proof — so a linux host builds, links, and verifies its own `.so` under its own name."
- `ts/scripts/build.ts:147-160` — `ensureLocalPlatformPackage` synthesizes the gitignored linux dev-twin manifest via `deriveDevTwinManifest` when `LOCAL_PLATFORM !== PUBLISH_PLATFORM`; on darwin-arm64 it is a no-op (`return` at line 150) — the only branch CI ever executes.
- `ts/scripts/platform.ts:62-67` — `localPlatformTarget` accepts `darwin` AND `linux`; `platform.ts:74-82` — `nativeArtifactName` maps linux to `libbumbledb_node.so`.
- Git history: `ca579ac5` (Sat Jul 18 15:29:35 2026, "ci: the sdk lane lands") introduced the comment; `caba6ffc` (Sun Jul 19 01:24:33 2026, "ts: the build follows its host — local platform derived, never assumed darwin") rewrote build.ts (146 lines changed), added platform.ts and test/build-platform.test.ts, and updated test/native-loader.test.ts for non-darwin hosts. No subsequent commit to ci.yml touched the sdk lane's runner comment (later ci.yml commits: `989eed33`, `c82ce80b`, `7043f96e`, `89086d4f` — other lanes).
- `.github/workflows/ci.yml:70-71` — the check lane already matrixes `os: [macos-latest, ubuntu-latest]`, so the engine is proven on x86_64-linux; the sdk lane (ci.yml:190) is the only macos-only lane whose macos-only-ness is justified by a claim about code that no longer behaves that way.
- Cross-host lock: `ts/crate/src/fingerprint_lock.rs:1` ("The cross-host fingerprint lock") and ci.yml:5,168 ("both halves of the cross-host fingerprint lock, per push") — today both halves execute on the same macos runner.
- Nothing else in the sdk lane's steps (ci.yml:190-243: rustup show, setup-node 24, corepack pnpm, pnpm install --frozen-lockfile, pnpm run build, tsc, biome, pnpm test, cargo clippy / cargo test on ts/crate) is darwin-specific.

### Failure scenario / impact

- A maintainer reading the lane trusts the "REQUIRED" comment and never considers a linux sdk entry — the comment actively misdocuments a constraint that `caba6ffc` was written to remove.
- The dev-twin machinery (`ensureLocalPlatformPackage`'s synthesis branch, `deriveDevTwinManifest`, the `.so` naming in `nativeArtifactName`, the linux path of the by-name loader) has never once executed in CI; only its darwin no-op branch runs. A regression in any of those (e.g. a field dropped from the twin manifest, a bad `.so` copy path) ships silently until someone builds on a linux host.
- The "cross-host fingerprint lock" is currently a same-host lock in CI: both the TS half and the Rust half run on one macos-arm64 runner, so an endianness/arch/platform-sensitive hashing regression would not be caught, despite the lock's name and stated purpose.

### Suggested fix

1. Fix the stale comment at ci.yml:178-181 — the runner is house style plus budget, not a hard constraint, since `caba6ffc`.
2. Matrix the sdk lane over `os: [macos-latest, ubuntu-latest]` exactly as the check lane does (ci.yml:70). This runs the full TS suite, the smoke-load through the by-name loader, the dev-twin synthesis, the `.so` artifact path, and the TS half of the fingerprint lock on x86_64-linux — making the "cross-host" lock genuinely cross-host and giving the linux branches of build.ts/platform.ts their first CI execution. The `sdk-cargo-${{ runner.os }}` and `sdk-pnpm-${{ runner.os }}` cache keys (ci.yml:215, 223) are already runner-scoped and need no change.
