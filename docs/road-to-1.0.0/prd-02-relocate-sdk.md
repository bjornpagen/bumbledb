# PRD-02 — Reconcile + relocate the SDK into `ts/`

Repo: bumbledb · depends on: 01 · blocks: 03, 04, 05, 06, 07

## Objective

Make `ts/` the canonical home of `@bjornpagen/bumbledb`: bring the UNION of primer
main's SDK and PR #70's exhume surface into `ts/`, stand up the TypeScript
toolchain in this Rust repo, move the napi bridge to `ts/crate/` pointed at the
in-repo engine (outside the Cargo workspace), and get `build` + the full node test
suite green against the current engine. After this PRD the SDK lives here and
builds here; primer is untouched until PRD-09.

## Context

- Source of truth is the UNION (ruling 10): primer main's `packages/bumbledb`
  (the 8 bug-fixes) ∪ PR #70's exhume files. If #70 has merged to primer main by
  now, "primer main's SDK" already IS the union — copy it wholesale. If not,
  perform the 3-way union: take primer main's SDK entire, then apply #70's exhume
  additions (`src/exhume.ts`, `src/native.ts` exhume export, `crate/src/lib.rs`
  exhume bridge, `test/exhume.test.ts`, `test/fixtures/{adopt-child,legacy-schema}.ts`,
  `test/fixtures/legacy-store/data.mdb`), resolving `src/db.ts` and `src/index.ts`
  by keeping BOTH the bug-fix hunks and the exhume hunks. Verify no exhume export
  or bug-fix is lost — grep the result for both `exhume` and each bug-fix's marker
  (`isWellFormed`, `orientation`, the keyed-`get` overload).
- The SDK inventory to relocate (primer `packages/bumbledb/`): `src/{brand, closed,
  count, db, exhume, face, fields, index, lower, marshal, native, relation, schema,
  spec, statements}.ts` + `src/query/{atom,lower,predicate,run,scope,select}.ts`;
  `crate/src/{fingerprint_lock,lib,marshal}.rs`; `test/**`; `package.json`,
  `tsconfig*.json`, `scripts/build.ts`. (Ignore `dist/`, `crate/target/`,
  `node_modules/`.)
- The engine crate is at `crates/bumbledb` in THIS repo. The bridge's cargo
  path-dep changes from the cross-repo `../../../../bumbledb/crates/bumbledb` to
  the in-repo relative path from `ts/crate/`.
- This repo is a pure Cargo workspace today (root `Cargo.toml`, `crates/`,
  `lean/`, `docs/`, `fuzz/`, `scripts/`). It has no node tooling. It does have
  `bumbledb-query` (`crates/bumbledb-query`) as the Rust downstream sugar — the TS
  SDK is the analogous downstream in another language, and lives OUT of the engine
  crate graph the same way primer's bench crate quarantines `rusqlite`.

## Work

1. **Land the union into `ts/`.** Create `ts/` and copy the reconciled SDK there:
   `ts/src/**`, `ts/test/**`, `ts/package.json`, `ts/tsconfig.json`,
   `ts/tsconfig.build.json`, `ts/scripts/build.ts`, and the bridge at `ts/crate/`
   (`ts/crate/Cargo.toml`, `ts/crate/src/**`). Preserve subpaths exactly (the
   `#*.ts` import map, the `query/` submodule). Bring the binary
   `test/fixtures/legacy-store/data.mdb` intact.
2. **Repoint the bridge at the in-repo engine.** In `ts/crate/Cargo.toml`, set
   `bumbledb = { path = "../../crates/bumbledb" }` (verify the depth from
   `ts/crate/` to `crates/bumbledb`). Confirm `ts/crate/` is NOT listed in the root
   workspace `Cargo.toml` `members`, and add it to the workspace `exclude` list if
   cargo would otherwise auto-discover it — the engine workspace must not gain the
   `napi` dependency (ruling 3). Doc-comment the path-dep and the ordering law on
   the dependency line (engine commits precede SDK builds).
