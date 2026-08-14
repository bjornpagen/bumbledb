# sdk-22: TS comment "program" sweep (wave-2)

Severity: low
Tree: sdk (ts)
Status: OPEN
Source: wave-2 hunt (grep over ts/)
Blocked-by: none
Blocks: none

## Bug

Comment/prose zombies (NOT the example relations named `Program` —
those are data, coupled to docs-F23):

- `ts/src/query/run.ts:42` — "a literal set folded into the
  program".
- `ts/test/query-closed-literals.test.ts:14,130,136-137,173,317` —
  "the pinned program", "wire program", "one-spelling program".
- `ts/test/answers-named-orderable-ban.test.ts:183` — "wire program
  never moved".

## Fix

Cites CONTRACT C7: reword to "query" / "wire IR". Prose-only; test
names and assertions untouched except where the word appears in an
assertion MESSAGE (equal strength).

## Acceptance criteria

- [ ] Grep `(?i)\bprogram\b` over `ts/src/` returns empty; over
      `ts/test/` returns only the `Program`/`program` example
      relation data owned by docs-F23.
- [ ] `pnpm test` in `ts/` green; zero behavior change.

## Constraints

Prose-only. No Program vocabulary in prose.
