# Lean specification

Lean models the database's value, schema, query, and admission semantics.
It does not verify the Rust implementation. Differential tests connect the
executable model to independent Rust evaluators and the production engine.

## Build and check

The toolchain is pinned in `lean-toolchain` to Lean 4.32.0. From the repository
root, run:

```sh
scripts/lean.sh
```

This builds the tree, checks for proof placeholders and axiom declarations,
evaluates the checked-in conformance cases, and runs the Rust-constructor
correspondence census. The workspace tests own the Rust oracle comparisons;
the Lean script does not run those tests a second time.

## Model map

| Modules under `Bumbledb/` | Subject |
|---|---|
| `Values`, `Float64`, `Float64/Order`, `FloatInterval` | Structural values, canonical floats, ordering, and intervals. |
| `Schema`, `Capacity`, `Dependencies`, `Subsumption` | Relational constraints and admissible forms. |
| `Query/` | Query syntax, denotation, membership, aggregates, and stages. |
| `Exec/` | Abstract planning, rewrites, deduplication, sweeps, and restricted recursion. |
| `Float64/Sum` | Exact accumulation and once-rounded sum/mean model. |
| `Txn`, `Txn/DeltaRestriction`, `Txn/Support`, `Decide` | Complete and incremental final-state judgment and their premises. |
| `Oracle`, `Admission` | Abstract consultation bounds and accepted enforcement forms. |
| `Countermodels` | Cases that refute stronger or unsupported claims. |
| `Bridge` | Theorems and the Rust constructors responsible for their premises. |
| `Conformance`, `Float64/Conformance` | Executable interchange and comparison. |

`Bumbledb.lean` imports the model. The tree uses core Lean rather than mathlib.
Keep implementation details—LMDB pages, SIMD, batching, caches, memory budgets,
and measured latency—in Rust and benchmark documentation, not the denotation.
Abstract algorithmic cost is distinct from measured machine performance.

## Scope of the evidence

Read [the conformance guide](conformance/README.md) for the executable subset,
[correspondence cases](correspondence.md) for independent expected results,
and [the proof bridge](proof-bridge-ledger.md) for premises and remaining gaps.
Passing those checks is evidence for their covered cases, not a proof of the
whole database.

Lean does not model LMDB durability, S3 conditional writes, crash recovery,
native resource lifetimes, or the host floating-point environment. Hashes are
also an implementation concern: logical identity uses canonical values, not
a theorem that distinct values cannot collide under a finite hash.

Application identity is a structural UUID value. There is no database ID
minting machine. Set-level commutation does not establish concurrent admission
or log publication safety; the history implementation has its own authority
protocol and independent tests.

Semantic changes must update the applicable model and correspondence cases.
Do not change expected results merely to hide a disagreement.
