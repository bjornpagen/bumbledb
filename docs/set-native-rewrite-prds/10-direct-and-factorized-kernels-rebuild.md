# 10 Direct And Factorized Kernels Rebuild

## Purpose

Rebuild direct and factorized fast paths only after set semantics are explicit. Existing product-of-fanout count kernels are not generally valid for domain aggregates.

## Required Policy

Every fast path must prove one of these:

- It emits exact result-set tuples.
- It computes exact aggregate domain cardinality.
- It computes exact aggregate domain sum/min/max inputs.
- It is only a non-semantic performance mode and cannot feed correctness output.

## Paths To Delete Or Quarantine First

- `try_execute_factorized_count`
- literal/range factorized counts
- bridge factorized counts
- precomputed driver count cache
- LFTJ suffix `count_current_suffix` when it means witness multiplicity
- prepared result count cache that stores scalar counts without domain proof

## Rebuilt Paths

Direct projection:

- Enumerate projected key domains where an access path supports them.
- Prove existential extensions using access exists/cardinality.

Direct domain count:

- Count distinct domain keys from access subtree cardinality only when access key exactly matches domain.

Factorized domain aggregate:

- Enumerate the smallest exact domain set.
- Probe existence of required extensions.
- Never multiply fanouts unless the aggregate domain is explicitly the product domain.

## Acceptance Gates

- Every direct/factorized kernel has a semantic proof comment or validator.
- No product-of-prefix-count kernel is used for `count_distinct` or domain count unless the domain is that product.
- Prepared result cache is snapshot/input/domain scoped.
- Direct/factorized/LFTJ outputs match on all golden examples.

## Tests Required

- Fanout product differs from distinct domain count test.
- Same query forced through generic LFTJ and direct path returns identical values.
- Prepared cache invalidates after write.
- Prepared cache differs for two input bindings.
- Static empty and direct count interactions preserve zero-row behavior.

## Non-Goals

- No performance win is accepted without semantic equivalence tests.
