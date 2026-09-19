# Direct conditional completion and reachable answer graphs

The completed comparison retains 160 configurations and 364 matched comparisons.
Direct ITE completes all four formerly capped cases and improves the difficult
readout, but a large gap to the packed control remains. Five shared suites and
32 native ownership/symbolic acceptance processes pass. This is another kernel of the existing completed-root representation;
it introduces no Event type, public operator or probability interpretation.

The [equal-cofactor control](COMPLETION.md) retained exactly the same storage
for all twenty paired cases and retained all four canonical-reimport cap cases.
It leaves two concrete questions: how much three-operation ITE reconstruction
costs, and whether the large retained arena resembles the answers' own graphs.

## Exact kernel

The completed-root normalizer is simultaneous substitution. For raw predicate
`a` and the owner's fixed legal decoder `rho`, its result denotes `a(rho(x))`.
For a split on coordinate `v`, complete each cofactor and select between them
using the raw decoder function `rho_v(x)`. Do not recursively decode that
inserted selector, and do not replace coordinates sequentially.

`staged` constructs `(low & !selector) | (high & selector)`. `equal` additionally
returns a shared completed cofactor before constructing these terms. `ite`
recurses over selector, high and low together. At each step it chooses the first
coordinate in their union, cofactors all three operands, and interns the direct
result. Equal result children collapse in the existing canonical branch builder.

When the union has at most K essential coordinates, align the three local tables
and compute `(s & h) | (!s & l)` word by word. The result passes through the same
exact-coordinate reduction and interner as every other operation. This code
currently allocates alignment/result vectors. It is a first direct kernel, not
a claim that allocation, vectorization or cache policy is finished.

The ternary memo absorbs a complemented selector by swapping branches, and a
complemented low operand by complementing both branches and the result. Its key
contains all three normalized raw references. Its capacity is charged to retained
cache bytes. All three policies coexist in one executable; `staged` remains the
default. Raw refs only have meaning inside their actual owner.

