# 02 Golden Examples And Correctness Contract

## Purpose

Make golden examples the permanent non-regression core before implementation churn begins. The rewrite is allowed to break APIs and storage, but it must preserve the domain examples and make their expected behavior richer and more exact.

## Golden Example Requirements

Each golden example must include:

- Schema definition.
- Seed data.
- Insert/delete mutation sequence.
- Projection queries with exact expected result sets.
- Aggregate queries with exact expected values.
- Duplicate witness traps.
- Duplicate insert and absent delete cases.
- FK/unique/restrict cases where relevant.

## Required Example Families

Ledger:

- Holders, accounts, instruments, journal entries, postings, posting tags.
- Balance by instrument with explicit posting domain.
- Time range posting lookup.
- Tag lookup where duplicate tag witnesses cannot multiply projection.
- Delete restriction for holder/account/posting dependencies.

Sailors:

- Sailors, boats, reserves.
- Red boat sailors where many reserve facts project to one sailor.
- High-rating red boats.
- Exact reserve delete and absent reserve delete.

Joinstress:

- Chain4 lookup.
- Triangle projection and triangle domain count.
- Count trap where edge fanout product differs from distinct projected values.

TPC-H subset:

- Customer, supplier, part, orders, lineitem.
- Revenue by customer using explicit lineitem domain.
- Supplier-nation orders projection.

IMDb/JOB:

- JOB q09/q16/q24/q33 behavior over limited dataset.
- Static empty proof examples.
- Count-domain examples with expected scalar values.

Lahman:

- Compound player/team/year facts.
- Salary and batting joins with explicit domain checks.

LDBC:

- Person, post, knows, likes.
- Two-hop knows projection where multiple first-hop witnesses cannot duplicate output.

## Required Code Changes

- Create a `golden` test-support module or equivalent fixture layer.
- Replace length-only SQLite comparisons with exact typed row comparisons.
- Add exact expected result files or inline expected rows for each golden query.
- Add mutation-sequence checks, not just bulk-load checks.
- Make golden examples runnable without external full datasets, except optional JOB dataset gates.

## Acceptance Gates

- Every golden example has at least one duplicate-witness projection test.
- Every aggregate golden query declares and validates its domain.
- Every golden example passes after every later PRD.
- Benchmark suites cannot report performance if golden correctness for the same query failed.
- SQLite comparisons use `SELECT DISTINCT` or domain-correct aggregate subqueries and compare values, not just row counts.

## Non-Goals

- No broad random generation in this PRD beyond deterministic golden examples.
- No benchmark performance gates in this PRD.
