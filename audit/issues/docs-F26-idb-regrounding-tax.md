# docs-F26: "the idb re-grounding tax" taught as current engine law

Severity: high
Tree: docs
Status: OPEN
Source: audit/docs.md F26
Blocked-by: none
Blocks: none

## Bug

`docs/feature-register.md`:
> The idb re-grounding tax (an idb atom is a join position) — engine
> law, documented, ~6 recursive queries carry one extra `.match`.

## Fix (cites CONTRACT C7)

Verify the law still holds, then speak: an `Interior` atom is a join
position (the re-grounding tax). Derived tables are `InteriorId`,
never `Idb`.

## Acceptance criteria

- [ ] Grep `(?i)\bidb\b` over `docs/` returns empty.
- [ ] The claim's content (the tax, the ~6-query count) verified or
      corrected against the current engine, not just re-worded.
