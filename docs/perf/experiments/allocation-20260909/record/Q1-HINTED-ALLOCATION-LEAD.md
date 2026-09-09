# Q1 next experiment: allocate result/group maps from actual insertions

Status: implemented in isolated q1-source; correctness gates and matched saved
allocation audit pass. **Not performance-accepted.** Q1-DEMAND-GROWTH-REVIEW.md
is the current implementation/measurement authority; Q1-OWNERSHIP-REVIEW.md
preserves the baseline discovery. Not a combination of P2/P3/M1/E1/G1/G2.
The design and remaining measurement obligations below still apply.

Q1-BROADER-CONTROLS.md now completes the wider allocation obligations and
documents additional first-use requests/bytes for large results. Matched
ordinary timing, including preparation and generation-rollover tails, remains.

## Mechanism and minimal design

Remove speculative join-cardinality hints from result/group hash allocation.
Use the existing ordinary WordMap::new plus its existing insertion-driven
geometric growth, with capacity retained by clear/reset. There is already one
growth implementation; no allocator policy, quota, fallback, RSS promise or
public Rust/TypeScript SDK/persisted-layout change is warranted.

Do not replace 2^21 with another unexplained cap, defer the same huge reserve
until first emit, infer output distinctness from a join estimate, or shrink
after every query. Preserve the actual mathematical distinctness witnesses.
Keep planner estimates for planning; delete only their allocation-hint route.

Affected ordinary route to audit/change together:

- prepared/build.rs: main, interior and recursive output_hint/free_join_hint
  plumbing; make_sink/make_plain_sink; computed adapters.
- ProjectionSink construction: remove the hint argument and name the actual
  constructor meaningfully. Avoid an ignored `_hint` compatibility façade.
- SpillSet construction: initialize its one WordMap empty. Applies uniformly
  to projection, recursive result history and all aggregate dedup regimes.
- Aggregate construction: remove speculative hints for binding/union/DNF seen
  sets and hashed group tables. Exact validated dense-group radix tables are
  a different representation/proof and must remain unchanged.
- WordMap::with_capacity_hint: after production callers are gone, either keep
  the explicit-capacity fixture constructor test-only or refactor those tests
  to establish their required table geometry. Do not silently destroy tests
  for stale generations, saturated duplicates, or collision geometry.

Small five-key arity-one map geometry under the existing 1/3 load rule is
16 slots: 23 control + 128 key + 16 stamp + 32 dense capacity = 199 bytes.
That began as an arithmetic discriminator; q1-audit-1 now measures the same
199-byte capacity for every saved nonempty triangle execution.
The empty constructor should own zero table payload, while other prepared
query metadata and exact dense-group tables may still allocate normally.

## Correctness gates before accepting any speed or memory result

First preserve a failing baseline discriminator for zero speculative table
backing before execution, including a very large estimate and an empty result.
Avoid turning the memory regression into a brittle whole-prepare byte quota.
The measured owner is the result/group map, not all planner/schema allocations.

Retain/check all of these:

- Empty/unexecuted query, zero-arity truth tuple and key widths 1/2/4/8/dynamic.
- Scalar/bulk first insertion, repeated duplicates at growth boundary, exact
  keys under collisions, insertion order and iter_since across growth.
- Multiple union arms, DNF rekeying, aggregate binding distinctness versus
  mathematical proof elision, Pack and computed-output adapters.
- Hashed groups versus exact dense radix tables; empty global aggregate.
- Recursive/interior set history and deltas; different outputs across rounds.
- Reset, generation rollover, explicit release then refill, changed parameters,
  post-commit rebuild, cancellation/sticky errors and healthy retry.
- Actual forced-spill tests and sealed/cursor result lifetimes, even though
  production has no new memory-triggered spill policy.
- Full ordinary/allocation-enabled engine tests, strict lint, ordinary build
  and existing independent 2,879-case oracle; preserve any fixture failures.

Existing WordMap::entry grows from capacity zero; contains_key/iteration/clear
already handle emptiness. Growth allocates replacements before mutating old
arrays, and publishes entries after initialization/dense insertion. Preserve
those invariants rather than inventing another lazy state machine.

## Measurement obligations

Re-use saved inputs and q1 owner observers, including preparation counters.
Report prepare + first execution separately AND jointly: insertion-driven
growth moves some requests into cold execution. A win cannot be claimed from
the preparation window alone. Report requested/freed bytes, request counts,
all result/group/position owners and total retained capacity without omission.
Include growing high-cardinality results to price extra reallocations/rehash,
not just the five-row case. Controls should include proven-distinct range
(currently allocates then drops its speculative map), point, union/recursive,
hashed aggregates and exact dense aggregates on their existing saved fixtures.

Ordinary timings must preserve cold, second and warmed modes, all raw tails,
baseline controls and clock flags. Smaller tables change probes and generation
clear behavior, so unchanged insertion code does NOT prove warm speed parity.
No repeat-until-green; do not combine isolated candidate savings. No full
trace/benchmark, commit, push, release/tag/publication or version bump.
These Mac capacity measurements still do not qualify the 512 MB Pi.
