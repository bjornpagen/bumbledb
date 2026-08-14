# plan-007: `ground_program` / `grounded_program` / "the grounded program"

- **Severity:** low
- **Tree:** plan
- **Status:** DUPLICATE(engine-034)
- **Source:** audit/plan-exec.md F19

engine-034 owns `ground_program` → `ground_main` and the test helper `grounded_program`. `plan/ground.rs:402` ("Rule subsumption over the grounded program") and `plan/ground/tests.rs:688-691` are cited sites. Remaining `program` prose in plan tests is engine-011's sweep (plan-008).
