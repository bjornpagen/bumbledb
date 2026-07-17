# Publishing @bjornpagen/bumbledb

The owner-run release runbook (PRD-08). This repo builds and verifies both
packages; it does NOT publish. `npm publish` / `pnpm publish`, the
`private:false` flip, the version bump, and the git tag are all owner ceremony.

## The two packages

| Package | Contents | `os`/`cpu` |
| --- | --- | --- |
| `@bjornpagen/bumbledb` | pure JS + `.d.ts` (no binary) | none (installs everywhere) |
| `@bjornpagen/bumbledb-darwin-arm64` | only `bumbledb.node` | `darwin` / `arm64` |

The main package declares the platform package as an `optionalDependency`
pinned EXACT to its own version. npm/pnpm install the platform package only on
a matching host; the main package's loader (`src/native.ts`) resolves it by
name at runtime and throws a typed unsupported-platform error everywhere else.

## Version lockstep

The version lives in ONE place: `ts/package.json` `version`. Three values must
match exactly, and the build (`assertVersionLockstep` in `scripts/build.ts`)
fails if they diverge:

1. `ts/package.json` `version`
2. `ts/package.json` `optionalDependencies["@bjornpagen/bumbledb-darwin-arm64"]`
3. `ts/npm/darwin-arm64/package.json` `version`

A release bump edits all three, then the build enforces the match.

## Runbook (darwin-arm64 host, owner)

```sh
# 0. From the ts/ package root, on a macOS Apple Silicon machine.
cd ts

# 1. Set the release version in all THREE places (lockstep). For 1.0.0:
#    - ts/package.json                    "version": "1.0.0"
#    - ts/package.json                    optionalDependencies pin -> "1.0.0"
#    - ts/npm/darwin-arm64/package.json   "version": "1.0.0"

# 2. Build + verify both trees (fails on version drift, unloadable artifact,
#    or a mispacked tarball). Produces dist/ and npm/darwin-arm64/bumbledb.node.
pnpm install
pnpm run build
node --test test/**/*.test.ts
pnpm exec tsc --noEmit
pnpm exec biome check .

# 3. Flip the publish toggle in ts/package.json:  "private": true  ->  false
#    (the platform package is already publishable; only the main is gated.)

# 4. Publish the PLATFORM package FIRST — the main's exact-pinned optional dep
#    must already exist in the registry when the main resolves.
pnpm publish --access public ./npm/darwin-arm64

# 5. Publish the MAIN package SECOND.
pnpm publish --access public

# 6. Revert "private" back to true in the committed tree (publish-ready, not
#    published), then commit the version bump + tag v1.0.0.
```

`--access public` is mandatory: scoped packages publish restricted by default,
and without it coworkers cannot install. Both manifests also carry
`publishConfig.access: "public"`, so the flag is belt-and-suspenders.

## Provenance (CI only)

`npm publish --provenance --access public` attaches a signed provenance
attestation, but requires a CI runner (a macOS-arm64 GitHub Actions runner that
builds the `darwin-arm64` artifact). It is NOT available from a plain local
publish. If/when CI is added, publish order stays platform-first, main-second.

## Verifying a published install

On a clean darwin-arm64 machine:

```sh
mkdir /tmp/bumbledb-smoke && cd /tmp/bumbledb-smoke
npm init -y
npm install @bjornpagen/bumbledb
node --input-type=module -e "import { Db } from '@bjornpagen/bumbledb'; console.log(typeof Db)"
```

The optional platform dep resolves automatically and the loader binds the
addon. On any non-darwin-arm64 host the install still SUCCEEDS (the main
package is pure JS), but the first load throws the typed unsupported-platform
error naming the running platform-arch and that only `darwin-arm64` ships.
