# plan-005: `INTERIOR_PLANNING_ROWS` aliases `ACCUMULATED_PLANNING_ROWS`; "delta-variant" comment

- **Severity:** medium
- **Tree:** plan
- **Status:** DUPLICATE(engine-018)
- **Source:** audit/plan-exec.md F17

engine-018 owns the third-name floor alias and the delta/finished side channel. engine-007 owns the "delta-variant" vocabulary at `plan/selectivity.rs:89-94`. No separate fix lands under this id; floor values 1 and 16 stay locked there.
