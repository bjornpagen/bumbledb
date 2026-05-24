# PRD 12: Scalar Free Join Executor

## Purpose

Implement the paper's recursive node/cover/probe Free Join executor in scalar form. This is the first PRD that executes formal Free Join plans instead of only validating them.

## Dependencies

- PRD 03.
- PRD 06.
- PRD 10.
- PRD 11.

## Scope

- Formal Free Join execution over GHT/COLT sources.
- Binding state that can bind multiple variables from one cover tuple.
- Probe key construction from current tuple plus prior bindings.
- Private result sink integration preserving set semantics.
- A sink/fold execution boundary that can receive complete typed bindings before final public materialization.
- No legacy LFTJ baseline remains; execution starts from the formal Free Join plan.
- Execution is driven from a `ReadTxn` over a real LMDB snapshot through PRD 09 base images and PRD 11 COLT sources.

## Required Execution Semantics

For each node:

- Select a cover. Static first-cover selection is acceptable in this PRD. Dynamic cover selection comes in PRD 13.
- Iterate cover tuples through the cover GHT/COLT source.
- Extend the current binding with all new variables in the cover tuple.
- For each non-cover subatom in the node, build an `EncodedTuple` key from available variables and newly bound cover variables.
- Probe the subatom source with `get(key)`.
- If any probe fails, skip the cover tuple.
- If all probes succeed, replace participating sources with returned child sources for the recursive call.
- Recurse to the next node.
- At the end, emit the binding to the set projection sink.
- The projection sink deduplicates projected facts and builds `QueryResultSet`.

## Technical Direction

- Use the formal PRD 03 plan as the execution input.
- Build one GHT/COLT source per atom occurrence using the atom's subatom partition sequence.
- Never bypass PRD 09/11 by reading an in-memory test store directly. Test fixtures must either use mock GHT sources for pure executor tests or real LMDB-backed base images for integration tests.
- Binding state must reject conflicting variable values.
- Static zero-variable atoms must be checked exactly once through existence semantics.
- Keep public output materialization through a duplicate-free projection sink unless PRD 17 has already replaced internals.
- Do not make `Vec<ResultFact>` construction the executor's only internal output path. The executor should recurse over bindings and call a private sink/consumer.
- The sink API may be minimal in this PRD, but it must be capable of seeing complete encoded variable bindings so later aggregation can fold over binding sets without changing Free Join recursion.
- Do not use leapfrog intersection inside this executor unless it is rebuilt as a formal singleton-plan fast path over GHT/COLT sources.
- Preserve exact product semantics internally but collapse projection to sets.

## Non-Goals

- Do not implement dynamic cover selection here.
- Do not implement vectorized batches here.
- Do not implement factorized output here.
- Do not revive the deleted LFTJ baseline here.
- Do not implement public aggregation or Logica aggregation syntax here.
- Do not add a non-LMDB execution data source for production.

## Acceptance Criteria

- Multi-variable cover node `[R(x, a), S(x)]` executes correctly.
- Paper clover binary Free Join plan executes correctly.
- Paper clover factorized plan executes correctly if PRD 06 produced it.
- Singleton Generic Join-style Free Join plan executes correctly.
- Chain binary-derived plan executes correctly.
- Self-join occurrence plans execute correctly.
- No valid query requires a predeclared physical index to execute.
- Projection remains duplicate-free and canonicalized.
- Invalid plan execution cannot bypass PRD 03 validation.
- Executor internals are sink-based: a test-only sink can observe complete bindings without requiring public aggregate APIs.
- The production sink remains `QueryResultSet` materialization.

## Required Tests

- Clover binary plan exact output.
- Clover factorized plan exact output.
- Triangle singleton plan exact output.
- Chain plan exact output.
- Star plan exact output.
- Self-join exact output.
- Static atom existence success and failure.
- Multi-variable cover conflict rejection.
- Free Join executor output equals reference evaluator for small hand-written queries.
- Test-only binding sink observes the expected full binding count for a duplicate-witness fixture while the public projection remains duplicate-free.

## Validation Commands

```text
cargo fmt --all --check
cargo test -p bumbledb-lmdb free_join_executor --all-features
cargo test --workspace --all-features
```
