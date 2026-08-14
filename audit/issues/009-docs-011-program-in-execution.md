# docs-011: the execution chapter still calls a query a program (8+ sites)

- **Severity:** high
- **Tree:** docs
- **Status:** FIXED(b87f3ad9)
- **Source:** audit/docs.md F11
- **Depends on:** none (prose; parallel-safe — own file)

## The bug

`docs/architecture/40-execution.md` (10 hits for "program"; representative lines):

- `:314` "multi-rule program whose heads are provably pairwise disjoint"
- `:321` "spanning a multi-rule program, keyed by provenance"
- `:376` "A **hand-written multi-rule program** keys the **head projection**"
- `:1045` "— multi-rule programs — the"
- plus "each rule of a program executes its own plan", "`union_regime_head_projection` for hand-written programs", "the single-rule key-probe program", "a program shrunk to one rule sheds its union machinery like any single-rule program".
- `:462` also cites `evalQuery_plain` as "the empty-interiors empty-rec case" — docs-004's embedding, in this file. Same sweep.

## Why it's wrong

Dual vocabulary in the chapter that teaches execution (Insight 1): `Program` is deleted, and every "program" here denotes a query's main rule-list. The doc trains readers to think in the deleted type while the code they'll read says `Query`.

## The fix

Per `audit/CONTRACT.md §C7`: mechanical sweep — "multi-rule **query**" (or "main rule-list" where the sentence is main-specific); "hand-written vs DNF-derived rule sets of one `Query`"; "single-rule key-probe **query**"; "a query shrunk to one rule sheds its union machinery". `:462` → constructor case of `evalQuery`, not `evalQuery_plain` (lean-001 / C4). Lock names (`union_regime_head_projection` etc.) DO NOT change — only the surrounding prose. `DeltaVariant` / `PreparedBody::Empty` in this file are docs-030 (depend on engine-007/023) — do not rewrite them here into a third query type.

## Acceptance criteria

- [ ] Gone: `rg -inw 'program|programs' docs/architecture/40-execution.md` → no matches naming a query (if a genuine non-query sense survives, list it in the commit message); `rg -n 'evalQuery_plain' docs/architecture/40-execution.md` → no matches.
- [ ] All cited lock/test/theorem names byte-identical.

## Constraints

- Prose only. The § linear-reach-driver section is already clean (audit: not a finding) — don't churn it.
