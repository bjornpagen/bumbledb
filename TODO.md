# TODO — the plan of record

**The execution detail lives in `docs/structural-1.0.0/` (the PRD packet). This
document is the operational status + owner-law summary; the packet is the
authoritative task spec. Read a PRD in full before touching its scope.**

## Done (do NOT redo)

- **`@bjornpagen/bumbledb@0.1.0`** live on npm (+ `@bjornpagen/bumbledb-darwin-arm64@0.1.0`,
  the prebuilt Apple-Silicon binary), **tagged `v0.1.0`** on GitHub.
- The SDK lives in-tree at **`ts/`**, arch-split-packaged (Biome pattern:
  pure-JS main + `os`/`cpu`-gated platform binary). The napi bridge is `ts/crate/`,
  kept OUT of the Cargo workspace.
- **Primer cut over** to the registry (`@bjornpagen/bumbledb` is a **devDependency**,
  since graph-builder is dev-only via the bun TUI) and its **Vercel build is fixed**
  (no Rust build on the deployer; the `--filter` workaround removed).
- **Engine:** the W-ledger, self-describing stores + `exhume`, the SysV→POSIX-sem
  EINVAL fix, and the unconditional fresh never-reissue law all landed. The one
  remaining known gap is the fresh-mint panic gap — PRD-A (in flight) closes it.

## In flight — Wave 1 of `docs/structural-1.0.0/`

The SDK's published `0.1.0` API is nominal-brand; this wave **hard-breaks it to
completely structural typing** (structural-B, kysely-inspired — the eight
ratified points, § rulings below) and closes the last engine gap:

- **A** — engine fresh panic-gap drop-guard (standalone-green).
- **S1 → {S2 ∥ S3} → S4** — the structural SDK refactor: field & domain kernel,
  statement algebra & `schema()`, query surface, then the `Db` runtime/results/
  rejection integration that restores whole-SDK green.
- **S5** — the SDK cookbook: the 29 recipes in the structural API, landed as
  `ts/COOKBOOK.md` + a compile-pin test (needs S1–S4 real).
- **E** — doc reconciliation (this document, the architecture SDK-skin text,
  the superseded packet's deletion).
- Then the FULL gate suite — the ONLY place all checks run — the adversarial
  review, and the Land phase (commit+push CODE; **no version/publish/tag**).

## Parked

- **Wave 2 (idle machine only — NOT autonomous):** C1 heed flags
  (`NO_MEM_INIT` + bulk `APPEND`), C2 all-cores fuzz hunt. The measurement law:
  co-tenant work voids the numbers, so these wait for an idle machine and the
  owner's go.
- **Wave 3 (idle machine + owner ceremony):** R1 bench re-true + charts +
  README, R2 version `1.0.0` + annotated tag (owner pushes), R3 republish SDK +
  primer bump (interactive-OTP publish; no release until approved).

## Rulings (owner-ratified, this session)

- **Structural-B, ratified in eight points** (full text:
  `docs/structural-1.0.0/00-README.md` § "The design, ratified"): structural
  values (no brands, no minting casts — `Brand<>` deleted); domains as string
  labels on the *descriptor type* via `.as("HolderId")` (`.newtype` gone);
  relational builders check domains structurally at compile time, the engine
  judging the semantic rest (the two-boundary split; the one conscious non-goal:
  host id-mixing on `insert` is caught by the engine at commit, not at compile
  time); field-list positions `on(R, "x")` / `on(R, ["a","b"])`; free-function
  statements `key/contained/mirrors/window` with `exactly/none/between/atLeast/
  atMost` partitioning the windows (banned spellings unconstructible); query =
  Datalog as values, kysely-shaped, no string parsing; the SDK ships its own
  compile-pinned cookbook; the marshal boundary becomes cast-free — "zero casts
  in product code" is now literal.
- **No release until owner approval.** No agent bumps `ts/package.json`'s
  version, publishes, or creates/pushes a tag — those are owner ceremony
  (Wave 3). Pushing CODE to `main` is fine and expected.
- **The open republish decision (owner's, Wave 3):** after the hard break,
  republish as `0.2.0` (recommended — hands teammates the structural surface
  now; 0.x churn expected) or hold for the `1.0.0` close. Either way it waits
  for the owner's explicit "publish."
- **Fable-only fanout.** Every subagent runs Fable; no Opus — set the model
  explicitly on every `agent(...)` call (subagents inherit the main-loop model
  otherwise).

## Owner laws (standing, unchanged)

- **Push discipline:** every ready commit goes to `origin/main` right away,
  never batched. ALWAYS `git commit --no-verify` (skip hooks; avoids
  hook-triggered stashing). **NEVER run `git stash` — ever.** ALWAYS push the
  whole branch (`git push --no-verify`), NEVER cherry-pick your own changes to
  remote. Serialize commits (one committer per checkpoint) when work fans out.
- **The measurement law:** performance numbers are taken on an idle machine
  only — co-tenant timing is void. Wave 2 and Wave 3's bench work are gated on
  it.
- **Quality mandates (no half-assing):** underscore-prefixed FUNCTIONS are a
  refactor hint — refactor them; underscore-prefixed PARAMS are dead args —
  remove them (except trait/interface-required, noted). ZERO casts in product
  code (`as`/`any`/`!`/unknown-launder); `@ts-expect-error` only in `test/*`,
  each real. PRDs are organizational, not atomic commit states: no transitional
  shims, rip to the end state; S4 restores whole-SDK green; the final Gate is
  where ALL checks run.
- **If a passing criterion cannot be met, STOP and tell the owner** — never
  hack green, never weaken a probe.

## Gates (run at the END of Wave 1)

- Engine: `scripts/check.sh` + `scripts/lean.sh` both exit 0.
- SDK (`ts/`): `pnpm run build` (cargo bridge + tsc + both package trees,
  loadable `.node`) + `pnpm exec tsc --noEmit` + `pnpm exec biome check .` +
  `node --test $(find test -name '*.test.ts')` 100% green.
  **`test/fixtures/*.ts` are spawned-child helpers, NOT tests.**

## Exit criterion (the release floor, owner's call)

Grep the repo for a known defect, a measured-but-unclaimed win, an unexplained
behavior, or an unresolved OPEN-ledger row — find nothing. Then 1.0.0 is the
owner's decision, tagged and published by the owner.
