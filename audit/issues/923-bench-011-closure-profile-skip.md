# bench-011: closure `exec: None` because "the profile path is query-shaped"

- **Severity:** low
- **Tree:** bench
- **Status:** DUPLICATE(engine-011)
- **Source:** audit/bench.md F11

engine-011 already names `bumbledb-bench/src/closure.rs:502` — "the profile path is query-shaped; rec queries skip it." engine-008 owns the profile-path CODE change. The comment is the skip's excuse, not a second defect. No residual edit under this id. (bench-007 is the leftover delta-variant / "one program" prose around the same file, excluding this line.)
