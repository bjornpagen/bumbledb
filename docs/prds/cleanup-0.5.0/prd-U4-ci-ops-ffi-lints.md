# PRD-U4 — CI/ops + the FFI lint regime

Wave 1 · Repo: bumbledb (`.github/`, `scripts/`, `ts/crate`, versions) ·
depends on: U3 for the lint half (land the lints after the lib.rs macros
settle) · executes rulings 3 (ubuntu lane + miri fix), 12 (ts/crate lints),
13 (0.5.0)

## Objective

Make the operational story true: the Miri cron stops being a standing red,
linux stops being a zero-coverage fiction, the FFI crate joins the unsafe
wall, the stale prose dies, and the wave's version is staged.

## Work — CI (ruling 3)

1. **Fix the Miri cron.** `scripts/miri-cross-cc.sh`: when a foreign
   `--target` was stripped and the input is a `.S` file, emit an empty object
   (`printf '' | cc -x c -c -o "$OUT" -`) — consistent with the script's own
   rationale (the staticlib is a build-graph artifact under Miri; nothing
   calls into it). Then `workflow_dispatch` the miri job and record the green
   run id in the PR. Also reconcile WHY local passes while CI never has
   (cached object or a permissive local clang — one look, one sentence).
2. **The ubuntu engine lane.** Add one `runs-on: ubuntu-latest` job running
   the engine's check + test (`cargo fmt --check`, `clippy -D warnings`,
   `cargo test -p bumbledb -p bumbledb-theory -p bumbledb-macros -p
   bumbledb-query`) — the first execution of the engine's linux arms
   (`posix_fallocate` dies in U1, but `ramdisk`-adjacent code, `devhonesty`'s
   /proc/mounts arm where testable, and the whole portable engine gain a real
   runner). Scope honestly: no NEON, no hdiutil, no sdk lane on linux (the
   sdk lane's darwin-arm64 hardcode is a standing owner question, not this
   PRD's). Cache keyed like the existing lanes. Prove green by dispatch
   before relying on it.
3. **Hygiene sweep** (census `cleanup-ops.md` FIX list):
   - ci.yml stale "First-run verification: pending" comments (×2) — delete.
   - check.sh line-3 comment "CI, when it exists" — CI exists; fix. Fix the
     "feature-off matrix" comment too (resolver-2 unifies {ground-off,…} via
     the bench dev-dep — say what the lanes actually build).
   - The x86_64 cross `cargo check` that executes nowhere: the ubuntu lane
     SUPERSEDES the aspiration — delete the self-skipping cross check from
     check.sh, or rewrite it to state the ubuntu lane is the real coverage.
   - `actions/checkout` + `actions/cache` off the deprecated Node-20 majors.
   - Scope `on: push` to `branches: [main]` and add `pull_request` — WIP
     pushes stop burning macOS minutes; cron + dispatch stay.
4. **Out of scope, flagged to the serial committer** (git is off-limits to
   agents): deleting the five stale remote branches; PR #10's README
   conflict and waveM `report.json` force-adds — PR #10's own debts.

## Work — the FFI lint regime (ruling 12)

5. `ts/crate`: add `unsafe_code = "deny"` (crate lints or `#![deny]`) and
   convert every FFI unsafe site (~35 in `lib.rs`/`marshal.rs`; fewer after
   U3's macros) to `#[expect(unsafe_code, reason = "…")]` with a real
   per-site reason in the house voice — no blanket `allow`, no reason-free
   expects. `fuzz/` stays deliberately detached (crucible ruling) — do not
   touch it.

## Work — the version (ruling 13)

6. Stage 0.5.0: `ts/package.json` + `ts/npm/*/package.json` (lockstep gate),
   engine crate versions if the house bumps them in lockstep (follow the
   0.4.0 precedent — read the bump commit). **No publish, no tag** — owner
   ceremony. Note the documented frozen-lockfile red window (TODO.md /
   PUBLISHING.md): the bump commit pins a platform package that doesn't exist
   on npm yet, so the sdk lane is expected-red between bump and owner
   publish — sequence the bump LAST in the wave and say so in the commit.

## Passing criteria

- Miri job: a green `workflow_dispatch` run recorded (run id in the PR).
- Ubuntu lane: a green dispatch run recorded; the lane runs on push-to-main
  and PRs thereafter.
- `grep -rn "when it exists\|First-run verification" .github scripts` empty.
- `ts/crate`: `cargo clippy` green under `deny(unsafe_code)`; every expect
  carries a reason; `#[expect]` count ≤ the site count U3 left behind
  (no new unsafe).
- Version 0.5.0 staged exactly once, lockstep gate green locally
  (`ts/scripts/build.ts` version check); no tag, no publish.
- `scripts/check.sh` + full SDK gate green on the macOS side (the new lane
  adds coverage, never replaces it).
