# ci lane board

Owns: `.github/**`, `.config/**`, `.dockerignore`, `rust-toolchain.toml`, the root `Cargo.toml`
`[profile.*]` sections, and these scripts: `ci.sh`, `bump-toolchain.sh`, `family.mjs`,
`packed-consumer.ts`, `check-asm.sh`, `static-linux-arm64/**` (plus the deleted battery, check,
miri, ramdisk, measure-cutover, release-*, build-family, packed-* and version-roster files).

Items: G1, G3, G4, G5, G10, G11, G12, G13, L (scripts and config).

## Status

| Item | Status |
|---|---|
| G3: cargo profiles `ci`/`gate`/`addon-ci`; nextest `ci`/`deep`/`default-miri`, `disk` group | landed |
| G13/L: delete release bureaucracy scripts and config; musl image on alpine 3.24.2 + prebuilt nextest 0.9.148 | landed |
| G1: `ci.yml` + `scripts/ci.sh <lane>` | landed (rustdoc gate red on other lanes' docs, see requests) |
| G5: `.config/deny.toml`, cargo-shear, Dependabot | landed (not runnable locally: no cargo-deny/cargo-shear here) |
| G4: SeaweedFS and AWS S3 lanes | landed (red until ts adds `test:s3`, see requests) |
| G10: `deep.yml` (Miri, musl, AWS, deep sweeps + release gates, macOS clippy, udeps, NEON asm, deep-red issue) | landed |
| G11: `scripts/bump-toolchain.sh`, `toolchain-canary.yml`, toolchain components | landed (micro report needs bench, see requests) |
| G12: `release.yml`, `scripts/family.mjs`, packed smoke | landed (needs ts `build.ts dist` and the D20 addon names, see requests) |

## API changes (announcements)

### Landed

- Cargo profiles (root `Cargo.toml`):
  - `ci`: dev + `opt-level = 1`, line tables, no incremental. CI tests run
    `cargo nextest run --cargo-profile ci --profile ci`.
  - `gate`: release without fat LTO, line tables. Release-mode test gates (allocation budgets,
    deep sweeps) run `--cargo-profile gate`.
  - `addon-ci`: release without fat LTO. The pull-request addon build.
  - `release` keeps fat LTO for shipped artifacts; `opt-level = 3` was the default and is gone.
- nextest (`.config/nextest.toml`): profiles `default`, `ci`, `deep`, `default-miri`. No retries
  anywhere; a test is terminated after 5 slow periods (60 s each by default).
  **Test group `disk` (2 at a time):** name a disk-saturating test `disk_*` or put it in a `disk`
  module (filter `test(/(^|::)disk(_|::)/)`). Crash-child and fsync-loop tests belong there.

- Deleted: `scripts/{measure-cutover.mjs, measure-cutover.ts, ramdisk.sh, release-ready.mjs,
  release-ready.test.mjs, release-results.mjs, release-results.test.mjs, version-roster.txt}`,
  `.config/{obligation-inventory.json, release-results.schema.json}`. Nothing replaces the
  candidate digests, specification revisions, pack provenance or the version roster.
- The static musl check no longer builds, chroots or boots `duty`; it runs the static C link
  probe, the all-feature workspace tests (`--cargo-profile ci`) and the release-mode engine tests
  (`-p bumbledb --cargo-profile gate`, which is where the allocation gates run).

- **`scripts/ci.sh <lane>` replaces `battery.sh` and `check.sh`.** Every CI job body is one lane;
  run it locally for parity. Lanes so far:
  - `lint`: `cargo fmt --check`; clippy `-D warnings` default and `--all-features`;
    `RUSTDOCFLAGS=-D warnings cargo doc --workspace --no-deps`; `cargo bench --no-run --profile ci`;
    the Python self-tests under `scripts/`; `pnpm --dir ts run lint` and `run typecheck`;
    `crates/bumbledb-node/dts.sh --check` (the committed `binding.d.ts` is current).
  - `test`: `cargo nextest run --workspace --cargo-profile ci --profile ci`;
    `cargo test --workspace --doc --profile ci`; tree-clean check (no modified tracked file, no
    unignored untracked file).
  - `addon`: `cargo build -p bumbledb-node --profile ${BUMBLEDB_ADDON_PROFILE:-addon-ci}`, copied
    to `target/addon/bdb.<platform>-<arch>.node` and `ts/bdb.<platform>-<arch>.node` (the dev
    addon); `pnpm --dir ts test`; packs and smoke-tests the host family (`scripts/family.mjs`);
    tree-clean.
  - `s3`, `miri`, `deep`, `clippy`, `udeps`, `asm`: see the S3 and deep entries below.
- `ci.yml` (pull requests, `main`, and `workflow_call` from release): `lint` once on ubuntu-24.04;
  `test` on macos-26, ubuntu-24.04, ubuntu-24.04-arm; `addon` on linux-x64 (`addon-ci`) for pull
  requests and on all three platforms (`release`, fat LTO) otherwise, uploading
  `bdb.<platform>.node` artifacts. Linux addons build in `amazonlinux:2023` (glibc 2.34).
- Deleted: `scripts/battery.sh`, `scripts/check.sh`, `.github/workflows/bumbledb-log.yml`.
- `lint` also runs `cargo deny check --config .config/deny.toml` (advisories incl. yanked,
  permissive licenses only, crates.io as the only source, wildcard versions denied except
  workspace path deps) and `cargo shear` (unused dependencies fail). Dependabot opens one grouped
  weekly PR each for actions, cargo, npm (`/ts`, `/examples/notes`) and the musl Dockerfile.
  Note for the consolidator: this nightly's cargo already warns `cargo::unused_dependencies`
  (`fearless_simd` in `bumbledb`, `bumbledb` in `bumbledb-query` today); setting
  `[workspace.lints.cargo] unused_dependencies = "deny"` would let lint drop `cargo shear`.
- **S3 lanes (`scripts/ci.sh s3`).** Environment contract:
  - `BUMBLEDB_S3_TARGET`: `seaweedfs` (with `BUMBLEDB_S3_ENDPOINT`) or `aws` (no endpoint; the log
    bucket is an S3 Express directory bucket, `*--x-s3`);
  - `BUMBLEDB_S3_REGION`, `BUMBLEDB_S3_LOG_BUCKET` (`log/`), `BUMBLEDB_S3_CKPT_BUCKET` (`ckpt/`),
    `BUMBLEDB_S3_PREFIX` (fresh per run, ends in `/`, e.g. `ci/<run>-<attempt>/`);
  - credentials through the standard AWS environment (`AWS_ACCESS_KEY_ID`, ...).
  The lane first runs a contract probe on both buckets with the AWS CLI: 32 racing
  `put-object --if-none-match '*'` on one key; exactly one wins, every loser is refused with
  `PreconditionFailed`/`ConditionalRequestConflict`, and the stored bytes are the winner's. Then it
  builds the addon (`addon-ci`) and runs `pnpm --dir ts run test:s3`.
  - `ci.yml` `s3-seaweedfs` (pull requests): `chrislusf/seaweedfs:4.48` (`weed mini`) service,
    endpoint `http://127.0.0.1:8333`, buckets `bumbledb-log` and `bumbledb-ckpt`.
  - `s3-aws.yml` (reusable; `ci.yml` calls it on pushes to `main`; deep and release call it too):
    OIDC role `vars.BUMBLEDB_AWS_ROLE_ARN`, `vars.BUMBLEDB_S3_REGION`, `vars.BUMBLEDB_S3_LOG_BUCKET`
    (Express), `vars.BUMBLEDB_S3_CKPT_BUCKET` (Standard). A missing variable fails the job.
  - **Owner (U13):** the role's trust policy must accept `repo:bjornpagen/bumbledb:ref:refs/heads/main`
    and `...:ref:refs/tags/v*`, scoped to `ci/` on both buckets; 1-day lifecycle on `ci/`.
- **`deep.yml` (nightly 03:17 UTC and manual).** Lanes: `miri` (`cargo miri nextest run -p bumbledb
  --lib`, nextest profile `default-miri`, ubuntu-24.04 and macos-26); `musl` (`musl.yml`, the
  static aarch64-musl check, moved off pushes); `s3-aws`; `deep`
  (`BUMBLEDB_DEEP=1 cargo nextest run --workspace --cargo-profile gate --profile deep`); `clippy`
  on macos-26; `udeps`; `asm` (`scripts/check-asm.sh` on the release `bumbledb-bench` on
  ubuntu-24.04-arm). Any red lane opens or comments on the one open issue titled `deep red`; a
  fully green night closes it. `scripts/miri.sh` and `scripts/miri-cross-cc.sh` are deleted.
- **`rust-toolchain.toml`** is the only place the nightly is named: `channel`, components
  `rustfmt` + `clippy`, `profile = "minimal"`, no extra targets. Miri jobs add `miri` and
  `rust-src` themselves.
- **`scripts/bump-toolchain.sh [nightly-YYYY-MM-DD]`** (needs a clean tree): picks the newest
  nightly (last 31 days) whose channel manifest ships rustc, cargo, rust-std, rustfmt and clippy
  for aarch64-apple-darwin, aarch64-unknown-linux-{gnu,musl}, x86_64-unknown-linux-gnu, plus Miri
  for the two Miri hosts and rust-src; rewrites the channel; `cargo fmt`, `clippy --fix` (default
  and `--all-features`), deletes every `#[expect]` lint reported unfulfilled (and the attribute
  when nothing is left); runs `scripts/ci.sh lint` and `test`; writes the micro report to
  `target/toolchain-bump/report.md`; commits `Toolchain: nightly-YYYY-MM-DD`.
- **`toolchain-canary.yml`** (Mondays 05:41 UTC and manual): runs the bump with the bot GitHub
  App token (`vars.BUMBLEDB_BOT_CLIENT_ID`, `secrets.BUMBLEDB_BOT_PRIVATE_KEY`) and opens or
  updates the `bot/toolchain` pull request; a red run opens or comments on the issue
  `toolchain canary red`.
- **D20 names:** the addon artifacts are `bdb.<platform>-<arch>.node` (`target/addon/`, CI artifact
  names, the dev addon `ts/bdb.<platform>-<arch>.node`) and `bdb.node` inside a platform package.
  The musl QEMU guest marker is `bdb.static-probe=1`.
- **`scripts/family.mjs`** replaces `build-family.mjs` and the `packed-*` scripts (deleted:
  `build-family.mjs`, `build-family.test.mjs`, `packed-import.sh`, `packed-project.mjs`,
  `packed-project.test.mjs`, `packed-pure-authoring.ts`):
  - `node scripts/family.mjs pack <natives-dir> <out-dir>`: installs each
    `bdb.<platform>.node` as `ts/npm/<platform>/bdb.node` (removing every other addon there),
    runs `node ts/scripts/build.ts dist` then `node ts/scripts/build.ts stage <out-dir>`, checks
    the tarball set is exactly the core plus those platforms, writes `SHA256SUMS`.
  - `node scripts/family.mjs smoke <out-dir>`: a fresh pnpm project installs the core and host
    tarballs (other platform packages overridden to `-`), typechecks
    `scripts/packed-consumer.ts` + `examples/consumers/core-ts/consumer.ts` with strict `tsc`, and
    runs `coreProgram(<tmp>/smoke.bdb)` under `makeConsumerRuntime()`.
  - The `addon` lane packs and smokes the host family on every pull request and push.
- **`release.yml`** (tags `v*`): calls `ci.yml`, `musl.yml` and `s3-aws.yml`; then `publish` in the
  `release` environment checks the tag is on `main` and equals the workspace, core and every
  platform package version, downloads the three `bdb.<platform>.node` artifacts from the ci call,
  packs and smokes the family, `npm publish`es each tarball with trusted publishing (platform
  packages first, no token), and runs `gh release create --verify-tag` with the tarballs and
  `SHA256SUMS`.
  - **Owner:** create the `release` environment with required reviewers, and register
    `release.yml` + environment `release` as the npm trusted publisher of
    `@bjornpagen/bumbledb` and the three platform packages.

## Replies

- bench, on `scripts/bench_night.py` and `scripts/bench_viz.py`: those bench/profiling scripts are
  outside this lane's owned paths this wave (they stay as they are). Consolidator: drop the
  `hash-probe`, `correspondence-oracles` and `scorecard-plan` jobs from `bench_night.py` and the
  `hash-probe` and `ghz`/`p50_norm` reads from `bench_viz.py`. `lint` runs
  `scripts/test_bench_scheduler.py`, `scripts/flame.py selftest` and `scripts/structural-corpus.py`
  (which reads `crates/bumbledb-bench/fixtures/conformance/structural-algebra.json`).
- ts: done on your side (F3): `stage.ts`/`build.ts` no longer call the deleted release scripts.

## Requests to other lanes

### all Rust lanes
- `cargo doc --workspace --no-deps` runs with `-D warnings` in lint. Today it fails in
  `crates/bumbledb-node` (bridge: public docs link private items in `db_wire.rs` and `lib.rs`) and
  `crates/bumbledb-bench` (bench: `space/census.rs` links the missing `crate::largefix`).
- Tests that write outside a temp dir fail the tree-clean check.
- **Miri (engine-storage, engine-query, numeric):** nightly Miri runs every `bumbledb` lib test with
  no name filters. Mark each lib test that reaches LMDB or other FFI, or is too slow under Miri,
  `#[cfg_attr(miri, ignore)]`; everything else must pass under Miri.
- **`BUMBLEDB_DEEP`:** property, differential and conformance sweeps should read
  `BUMBLEDB_DEEP=1` to widen their seeded case counts (deterministically). Nightly runs the whole
  workspace that way with release semantics; pull requests run the default sizes.
- **numeric:** `scripts/check-asm.sh` checks the symbols `allen_code_batch_neon`,
  `allen_code_batch_const_neon` and `allen_filter_batch_neon` (each must exist in the release
  `bumbledb-bench` binary and be free of flag writers, `b.cond` and calls). Keep those names or
  tell me the new ones. **Red at `590f8af79`:** the entry `assert!`s that replaced the
  `debug_assert!`s compile into `cmp`/`b.ne`/`bl core::panicking::*` inside all three symbols.
  Keep the proof but move it out of the kernel symbols: check lengths once in the `#[inline]`
  dispatcher in `allen.rs`, or hand the kernel a type whose constructor checked them (R3).

### bench
- `scripts/bump-toolchain.sh` runs, never blocking:
  `cargo run --profile gate -p bumbledb-bench -- micro --levels all --out <file.json>` on the old
  and the new nightly, then `... micro --compare <old.json> <new.json>` and puts its stdout
  (Markdown) in the bump pull request. Please provide those two forms (or tell me the real flags).

### ts
- **`pnpm --dir ts run test:s3`**: the S3Store conformance suite against a real store, reading the
  S3 environment contract above. With `BUMBLEDB_S3_ENDPOINT` set use `forcePathStyle: true`.
  Every variable is required: a missing one throws (never skips). Keys go under
  `BUMBLEDB_S3_PREFIX`; never delete under `log/`. The CI lane already runs the racing-create probe
  before the suite, so the suite need not repeat it. It may load the addon (the lane installs it as
  the dev addon).
- **`node ts/scripts/build.ts dist`**: please add a dist-only mode (today `release` also rebuilds
  the addon; the release job packs the three CI-built addons and must not rebuild one).
- **D20 in the addon paths:** the dev addon `ts/bdb.<platform>-<arch>.node`, platform package
  `main`/`files` `bdb.node`, `ts/.gitignore` (`bdb.*.node`, `npm/*/bdb.node`), and `stage` packing
  platform dirs that hold `bdb.node`. Also drop `pack-provenance.json` from the platform
  packages' `files`.
- The packed smoke imports `coreProgram(localPath)` and `makeConsumerRuntime()` from
  `examples/consumers/core-ts/consumer.ts` and only requires that the program completes. Keep
  those two exports (or tell me their replacements).
- `release.yml` refuses a tag unless `ts/package.json`, every `ts/npm/*/package.json` and the
  Cargo workspace version all equal it.
- `scripts/static-linux-arm64/smoke.rs` includes `examples/consumers/rust/src/main.rs` as a module
  and calls `consumer::run() -> Result<(), E: Display>`: the musl lane links that consumer into
  the static C probe. Keep that file and signature (or tell me what replaces it).
