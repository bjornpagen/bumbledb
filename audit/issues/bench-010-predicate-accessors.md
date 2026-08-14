# bench-010: `.predicate()` on every prepared-query consumer

- **Severity:** low
- **Tree:** bench
- **Status:** DUPLICATE(engine-041)
- **Source:** audit/bench.md F10

engine-041 already owns the mechanical rename `Predicate` → `Signature`, `predicate()` → `signature()`, "across `crates/bumbledb` and `crates/bumbledb-bench` (~20 bench call sites)." These are those sites: `driver/read_family.rs:116`, `verify/check.rs:46`, `closure.rs:371`, `closure/tests.rs:64`, `scenarios/run_query.rs:102`, `lanes/curves.rs:587,765,1555`, `displaced.rs:530,661`, `churn/probes.rs:230`, `crud/run.rs:449`, `calendar/tests.rs:242`, `sqlite_run.rs:46` (comment "mirroring the bumbledb query's predicate"), `sqlite_run/tests.rs:44,85`, `compare.rs:28`. No residual edit under this id.