3. **Stand up the TS toolchain in this repo.** Add the minimal node setup the SDK
   needs: a root or `ts/`-local `package.json` for the SDK package (`name`
   `@bjornpagen/bumbledb`, `type: module`, the existing exports/imports maps,
   `files: ["dist"]`, dep `@superbuilders/errors`, `engines.node >=24`, scripts
   `build`/`typecheck`), a `biome.json` for the `ts/` tree (match primer's SDK lint
   config — strict, the same rules the SDK was written under), and a lockfile
   strategy (pnpm or npm — pick pnpm to match the SDK's origin; a `ts/`-scoped
   workspace or a standalone package, your call, but it must `install` cleanly with
   no reference to primer). Keep `dist/`, `crate/target/`, `node_modules/` gitignored.
4. **Rewrite the native build for the in-repo layout.** `ts/scripts/build.ts`
   already spawns `cargo build --release --manifest-path crate/Cargo.toml`, copies
   the artifact to `dist/bumbledb.node`, smoke-loads it, then `tsc`s. Update paths
   for `ts/crate/` and the in-repo engine. (The arch-split packaging that changes
   WHERE the `.node` lands is PRD-03; this PRD keeps the single-`.node`-in-dist
   build working so the test suite can run.)
5. **Delete the deliberately-unhandled-portability doc note** from `build.ts` /
   `native.ts` that cites "nothing here publishes" — that ruling is reversed
   (ruling 1, 7). Replace with a one-line pointer to PRD-03's packaging. Do not yet
   change the loader's resolution (PRD-03 owns that).
6. **Sweep any primer-specific references** the SDK carried: import specifiers,
   tsconfig `extends` of a primer base config, turbo tags, `workspace:` refs — the
   `ts/` package must be self-contained (it depends only on `@superbuilders/errors`
   from the registry and its own `ts/crate/` bridge).

## Technical direction

- Do NOT bring primer's `turbo.json`/monorepo wiring; `ts/` is a standalone
  package in the engine repo. If a build orchestrator is wanted it is `ts/`-local.
- The `@superbuilders/errors` dep stays (ruling 2) — install it from the registry.
- Keep the bridge DUMB (its existing law): no logic beyond schema-directed
  marshaling. You are relocating and repointing, not rewriting the bridge.
- Reconciliation proof is mandatory: after landing the union, `grep -r exhume
  ts/src` must find the exhume surface AND the bug-fix markers must all be present.
  A lost improvement is a failed PRD (ruling 10, nonnegotiable).
- Between this PRD and PRD-03, the loader still resolves `dist/bumbledb.node`
  co-located — that is fine; the tree need not be publish-shaped yet.

## Passing criteria

- `ts/` exists with the full reconciled SDK; primer's `packages/bumbledb` is
  UNTOUCHED (its deletion is PRD-09).
- `cd ts && <install>` succeeds with no primer reference; `node scripts/build.ts`
  (or the wired `build`) compiles the bridge against `crates/bumbledb`, produces a
  loadable `dist/bumbledb.node`, and emits `dist/*.js` + declarations.
- `tsc --noEmit` (the SDK typecheck) is green; `biome check ts/` is clean.
- `node --test ts/test/**/*.test.ts` runs the full suite green — including the
  FFI/db/query suites against a real store AND `exhume.test.ts` opening the
  `legacy-store` fixture. Note: the pre-existing "fresh mint across a rejected
  commit" consumer test should now PASS (the engine's unconditional fresh law
  landed); if it does not, that is a finding for this PRD to reconcile, not to skip.
- The root `cargo build`/`cargo check -p bumbledb` and `scripts/check.sh` still
  pass — proving `ts/crate/` did NOT enter the engine workspace (ruling 3).
- Commit(s) in the repo's voice; push.