This follows established k-argument APPLY and ITE machinery, reviewed in
[Bryant, Section 6](https://arxiv.org/pdf/1710.06500v1)
and [CUDD's recursive implementation](https://raw.githubusercontent.com/ivmai/cudd/master/cudd/cuddBddIte.c).
[The reading record](results/ite-reading.json) fixes the source hashes. Our local
tables, completion owner, diagnostics and database measurements are adaptations;
the papers do not establish performance for this workload.

## What the graph census establishes

An optional diagnostic walks raw nodes reachable from:

- The original input Events.
- The 64 output Events.
- Owner infrastructure: support, decoder coordinates and prepared count supports.
- The union of all three sets.

It also records each output's exact physical dependence mask and individual
reachable-node count. Complements share the same records. Traversal is read-only,
uses temporary scratch storage, and runs before canonical reimport so that a
later verification cap cannot hide the already constructed result. Such a
process still lacks complete semantic acceptance if verification caps.

Cache entries are deliberately excluded from the root union. Nodes outside it
are **not automatically safe to free**: a collector would have to evict or repair
cache entries and the interner. The census distinguishes output structure from
retention; it does not measure a garbage collector, peak memory or recoverable
bytes. Diagnostic processes are excluded from latency comparisons.

For a root depending on d coordinates with cutoff K, its unfolded binary/table
tree has at most `2^(d-K+1)-1` records if d exceeds K, otherwise one. Every split
removes at least one coordinate and the builder collapses to a table at K.
Sharing can only reduce the reachable count. The readout fixture's answers
depend on X, Y and the retained environment, so width six permits at most thirteen
coordinates and 31 records per 512-cell root. This is a derived upper bound,
checked against actual masks/graphs; it is not a Lean proof of the Rust arena.

## Checks and proof boundary

The shared suite checks 32,768 exhaustive small conditional triples at four
cutoffs and both orders, plus 192 scattered twelve-axis cases in 61-coordinate
owners. Every scattered result is compared with an independent 4,096-assignment
truth table and exact canonical identity. Complement polarities, graph bounds
and read-only traversal are checked. The decoder suite now compares every input
against both alternate normalization policies, retaining all prior algebra,
transport, original-support count and native-relation checks.

[DirectConditional.lean](lean/DirectConditional.lean) checks simultaneous
substitution, composition and identity, conditional polarity and Shannon rules,
and a counterexample to sequential replacement. These six reports extend the
central suite to 115; the later projection-gate derivation brings it to 120. They establish the pure substitution algorithm's meaning;
Rust refs, memo implementation, canonical interning and ARM64 remain separate
verification obligations. The first check exposed a Lean constructor-name syntax
error; [that failed check](results/ite-lean-initial/lean-check.json) is preserved.

## Measurement decision

Compare the difficult fibred high-X-bit query first, then every prior cap case,
suffix readouts, full support, both physical orders and the holed half-X/Y query.
Use complete processes only; keep construction, query and canonical reimport
distinct. A faster ITE with unchanged answers cannot by itself select a decoder.
If the reachable answer graphs remain small while intermediate growth remains
large, test a demand-driven projection/substitution kernel against this control.
Its target is still the exact joint-witness expression in
[the readout report](PREFIX-READOUTS.md#the-next-fallback-experiment-has-a-precise-algebraic-target).

## First native structural diagnosis

[Two diagnostic processes](results/ite-first-diagnostics.json) complete with
full output/reimport acceptance. On the width-six fibred high-X-bit query, both
policies return the same reachable graphs: 343 output records across 64 Events,
392 input records, 430 infrastructure records, and 1,163 in their shared union.
Every output uses thirteen physical coordinates and at most 24 reachable
records, below the derived bound of 31. These counts repeat with the outer memo
on and off. The policies also agree on the width-four graph census.

Staged retains 1,363,847 records and 162,947,404 estimated carrier bytes; direct
ITE retains 708,405 and 83,841,612. Neither total includes the external operator
memo. The graph is small; the current construction/retention policy is large.
This diagnoses where to investigate, without proving that every non-root node
can be reclaimed or that a fused kernel will attain the graph's size bound.
Timings from these diagnostic processes are excluded from comparisons.

## Completed native comparison

[The comparison](ITE-MEASUREMENTS.md) retains fifty processes: 46 complete and
four hit the node cap. All four caps belong to staged normalization; their
same-executable ITE counterparts pass, including canonical reimport. There are
no semantic assertion failures. The new backtraces again locate the staged caps
in untimed canonical reimport. Ten diagnostic processes retain graph censuses
separately from competitive timings.

The difficult width-six fibred high-X-bit query has three samples per case.
With the outer memo, fresh-query medians in milliseconds are:

| Order | Staged prefix512 | Direct ITE prefix512 | Packed512 | Dense |
| --- | ---: | ---: | ---: | ---: |
| Bit-major | 1643.748 | 377.951 | 1.486 | 1.451 |
| Face-major | 14.269 | 9.062 | 0.564 | 1.449 |

The 4.35-fold bit-major improvement does not close the performance gap. Its
query-retained estimate falls from 162,956,364 to 83,850,572 bytes (including
the outer memo), while packed retains 6,660,848. Warm replay improves to 0.263 ms,
but does not pay back the first-use work or memory automatically.

The full-support screen preserves all storage fields in sixteen matched policy
pairs. On holed half-X/Y readouts, direct ITE also reduces construction residue:
prefix512's bit-major retained estimate falls from 2,926,208 to 1,440,368 bytes.
Those controls use one sample; small timing differences are not conclusions.
The four formerly capped configurations use a one-sample screen, so successful
completion is established without asserting a stable latency rank for them.

[Native acceptance](results/ite-acceptance.json) passes 32 processes, retaining
96 owned relation rows and 48 symbolic relation rows across both decoders,
both cutoffs, staged/ITE and materialized/mapped products. The older equal-cofactor
control remains available and participates in shared cross-policy checks.

[Actual ARM64](results/essential-words-select-ite-arm64.s) from this executable
contains four `BSL.16B` instructions in the aligned local select loop. The
[assembly record](results/assembly-ite.json) preserves its symbol and hash.
This verifies local vector lowering, not vectorization of the full recursive
normalizer or elimination of alignment allocations.

Keep staged as the laboratory default and retain the frozen comparison. The
next design experiment is [support-factor elimination](PROJECTION-GATES.md):
use checked dependencies and joint witness coverage before normalization. A
fully fused projection/substitution kernel remains a later option. The census
shows considerable room to improve construction, without selecting a universal
carrier or proving that the output graph alone determines total query cost.
