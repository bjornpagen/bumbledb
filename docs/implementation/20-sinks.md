# PRD 20 — Sinks: Projection and Aggregation

Authority: `docs/architecture/30-execution.md` (set semantics in the executor; D2
skip legality; D3 sinks), `20-query-ir.md` (aggregation semantics — normative).

## Purpose

The two consumers of bindings: set-projection with dedup and subtree-skip, and
aggregate folds with binding dedup.

## Technical direction

- `exec::sink`. Both sinks arena-backed, built from the `ValidatedPlan` + find
  descriptors.
- **Projection sink**: projects find-var slots to a key tuple; open-addressed seen-set
  (reuse PRD 18's map machinery over inline word tuples; String/Bytes find values are
  id-words here — decode is PRD 25). `emit` returns `SkipSuffix` when the projected
  tuple was already present (D2: legal for this sink only — the suffix bound nothing
  projection-relevant by the plan's precomputed bits; the executor enforces the
  bits, the sink just reports staleness).
- **Aggregate sink**: group map keyed by group-key var words → accumulator row
  (`i128`/`u128` for Sum per doc; u64 count; u64 min/max words compared in word order
  — correct because words are order-preserving, PRD 10). **Binding dedup**: unless the
  plan's provably-distinct-bindings flag is set, a seen-set over the full binding
  tuple gates every fold (fold only first occurrences). Always returns
  `Flow::Continue` (the skip is illegal here — D2).
- Finalization: `ProjectionSink::into_rows()` yields the distinct tuples;
  `AggregateSink::into_rows()` yields (group key words, finalized aggregate values)
  with Sum range-checked into its result type — overflow yields the typed `Overflow`
  error at finalization (deterministic by construction: i128 cannot overflow summing
  <2⁶⁴ i64 terms; the check is once, at the end). Empty input → zero rows (the
  empty-set rule for global aggregates falls out).

## Non-goals

Result decoding/buffers (PRD 25). SQL-oracle comparability (human-owned).

## Passing criteria

- Unit tests (drive via PRD 19's executor on fixtures): duplicate-witness projection
  emits each distinct tuple once and skip-counters (test Counters) show suffix skips
  occurred on a constructed high-multiplicity existential fixture; Sum over two equal
  amounts with serials bound = their sum (distinct bindings), and with serials
  *unbound* = one amount (set semantics — assert both, they document the footgun);
  the multiplicity footgun: joining a 3-tag relation triples an unprotected... — no:
  with binding dedup the 3 tag-vars are bound and distinct so Sum triples — assert
  exactly that documented behavior; distinct-flag elision path produces identical
  results to the seen-set path on a unique-bound fixture; global aggregate over empty
  input yields zero rows; Sum near i64::MAX in adversarial orders is
  order-independent (shuffle input, same result/error); Min/Max on I64 words honor
  logical order across the sign boundary.
- Global commands green.
