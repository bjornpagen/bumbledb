# Exact finite revisions and expectations

The source API now materializes three distinct revisions of a fixed rational
law. Each retains its prior and mathematical inputs, returns an explicit result,
and preserves every original legal world. Old Event keys remain immutable.
These are host operations on the owned Event algebra; they add no schema weights
or parameterized field types.

| Operation | Input meaning | Resulting density |
| --- | --- | --- |
| `space.condition(event, ...)` | This named event was observed | `mu(w) * 1_event(w) / mu(event)` |
| `space.likelihood(function, ...)` | An explicitly interpreted nonnegative factor | `mu(w) * L(w) / sum mu*L` |
| `space.jeffrey(partition, targets, ...)` | Replace this partition's masses, preserving within-cell conditionals where defined | `q_i * mu(w) / mu(C_i)` inside cell i |

A probability observation remains read-only. Materializing a revision produces a
new measured space and an identity-on-worlds translation. Pullback through
`revised.translation()` translates prior Events into that space. The map runs
from posterior to prior, so old Events are its pullback inputs. It is structurally
bijective and does not claim to preserve the prior law. Alignment alone refuses
changed laws, including for full and empty Events. If the final law is unchanged,
its canonical identity is unchanged too; revision history is separate metadata.

```rust
let update = prior.condition(&tax, functions, laws, &mut arithmetic)?;
match update.outcome() {
    RevisionOutcome::Revised(revised) => {
        let duke = revised.translation().pullback(&old_duke, &control)?;
        let posterior_mass = duke.mass(&mut arithmetic)?;
    }
    RevisionOutcome::Impossible(reason) => {
        // update.prior() and update.receipt() still retain the original inputs.
    }
}
```

The receipt records evidence and its actual prior mass for conditioning, or the
supplied likelihood function and its normalizer for reweighting. A likelihood
factor may exceed one; its normalizer is not inherently an observation
probability. Scaling every factor by a positive constant preserves the posterior
and changes the receipt. Fixed factors compose by multiplication and commute
where defined. Their joint observation interpretation is the caller's obligation.
Reusing an actual observation does not license multiplying its likelihood again;
reusing its Event uses idempotent intersection.

A Jeffrey receipt retains the ordered full partition, every old cell mass and
every target. Targets must be nonnegative and sum exactly to one. A positive
target needs positive old mass. A zero target contributes zero without evaluating
an undefined conditional. All offending positive-target positions are returned
as an impossible outcome, including empty and nonempty zero-mass cells. Jeffrey
revision has no intrinsic evidence probability. Repeating the same targets on the
same partition is idempotent; revisions on overlapping partitions need not commute.

For prior `P(A)=1/5`, replacement targets `(4/5,1/5)` give `P_new(A)=4/5`.
Using those same numbers as likelihood factors instead gives `P_new(A)=1/2`.
A model's posterior-style answer cannot silently take the likelihood path.

`ZeroEvidence`, `ZeroLikelihood` and `UnsupportedTargets` are owned mathematical
outcomes. Missing laws, malformed targets, scope mismatches and resource limits
are errors. Neither path fabricates an empty world space or repairs probabilities.
Zero posterior mass leaves structural possibility unchanged; explicit restriction
is still a separate operation. Receipts retain the prior even after external
owners are dropped. Provider request IDs, actual observation IDs, raw responses
and import assumptions remain application metadata and adapter obligations.

## Signed expectations

`FiniteFunction::expectation(given, arithmetic)` returns an owned
`ExpectationObservation` retaining the function, evidence Event, numerator and
evidence mass. `value()` returns an exact signed rational or `None` for zero-mass
evidence. The result can be negative or exceed one; it is neither a probability
nor a binary64 field. Contexts and law availability are checked even for the
zero function.

```text
numerator = sum_cells value(cell) * mass(cell & given)
value = numerator / mass(given), when mass(given) > 0
```

A `FiniteFunction` is already total: its constructor explicitly assigns zero
outside its supplied pieces. A database observable roster has a different input
contract: union equal values, check distinct-value disjointness and coverage on
evidence, then construct the function. Missing value rows must not be turned
into implicit zeros. Native `Expectation(...)` aggregate heads and observation
query slots remain separate integration gates.

`FiniteFunction::parameter_expectation` supplies the corresponding owned
[signed conditional function](event-parameter-expectations.md) under a family
law. Numerator cancellation cannot erase a zero-evidence hole. Its native
query/SDK observation and query-roster coverage gates also remain open.

## Representation, validation and evidence

Revision reuses canonical exact finite functions and density partitions. It
multiplies density by the indicator/factor, divides by the checked normalizer,
and admits the resulting law through the existing normalized-law constructor.
Jeffrey's internal factor is `q_i / old_mass_i` on positive-target cells. That
computational factor does not become an observation probability in its receipt.
No legal-world enumeration is required; a restricted 62-coordinate source is
covered by the native tests. Graph, function shape/work, law payload and shared
arithmetic limits apply separately. These are not aggregate retained-memory
quotas. A refused operation publishes no partial posterior.

[Revisions.lean](../crates/bumbledb-event/semantics/Revisions.lean) adds 30 checked
reports over exact `Rat` functions: normalization and nonnegativity of Pearl and
Jeffrey updates, likelihood scaling/composition, partition target masses,
within-cell ratios, Jeffrey idempotence, signed cell contraction and expectation
linearity of the numerator. Checked counterexamples distinguish posterior targets
from likelihoods, zero mass from impossibility and overlapping revision order.
The indexed partition reference is a total bucket function; native admission
must establish its correspondence to the disjoint/covering Event roster.
The proofs do not verify Rust arithmetic, BDD contraction, map construction,
receipts, codecs or query aggregate execution.

Nine core tests include 1,280 four-world conditioning/expectation cases, 240
partition revisions, immutability, owned impossible outcomes, zero-target rules,
likelihood composition/scaling, independent decoded owners, original support,
large symbolic sources and explicit refusals. A persisted Coup query consumer
materializes Tax conditioning, reopens prior and posterior Events, imports the
translation map and executes both database paths. It obtains `92/147` for Bob's
Duke and `5/21` for Cleo's Duke. An explicit toy payoff of +1 on Bob's bluff and
-1 on his Duke gives signed expectation `-37/147` after owners/database drop.

The [semantic record](event-evidence/native-revision-semantics/check.json) and
[qualification](event-evidence/resumed-revision-qualification/check.json) pin this
slice. BEVT v2 carries posterior laws and BEDC carries translation maps.
[BESC v1](event-source-descriptors.md) now transports complete mathematical
receipts, functions and channels, reconstructing every claim on import. Full SDK
construction, general parameterized revisions, TypeSafe adapters and query
observation/expectation heads remain unfinished.

[Family conditioning](event-parameter-conditioning.md) now materializes a posterior
on the exact positive-evidence parameter domain and retains the original mass
function in an owned receipt. It keeps all outcomes within that domain; its map
to a refined prior is generally not onto. [Family likelihood](event-family-functions.md)
now also retains its exact supplied factor and normalizer, using the same domain
inclusion contract. Family Jeffrey revisions and family receipt transport remain
open. Fixed-law surjectivity above must not
be assumed for a family with excluded zero-evidence parameters.
