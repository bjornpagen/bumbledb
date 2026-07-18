# PRD-P2 — Primer: the full 0.3.0 cutover sweep

Wave P · Repo: **primer** · depends on: P1 · completes downstream consumption

## Objective

Every remaining `@bjornpagen/bumbledb` importer in primer moves to the 0.3.0
surface and idioms, the dependency pins `0.3.0`, and the whole repo is green
against the locally-linked build — so that after the owner publishes, a plain
install resolves and nothing else changes. (At the 0.2.0 cutover, driver/etl/
prompts needed zero edits because they consumed only stable type names — do
NOT assume that holds again: K-wave changed query construction, closed
values, and statement types. Re-derive the truth.)

## Work

1. **The battlefield map, fresh**: grep the whole repo for
   `@bjornpagen/bumbledb` importers (last count 27 files across
   graph-builder driver/etl/prompts, the two benchmark seeds, and tests).
   For each, list which imported names' types or spellings K-wave moved
   (closed values now carry `where`/`match`; query rule scopes carry `vars`;
   statement/`Fact` types flow coordinates in domains; the curried `closed`
   died). Files consuming only `Fact`/`Tx`/`Violation`-shaped types may again
   be no-ops — PROVE each no-op by typechecking, not by analogy.
2. **Query-construction sites** (`prompts/store-reads.ts` and any
   driver/etl site building rules): adopt `vars()`, free comparisons, closed
   atoms where they simplify — and anywhere the old spelling no longer
   compiles, the new spelling is mandatory (no compat imports).
3. **The pin**: `package.json` devDependencies `@bjornpagen/bumbledb` →
   `"0.3.0"` exactly. Do not run any install after the local link (the
   0.2.0-train law); the lockfile stays stale until the owner's post-publish
   install — state this in the commit body.
4. **Green**: `pnpm typecheck` (all turbo tasks incl. `typecheck:root`),
   `pnpm knip`, and the graph-builder + any touched clusters' test files via
   the repo's own test invocation — all green against the linked build.
   Other agents' in-flight files may be red: list them by path with evidence
   they're untouched by this PRD (git status vs your file list).
5. Leave a PR-ready branch: commits in primer's voice, `--no-verify`, pushed;
   the PR body (or commit body if no PR yet) carries the owner's post-publish
   runbook: publish 0.3.0 → `pnpm update -i` (or `pnpm install
   --no-frozen-lockfile`) → typecheck → commit lockfile → merge.

## Passing criteria

- Zero `@bjornpagen/bumbledb` importer uses a spelling that no longer exists
  (tsc proves it; the map documents it per file, including proven no-ops).
- `package.json` pins exactly `0.3.0`; no other manifest/lockfile edits.
- `pnpm typecheck` + `pnpm knip` + owned tests green against the linked
  build; foreign red files listed with proof of non-ownership.
- Branch pushed with the runbook text in place.
