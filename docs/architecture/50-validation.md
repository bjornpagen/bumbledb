# 50 — Validation

The old repo's best asset was its correctness discipline; the worst was its gate
theater. We keep the former and refuse the latter.

## The oracle

**SQLite is the external correctness oracle.** Every benchmark and golden query is
executed against SQLite with `SELECT DISTINCT`, and Bumbledb's result set must equal
SQLite's **exactly, by value** — never by count (count-equality masked real bugs twice
in the old repo's history). No timing number means anything for a query that hasn't
passed exact comparison in the same run. SQLite appears only as an oracle; it is never
infrastructure.

## The primary benchmark: ledger

The ratchet benchmark mirrors the product thesis (schema from `00-product.md`'s
workload):

```
Holder, Account, Instrument, JournalEntry, Posting, PostingTag, Org, OrgParent
```

Query families: postings for a holder over a time range; postings for an account;
entries touching an account set; multi-hop joins across holders/accounts/postings/
instruments/entries; balance-style aggregates by account and instrument (when aggregate
execution lands); membership point-lookups via unique keys; a cyclic-ish join for WCOJ
honesty. Data generated at design scale (~10⁵–10⁷ facts), seeded and reproducible.

JOB or other stress suites may be *run* for curiosity; they never gate anything.

## Differential and property tests

- A tiny **in-memory reference engine** (naive nested loops + BTreeSets, correctness
  only) executes the same IR; randomized queries over randomized ledger-shaped data
  must agree with both the engine and SQLite. This is the bug-finder; it stays brutally
  simple so it is obviously correct.
- Operation-sequence property tests for the write path: random insert/delete
  interleavings with constraint checks, verifying set semantics (idempotence), guard
  consistency, and serial monotonicity across commits and aborts.
- Scalar vs vectorized execution equality on every fixture, across batch sizes
  (1, 2, 64, 256, partial final batches, empty sources).
- Image-cache invariants: identical instances across read txns at the same tx id;
  invalidation after every commit.

## The allocation gate

The one numeric gate this project keeps: a counting allocator asserts the
prepared-query zero-steady-state-allocation contract (`30-execution.md`) on
representative ledger queries. It exists because allocation regressions are silent,
gradual, and were the old repo's chronic disease. It is a boolean (zero or not), not a
budget file to ratchet.

## What we deliberately do not have

Line-count gates (they fragmented v5 into ~70 just-under-the-limit files). PRD-map
checks. Banned-identifier greps. Coverage percentages. Trace-field completeness gates.
Allocation *budget tables*. The standard toolchain — `cargo fmt`, `clippy -D warnings`,
`cargo test` — plus the oracle, the differential suite, and the allocation boolean is
the entire gate surface. A gate earns its place by catching a real bug class, not by
making progress measurable.
