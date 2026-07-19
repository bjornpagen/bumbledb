# PRD-P1 — Primer: the 0.4.0 cutover (new worktree + PR)

Wave P · Repo: **primer** (`/Users/bjorn/Documents/primer`) · depends on: H7
(the final SDK, locally linked) · hard break · code deletes at every site

## Objective

Primer speaks the host idiom: its 20 `fromId` decode sites become direct
reads, the `DISPATCH` table keys on row values, the hand-maintained handle
union derives from the schema, and every handle constant/`oneOf` spelling
becomes the literal/array. This PRD is net-negative in primer lines.

## Setup (execution law, same as the 0.2.0/0.3.0 trains)

Fresh worktree of primer (`.claude/worktrees/bumbledb-040`, branch
`worktree-bumbledb-040`, PR opened immediately with the do-not-merge-until-
published note). `pnpm install` at the current published pin FIRST, then
build the SDK in the bumbledb worktree (`pnpm run build`, exit 0) and
hand-link `node_modules/@bjornpagen/bumbledb` → the worktree `ts/` and the
darwin-arm64 package → `ts/npm/darwin-arm64`; NEVER run install after the
link. The `package.json` pin flips to `"0.4.0"` in this PRD (the lockfile
stays stale until the owner's post-publish install — say so in the commit
body; the CI-red window is the known release-flow gap, documented in
TODO.md).

## Work (the audited inventory — re-derive at HEAD, counts may have moved)

1. **The `fromId` sites (20 at last audit)**: view.ts:268,470;
   mint.ts:516,523,534; supervisor.ts:323,358,424,479; dispatch.ts:437,570;
   store-reads.ts:449,560,603; diagnostics.tsx:72; etl.ts:384;
   observe.ts:567,1113,1259; gates.ts:113. Each is
   `X.fromId(row.field)` + a hand-written impossible-`undefined` arm —
   both collapse to the direct field read (the row value IS the name).
   Delete the dead error arms; do not preserve them as comments.
2. **The dispatch table** (`driver/dispatch.ts` ~line 2958,
   `DISPATCH: Record<TaskKindHandleName, KindDispatch>`, consumed via
   `DISPATCH[view.kindName]`): keys directly on the row value
   (`DISPATCH[view.kind]`); the intermediate `kindName` plumbing dies.
3. **The hand-maintained union** (`driver/seats.ts` ~lines 39–47,
   `TaskKindHandleName`): derives from the schema —
   `Infer<typeof task.fields.kind>` or the equivalent read off the closed
   value's type; the hand copy is deleted. (It deliberately excluded
   `Supervise` — preserve that as `Exclude<…, "Supervise">` with the
   original comment, which is now enforceable instead of aspirational.)
4. **Handle constants + `oneOf` spellings** across gates/selections/steers
   (store/schema.ts selection gates like
   `program.where({ kind: ProgramKind.hierarchy_program })` ~line 1352;
   steers.ts:535 `row.kind === TaskKind.Cartograph`; every
   `MemberKind.X`/`ToiType.X`/`Outcome.X`/etc. literal; grep the closed
   value names): become string literals and arrays. Where a comparison
   chain becomes a `switch`, add `satisfies never` exhaustiveness ONLY
   where the code intends totality — do not manufacture exhaustiveness the
   logic never claimed.
5. **Tests**: the store/driver test files' handle spellings sweep the same
   way; assertions comparing bigint ids to constants become string
   comparisons.
6. **Green**: `pnpm typecheck` (all turbo tasks), `pnpm knip`, and the
   graph-builder store + driver test files via the repo's own invocation —
   all green against the linked build. Foreign in-flight files listed with
   proof of non-ownership (other agents work in primer).

## Technical direction

- NEVER run git stash; commit `--no-verify` in primer's voice; push the
  branch; the PR body carries the owner's post-publish runbook (publish →
  `pnpm update -i` → typecheck → lockfile commit → merge).
- Where the SDK's stricter insert typing exposes a latent primer bug (a
  bigint that was never in any roster), that is a FINDING — report it in
  the commit body, fix it visibly, never quietly.
- Zero casts introduced; deletions need no shims by definition.

## Passing criteria

- `grep -rn "fromId" src/` → zero; `grep -rn "oneOf(" src/` → zero; the
  hand union gone (grep `TaskKindHandleName =` shows only the derived
  form).
- The diff is net-negative in lines (state the count).
- `pnpm typecheck` + `pnpm knip` + owned tests green against the linked
  build; `package.json` pins exactly `0.4.0`; no other manifest edits.
- Branch pushed; PR open with the runbook and the do-not-merge note.
