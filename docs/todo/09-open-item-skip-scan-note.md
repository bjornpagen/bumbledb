# 09 — Doc-only: skip-scan note on the range/stabbing-accelerator OPEN item

**Kind:** doc amendment only. No code. Purpose: when the OPEN item's trigger fires,
future-you reaches for a cursor discipline over existing structures instead of a
new index kind.

## Context

The range/stabbing-accelerator OPEN item (`docs/architecture/README.md` OPEN list;
`40-execution.md` access paths) records that time-range, point-membership, and
overlap scans are O(n) by decision, acceptable while the latency budget holds.
Separately, a query binding a **non-prefix subset** of an FD guard's fields cannot
use the `U` guard today and falls to the same O(n) image scan.

Postgres 18 shipped the relevant prior art: B-tree **skip scan** — serving a
multicolumn index when the leading column is unbound, by iterating distinct
leading-prefix values and range-seeking within each
(`postgres/postgres` `src/backend/access/nbtree/`, `skipsupport.h`). The cost
model: O(distinct-leading-prefixes × log n), a win whenever the leading column is
low-cardinality (enums, discriminators — exactly this workload's guard prefixes).

## The note to add

`U` guards are already ordered composite LMDB keys with fixed per-statement width
(`50-storage.md` key derivation; equal-width per statement is what makes range
semantics unambiguous — `storage/keys.rs`). A skip scan is therefore **zero new
structures**: an LMDB cursor `set_range` hop to the successor of the current
leading-prefix value, then a bounded scan within the prefix, repeated — the same
namespace the neighbor probe and coverage walk already traverse. Candidate
applicability: non-prefix guard lookups, and scalar range scans where the guard's
leading field is a low-cardinality enum. Not applicable to interval stabbing (the
pointwise layout puts the interval last precisely so the scalar prefix groups —
stabbing needs the coverage-walk shape, not skip).

Amend the OPEN item's text: add "candidate mechanism on trigger: guard skip scan
(cursor `set_range` prefix-hopping over existing `U` namespaces, O(distinct-prefix
× log n)); prior art Postgres 18 nbtree skip scan" — with the trigger unchanged
(latency-budget violation on a range/interval family; measured, never speculative).

## Acceptance

The OPEN item paragraph in `docs/architecture/README.md` (and the O(n) decision
paragraph in `40-execution.md`) carry the note. Nothing else changes.
