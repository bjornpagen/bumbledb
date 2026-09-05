# Packaging — artifacts, staging, pins, handshake

Status: permanent doc for the shipped packaging design. Physical
layout roles are in [performance.md](performance.md). Packed-consumer
expectations are in [api.md](api.md). Publication (`PKG-07B`) is
separately authorized and is not implied by local staging.

## Artifact roster

| Artifact | Contents | Source |
| --- | --- | --- |
| `@bjornpagen/bumbledb` | dist JS + declarations, `src/` (types-only isolation map), COOKBOOK | `ts/` |
| `@bjornpagen/bumbledb-log` | dist JS + declarations, `/schema` + `/migrations` subpaths, `bumbledb-log` CLI | `ts-log/` |
| `@bjornpagen/bumbledb-darwin-arm64` | exactly `[LICENSE, bumbledb.node, package.json]` | `ts/npm/darwin-arm64` |
| `@bjornpagen/bumbledb-linux-arm64` | same; built on amazonlinux:2023 (glibc 2.34 floor) | `ts/npm/linux-arm64` |
| `@bjornpagen/bumbledb-linux-x64` | same; built on amazonlinux:2023 (glibc 2.34 floor) | `ts/npm/linux-x64` |

ONE full-capability native artifact per platform, shared by core and log
(core plus the internal log implementation; a core-only import starts no
transport/maintenance work). No separate log addon, slim distribution or
optional native flavor exists. Rust crates are `publish = false`; Rust
consumers build from source (see `examples/consumers/rust`).

## Exact pins (the handshake)

- `effect` is EXACTLY `4.0.0-rc.112` as peer and dev dependency of both
  TS packages. An RC upgrade is an explicit lockfile/API/requalification
  change, never a range.
- `@bjornpagen/bumbledb-log` peers on `@bjornpagen/bumbledb` at the EXACT
  same version — the log can never silently select a different native
  command/runtime contract.
- The packed core manifest pins every platform package at the EXACT
  release version in `optionalDependencies` — injected into the STAGED
  manifest only (below); the committed manifest carries no pins so a
  lockfile can never demand an unpublished version.
- One version everywhere: `scripts/version-roster.txt` +
  `ts/scripts/build.ts`'s lockstep gate hold the workspace, npm and crate
  manifests to a single number; `engineVersion()` bakes it into the
  binary and the loader refuses any other-version artifact.
- Runtime handshake: a bootstrap descriptor (ABI major/minor, build
  revision, feature bitmap, engine format, codec/protocol versions,
  architecture, libc/OS floor, CPU baseline, N-API level) is checked
  before opening or mutating data; missing artifact and incompatible
  artifact are distinct refusals (FFI-08).

## Immutable staging (no source-mutating pack hooks)

Packing NEVER rewrites the checkout (PKG-02): there are no
prepack/postpack lifecycle hooks anywhere. `ts/scripts/stage.ts` and
`ts-log/scripts/stage.ts` copy the built outputs and committed files into
an isolated temp staging tree, derive the packed manifest there (pins
injected; `scripts`/`devDependencies`/`packageManager` stripped), and run
`pnpm pack` inside that tree. Interruption at any phase leaves the
checkout byte-identical; both build scripts assert it on every build.

```sh
node ts/scripts/stage.ts --out <dir>        # core + platform tarballs
node ts-log/scripts/stage.ts --out <dir>    # log tarball
```

Each staged tarball carries `pack-provenance.json` binding
`candidateSourceDigest` and `specificationRevision` to the same values
computed by `scripts/release-results.mjs`. `scripts/packed-import.sh`
refuses tarballs whose provenance does not match the current checkout.

`scripts/packed-import.sh` consumes exactly these staged tarballs for the
isolated-consumer gate (`PKG-03` / D07 / D22 / D27): fresh empty project,
no workspace links, strict downstream tsc over copied
`examples/consumers/{core-ts,log-ts,native-ledger}`, a ManagedRuntime
consumer (`scripts/packed-consumer.ts` —
`ManagedRuntime.make(NativeRuntime.layer(...))` for programs that no
longer self-provide), a second addon-unavailable project
(`scripts/packed-pure-authoring.ts`), the Rust consumer, and Notes
`specimens.test` + `routes.test` (missing migrations fail, never skip
green). D07 tiny collect must refuse. That gate is not registry
publication proof.

## Deletion inventory (PKG-06)

`node ts/scripts/absence-gate.ts` is the affirmative check that the
removed products STAY removed: no C crate/header/example/workflow/ABI
artifact, no public Rust log SDK (publish=false + doc(hidden) internals),
a log/AWS-free core dependency graph, exact Effect pins, no committed
platform pins or pack hooks, no tracked binaries, and no
`@superbuilders/errors` anywhere in maintained code.

## Release flow (F3 / promotion)

1. Fresh locked builds per platform at the pinned toolchain
   (`rust-toolchain.toml`); provenance records source revision, flags,
   locks and digests (PKG-01).
2. Stage + pack (above); interrupt/retry coherence is PKG-02.
3. Tarball-isolated consumers and the canonical target matrix
   (PKG-03/04); golden data compatibility (PKG-05).
4. Pre-promotion: exact staged digests, empty-project/private-registry
   installs, pins, allowlists, simulated partial publication (PKG-07A).
5. After separately AUTHORIZED publication: download the actual registry
   artifacts, verify identical digests, clean remote install (PKG-07B).
   A mismatch is a release incident, never retroactive qualification.

Publish the tested immutable staged tarballs; never rebuild during
promotion. Patch releases change no equality, float, ID, plan or receipt
meaning; storage/protocol/plan versions are independent of npm semver.
