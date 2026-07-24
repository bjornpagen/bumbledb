## Platform allowlist spelled twice as branches — one table would erase both

category: inappropriate-branching | severity: low | verdict: CONFIRMED | finder: ts:bridge

### Summary

`ts/scripts/platform.ts` spells the native-build platform allowlist ({darwin, linux}) twice, as independent branch chains in `localPlatformTarget` and `nativeArtifactName`, each with its own copy of the identical refusal string. The supported-platform set is data — a single record mapping platform → cdylib artifact name — whose key set is the allowlist. Both functions become lookups with one shared refusal, and adding a compile platform becomes one record entry instead of two coordinated branch edits. This is a direct application of the repo's own doctrine (`docs/design/representation-first.md`), which names table-driven code — Brooks's "show me your tables" and Raymond's fetchmail protocol-branching-to-table replacement — as the canonical technique for exactly this pattern.

### Evidence (verified against the working tree)

- `ts/scripts/platform.ts:62-67`:
  ```ts
  function localPlatformTarget(platform: string, arch: string): string {
      if (platform !== "darwin" && platform !== "linux") {
          throw errors.new(`unsupported platform for the bumbledb native build: ${platform}`)
      }
      return `${platform}-${arch}`
  }
  ```
- `ts/scripts/platform.ts:74-82`:
  ```ts
  function nativeArtifactName(platform: string): string {
      if (platform === "darwin") { return "libbumbledb_node.dylib" }
      if (platform === "linux") { return "libbumbledb_node.so" }
      throw errors.new(`unsupported platform for the bumbledb native build: ${platform}`)
  }
  ```
  Same predicate, same error string, duplicated verbatim (lines 64 and 81).
- No lockstep mechanism exists for this pair. The file's celebrated single-source pin (`ts/test/build-platform.test.ts:66-86`) holds `PUBLISH_PLATFORM` === `SHIPPED_PLATFORMS` === the `.gitignore` carve-out — it does not touch the compile allowlist. The tests re-spell darwin/linux/win32 independently for each function (`test/build-platform.test.ts:28,32,46,57-60`).
- The two allowlists are conceptually one set: both callers in `ts/scripts/build.ts` (line 35 `localPlatformTarget(process.platform, process.arch)`, line 60 `nativeArtifactName(process.platform)`) feed the same host platform, and a compilable platform necessarily has both a target-dir spelling and a cdylib name. So a record keyed by platform is a faithful representation, not a forced unification.
- Doctrine check: `docs/design/representation-first.md` explicitly lists "reifying control flow as data — table-driven code" among the named branch-removing techniques and cites Brooks ("Show me your tables, and I won't usually need your flowcharts") and Raymond's fetchmail table replacement. The finding is the doctrine applied to its own build scripts.

### Failure scenario

A developer adds a third compile platform (say `win32`) by editing `localPlatformTarget` and forgetting `nativeArtifactName` (or vice versa). Nothing in the test suite catches the half-edit — the existing tests only pin the current two platforms plus a win32 refusal per function. One correction to the finder's framing: the drift is loud, not silent — `build.ts:60` calls `nativeArtifactName` for the same host immediately after `localPlatformTarget` succeeded, so the half-added platform throws at build time on that host. The real cost is the duplication itself (the same set and the same error string maintained in two places, and re-spelled twice more in the tests), which is why severity stays low.

### Suggested fix

```ts
const NATIVE_ARTIFACT: Record<"darwin" | "linux", string> = {
    darwin: "libbumbledb_node.dylib",
    linux: "libbumbledb_node.so"
}

function isSupported(platform: string): platform is keyof typeof NATIVE_ARTIFACT {
    return Object.hasOwn(NATIVE_ARTIFACT, platform)
}

function localPlatformTarget(platform: string, arch: string): string {
    if (!isSupported(platform)) {
        throw errors.new(`unsupported platform for the bumbledb native build: ${platform}`)
    }
    return `${platform}-${arch}`
}

function nativeArtifactName(platform: string): string {
    if (!isSupported(platform)) {
        throw errors.new(`unsupported platform for the bumbledb native build: ${platform}`)
    }
    return NATIVE_ARTIFACT[platform]
}
```

The record's key set is the allowlist; adding a platform is one entry (plus the deliberate publish-set edits the file already documents separately). The refusal is written once, and the type `keyof typeof NATIVE_ARTIFACT` makes the supported set a type rather than a pair of predicates.
