# PRD-R3 — Republish the SDK + primer bump

Wave 3 · Repo: both · depends on: R2 (engine tagged) + S4/S5 (structural SDK green) + OWNER APPROVAL to publish

## Objective

Publish the structural SDK to npm and repoint primer at it. The structural refactor
(S1–S4) is a hard break over the published `0.1.0`; this PRD ships that break as a
new version and moves primer's dev-dep forward. **Every publish is owner ceremony**
— the packet PREPARES the build + the runbook; the owner runs `pnpm publish`.

## The open decision the owner resolves before this fires

Publish as **`0.2.0`** (hand teammates the structural API + cast-free surface now;
0.x churn is expected) OR as **`1.0.0`** (lockstepped to the engine tag, if the
owner wants the SDK to hit 1.0.0 with the engine). The version is one edit
propagated to both package manifests + the platform optional-dep pin (the arch-
split lockstep from the 0.1.0 packaging). This PRD does not choose — it wires the
chosen version and stops at the runbook.

## Work

1. **Set the release version** (the owner's choice above) on `ts/package.json` and
   `ts/npm/darwin-arm64/package.json` and the `optionalDependencies` exact pin, via
   the single-source lockstep the build already enforces (build fails if they
   diverge). Do NOT flip `private` or publish yet.
2. **Build both package trees** (`pnpm run build`) — the structural SDK + the fresh
   `darwin-arm64` binary built against the tagged engine (R2). Re-verify the tarball
   manifests (main = `dist/` + `src/`, no `.node`; platform = only the binary) and
   the scratch-install smoke on darwin-arm64 (install both packed tarballs, load,
   run a trivial typed op) — the same proof the 0.1.0 publish used.
3. **Draft the publish runbook** (`ts/PUBLISHING.md`, already exists — update it):
   the exact ordered commands — flip `private:false`, `pnpm publish` the PLATFORM
   package first then the MAIN package, `--access public`, `--no-git-checks` as
   needed, then `npm view` verification. Note the OTP/2FA step is interactive (the
   owner runs it; an agent cannot complete `pnpm publish` non-interactively —
   established at the 0.1.0 publish).
4. **The owner publishes** — runs the runbook. An agent does NOT run `pnpm publish`.
5. **Primer bump** (AFTER the packages are live — verify via a scratch registry
   install): update primer's `devDependencies` `@bjornpagen/bumbledb` from `^0.1.0`
   to the new version; `CI=true pnpm install --no-frozen-lockfile`; adapt the 27
   `src/tools/**` importers to the structural API (the hard break — bare values,
   `.as` domains, the new query shape) until primer `pnpm typecheck` + `pnpm knip`
   are green. Guarded push (no foreign unpushed commit clobbered). Primer's Vercel
   is unaffected (still a dev-dep, still no Rust build). This is the ONE place this
   packet touches primer, and only after publish.

## Technical direction

- Hard gate before the primer bump: the new version is PUBLISHED and installable
  from the registry (a clean scratch `pnpm add @bjornpagen/bumbledb@<v>` resolves it
  + the platform binary) — do not repoint primer against unpublished packages
  (there is no workspace fallback; that would break primer).
- The primer importer adaptation is a real hard-break migration of consumer CODE
  (not data — no data migration here); it follows the structural surface S1–S4
  froze. No shims; adapt to the end-state API.
- Engine-first ordering in its final form: engine tagged (R2) → SDK published →
  primer bumped.

## Passing criteria

- Both manifests at the chosen version (lockstepped, exact optional-dep pin); build
  produces both trees; tarball manifests + scratch-install smoke pass on
  darwin-arm64.
- `ts/PUBLISHING.md` runbook exact and current; `private` flip-ready (not flipped/
  published by an agent).
- After the owner publishes: `npm view` shows both packages at the version; a clean
  scratch install resolves them.
- Primer bumped to the published version; the 27 importers adapted to the structural
  API; primer `pnpm typecheck` + `pnpm knip` green; guarded push.
- No agent ran `pnpm publish` or pushed a tag; the version bump + primer commits are
  the only agent commits, pushed.
