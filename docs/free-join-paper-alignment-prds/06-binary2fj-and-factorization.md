# PRD 06: Binary2FJ And Factorization

## Purpose

Implement the paper's binary-plan-to-Free-Join conversion and conservative factorization as pure physical plan rewrites. This is the first PRD that constructs real paper Free Join plans from binary plan input.

## Dependencies

- PRD 03.
- PRD 05.

## Scope

- Binary plan to Free Join conversion.
- Conservative factorization rewrite.
- Plan golden tests using paper examples.
- Explain/debug representation for pre-factor and post-factor plans.

## Required Algorithm: `binary2fj`

For a left-deep binary plan `[R1, R2, ..., Rm]`:

- Start with node `phi` containing the full atom subatom for `R1`.
- For each next relation `S` in order, add a probe subatom `S(S.vars intersect available_vars(phi))` to the current node.
- Push the current node.
- Start a new node with `S(S.vars minus available_vars(phi))`.
- Continue until all relations are consumed.
- Push the final node.
- Validate the resulting Free Join plan with PRD 03 validation.

The exact representation may differ, but it must produce the paper-equivalent plan shapes.

## Required Algorithm: Conservative Factorization

Traverse nodes in reverse order. For each node `phi_i`, try to move subatoms into `phi_{i-1}` only when all are true:

- The subatom variables are a subset of variables available before `phi_i`.
- The previous node does not already contain the same atom occurrence.
- All earlier subatoms in the current node that would preserve lookup order have also moved, matching the paper's conservative stop behavior.
- The rewritten plan validates after the move.

Record attempted moves, rejected moves, and successful moves for explain and tests.

## Technical Direction

- Implement conversion over atom occurrence IDs, not relation names.
- Preserve subatom variable order from the atom occurrence field order unless a later PRD explicitly changes tuple-key order.
- Use an internal `PlanRewriteTrace` with before/after snapshots for tests.
- Keep rewrites pure. They must not inspect LMDB, query images, COLT state, or runtime statistics.
- If a relation contributes no remaining variables to a new node, represent that as a static zero-variable subatom only if PRD 03 validator supports it; otherwise define and reject that shape clearly.

## Non-Goals

- Do not choose the starting binary plan beyond using PRD 05 output.
- Do not execute Free Join plans here.
- Do not implement dynamic cover choice here.

## Acceptance Criteria

- `binary2fj` emits the exact clover, chain, triangle, and star plan shapes expected from the paper examples after normalizing occurrence IDs.
- Factorization transforms clover binary Free Join into the paper's factorized Free Join plan.
- Factorization preserves plan validity after every move.
- Factorization refuses moves with unavailable variables.
- Factorization refuses moves into a node already containing the same atom occurrence.
- Factorization preserves conservative lookup-order behavior and stops when an earlier subatom cannot move.
- Pre-factor and post-factor plans are printable for debugging and future explain output.

## Required Tests

- Golden `binary2fj` for clover.
- Golden `binary2fj` for chain query.
- Golden `binary2fj` for triangle.
- Golden `binary2fj` for self-join with occurrence IDs.
- Golden factorized clover.
- Factorization no-op when variables are unavailable.
- Factorization no-op when previous node has same atom occurrence.
- Factorization conservative stop case.
- Every generated and rewritten plan validates under PRD 03.

## Validation Commands

```text
cargo fmt --all --check
cargo test -p bumbledb-lmdb binary2fj --all-features
cargo test -p bumbledb-lmdb factor --all-features
cargo test --workspace --all-features
```
