# engine-037: `Query::single` is the RIGHT coordinate — recorded so nobody "fixes" it

- **Severity:** low
- **Tree:** engine
- **Status:** WONTFIX (non-violation recorded by the audit itself)
- **Source:** audit/engine.md F37

`Query::single` (`ir.rs:450-478` — empty interiors, no rec, one rule) is Dijkstra's half-open interval done correctly: the CQ is the empty prefix, not a different type. The audit filed this row explicitly so the constructor is NOT "fixed" into a separate plain-query type (which would reintroduce the third arm CONTRACT §C1/§C4 rejects). The branches that special-case around it (`interiors.is_empty() && matches!` fast lanes) are charged to engine-001 and engine-008/031, where they are fixed. No edit under this id.
