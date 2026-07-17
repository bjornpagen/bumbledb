# TODO — handoff to a fresh Fable session

**Read this, then `docs/structural-1.0.0/` (the PRD packet). This document is the
operational handoff; the packet is the authoritative task spec.**

## ⚠️ THE ONE THING THAT MUST BE TRUE FIRST — the model

The prior session ran on **Opus 4.8**, and ultracode/Workflow subagents **inherit
the main-loop model** unless each `agent(...)` sets `model` explicitly. So the
fanout ran on Opus by accident. For the all-Fable fanout the owner wants:

- **Start the new session on Fable** (then inherited = Fable — the clean fix), AND
- **belt-and-suspenders: set `model: "fable"` on every `agent(...)` call** in the
  workflow anyway (Agent/Workflow `opts.model` accepts `"fable"`).

Do not launch any fanout until this is guaranteed. This is why the session was
restarted.

## What is already shipped (do NOT redo)

- **`@bjornpagen/bumbledb@0.1.0`** live on npm (+ `@bjornpagen/bumbledb-darwin-arm64@0.1.0`,
  the prebuilt Apple-Silicon binary), **tagged `v0.1.0`** on GitHub.
- The SDK lives in-tree at **`ts/`**, arch-split-packaged (Biome pattern:
  pure-JS main + `os`/`cpu`-gated platform binary). The napi bridge is `ts/crate/`,
  kept OUT of the Cargo workspace.
- **Primer cut over** to the registry (`@bjornpagen/bumbledb` is a **devDependency**,
  since graph-builder is dev-only via the bun TUI) and its **Vercel build is fixed**
  (no Rust build on the deployer; the `--filter` workaround removed).
- **Engine** is at zero known issues EXCEPT the fresh-mint panic gap (PRD-A closes it).
  The W-ledger, self-describing stores + `Db.exhume`, the SysV→POSIX-sem EINVAL fix,
  and the unconditional fresh never-reissue law all landed.

## The design (ratified — structural-B, kysely-inspired)

The SDK's `0.1.0` API is nominal-brand; **Wave 1 hard-breaks it to completely
structural typing**. Full detail in `docs/structural-1.0.0/00-README.md` § "The
design, ratified"; the eight rulings in one breath:

1. **Structural values** — `u64`/`i64`→`bigint`, `str`→`string`, `bool`→`boolean`,
   `bytes<N>`→`Uint8Array`, `interval<E>`→`{ start; end }`. No brands, no phantom
   tags, no minting casts. Delete the `Brand<>` machinery.
2. **Domains are labels in the schema type** — `.as("HolderId")` attaches a string
   label to the field's *descriptor type* (mirrors Rust `as`), not a value brand.
   The old `.newtype` is gone.
3. **Relational builders check domains structurally** — `contained`/`mirrors`/
   `window`/query joins reject mismatched domains at compile time by comparing the
   schema's descriptor shapes, not value brands. What's only semantic
   (target-resolves-a-key) stays a typed `Db.create` error. The one conscious
   non-goal: host id-mixing on `insert` isn't a compile error (engine catches it at
   commit) — recovering it would need the brands we're deleting.
4. **Field-list positions** — `on(R, "x")` and `on(R, ["a","b"])`.
5. **Free-function statements** — `key/contained/mirrors/window` in the `schema()`
   array; `exactly/none/between/atLeast/atMost` partition the windows (banned
   spellings unconstructible).
6. **Query = Datalog as values** — `query(S).rule(r => r.match(Rel,{f:r.var("v")}).where(pred).select(...))`,
   `program`/`p.rec` for recursion; string-named vars domain-typed, joining by
   reuse; params typed by use. No string parsing.
7. **The SDK ships its own cookbook** — the 29 recipes in the structural API,
   compile-pinned.
8. **The elegance dividend** — bare values make the marshal boundary cast-free;
   "zero casts in product code" is now literal (the lone marshal brand-assertion is
   deleted).

The 29 cookbook recipes were fully translated to the structural API and approved
(they're in this conversation's history and re-derived in PRD-S5); PRD-S5 lands
them as `ts/COOKBOOK.md` + a compile-pin test.

## The plan — `docs/structural-1.0.0/` (12 PRDs, three waves)

