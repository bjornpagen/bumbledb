# Event: a database type

**Database-focused design revision, September 19, 2026. Not implemented.**
Baseline: `main` at `d76d31abca00226bc817a549d7365f1638f4b33c`.
The active branch changes documentation only.

An interval is a region of a timeline. An Event is a region of a caller-defined
set of possibilities. Store it in a field; constrain it with ordinary keys and
containments; compare it during Free Join; combine answers with `Pack`.

For Coup, “Bob has a Duke” remains the set of supplied deals where it is true.
Intersect it with another condition to retain deals where both are true in the
same world. Nothing in this type discovers legal deals or assigns probabilities.

## Read the proposal

| Document | Owns these decisions |
| --- | --- |
| [Type and dependencies](proposal.md) | Mathematical value, universe identity, schema syntax and admission |
| [Storage and ownership](storage.md) | Canonical bytes, resident layout, transactions, caches and compatibility |
| [Queries](queries.md) | Total pair predicates, Free Join, expressions, Pack, cancellation and optional closure |
| [Coup](coup.md) | Proposed Rust schema and complete staged examples |
| [Implementation plan](implementation-plan.md) | Ordered work, acceptance tests and stopping point |
| [Decision register](decisions.md) | Resolved questions, remaining implementation probes and deferred work |
| [Source audit](source-audit.md) | Current-code evidence and disposition of historical research |

The original [proposal and research](reference/README.md) remain unedited,
non-normative references. Their milestone lists do not apply to this revision.

## Changes from the first restart draft

The code audit exposed several important gaps:

- Canonical rows already own variable-length values inline. Event will follow
  that path; only query representations intern it. No persistent Event catalog.
- A pair predicate must be safe to move earlier in a join. Classification now
  has fifteen same-universe cases plus `DIFFERENT_UNIVERSE`, a total sixteenth
  case. Universe mismatch is not a data error in a pair filter.
- `WorkContext` supplies cooperative cancellation, not byte or time quotas.
  The proposal no longer promises budgets that the engine does not provide.
- Empty Events require different key-uniqueness reasoning from nonempty intervals.
- A type reaches beyond a macro: canonical rows, snapshots, change sets, query
  parameters, sealed results, spill, generated bindings and log replay all count.

The first carrier remains a dense bitmap with a complement bit. Its finite
scope is explicit. A future representation change must preserve the database
contract; symbolic diagrams are not a prerequisite for this implementation.

The completion target is the Coup queries passing through those normal database
paths. Optional indexing, SIMD, join factoring and graph closure have separate
acceptance criteria. None authorizes a general solver, probability subsystem,
strategy engine, or another open-ended research program.
