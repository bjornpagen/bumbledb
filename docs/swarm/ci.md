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
| G10: `deep.yml` | todo |
| G11: `scripts/bump-toolchain.sh`, `toolchain-canary.yml`, toolchain components | todo |
| G12: `release.yml`, `scripts/family.mjs`, packed smoke | todo |

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
    the Python self-tests under `scripts/`; `pnpm --dir ts run lint` and `run typecheck`.
  - `test`: `cargo nextest run --workspace --cargo-profile ci --profile ci`;
    `cargo test --workspace --doc --profile ci`; tree-clean check (no modified tracked file, no
    unignored untracked file).
  - `addon`: `cargo build -p bumbledb-node --profile ${BUMBLEDB_ADDON_PROFILE:-addon-ci}`, copied
    to `target/addon/bumbledb.<platform>-<arch>.node` and `ts/bumbledb.<platform>-<arch>.node`
    (the dev addon `ts/src/native/load.ts` prefers); `pnpm --dir ts test`; tree-clean.
- `ci.yml` (pull requests, `main`, and `workflow_call` from release): `lint` once on ubuntu-24.04;
  `test` on macos-26, ubuntu-24.04, ubuntu-24.04-arm; `addon` on linux-x64 (`addon-ci`) for pull
  requests and on all three platforms (`release`, fat LTO) otherwise, uploading
  `bumbledb.<platform>.node` artifacts. Linux addons build in `amazonlinux:2023` (glibc 2.34).
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

## Requests to other lanes

### all Rust lanes
- `cargo doc --workspace --no-deps` runs with `-D warnings` in lint. Today it fails in
  `crates/bumbledb-node` (bridge: public docs link private items in `db_wire.rs` and `lib.rs`) and
  `crates/bumbledb-bench` (bench: `space/census.rs` links the missing `crate::largefix`).
- Tests that write outside a temp dir fail the tree-clean check.

### ts
- **`pnpm --dir ts run test:s3`**: the S3Store conformance suite against a real store, reading the
  S3 environment contract above. With `BUMBLEDB_S3_ENDPOINT` set use `forcePathStyle: true`.
  Every variable is required: a missing one throws (never skips). Keys go under
  `BUMBLEDB_S3_PREFIX`; never delete under `log/`. The CI lane already runs the racing-create probe
  before the suite, so the suite need not repeat it. It may load the addon (the lane installs it as
  the dev addon).
- `ts/scripts/stage.ts` and `ts/scripts/build.ts` call `scripts/release-results.mjs`
  (`--candidate-digest`, `--specification-revision`, `--write-native-provenance`) and read
  `scripts/version-roster.txt`; both are deleted. Drop pack provenance (`pack-provenance.json` in
  `files`) and the roster check.
- `scripts/static-linux-arm64/smoke.rs` includes `examples/consumers/rust/src/main.rs` as a module
  and calls `consumer::run() -> Result<(), E: Display>`: the musl lane links that consumer into
  the static C probe. Keep that file and signature (or tell me what replaces it).
