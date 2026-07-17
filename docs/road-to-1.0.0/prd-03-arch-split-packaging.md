# PRD-03 — Arch-split packaging + the native loader

Repo: bumbledb · depends on: 02 · blocks: 08

## Objective

Package the SDK the modern way — the pattern Biome, esbuild, swc, and napi-rs all
use: a pure-JS main package (`@bjornpagen/bumbledb`) that carries no binary, plus a
per-platform binary package (`@bjornpagen/bumbledb-darwin-arm64`) declared as an
`optionalDependency` and gated by `os`/`cpu` so npm installs it only on the
matching platform. The main package's native loader resolves the platform package
at runtime and throws a precise, typed error on any unsupported platform. Today we
ship exactly one platform (`darwin-arm64`); the structure makes adding
`darwin-x64`/`linux-*`/`win32-*` later a matter of more platform packages + a CI
matrix, never a redesign.

## Context

- The Biome pattern, concretely: `@biomejs/biome` (main) lists
  `optionalDependencies: { "@biomejs/cli-darwin-arm64": "<exact version>", … }`;
  each `@biomejs/cli-<platform>` package has `"os": ["<os>"], "cpu": ["<arch>"]`
  (and `libc` where relevant) and contains just the binary; the main package
  resolves `require("@biomejs/cli-" + platform + "-" + arch)` at runtime. npm/pnpm
  skip non-matching optional deps silently at install; the main package installs
  everywhere and fails LOUDLY at load if no platform package resolved.
- Our binary is `bumbledb.node` (a napi cdylib), currently produced into
  `dist/bumbledb.node` by `ts/scripts/build.ts` and loaded by `ts/src/native.ts`
  via `createRequire("./bumbledb.node")`.
- `native.ts` is the SDK's single sanctioned FFI boundary — ALL `.node` knowledge
  lives there. The loader change is confined to it.

## Work

1. **Create the platform package** under `ts/` (e.g. `ts/npm/darwin-arm64/`):
   its own `package.json` — name `@bjornpagen/bumbledb-darwin-arm64`, version
   lockstepped to the main package, `"os": ["darwin"]`, `"cpu": ["arm64"]`,
   `"main"` pointing at the packaged `bumbledb.node`, `files: ["bumbledb.node"]`,
   `license` matching the main package, no dependencies. The `.node` is placed
   here by the build (item 3), not committed.
2. **Rewire the main package** (`ts/package.json`): add
   `optionalDependencies: { "@bjornpagen/bumbledb-darwin-arm64": "<version>" }`
   pinned EXACT to the main version. The main package no longer ships a binary —
   drop `bumbledb.node` from its `files` (keep `files: ["dist"]` for the JS/types).
   Do NOT put `os`/`cpu` on the main package (it must install everywhere so the
   loader can throw a clean error on unsupported platforms).
3. **Restructure the native loader** (`ts/src/native.ts`): resolve the platform
   package by `` `@bjornpagen/bumbledb-${process.platform}-${process.arch}` `` via
   `createRequire`, then load its `bumbledb.node`. On resolution failure throw a
   typed, actionable error naming the running `platform-arch` and that only
   `darwin-arm64` is shipped today (matching the SDK's error idiom —
   `@superbuilders/errors`, no bare throws). Keep this the SOLE FFI boundary.
4. **Update the build** (`ts/scripts/build.ts`): after `cargo build --release`,
   copy the artifact into the platform package dir
   (`ts/npm/darwin-arm64/bumbledb.node`) instead of `dist/`, smoke-load it FROM
   THERE through the new loader path, then `tsc` the main package. The build must
   produce both publishable trees: the main package (`ts/dist` + manifest) and the
   platform package (`ts/npm/darwin-arm64` + binary). A build producing an
   unloadable artifact still fails (keep the smoke-load).
5. **Version-lockstep helper**: a single source of the version (the main
   `package.json` `version`) propagated to the platform package's `package.json`
   and the `optionalDependencies` pin at build time, so a release bump touches one
   place. Fail the build if they diverge.

## Technical direction

- Exact-pin the optional dep (`"1.0.0"`, not `"^1.0.0"`) so a main package can
  only ever resolve its own-version binary — the FFI ABI is not semver-stable.
- The loader must distinguish "unsupported platform" (no matching optional dep —
  expected on non-darwin-arm64) from "package present but `.node` unloadable"
  (a real corruption/ABI error) — different messages, both typed.
- Do NOT add a build-from-source install fallback — coworkers lack the pinned
  nightly Rust toolchain, and ruling 1 ships prebuilt only. Unsupported platform =
  a clean typed error, not a compile attempt.
- `ts/npm/`'s per-platform dirs are the future matrix; name and structure them so
  adding `ts/npm/linux-x64/` etc. later is pure addition.

## Passing criteria

- `node scripts/build.ts` produces (a) the main package tree (`dist/*.js`, `*.d.ts`,
  `package.json` with the `optionalDependencies` pin, no binary) and (b)
  `ts/npm/darwin-arm64/` with `bumbledb.node` + its `os`/`cpu`-gated manifest, and
  the smoke-load through the new loader path succeeds.
- `node --test ts/test/**/*.test.ts` stays green with the loader resolving the
  binary from the platform package (the tests exercise the real FFI).
- `npm pack --dry-run` (or the pnpm equivalent) on BOTH package dirs lists exactly
  the intended files: the main tarball has `dist/` + manifest and NO `.node`; the
  platform tarball has `bumbledb.node` + manifest and nothing else.
- A negative check (unit-shaped, part of this change): simulate a foreign platform
  (inject `platform`/`arch`) and assert the loader throws the typed
  unsupported-platform error rather than a raw module-not-found.
- `tsc --noEmit` green; `biome check ts/` clean.
- Commit in the repo's voice; push.
