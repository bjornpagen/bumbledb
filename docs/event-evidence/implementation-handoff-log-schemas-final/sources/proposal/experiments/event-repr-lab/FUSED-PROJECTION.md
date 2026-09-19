# Direct completed projection: correct, slower on the repeated readout

This follows the frozen local-completion round. Eight [Lean reports](lean/FusedProjection.lean)
pass, all without axioms. The Rust control passes five differential suites,
81 comparative native processes, sixteen ownership/symbolic acceptance
processes and two separate graph diagnostics. The [matched measurements](FUSED-PROJECTION-MEASUREMENTS.md)
retain 256 configurations and 856 comparisons. The repeated difficult query
favors staged local completion; direct construction remains an exact alternative,
not the preferred schedule for that workload.

The exact operation is a relational contraction:

```text
result(target) = exists source:
    legal_gate(source) & A(source)
    & observed_source(source) = observed_decoder(target)
```

The completed projection can be built by traversing the original source
predicate. At a hidden source coordinate, combine the recursively transformed
cofactors with OR. At an observed coordinate needing repair, combine them with
ITE using its target decoder function. Keep unselected observed coordinates as
raw selectors. When no source coordinate needs elimination or substitution,
return the remaining function directly. Cofactoring is applied to source roots;
inserted target decoder functions are opaque to that recursion.

This equals ordinary existential abstraction followed by simultaneous
substitution for every input, including arbitrary coupled support. The earlier
witness/local-completion certificates determine which gates and decoder axes
are sufficient; fusion supplies no new permission to omit either. The control
must keep those choices, constructor inputs and canonical publication fixed.

A critical representation distinction appears in the memo. Substitution alone
commutes with complement, allowing an unsigned source root plus result polarity.
Existential abstraction does not. The combined operation must retain the full
signed source root, hidden-coordinate set and selected decoder-coordinate set.
It must preserve one shared source witness for all conjuncts; projecting operands
independently is unsound. Target coordinates introduced by a decoder must not
subsequently be mistaken for source variables to eliminate.

The finite checks require exact cross-strategy canonical IDs, partial
substitution, all hidden masks, repaired/hidden environments, coupled supports,
and the complementary-source counterexample. Every native comparison returns
and verifies all 64 readout Events, including exact legal counts and canonical
reimport. The existing local-completion round remains immutable.

## Candidate Rust control

Keep `EVENT_LAB_RETRACTION_EXECUTE=staged|fused` separate from the current gate,
completion-locality and traversal policies. Constructor/import normalization
remains unchanged. The existing complete-fibre direct paths remain unchanged.

Use one internal joint transformation for two source roots. Sort the signed pair
for conjunction's commutativity, then memoize `(a, b, hidden, selected_decoder)`.
Only source dependence matters to those masks; selected source coordinates that
are hidden are quantified, not substituted. Base cases delegate to ordinary
Apply when no work remains, relational product when no substitution remains,
and simultaneous substitution of the conjunction when no abstraction remains.
Otherwise cofactor **both** source operands at the same source coordinate,
recurse, then combine by OR for hidden coordinates or by the target selector for
observed coordinates. Unary projection uses the same operation with a true
second operand. Existing gate construction is held fixed in the comparison.

Charge all new memo capacity and owner metadata equally in every policy. Clear
this memo when installing a decoder or switching test policies. A prepared
production operation can carry these masks; the experiment's switches are not
intended for the schema API.

The small raw oracle should explicitly enumerate source witnesses after applying
the target map to observed coordinates. Test arbitrary raw functions, all hidden
and selected masks, complements, simultaneous substitution and both traversal
policies. Public operation tests additionally need all gate/completion policies,
both stores, both decoders/cutoffs and exact canonical IDs. This round can begin
only after the preceding native processes have finished and their evidence is
frozen; never edit Rust during compilation or checking, and never benchmark
alongside compilation, Lean checking, profiling or another benchmark.

## Shared checks completed before native timing

