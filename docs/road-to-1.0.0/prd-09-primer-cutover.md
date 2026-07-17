# PRD-09 — Primer hard-cutover to the registry

Repo: primer · depends on: 08 AND the packages actually PUBLISHED to npm ·
blocks: nothing (the last step)

## Objective

Delete `packages/bumbledb` from primer entirely and repoint every consumer at the
published `@bjornpagen/bumbledb` from the registry. After this PRD the SDK has ONE
home (the bumbledb repo, `ts/`), primer builds the native module never (it pulls
the prebuilt platform package), and there is no SDK source in primer.

## Hard gate (do not start until true)

`@bjornpagen/bumbledb@<version>` and `@bjornpagen/bumbledb-darwin-arm64@<version>`
are PUBLISHED and installable from the registry (verify: a clean `npm view
@bjornpagen/bumbledb version` returns it; a scratch `npm install` on darwin-arm64
resolves the platform binary). This PRD is gated on the owner's publish ceremony
having run — it cannot proceed against unpublished packages (there is no
workspace copy to fall back to once deleted). Engine-first ordering law (ruling 9)
in its final form.

## Context

- Primer main consumes the SDK as a workspace package: `packages/bumbledb`, with
  `"@bjornpagen/bumbledb": "workspace:*"` in the root `package.json` and ~23 `src`
  files importing it. The napi crate builds in-monorepo today.
- After the move (PRD-02) the SDK's canonical source is `ts/` in bumbledb; primer's
  `packages/bumbledb` is now a stale duplicate.
- Other sessions are active in primer; this is a wholesale delete + dependency flip
  that breaks primer's build in the window between delete and a successful install
  — it must run as ONE tight operation when the tree is clear, coordinated with the
  owner (not interleaved with other in-flight primer work).

## Work

1. **Delete `packages/bumbledb` entirely** from primer — the `src/`, `crate/`,
   `test/`, `dist/`, manifests, everything. No trace remains; the SDK does not live
   in primer anymore.
2. **Repoint the dependency**: in primer's root `package.json`, replace
   `"@bjornpagen/bumbledb": "workspace:*"` with the published version
   (`"@bjornpagen/bumbledb": "^1.0.0"` — or exact if the team pins). Remove the
   package from the pnpm workspace globs if it was explicitly listed; remove any
   turbo wiring, `allowBuilds`, or build-order references to the in-monorepo crate.
3. **Verify the 23 importers unchanged at the import specifier**: they already
   import from `@bjornpagen/bumbledb` (the rename landed earlier), so the specifier
   is stable — only the RESOLUTION changes (workspace → registry). No import edits
   should be needed; if any deep-imported a subpath the published `exports` map
   does not expose, either the published `exports` must expose it (kick back to
   PRD-07/08 — do NOT widen exports ad hoc here) or the consumer adjusts to the
   public surface.
4. **Install + reconcile**: `pnpm install` to pull the published packages; on
   darwin-arm64 dev machines the platform binary resolves automatically. Remove any
   now-dead scripts that built the in-monorepo native module.
5. **Restore primer green**: `pnpm typecheck` and `pnpm knip` (and whatever primer's
   standing gates are) pass repo-wide against the registry-sourced SDK. This is the
   packet's final restore-green step for primer.

## Technical direction

- No shims, no back-compat, no dual-sourcing: the workspace copy is gone, the
  registry copy is the only one. If the published surface differs from what primer
  used, primer adapts to the published surface (the SDK froze at PRD-07; primer is
  the follower).
- This touches only primer and only the SDK-consumption surface (the dependency,
  the deleted package, dead build wiring). Do NOT refactor consumer logic; the
  import specifier is already correct from the earlier rename.
- Coordinate timing with the owner — one atomic-feeling operation on a clear tree.
  Guarded push (do not clobber other sessions' unpushed primer work).

## Passing criteria

- `packages/bumbledb` does not exist in primer; `git grep` finds no
  `workspace:*`/path reference to it and no in-monorepo crate build wiring.
- Root `package.json` depends on the published `@bjornpagen/bumbledb`; `pnpm install`
  resolves it and the platform binary from the registry (no local cargo build of
  the SDK).
- The 23 importers resolve against the published package; primer `pnpm typecheck`
  and `pnpm knip` are green repo-wide.
- Commit(s) in primer's voice; guarded push after confirming no foreign unpushed
  commits.
