# docs-F10: prepared-query section written in the one-list coordinate

Severity: med
Tree: docs
Status: OPEN
Source: audit/docs.md F10
Blocked-by: none
Blocks: none

## Bug

`docs/architecture/20-query-ir.md` (prepared section):
> the prepared query holds one validated plan per rule and **one**
> sink configuration, owned by the head

— contradicting the same chapter's one-sink-per-rule-list teaching.

## Fix (cites CONTRACT C7, C3)

Speak: each rule-list has its own sink — interiors in declaration
order, then the rec, then main. The prepared object holds one plan
per rule of each list and one sink per list; main's sink is the
answer.

## Acceptance criteria

- [ ] The prepared section states one sink per rule-list; the
      contradiction is gone (both passages agree).
- [ ] `bash scripts/lean.sh` green.
