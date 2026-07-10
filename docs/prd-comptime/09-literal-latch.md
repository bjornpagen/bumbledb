# PRD 09 — The literal latch: monotone resolution

**Depends on:** baseline only (independent; coordinates with 07's
plan-constant sets if both have landed — see direction 4).
**Modules:** `api/prepared/bind.rs` (`resolve_predicates`, `PendingIntern`),
`api/prepared/` (the prepared query's resolution state), `obs.rs` (one
counter name).
**Authority:** the staging audit item 3; `40-execution.md` (allocation
contract — the latch must not disturb the warm-path gates).
**Representation move:** the dictionary is append-only, so literal
resolution is **monotone**: a hit is a hit forever, a miss may become a hit
but never the reverse. Monotone facts deserve a one-way latch, not a
per-execution recomputation — the audit found `resolve_predicates` re-doing
dictionary lookups for `PendingIntern` literals and re-copying resolved
constants on every execution of even param-free queries.

## Context (decided shape)

- **Per-literal latch:** each `PendingIntern { text }` in the prepared
  query's templates gains a latched state: `Unresolved | Latched(word)`.
  On resolution success (dict hit), the template slot itself is overwritten
  with `Const::Word(word)` — the latch IS the rewrite, no parallel state
  table (weaker-model note: mutate the template in place exactly as
  `resolve_predicates` already writes `resolved_*` slots; the difference is
  the write happens once, into the template, not per-execution into the
  resolved copy).
- **Misses stay live:** an unresolved literal re-checks each execution
  (something may have interned it since) and continues to produce the
  short-circuit-to-empty behavior on miss — unchanged semantics, verbatim.
- **The fully-latched fast path:** the prepared query tracks
  `unresolved_literals: u32` (decremented on latch). When zero AND the query
  has no params of any shape, `resolve_predicates` is skipped entirely — the
  resolved tables were written once and are final (they already survive
  across executions in the pooled slots; the skip is the point). One branch
  at `execute` entry, cold.
- **Generation-independence argued once, in the doc amendment:** a latched
  word is valid for the LIFETIME of the environment (ids never reused,
  dictionary never shrinks — the accepted-leak axiom's second dividend, the
  first being the words-on-access analysis of 2026-07-09). The env-instance
  guard already prevents cross-environment reuse of the prepared query.
- **Interaction with the allocation contract:** latching writes fixed-size
  words into existing slots — no allocation; the fast path only removes
  work. The alloc-gate scenarios extend to cover a PendingIntern query
  crossing its latch (first execution resolves, second executes the fast
  path, both inside the measured window's rules as first-execution
  sanctioned vs warm).

## Technical direction

1. `bind.rs`/`resolve_predicates`: on `Const::PendingIntern` hit, write the
   resolved `Const::Word` back into the TEMPLATE (the plan-owned filter/
   selection array), decrement the counter; on miss, behavior unchanged.
   Audit every template consumer to confirm templates are prepared-query-
   owned and never shared across environments (they are — the prepared
   query is `!Sync` and env-guarded; cite in code comment).
2. The counter + the skip branch in `execute`'s resolve phase; `obs` gains
   `LITERAL_LATCH` (fires once per latch — the trace count equals the
   distinct-literal count, the `DICT_RESOLVE` precedent).
3. If PRD 07 is landed: plan-constant membership sets (the fold's output)
   count as pre-resolved from birth — they never increment
   `unresolved_literals`. If 07 is not yet landed, nothing here references
   it (no coordination code — the counter simply never sees them later
   because 07 emits resolved constants).

## Passing criteria

- `[test]` A query with one `str` literal: first execution resolves and
  latches (obs counter fires once); subsequent executions never call
  `dict::lookup` for it (counter stays; assert via the trace or a
  test-only hook on the lookup path).
- `[test]` Miss-then-intern: execute (empty result, miss), commit a fact
  interning the text, execute again — the literal latches now and results
  appear; a third execution takes the fast path (param-free fixture).
- `[test]` The fully-latched + param-free fast path: `resolve_predicates`
  provably skipped (obs span absent) with results identical to the slow
  path on the same snapshot.
- `[shape]` No new allocation in the latch path (the alloc gate's scenario
  extension passes); no parallel resolution-state table exists (grep: the
  latch writes into the template arrays).
- `[gate]` Workspace gates green at campaign close.

## Doc amendments (rule 5)

`40-execution.md`: the measured-mechanisms list gains the latch with its
monotonicity argument (append-only dictionary ⇒ one-way resolution), and
the finalize-memo paragraph cross-references it.
