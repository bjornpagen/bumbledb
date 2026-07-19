# PRD-H7 — Whole-SDK green restored

Wave H · Repo: bumbledb `ts/` · depends on: every other H PRD

## Objective

The integration close, same role K8 played in 0.3.0: H1–H6 were allowed to
leave the tree red between PRDs; this PRD sweeps the fallout and restores
the whole SDK to green. It adds no surface.

## Work

1. Sweep every consumer of the changed surfaces in `ts/src` and `ts/test`:
   dead imports (`oneOf`, `fromId`, the match types), test spellings still
   using bigint handles or constants, probes H5 marked `// needs H2` now
   ordered correctly, `@ts-expect-error` directives made vacuous by the
   wave (a directive that no longer errors is itself an error — delete it
   or restore its teeth).
2. The mandate sweeps, tree-wide over `ts/src`: zero casts
   (`as`-cast/`any`/non-null `!`/`unknown`-launder, minus `as const` and
   import aliasing); zero `@ts-expect-error` outside `ts/test`; no
   underscore-prefixed functions or dead underscore params anywhere the
   wave touched; every new type claim spot-checked live (the precise
   roster's runtime twin, the decoded string values, the slimmed `Closed`
   key set).
3. Run and fix until green, in one session, in order: `pnpm run build`;
   `pnpm exec tsc --noEmit`; `pnpm exec biome check .`;
   `node --test $(find test -name '*.test.ts')` (fixtures are helpers,
   never tests).
4. The engine stayed untouched — verify by scope, not by running the world:
   `git diff main...HEAD --stat` shows zero `crates/`, `lean/`, `ts/crate`
   files. (The full engine gates run at V1's precondition check; they
   cannot have moved if no file did.)

## Technical direction

- Root-cause fixes only; a red test is fallout (fix the consumer) or a
  genuine H-PRD defect (fix that module and say so in the commit body) —
  never weaken a probe.
- If two H PRDs left contradictory surfaces, STOP and report the
  contradiction; the packet ratified no resolution for one.

## Passing criteria

- The four commands exit 0 in one session on the committed tree.
- Mandate greps zero; the type-lie spot-checks pass.
- `git diff main...HEAD --stat` contains no engine/bridge/lean paths.
- The T5 fixture + CrossHost constants still byte-identical (re-assert —
  this is the packet's standing invariant).
- Commit in the repo's voice; push.