Five suites pass: slab/enum stores, ITE/staged/equal reconstruction, full/needed/
local-needed selected defaults, and both debug and optimized checker builds.
Every suite runs six decoder/cutoff combinations. Each combination checks
393,216 product-domain results and 18,432 coupled-support results under all
three gate policies, four completion strategies and both execution paths,
requiring exact root identity as well as the independent legal-world answers.
Another 65,536 cases check simultaneous partial substitution. The raw joint
transformation adds 262,144 cases per combination: all three-bit predicates,
all hidden/selected masks, both source traversal policies, original bit patterns
as the oracle, and a decoder with coupled support. Signed complements share the
same persistent test memo, so a polarity-omitting key cannot quietly pass.

Existing large symbolic readouts, source-law observation, transport and role
checks remain part of these suites. The partial readout on a 61-coordinate
minimum-repair owner exercises the general path; prefix-certified suffix
readouts deliberately retain their direct path.

## Same-binary native result

Three samples, width six, fibred legal support, high-X-bit readout, outer memo,
witness gates, ITE and local-needed completion:

| Order | Staged ms | Direct ms | Packed ms | Dense ms | Staged records | Direct records |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Bit-major | 2.229 | 2.711 | 1.531 | 1.448 | 22,748 | 23,351 |
| Face-major | 2.964 | 3.286 | 0.585 | 1.423 | 18,579 | 19,323 |

These are medians of complete fresh query computation and original-support
counts. Native setup, input construction and final oracle/reimport verification
are outside that timer and retained separately. Each output is an actual Event,
not only an emptiness answer. Bit-major staged samples span 2.183–2.368 ms;
direct samples span 2.671–2.852 ms. Face-major spans 2.901–3.029 and
3.236–3.374 ms respectively. Three samples support this local comparison, not a
machine-independent ranking.

Bit-major grouping remains about 1.0 ms. Readout construction grows from 1.107
to 1.562 ms. Both start with 19,801 input records, but direct construction retains
603 extra records. Charged final query bytes are 3,354,460 versus 3,356,784;
capacity rounding means extra records need not imply proportional byte growth.
Face-major bytes are 2,392,700 versus 2,516,880.

Full/holed support, prefix-suffix bypasses, joint/active gates, both decoder
families, both cutoffs and staged-conditional reconstruction remain controls.
Twenty-four pairs taking the unchanged direct path have identical storage.
Single-sample variations on those bypasses are timing noise, not an alternative
algorithm. A conspicuous apparent full-completion win with the minimum-repair
decoder was repeated explicitly: the original uncached staged sample was
32.917 ms, but its three-sample repeat gives 15.899 ms against direct's 21.783 ms.
With the outer memo the repeat gives 15.991 versus 21.598 ms. That exception does
not survive repetition. All raw samples remain in the evidence.

Both read-only diagnostics preserve the same 343-record output union, 101
output tables, 758 output words, maximum 24 records per answer, 431 infrastructure
records, and 1,164 records reachable from outputs, inputs and infrastructure
together. Retained arena records differ. Reachability is not reclaimed memory;
there is no garbage-collection timing claim.

## What this settles

The fused recurrence is semantically sound. It must retain signed operands,
shared source witnesses and source/target separation. Those are algebraic
requirements, independent of its timing result.

Canonical publication permits both schedules. Eliminating source coordinates
first can collapse the function before decoder substitution. Direct recursion
can build transformed branches that later merge. The extra retained nodes and
readout time are consistent with that explanation; the experiment does not
prove a universal cost theorem or attribute every cycle to it.

Keep staged witness-gated local completion as the preferred schedule for this
readout family. Preserve the direct candidate and its negative evidence. The
public Event API gets neither a fusion flag nor a different result type. This
round strengthens the separation between exact semantic certificates and
measured execution choices.

Frozen evidence: [build](results/build-fused.json),
[comparison](results/fused-comparison.json), [acceptance](results/fused-acceptance.json),
[staged census](results/fused-staged-diagnostics.json),
[direct census](results/fused-fused-diagnostics.json), and
[final audit](results/fused-final-audit.json). The executor has SHA-256
`2ebe0641b45c4e625a8249bb522a62c056c37fb49d14747bbe25e1adc003ea82`.
