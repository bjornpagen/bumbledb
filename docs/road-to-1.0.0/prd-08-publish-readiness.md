# PRD-08 — Publish readiness

Repo: bumbledb · depends on: 03, 07 · blocks: 09 (and the owner's publish
ceremony)

## Objective

Bring both packages to a state where the owner's `npm publish` is a single,
safe, reproducible step — the manifests final, the tarballs proven to contain
exactly the right files, the version wiring lockstepped, provenance ready — with
zero remaining publish blockers. This PRD produces artifacts and a runbook; it
does NOT run `npm publish` (owner ceremony, ruling 7).

## Context

- Two packages (PRD-03): `@bjornpagen/bumbledb` (main, JS/types) and
  `@bjornpagen/bumbledb-darwin-arm64` (the `.node`, `os`/`cpu`-gated). The
  `@bjornpagen` scope exists and the owner has published under it before.
- `@superbuilders/errors` is a public registry dependency (ruling 2).
- The SDK API is frozen after PRD-07. Version target is `1.0.0` (ruling 8),
  lockstepped across both packages, corresponding to the engine's `1.0.0` tag
  (owner close-out).

## Work

1. **Finalize the main package manifest** (`ts/package.json`): `name`
   `@bjornpagen/bumbledb`; `version` the release version (the owner sets the final
   `1.0.0` at the bump; the manifest carries the current dev version until then);
   `private` set so it is READY to flip to `false` at publish (leave a single,
   obvious toggle — do NOT publish from the PRD); complete `exports`/`types`/`files`
   (`dist` only, no binary); `optionalDependencies` exact-pinned platform package;
   `dependencies: @superbuilders/errors`; `engines.node`; `license` (0BSD, matching
   the engine); `repository`/`homepage`/`description` fields for the registry page.
2. **Finalize the platform package manifest** (`ts/npm/darwin-arm64/package.json`):
   name, lockstep version, `os`/`cpu`, `main` → `bumbledb.node`, `files`
   (`["bumbledb.node"]`), `license`. No dependencies, no JS.
3. **Provenance/CI (optional, recommended):** if adding CI to this repo is in
   scope, author a minimal publish workflow (GitHub Actions) that builds the
   `darwin-arm64` artifact on a macOS-arm64 runner and publishes BOTH packages with
   `npm publish --provenance --access public`, platform package FIRST then main
   (so the main's optional dep resolves). If CI is out of scope now, write the
   MANUAL runbook instead (item 5) and note provenance requires CI.
4. **Tarball proof (build-time assertion, not a test PRD):** extend the build (or a
   `ts/scripts/verify-pack.ts`) to run `npm pack --dry-run --json` on both package
   dirs and assert the file manifests: main tarball = `dist/**` + `package.json` +
   `README`/`LICENSE`, and NO `.node`; platform tarball = `bumbledb.node` +
   `package.json` + `LICENSE`, and nothing else. A wrong manifest fails the build.
5. **The publish runbook** (`ts/PUBLISHING.md` or a section the owner runs):
   the exact ordered commands — set version on both (lockstep), build both, flip
   `private:false`, `npm publish` the platform package, then the main package,
   `--access public` (scoped packages default to restricted); how to verify the
   published main resolves the platform dep on a clean `npm install` on
   darwin-arm64; and the note that non-darwin-arm64 installs succeed (main is
   pure-JS) but throw the typed unsupported-platform error at load.
6. **A README for the package** (`ts/README.md`, becomes the npm page): what it is,
   the darwin-arm64-only note, install, a minimal typed example (schema → write →
   query → typed results, and a rejection-as-data snippet), a pointer to the
   engine's architecture docs. Keep it honest (research-grade, one platform now).

## Technical direction

- Scoped packages publish restricted by default — the runbook MUST use
  `--access public` or coworkers cannot install.
- Publish order is platform-package-first, main-second; document why (the main's
  exact-pinned optional dep must exist in the registry when the main resolves).
- Do NOT run `npm publish` in this PRD, do NOT flip `private:false` as the final
  committed state — leave it flipped-ready with the toggle obvious. The owner
  publishes.
- Version bump to `1.0.0` is owner close-out; the PRD wires lockstep so the bump is
  one edit propagated to both manifests + the optional-dep pin.

## Passing criteria

- Both manifests are complete and registry-valid (all required fields; scope,
  `os`/`cpu`, exports, files correct).
- `verify-pack` (or `npm pack --dry-run`) proves both tarballs contain EXACTLY the
  intended files — main has no binary, platform has only the binary — and the build
  fails if not.
- On a scratch clean install of the built main package on darwin-arm64 (local
  `npm install <packed tarball>` into a throwaway dir — a build/verify step, not an
  e2e PRD), the loader resolves the platform binary and a trivial `engineVersion()`
  smoke succeeds; the same install on a simulated foreign platform throws the typed
  unsupported-platform error.
- The runbook exists and is exact; `private` is flip-ready (not published).
- `tsc --noEmit` green; `biome check ts/` clean; `node --test` green.
- Commit in the repo's voice; push. (Publish + version bump + tag remain owner
  ceremony.)
