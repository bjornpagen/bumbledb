## SDK test glob survives only by sh non-match; one subdir .test.ts silently shadows all 36 suites

category: bug | severity: medium | verdict: CONFIRMED | finder: r2:scripts-ci-packaging
outcome: fixed f163e579

### Summary

The single sanctioned test invocation, `ts/package.json:28` — `"test": "pnpm run build && node --test test/**/*.test.ts"` — leaves the glob unquoted. pnpm executes scripts through `/bin/sh` (no `shell-emulator` or `script-shell` override exists in ts/.npmrc or the repo root), and on macOS — which the CI sdk lane *requires* (macos-latest, arm64) — `/bin/sh` is bash 3.2 in sh mode, where `**` has no globstar meaning and degrades to `*`. Today `test/*/*.test.ts` matches nothing (the only subdirectory, `test/fixtures/`, contains helper `.ts` files, never tests), so the unexpanded literal reaches node, whose test runner globs `**` recursively and runs the full suite. The gate's correctness therefore rests on the shell *failing* to match — a state one innocently placed file destroys.

### Evidence (all verified on the real repo)

- `ts/package.json:28` — `"test": "pnpm run build && node --test test/**/*.test.ts"` (unquoted); `ts/package.json:59` — `"node": ">=24"`.
- `ts/test/` holds 36 top-level `.test.ts` files; `ts/test/fixtures/` holds only non-test helpers (adopt-child.ts, law-scale.ts, legacy-schema.ts, …).
- In `ts/`: `/bin/sh -c 'echo test/**/*.test.ts'` prints the literal pattern (0 matches today); `/bin/sh --version` → GNU bash 3.2.57, i.e. no globstar in sh mode.
- `node --test 'test/**/*.test.ts'` in `ts/` (node v26.4.0) discovers and passes **309 tests across all 36 files** — node's own glob engine is what runs the suite today.
- End-to-end reproduction through pnpm itself: a demo package with `test/a.test.ts`, `test/b.test.ts`, `test/fixtures/c.test.ts` and script `echo ARGS: test/**/*.test.ts` — `pnpm run t` prints `ARGS: test/fixtures/c.test.ts` **only**; both top-level files vanish from the argument list.
- `.github/workflows/ci.yml` (~lines 169-175, sdk lane comment) leans on this exact spelling and holds the invariant in prose only: "`pnpm test` (the ONE test-glob spelling — package.json's `test` script; … test/fixtures/*.ts are spawned-child helpers, never tests)". The comment *knows* the trap and encodes it as a convention instead of erasing it in the representation.
- `scripts/lean.sh:80-85` is the repo's own precedent for refusing exactly this shape: "`--exact` with a stale name runs zero tests and still exits 0 — refuse the vacuous pass so a rename can never silently drop the third oracle." The TS gate has no equivalent guard.

### Failure scenario

A contributor adds `ts/test/fixtures/roundtrip.test.ts` (or any `test/<dir>/*.test.ts` — a natural place for a spawned-child test to grow a sibling). From that commit forward, `pnpm test` locally and in the CI sdk lane expands the glob in the shell, node receives an explicit one-file list, and the other 36 suites — including `fingerprint.test.ts`, the TS half of the cross-host fingerprint lock — never execute. The lane reports green with 309 tests silently reduced to a handful. Per design doctrine (docs/design/representation-first.md — make illegal states unrepresentable), an invariant held by a ci.yml comment where the argument representation could hold it is itself the defect.

### Suggested fix

Make the representation carry the invariant instead of the comment:
- Quote the pattern so the shell can never expand it and node always globs: `node --test 'test/**/*.test.ts'`; or
- Drop the pattern entirely — `node --test` on node ≥ 22 discovers `test/` recursively by default convention (excluding non-matching helper files by the built-in `*.test.ts` name filter).

Either way, delete the "never tests" caveat from the ci.yml lane comment once the spelling no longer depends on it.
