# docs-F11: the execution chapter calls a query a program (eight sites)

Severity: high
Tree: docs
Status: OPEN
Source: audit/docs.md F11
Blocked-by: none
Blocks: none

## Bug

`docs/architecture/40-execution.md`: "multi-rule program whose heads
are provably pairwise disjoint"; "spanning a multi-rule program";
"each rule of a program executes its own plan";
"`union_regime_head_projection` for hand-written programs"; "A
**hand-written multi-rule program** keys the **head projection**";
"for a hand-written program"; "the single-rule key-probe program";
"a program shrunk to one rule…".

## Fix (cites CONTRACT C7)

Speak: multi-rule **query** / main rule-list; hand-written vs
DNF-derived rule sets of one `Query`; single-rule key-probe
**query**. Content of each claim unchanged.

## Acceptance criteria

- [ ] Grep `(?i)\bprogram\b` over `docs/architecture/40-execution.md`
      returns empty.
- [ ] `bash scripts/lean.sh` green (citations intact).
