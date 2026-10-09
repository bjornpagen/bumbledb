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
| G1: `ci.yml` + `scripts/ci.sh <lane>` | todo |
| G5: `.config/deny.toml`, cargo-shear, Dependabot | todo |
| G4: SeaweedFS and AWS S3 lanes | todo |
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

## Requests to other lanes

### ts
- `ts/scripts/stage.ts` and `ts/scripts/build.ts` call `scripts/release-results.mjs`
  (`--candidate-digest`, `--specification-revision`, `--write-native-provenance`) and read
  `scripts/version-roster.txt`; both are deleted. Drop pack provenance (`pack-provenance.json` in
  `files`) and the roster check.
- `scripts/static-linux-arm64/smoke.rs` includes `examples/consumers/rust/src/main.rs` as a module
  and calls `consumer::run() -> Result<(), E: Display>`: the musl lane links that consumer into
  the static C probe. Keep that file and signature (or tell me what replaces it).
