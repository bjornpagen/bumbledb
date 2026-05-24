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
- Projection sink integration preserving set semantics.
- No legacy LFTJ baseline remains; execution starts from the formal Free Join plan.

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

## Technical Direction

- Use the formal PRD 03 plan as the execution input.
- Build one GHT/COLT source per atom occurrence using the atom's subatom partition sequence.
- Binding state must reject conflicting variable values.
- Static zero-variable atoms must be checked exactly once through existence semantics.
- Keep output materialization through the existing duplicate-free projection sink unless PRD 17 has already replaced internals.
- Do not use leapfrog intersection inside this executor unless it is rebuilt as a formal singleton-plan fast path over GHT/COLT sources.
- Preserve exact product semantics internally but collapse projection to sets.

## Non-Goals

- Do not implement dynamic cover selection here.
- Do not implement vectorized batches here.
- Do not implement factorized output here.
- Do not revive the deleted LFTJ baseline here.

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

## Validation Commands

```text
cargo fmt --all --check
cargo test -p bumbledb-lmdb free_join_executor --all-features
cargo test --workspace --all-features
```
