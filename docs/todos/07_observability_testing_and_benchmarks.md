# 07: Observability, Testing, And Benchmarks

**Goal**
- Make the engine inspectable and build the test and benchmark harness that defines whether the product thesis is true.

**Why This Stage Exists**
- A join-focused database without explain plans is not debuggable.
- Benchmarks need to guide optimization before speculative complexity creeps in.

**Concrete Work**
- Render explain plans for typed variables, atoms, chosen indexes, variable order, estimates, and actual counters.
- Expose storage diagnostics: schema fingerprint, transaction ID, relation row counts, index entry counts, dictionary size, and LMDB map usage if available.
- Add an in-memory reference evaluator for the supported positive Datalog subset.
- Add differential tests comparing LMDB execution to the reference evaluator.
- Add golden tests for parser, typechecker, planner, and explain output.
- Add property-style tests for encoding order, index consistency, and query equivalence where practical.
- Add the benchmark schema from the Rosetta Stone.
- Add benchmark data generation for realistic ledger-shaped data.
- Add SQLite comparison with good indexes.
- Add Postgres comparison if local setup is practical and not too costly.
- Record benchmark results in a repeatable format.

**Out Of Scope**
- Major optimizer rewrites driven by one benchmark result.
- Production dashboarding.
- Networked observability.
- Automatic benchmark publishing.
- Supporting arbitrary user benchmark schemas.

**Passing Criteria**
- Every executed query can produce an explain plan.
- Explain shows chosen variable order and index per atom.
- Explain includes actual cursor seek, scan, yield, filter, and aggregate counters where available.
- Storage diagnostics can be queried without unsafe or raw LMDB access.
- Differential tests pass for representative positive Datalog queries.
- Benchmark schema can be loaded reproducibly.
- Benchmark queries run against Bumbledb and SQLite.
- Benchmark output includes enough context to compare warm read, write batch, and join-heavy query behavior.
- At least one multiway join benchmark demonstrates the intended advantage or exposes a specific planner/executor deficiency.

**Notes**
- Benchmarks are not marketing at this stage; they are design instruments.
- If SQLite wins a target query, inspect the explain output before changing architecture.
- Do not tune for benchmark-only shapes that violate the product thesis.
