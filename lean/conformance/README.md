# Executable conformance corpus

The JSON files in `cases/` connect independent Rust evaluators, the production
engine, and Lean's executable denotation. They are semantic test inputs and
expected results, not benchmark measurements.

The Rust serializer and comparator live in
`crates/bumbledb-bench/src/conformance.rs` and its submodules. Lean's
decoder and dispatcher live in `lean/Main.lean`, using
`Bumbledb/Conformance.lean`, `Bumbledb/Decide.lean`, and the query model.

## Case families

| Filename | Comparison |
|---|---|
| `judgment-*.json` | Apply a delta to a lawful parent and compare the final-state verdict. |
| `complete-*.json` | Judge a populated candidate without borrowing a lawful-parent premise. |
| `reach-*.json` | Evaluate the tagged query with derived stages and restricted recursion. |
| Other `*.json` | Compare query answers in the supported interchange subset. |

Query documents distinguish `cq` and `reach`. Values have explicit tags;
canonical floats use hexadecimal payload bits, integers preserve their full
range, and string values use a per-case dictionary. This is a test interchange
format, not the public SDK API or persistent store format. Inspect a nearby
case and its decoder when extending it rather than maintaining a second
copy of the grammar in prose.

Expected answers are compared as canonical sets. Judgment cases compare the
complete sorted violation set. The historical `phase` JSON field describes
the violations; it does not license skipping other statements.

## Run

From the repository root:

```sh
scripts/lean.sh
cargo test -p bumbledb-bench the_corpus_replays_byte_identical
cargo test -p bumbledb-bench three_way_conformance -- --ignored --nocapture
```

The first command includes Lean replay and the constructor census. The second
replays the Rust/file side. The third includes the three-way comparison and
requires `lake`. These are correctness checks, not timed performance lanes.

Intentional regeneration uses:

```sh
cargo test -p bumbledb-bench regenerate_the_conformance_corpus -- --ignored --nocapture
```

Review the coverage report and every changed expectation. Generation has a
wall-time exclusion, so do not assume a noisy machine necessarily regenerates
the identical selected case roster. Never regenerate solely to silence a
disagreement.

## Coverage boundaries

The Rust generator explicitly counts unresolved literals, engine errors,
slow cases, wide answer sets, computed heads, and values not expressible in
this interchange grammar. Its current slow-case budget is 25 ms and its
answer bound is 512 rows. UUID and dense float-interval values are excluded
from this serializer even though the engine supports them. A Lean value model
and an executable case decoder are different coverage boundaries.

Complete admission checks all statements on a candidate. Incremental
admission requires a parent already satisfying the theory
(`LawfulParent` in Rust). An unready populated store cannot use an empty
incremental delta as a substitute for complete validation.

This corpus does not qualify LMDB, S3, crash recovery, native resource
lifetimes, or host floating-point controls. See
[the bridge ledger](../proof-bridge-ledger.md) for the proof boundary and
[correspondence cases](../correspondence.md) for independent discriminators.
