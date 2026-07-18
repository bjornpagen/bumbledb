# PRD-P1 — Primer: the store schema goes derived

Wave P · Repo: **primer** (`/Users/bjorn/Documents/primer`) · depends on: K8
(the final SDK surface, locally linked) · hard break

## Objective

Primer's graph-builder store schema
(`src/tools/graph-builder/store/schema.ts` — at last count 19 `.as(` sites and
40 `contained(` statements, 31 of them the simple derivable shape) is the
first real consumer of the coordinate kernel. Cut it to the 0.3.0 idioms
fully: derived coordinates, `ref`/`cites`, ψ where its closed vocabularies are
being complement-worked-around, no hand statement that a `ref` derives.

## Context / constraints

- Primer is dev-only through the bun TUI; its stores are rebuildable
  artifacts. The schema rewrite WILL move the store fingerprint (derivation
  reorders/replaces statements). That is accepted: humans rebuild dev stores;
  no migration work in this PRD, and the commit body must SAY the fingerprint
  moved.
- Work against the LOCALLY LINKED SDK build (the 0.2.0-train procedure:
  `pnpm install` while the manifest still points at the published version,
  then symlink `node_modules/@bjornpagen/bumbledb` → the bumbledb worktree's
  `ts/` and the darwin-arm64 package → `ts/npm/darwin-arm64`; never run
  install after the link). P2 owns the version pin; this PRD may develop
  linked without touching `package.json`.
- Other agents work in primer — stay inside the graph-builder store cluster
  and its tests; ignore unrelated dirt.

## Work

1. Read the whole schema file first and MAP it: every `.as` label → is it an
   owned declaration (→ derived coordinate / `ref`), a reference to a fresh
   id (→ `ref`), a deliberate statement-free link (→ `cites`), or a genuinely
   shared value domain (→ keep `.as`, justify in a comment only if
   non-obvious)? Every `contained` → derivable by a `ref` (→ delete the hand
   statement) or composite/selected/ψ-shaped (→ keep, in the new spellings)?
2. Rewrite the schema: fresh fields drop their `.as` (K3 derives the
   coordinate; keeping the label is now a construction error); references
   become `ref(...)`; deliberate-weak links become `cites(...)`; derivable
   hand containments are DELETED (recipes' dedupe-keeps-hand rule is for
   fingerprint-stable migrations — primer accepts the motion, so go full
   derivation: the schema file is the representation, the statements the
   derivation).
3. Adopt ψ/closed atoms and the new ergonomics in the store cluster's OWN
   code and tests (`schema.test.ts`, `rebirth.test.ts`, and any store file
   constructing queries): closed `.where` targets where the schema previously
   spelled complements; `vars()`/free comparisons/`Kind.match`/3-arg `closed`
   where the old spellings no longer compile.
4. The cluster's tests: rewrite expectations that pinned old manifest
   spellings or old fingerprints; assertions on statement lists follow the
   derived tail order.

## Passing criteria

- `pnpm exec tsc --noEmit -p tsconfig.json` shows ZERO errors in the store
  cluster's files (the wider repo may be red only from OTHER agents' in-flight
  files — list any such file in the report and prove it's not yours by paths).
- The store cluster's test files pass under the repo's own test invocation.
- Grep the schema file: no `.as(` site remains that the map in step 1
  classified derivable; no `contained(` remains whose exact statement a `ref`
  in the same file derives; every `cites` has the selected/deliberate
  statement it defers to nearby.
- The commit body names the fingerprint motion and the rebuild consequence.
- Commit with `--no-verify` in primer's voice; push the branch.
