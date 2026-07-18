# PRD-V1 — 0.3.0 release staging (the owner publishes)

Wave V · Repo: bumbledb `ts/` · depends on: everything (final gates green) ·
owner ceremony at the end

## Objective

Stage `0.3.0` to the same proof bar the 0.2.0 release used: lockstep version
bump, green build, tarball manifests verified, an EXECUTED scratch-install
smoke, and a current one-liner runbook. No agent publishes, flips `private`,
or tags; the 0.3.0 publish and the (already-owner-deferred) 1.0.0 remain the
owner's.

## Preconditions (verify, do not assume)

- The final gate suite is green on the committed tree: engine
  (`scripts/check.sh`, `scripts/lean.sh`), SDK (the four commands of K8), and
  primer (P2's report).
- T3's README rewrite is in the tree (it ships inside the npm tarball).
- T5/T6/M4 locks green (CI config present; goldens pass locally).

## Work

1. **Lockstep bump**: `ts/package.json` version `0.3.0`;
   `ts/npm/darwin-arm64/package.json` version `0.3.0`; the
   `optionalDependencies` exact pin `0.3.0`. The build's
   `assertVersionLockstep` (`ts/scripts/build.ts` ~line 78) is the enforcement
   — run it, don't reimplement it.
2. **Build both trees**: `pnpm run build` — fresh `.node` against the in-repo
   engine at HEAD; the build's own manifest verification must print its
   version-lockstep and tarball-shape confirmations.
3. **Pack + verify**: `pnpm pack` both packages into `/tmp/rel-030/`;
   `tar -tzf` both: main = `dist/` + `src/` + COOKBOOK/README/LICENSE/
   package.json, NO `.node`; platform = the binary + manifest + license only.
4. **Scratch smoke, executed**: fresh `/tmp` dir; install the main tarball
   with the platform tarball satisfying the exact optional-dep pin (pnpm 11
   ignores `package.json#pnpm.overrides` — use the `pnpm-workspace.yaml`
   override mechanism the 0.2.0 smoke proved); run a script exercising the
   NEW surface end to end: `relation` with a derived coordinate + `ref`,
   a ψ statement, `Db.create`, an insert, a closed-atom query through
   `prepare`/`execute`, a `Kind.match` dispatch — assert real values, print
   an unambiguous success line.
5. **The runbook**: update `ts/PUBLISHING.md` — the "one-liner" section reads
   0.3.0, platform first then main, `--access public --no-git-checks`,
   the interactive-OTP note, `pnpm view` verification lines, and the
   post-publish primer steps (P2's runbook, referenced not duplicated).
6. Commit (the two manifests + PUBLISHING.md and nothing else) in the repo's
   voice; push. Then STOP — the publish command is the owner's, as is any
   tag (`v0.3.0` on the release-staged commit is the owner's call to delegate
   explicitly, per the v0.2.0 precedent).

## Passing criteria

- Both manifests + the pin at exactly `0.3.0`; `pnpm run build` green with
  its lockstep line printed.
- Both tarballs packed and manifest-verified as specified.
- The smoke script RAN (output shown in the report) and exercised: derived
  coordinate + ref, ψ statement, closed-atom query, `Kind.match` — not just a
  load check.
- `ts/PUBLISHING.md` current; no `private` flip, no publish, no tag in the
  diff or the shell history of this PRD.
- Commit pushed. The packet ends here; delete `docs/hardening-0.3.0/` only
  after the owner's publish confirms (house convention: the packet dies when
  shipped).
