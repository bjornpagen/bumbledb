# PRD-K8 — Whole-SDK green restored

Wave K · Repo: bumbledb `ts/` · depends on: every other K PRD

## Objective

The integration close: K1–K7 were allowed to leave the tree red between PRDs
(no shims, hard breaks). This PRD sweeps the fallout and restores the whole
SDK to green — the same role S4 played in the structural wave. It adds no new
surface; it finishes the cut.

## Work

1. Sweep every consumer of the changed kernels inside `ts/src` and `ts/test`:
   dead imports, old spellings (the curried closed, hand statements that now
   collide with derivation probes, `.as` labels the dot-ban rejects, string
   var repetition in tests that K5's probes replaced), stale type
   references — rip to the end state, never shim.
2. `ts/test` hygiene: every `@ts-expect-error` in the suite is REAL (removing
   it breaks compilation) — re-verify the ones the wave's changes may have
   made vacuous (a directive that no longer errors is itself an error under
   the suite's config; fix by deleting or by restoring the probe's teeth).
3. The mandate sweeps, tree-wide over `ts/src`:
   - zero casts (`as`-cast/`any`/non-null `!`/`unknown`-launder — the grep
     patterns the 0.2.0 verifier used, minus `as const` and import aliasing);
   - zero `@ts-expect-error` outside `ts/test`;
   - no underscore-prefixed functions (refactor them away) and no
     underscore-prefixed params (delete the dead arg) anywhere the wave
     touched;
   - every runtime-property type claim spot-checked live (the type-lie law):
     `columns`, `refTo`, `citeTo`, the `where`/`match` mints.
4. Run and fix until green, in order: `pnpm run build` (bridge + tsc + both
   package trees + loadable `.node`); `pnpm exec tsc --noEmit`;
   `pnpm exec biome check .`; `node --test $(find test -name '*.test.ts')`
   (fixtures are helpers, never tests).

## Technical direction

- Root-cause fixes only: a red test is either fallout (fix the consumer) or a
  genuine defect in a K PRD's work (fix THAT module and say so in the commit
  body) — never weaken a probe to pass.
- If two K PRDs left genuinely contradictory surfaces (an integration
  conflict, not fallout), STOP and report the contradiction rather than
  inventing a resolution the packet didn't ratify.

## Passing criteria

- The four commands above exit 0, in one session, on the committed tree.
- The mandate greps return zero hits; the type-lie spot-checks pass.
- No `ts/src` file imports anything that no longer exists (tsc proves it; say
  it anyway).
- Commit in the repo's voice; push.
