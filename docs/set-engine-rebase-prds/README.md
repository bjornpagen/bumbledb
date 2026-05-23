# Set Engine Rebase PRD Suite

## Purpose

This directory is the implementation contract for finishing the Bumbledb set-engine rebase.

Bumbledb began with multiplicity-oriented assumptions in storage, execution, and benchmarks. The public surface now says facts and result sets, but parts of the physical engine still behave like a full-witness pipeline with late duplicate suppression. That is correct in many final-value cases, but it leaves correctness holes, unnecessary storage duplication, unnecessary query-image copying, and major performance on the table.

These PRDs define the remaining process as ordered, strict, implementation-grade work. Each PRD is intentionally small enough to be completed independently. If an implementation discovers a PRD is still too broad, split it before writing code.

## Core Priors

- A relation is a set of full facts.
- Exact duplicate insert is a successful no-op.
- Delete is exact fact deletion.
- Projection output is a set of result facts.
- Aggregate domains are explicit sets and must be validated before execution.
- Query execution should optimize for result-set work and explicit aggregate-domain work, not full witness completion.
- Free Join remains the target execution framework, with the current code implementing a sorted-leapfrog slice plus lazy access slices for selected atom shapes.
- LMDB remains the durable storage substrate.
- No migrations, compatibility readers, legacy aliases, or dual old/new storage readers are allowed.
- Breaking storage format bumps are required whenever on-disk layout changes.

## Research Basis

The Free Join paper gives the target direction:

- Generalize binary-style iteration/probe and WCOJ-style value intersections under one plan model.
- Use plan nodes that can bind any number of variables and involve any number of relations.
- Factor lookups earlier when their keys are already available.
- Use lazy trie construction, specifically COLT-style column-oriented lazy tries, to avoid eager builds.
- Vectorize iteration and probe batches.
- Choose covers based on build/probe/iterate cost, not one fixed algorithmic tradition.

Bumbledb must adapt those ideas to stricter set semantics:

- There are no derivation multiplicities to preserve.
- The desired output is usually a projected result set or explicit aggregate-domain set, not every full binding.
- Existential parts of a query should become semijoin/existence work once projected/domain variables are determined.
- Physical planning must measure witness work separately from result-set work.

## Execution Rules

- Complete PRDs in numeric order unless a PRD states it is independent.
- Do not start a dependent PRD until every prerequisite acceptance gate is green.
- Add failing regression tests before implementation whenever the PRD fixes a correctness bug.
- Do not hide old behavior behind configuration flags.
- Do not preserve old execution paths unless the PRD explicitly calls for a temporary comparison harness.
- Do not use final result deduplication as a substitute for a correct internal set pipeline.
- Each PRD must update docs, diagnostics, and tests when behavior changes.

## Global Validation Gate

Every PRD that changes Rust code must pass:

```text
cargo fmt --all --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check --manifest-path fuzz/Cargo.toml
```

Every PRD that changes query semantics must additionally pass:

```text
cargo test -p bumbledb-test-support --test golden_examples --all-features
cargo test -p bumbledb-test-support --test property_and_differential --all-features
cargo test -p bumbledb-test-support --test sqlite_comparison --all-features
```

Every PRD that changes benchmark SQL, correctness checks, or timing modes must prove exact value correctness before reporting timing numbers.

## Stale-Term Gate

After each PRD, run the repository's strict removed-concept grep gate over source and normative docs. The gate must have zero matches unless an individual PRD explicitly lists an allowed temporary code identifier exception.

## Ordered PRDs

3. `03-aggregate-domain-and-ir-hardening.md`
4. `04-cardinality-reference-and-benchmark-correctness.md`
5. `05-storage-transaction-and-schema-safety.md`
6. `06-canonical-fact-storage-single-owner.md`
7. `07-access-and-constraint-layout-rebase.md`
8. `08-query-image-minimal-compact.md`
9. `09-set-native-projection-execution.md`
10. `10-set-native-aggregate-execution.md`
11. `11-free-join-plan-rebase.md`
12. `12-free-join-factoring.md`
13. `13-lazy-ght-colt.md`
14. `14-vectorized-free-join.md`
15. `15-optimizer-cover-cost-free-join.md`
16. `16-cache-memory-observability-gates.md`

## Final Definition Of Done

- No known correctness bug from the audit remains open.
- Public IR cannot bypass builder invariants.
- Aggregate domains are proven valid before execution.
- Cardinality-only execution reports result-set cardinality exactly for every output shape.
- Storage has one durable owner for canonical fact bytes.
- Access structures store access keys and durable fact identity, not hidden fact copies.
- Query images are field-scoped, access-scoped, compact, and budgeted.
- Projection execution can stop after projected result facts are determined.
- Aggregate execution operates on explicit domains directly.
- Free Join plans are executable physical plans, not explanatory metadata.
- Free Join supports factoring, lazy GHT/COLT access, cover choice, and vectorized batches.
- Deleted auxiliary execution paths do not return.
- Optimizer cost distinguishes result-set work from witness work.
- Benchmarks cannot report timings when exact value correctness or cardinality parity fails.
- Caches have explicit budgets and diagnostics.
- The full global validation gate is green.
