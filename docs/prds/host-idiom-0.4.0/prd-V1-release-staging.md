# PRD-V1 — 0.4.0 release staging (the owner publishes)

Wave V · Repo: bumbledb `ts/` · depends on: everything · owner ceremony at
the end

## Objective

Stage `0.4.0` to the proof bar the 0.2.0/0.3.0 releases set: lockstep
version bump, green build, tarball manifests verified, an EXECUTED scratch
smoke exercising the NEW surface, a current one-liner runbook. No agent
publishes, flips `private`, or tags without an explicit owner delegation
(the v0.2.0/v0.3.0 precedent: the owner delegates the tag per release, in
words).

## Preconditions (verify, do not assume)

- H7 green on the committed tree; P1's report green against the linked
  build; engine gates green by scope (no engine file in the packet's diff —
  re-verify the stat).
- The zero-store-surface invariant held end to end: T5 fixture + CrossHost
  constants byte-identical to the packet's base commit.

## Work

1. **Lockstep bump**: `ts/package.json` version `0.4.0`;
   `ts/npm/darwin-arm64/package.json` version `0.4.0`; the
   `optionalDependencies` exact pin `0.4.0`. `assertVersionLockstep`
   (`ts/scripts/build.ts` ~line 78) is the enforcement — run it via the
   build, don't reimplement.
2. **Build both trees**: `pnpm run build` — fresh `.node` against the
   in-repo engine at HEAD (unchanged engine ⇒ the binary differs only by
   embedded metadata, but build it anyway; staleness is not a proof).
3. **Pack + verify**: `pnpm pack` both into `/tmp/rel-040/`; `tar -tzf`
   both: main = `dist/` + `src/` + COOKBOOK/README/LICENSE/package.json,
   NO `.node`; platform = binary + manifest + license only.
4. **Scratch smoke, executed** (fresh `/tmp` dir; the pnpm-workspace.yaml
   override mechanism from the 0.2.0/0.3.0 smokes satisfies the exact
   optional pin from disk): the script exercises the HOST IDIOM end to
   end — a closed vocabulary; an insert with a string handle
   (`kind: "DirectPass"`); a wrong-string insert asserted to THROW the
   pointed marshal error; a prepared query whose result row's closed
   column strict-equals the handle name; a `switch` over that value with
   `satisfies never`; an array-membership match; `Kind.axioms` readback.
   Print an unambiguous success line.
5. **The runbook**: `ts/PUBLISHING.md` — the one-liner section reads 0.4.0
   (platform first, then main, `--access public --no-git-checks`,
   interactive OTP), the `pnpm view` verification lines, the post-publish
   steps: the bumbledb lockfile-regeneration commit (the known
   frozen-lockfile bootstrap gap — cite TODO.md's standing note) and
   primer's update→typecheck→lockfile→merge flow.
6. Commit (the two manifests + PUBLISHING.md, nothing else) in the repo's
   voice; push. Then STOP — the publish, the `v0.4.0` tag, and the merges
   are the owner's to run or to delegate in words.

## Passing criteria

- Both manifests + the pin at exactly `0.4.0`; build green with the
  lockstep line printed.
- Both tarballs packed and shape-verified as specified.
- The smoke RAN (output in the report) and exercised: string-handle
  insert, the wrong-string throw, the named result row, the native switch,
  array membership, axioms — not a load check.
- `ts/PUBLISHING.md` current; no publish/`private` flip/tag in the diff or
  this PRD's shell history.
- Commit pushed. The packet ends here; delete
  `docs/prds/host-idiom-0.4.0/` only after the owner's publish confirms
  (house convention: the packet dies when shipped).