The DAG (fan out Wave 1 with all-Fable agents):
```
WAVE 1 (autonomous, now):  A ∥ E ∥ S1 → { S2 ∥ S3 } → S4 → S5
  A  engine fresh panic-gap drop-guard                     (prd-A)
  S1 the structural field & domain kernel (FOUNDATION)     (prd-S1)
  S2 statement algebra & schema()        depends S1        (prd-S2)
  S3 query surface                       depends S1        (prd-S3)
  S4 Db runtime/results/rejection + restore whole-SDK green  depends S2,S3  (prd-S4)
  S5 SDK cookbook                        depends S4        (prd-S5)
  E  doc reconciliation                  parallel          (prd-E)
  → then the FULL gate suite (the ONLY place all checks run)

WAVE 2 (idle machine only — NOT autonomous):  C1 heed flags · C2 fuzz hunt
WAVE 3 (idle machine + owner ceremony):        R1 bench re-true · R2 tag · R3 republish
```

Each PRD file carries strict compile-must-pass / compile-must-fail probes and its
own gate. Read the whole PRD before starting it; meet ALL its passing criteria
before checking it off; **if you cannot, STOP and tell the owner** (do not hack
green).

## Execution rulings (owner-ratified, for the fanout)

- **Worktree + observable PR.** Create a worktree, open a GitHub PR immediately,
  and keep pushing to it so commits roll in as the owner watches. (The prior
  worktree `worktree-structural-sdk` / PR #4 already holds the packet — reuse it or
  start fresh with Fable; recommend fresh so the branch history is clean Fable work.)
- **Commit discipline:** ALWAYS `git commit --no-verify` (skip hooks; avoids
  hook-triggered stashing). **NEVER run `git stash` — ever.** ALWAYS push the whole
  branch (`git push --no-verify`), NEVER cherry-pick your own changes to remote.
- **Ignore other agents.** Other agents work elsewhere in the codebase; the
  worktree isolates you — ignore all ongoing changes outside it.
- **PRDs are organizational, not atomic commit states.** Do NOT keep the tree
  typechecking between PRDs; no transitional shims; rip to the end state. S4
  restores whole-SDK green; the final Gate is where ALL checks run.
- **Quality mandates (no half-assing):** underscore-prefixed FUNCTIONS are a
  refactor hint — refactor them; underscore-prefixed PARAMS are dead args — remove
  them (except trait/interface-required, noted). ZERO casts in product code
  (`as`/`any`/`!`/unknown-launder); `@ts-expect-error` only in `test/*`, each real.
- **Do the entire wave without stopping for input** — the only stop is a genuine
  blocker (a PRD's criteria you cannot meet).
- **Serialize commits** (one committer per checkpoint) to avoid push races when
  work fans out in parallel.

## Gates (run only at the END of Wave 1)

- Engine: `scripts/check.sh` + `scripts/lean.sh` both exit 0.
- SDK (`ts/`): `pnpm run build` (cargo bridge + tsc + both package trees, loadable
  `.node`) + `pnpm exec tsc --noEmit` + `pnpm exec biome check .` +
  `node --test $(find test -name '*.test.ts')` 100% green. **`test/fixtures/*.ts`
  are spawned-child helpers, NOT tests.**

## The todo list (one per PRD, in order)

| # | PRD | Wave | Autonomous? |
|---|---|---|---|
| A  | engine fresh panic-gap drop-guard | 1 | yes |
| S1 | structural field & domain kernel | 1 | yes |
| S2 | statement algebra & schema() | 1 | yes |
| S3 | query surface | 1 | yes |
| S4 | Db runtime/results/rejection (+ restore green) | 1 | yes |
| S5 | SDK cookbook | 1 | yes |
| E  | doc reconciliation | 1 | yes |
| C1 | heed flags (NO_MEM_INIT + bulk APPEND) | 2 | NO — idle machine (measurement law) |
| C2 | all-cores fuzz hunt | 2 | NO — idle machine (must not overlap other work) |
| R1 | bench re-true + charts + README | 3 | NO — idle machine (co-tenant timing is void) |
| R2 | version 1.0.0 + tag | 3 | NO — owner ceremony (owner pushes the tag) |
| R3 | republish SDK + primer bump | 3 | NO — owner ceremony (interactive-OTP publish; no release until approved) |

## The one open decision the owner holds (Wave 3)

After Wave 1 hard-breaks the API, republish as **`0.2.0`** (recommended — hands
teammates the structural surface now; 0.x churn expected) or **hold** for the
`1.0.0` close. Either way it waits for the owner's explicit "publish."

## Exit criterion (the release floor, owner's call)

Grep the repo for a known defect, a measured-but-unclaimed win, an unexplained
behavior, or an unresolved OPEN-ledger row — find nothing. Then 1.0.0 is the
owner's decision, tagged and published by the owner.
