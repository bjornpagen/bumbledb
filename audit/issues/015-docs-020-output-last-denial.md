# docs-020: lowering contract denies the deleted Program layout instead of stating the Query layout

- **Severity:** high
- **Tree:** docs
- **Status:** OPEN
- **Source:** audit/docs.md F20
- **Depends on:** none (prose; same file as docs-019)

## The bug

`docs/architecture/75-cpp-lowering.md:458-459` — "There is no output-last predicate slot and no `output = recs.length`." — and the checklist at `:696` — "interiors then rec then main — no output-last predicate slot".

## Why it's wrong

The deleted `Program` layout (predicates table, recs list, output index) survives as the reference frame (Insight 1): a lowering contract should describe `Query` field-for-field; denying old slots teaches them, and an SDK author who never saw Program now knows its shape.

## The fix

Per `audit/CONTRACT.md §C7`: "Wire shape is `QueryIr { interiors, rec, head, rules }`. Evaluation order is interiors, optional rec, main. Main is `head` + `rules`." Checklist item: "interiors then rec then main". Delete both denials.

## Acceptance criteria

- [ ] Gone: `rg -n 'output-last|output = recs' docs/architecture/75-cpp-lowering.md` → no matches.
- [ ] The wire-shape and evaluation-order facts unchanged.

## Constraints

- Prose only.
