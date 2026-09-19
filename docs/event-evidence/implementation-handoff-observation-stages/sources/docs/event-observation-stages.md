# Native observation stages

Rust queries can now bind a completed `Probability` or `Expectation` result in
an interior relation, then project, join, group, deduplicate, or antijoin that
observation in later stages. Its original source, evidence, payoff, exact value,
and parameter domain remain owned. No probability is converted to a Boolean,
floating-point scalar, or database field.

```rust
let query = bumbledb::query!(Game {
    interior beliefs(action, chance: Probability(success, evidence)) |
        Outcome(action, success, evidence);
    interior utilities(action, expected: Expectation(value, when, evidence)) |
        Payoff(action, value, when, evidence);
    (action, chance, expected) |
        beliefs(action, chance), utilities(action, expected), LegalAction(action);
});
```

Here `chance` and `expected` are query variables with observation types. The
expectation is calculated in `utilities`; projecting `expected` in the final
head does not perform another aggregate. Interior atom arguments use head
positions, as elsewhere in the Rust macro API. Sparse binding uses positions
such as `utilities(1: expected)`.

## Identity and operations

Two occurrences of the same observation variable join by observation identity.
Explicit `==` and `!=` between observations of the same kind have the same
meaning. Equal numerical answers are insufficient: `P(A | E)` and `P(B | E)`
remain different when their original Events differ, even if both equal `1/2`.
An expectation retains its evidence and exact function identity. For family
functions, identity means the encoded arithmetic presentation; checked numerical
equivalence remains a separate host operation.

Projection produces a set of completed values. Repeated groups with identical
expectations can collapse after the group key is projected away. Different
functions with equal means remain distinct. `Count` can count the relational
bindings associated with one observation; `Sum`, scalar arithmetic, numeric
ordering, Event operators, scalar literals and external scalar parameters do not
implicitly coerce an observation. Stored schema fields still use `ValueType`.

A union head keeps the existing agreement on projection versus producer/fold
roles. To union a newly produced observation with an existing one, materialize
the producer in its own interior, then union their projections. This also makes
the producer's admission boundary explicit.

## Representation and execution

`QueryType` distinguishes stored values from `ObservationKind::{Probability,
Expectation}`. `SignatureColumn::ProjectObservation` describes a completed
observation; the existing producer variants describe probability computation or
expectation collection. A projected expectation is not classified as a fold.

Derived images and scratch rows carry one execution-local identity word per
observation. `ObservationRegistry` retains the owned values and canonical keys;
that word is neither a numeric probability nor a serialized schema value.
The physical `ImageField` contract describes widths and text ownership without
inventing a stored integer/byte field for an observation. Both Free Join paths
consume the same typed column layout. Event fields retain their two-word identity
on the cursor path as well.

At a producer boundary:

1. The computed sink finishes complete operand/context admission.
2. Every expectation roster is admitted structurally, then contracted.
3. Probability pairs are contracted and completed observations receive identity
   words. The transformed rows enter the ordinary distinct/spill sink before
   publication, so canonicalized duplicates coalesce.
4. Only the successfully sealed stage becomes visible to consumers.

An already spilled producer keeps its transformed rows in scratch; the
conversion does not reconstruct a complete resident row set. Pure projections
of completed observations use the ordinary stage sealing path without another
contraction or transformation set. The registry itself retains owned observation
payloads in memory; this is not a claim that all retained memory is spillable.
The proposal's retained-memory policy remains open.

One exact arithmetic counter spans observation admission/contraction in every
interior and the final head. Stages do not reset that budget. Existing budgets
for Event expression evaluation and imported descriptor admission keep their
separate scopes. Cancellation, invalid coverage, missing laws and exhaustion
fail before any final answers publish. A downstream filter cannot hide a
producer's failure, including missing coverage on a possible zero-mass world.

Final answers copy the selected observations into independent ownership. Rebind,
registry reset, plan memory release, database close and scratch cleanup do not
change returned answers. Failed internal appends preserve their initialized
prefix and roll back newly copied observations.

## Evidence and remaining work

`event_observation_stages.rs` checks all 256 Event/evidence pairs on four worlds,
including a possible zero-mass world, on resident and forced cursor paths.
Other cases cover identity joins, explicit equality, antijoins, grouping,
expectations with equal means and different functions, mixed projected/new
observations, persistence/reopen, parameter holes, imported family payoffs,
producer errors, type refusals, repeat execution and changed bindings. Native
unit tests exercise forced spill, registry/output ownership, shared arithmetic
exhaustion, cancellation, and atomic append rollback.

`ObservationStages.lean` supplies eighteen reference reports for identity
encoding, joins/antijoins, projection composition, duplicate/union semantics,
group membership, source/evidence/function retention, partial values and
producer errors. Registry injectivity and decoding round trips are explicit
premises. These proofs do not establish Rust, allocator, codec, or compiler
refinement, nor a performance result.

Typed SDK authoring/imports still refuse observation interiors; widening its
query-variable domain is the next integration step. Native observation
arithmetic/comparison by numerical value remains open. This checkpoint does not
complete M5/M6 or any remaining M0–M8 obligation. No release, tag or version bump.
