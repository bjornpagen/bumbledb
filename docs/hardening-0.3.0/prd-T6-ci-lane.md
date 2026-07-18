# PRD-T6 — The CI lane that runs the locks

Wave T · Repo: bumbledb (`.github/workflows/`) · depends on: — (T5 lands the
goldens it will run; order T5 → T6 preferred but not required)

## Objective

CI currently runs NEITHER half of the cross-host fingerprint lock: there is no
node/TS lane in `.github/workflows/ci.yml`, and `ts/crate` is
workspace-excluded so root `cargo test` never executes
`ts/crate/src/fingerprint_lock.rs`. Every TS-side law (cookbook compile-pin,
render-golden, fingerprint stability, the T5 goldens) is enforced only on
laptops. Add the lane.

## Work

1. Read `.github/workflows/ci.yml` and `ts/scripts/build.ts` first — mirror the
   existing job style (runner choice, cache actions, step naming voice).
2. Add a job `sdk` (or extend the matrix) that:
   - checks out, installs Rust (same toolchain-pin mechanism the existing jobs
     use — the repo pins via `rust-toolchain.toml`) and pnpm/node (pin the
     pnpm major the repo uses; read `ts/package.json#packageManager`),
   - caches: cargo registry+target for the bridge build, and the pnpm store,
   - builds the napi bridge + SDK from source: `cd ts && pnpm install
     --frozen-lockfile && pnpm run build` (the build script compiles
     `ts/crate` for the runner's own platform — verify the script supports the
     runner OS/arch; if the build is darwin-arm64-only in places, use a
     `macos-14` (arm64) runner and say so in a comment),
   - runs the full SDK suite: `pnpm exec tsc --noEmit`, `pnpm exec biome check
     .`, `node --test $(find test -name '*.test.ts')`,
   - runs the Rust half of the lock: `cargo test --manifest-path
     ts/crate/Cargo.toml` (executes `fingerprint_lock.rs`).
3. Keep the lane additive: do not modify the existing engine jobs. If the SDK
   lane is slow, it may be `needs:`-independent so it parallelizes; do not gate
   it behind the engine job.
4. Document the lane's runtime budget in a workflow comment (measured once,
   locally or from the first CI run mechanics you can verify — a number, not a
   guess; if you cannot measure, write "unmeasured" honestly).

## Technical direction

- `test/fixtures/*.ts` are spawned-child helpers — the `find test -name
  '*.test.ts'` invocation matches how the local gate runs; keep it identical.
- No new tooling (no turborepo, no custom actions); plain steps in the
  existing workflow file.
- If the runner cannot build the bridge (missing system dep), the fix is a
  setup step in the workflow, never a change to the build script's behavior
  for local users.

## Passing criteria

- `ci.yml` contains the new job with every step above; YAML is valid
  (`actionlint` if available locally, else a YAML parse check).
- The job runs BOTH lock halves (grep the workflow for the `--manifest-path
  ts/crate/Cargo.toml` test step and the `node --test` step) and the T5
  goldens run inside the suite by construction.
- Existing jobs are untouched (`git diff` shows only additions inside the
  workflows file).
- Commit in the repo's voice; push. (Verifying the lane green on GitHub is a
  human step; the PRD's bar is a correct, complete, valid workflow.)
