# docs-026: feature register teaches "the idb re-grounding tax" as current engine law

- **Severity:** high
- **Tree:** docs
- **Status:** FIXED(b87f3ad9)
- **Source:** audit/docs.md F26
- **Depends on:** none (prose; same file as docs-024/025)

## The bug

`docs/feature-register.md:300` — "The idb re-grounding tax (an idb atom is a join position) — engine law, documented, ~6 recursive queries carry one extra `.match`."

## Why it's wrong

`Idb` is deleted (Insight 1): derived-table atoms are `AtomSource::Interior`, and the living ledger teaching "idb atom" as CURRENT law is the old Program coordinate presented as today's engine.

## The fix

Per `audit/CONTRACT.md §C7`: "An `Interior` atom is a join position (the re-grounding tax) — engine law, documented, ~6 recursive queries carry one extra `.match`." Verify the law still holds as stated before rewording (it is a claim about the shipping planner, not vocabulary — if stale, flag rather than restate).

## Acceptance criteria

- [ ] Gone: `rg -inw 'idb' docs/feature-register.md` → no matches.
- [ ] The tax's factual content (join position, ~6 queries, one extra `.match`) verified against the engine and unchanged (or corrected with a note).

## Constraints

- Prose only; the engine-side `Idb` vocabulary in doc-comments is engine-011's job — do not edit code here.
