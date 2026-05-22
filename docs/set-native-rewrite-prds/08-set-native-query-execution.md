# 08 Set Native Query Execution

## Purpose

Rewrite query execution so projection and existence are set-native. The executor should not enumerate all hidden witness bindings when only existence of an extension matters.

## Current Bad Shape

Projection executes as:

```text
append projected row
sort/dedup at sink finish
```

This is correct but bag-shaped and expensive.

## New Execution Model

The executor classifies query variables:

- projected variables
- aggregate domain variables
- aggregate measured variables
- predicate-only variables
- existential variables

Projection queries should execute as:

```text
prove at least one extension exists
emit projected tuple once
```

Existential proof should use semijoin/access existence, not witness materialization.

## Required Code Changes

- Add variable-role analysis during normalization/planning.
- Replace projection sink with a `ResultSetSink` that receives already-distinct projected tuples where possible.
- Keep a defensive dedup boundary in debug/tests, but do not rely on it for normal execution.
- Rebuild LFTJ planning to use payload demand and existential-only relations.
- Remove `bindings_yielded` as primary semantic counter; split candidate domains, existence probes, and result tuples.

## Acceptance Gates

- Duplicate hidden witnesses do not increase projection work beyond explicit existence proof needs.
- Projection result set does not require sorting all witness emissions to be correct.
- `red_boat_sailors` duplicate projected rows counter disappears or becomes zero in normal path.
- LFTJ remains the backbone for cyclic/multiway joins.
- Direct paths and LFTJ agree on all golden projection queries.

## Tests Required

- Projection with one projected var and many existential witnesses emits once.
- Projection with two projected vars and many existential witnesses emits once per pair.
- Cyclic triangle projection still correct.
- Static empty proof still short-circuits.
- Direct storage project and LFTJ project produce identical result sets.

## Non-Goals

- No SQL `DISTINCT` compatibility layer.
- No hash-probe resurrection.
