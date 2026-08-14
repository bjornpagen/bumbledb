# lean-020: Lean doc comments call the one rec an "SCC" and teach "no Tarjan" / "No strata" by negation

- **Severity:** low
- **Tree:** lean
- **Status:** OPEN
- **Source:** adversarial pass (not in audit/lean.md; the docs audit's F3/F6 pattern, found in the Lean spec itself)
- **Depends on:** none (comment-only; textually overlaps lean-001/002 in Syntax.lean — land after them to avoid churn)

## The bug

The spec-of-record's own comments keep the condensation coordinate:

- `lean/Bumbledb/Query/Syntax.lean:10` — "at most one linear rec SCC"
- `lean/Bumbledb/Query/Syntax.lean:271` — "/-- One recursive SCC (this cut: one name, linear arms)."
- `lean/Bumbledb/Query/Syntax.lean:282` — "rec SCC, then the main query."
- `lean/Bumbledb/Query/Syntax.lean:448` — "/-! ## Well-formedness — one recursive SCC, no Tarjan -/"
- `lean/Bumbledb/Query/Syntax.lean:479` — "Bans **all** negation in the rec SCC"
- `lean/Main.lean:384` — "/-- One linear rec SCC. -/"
- `lean/Bumbledb/Exec/Reach.lean:7` — "No fuel. No strata." (negation of retired coordinates in the module header)

## Why it's wrong

Insight 1: the Lean tree is the specification humans read to learn the model, and it names the rec after the Tarjan/stratum artifact the cut deleted — the exact defect docs-003/docs-006 fix in `docs/`, living upstream of them in the spec itself. "No Tarjan"/"No strata" teach the deleted machinery by negation.

## The fix

Per `audit/CONTRACT.md §C7` vocabulary (it governs all prose, including Lean docstrings): "one linear rec" replaces "rec SCC" at every listed site; the `:448` section header becomes "## Well-formedness — one linear rec"; `Reach.lean:7`'s header states the positive ("The denotation is `evalQuery`; the budget is a resource abort") without the negation list.

## Acceptance criteria

- [x] Gone: `rg -inw 'scc|tarjan|strata' lean/Bumbledb lean/Main.lean` → no matches.
- [x] Comment-only: `lake build` output identical; zero theorem/def/name changes; `./scripts/lean.sh` fully green (battery + census + 268-case conformance + comparator).

## Constraints

- Prose only; no identifier changes (identifier-level vocabulary is lean-011/lean-015's scope). Land after lean-001/002 touch these files.
