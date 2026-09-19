# Count legal completions through a membership dependency

This is a matched implementation of exact finite cardinality. The Rust
checks, four new Lean reports and all 110 native processes pass. The complete
query improves strongly in the interleaved layout; layout-dependent winners remain.
Event operations, canonical equality and the source-law spectrum are unchanged.
The [round audit](results/factor-final-audit.json) verifies source snapshots,
all comparisons, proof records and production isolation.

## The same FD removes work from observation

For legal product `S(e,x,y,z) = D(e,x) & D(e,y) & D(e,z)`, the completed Event
physically omits a face exactly when its legal membership FD ignores that face.
The per-face decoder theorem already supplies that bridge. Let I be the faces
on which the completed root depends. Count its retained legal tuples, then use
the number of legal completions for each omitted face:

```text
Count_S(A) = sum over nonempty environments e:
               |D(e)|^(3 - |I|) * Count_on_D(e)^I(A)
```

The empty/full shortcuts remain exact. A three-face Event uses the existing
joint traversal. An arbitrary support owner has no product certificate and
keeps that traversal too. A copied face is a concrete counterexample to applying
the new formula from marginal domain sizes alone.

This is the natural-number sum/product instance of the variable-elimination
rule in FAQ §5.1.2. Its §5.2.1 also explains why restrictions that can still remove
answers cannot simply be discarded. The [reading record](results/factor-reading.json)
retains those claims and source hash. This experiment is not a complete InsideOut
implementation or a claim of its asymptotic bounds.

## Prepared support, no count-time mutation

`Retraction` now keeps eight raw partial-support roots per nonempty environment.
Each is its **original** environment selector conjoined with the selected state
domain predicates. Preparation occurs before installing the completed decoder.
These roots are private support predicates; they are not published Event values.
Their arrays, capacity and constructed arena residue are charged to memory.

For a completed Event, its exact physical face mask selects one prepared root.
The unchanged joint support/Event kernel counts over the ambient raw cube.
Neither operand uses an omitted face, so its count contains an exact factor
`2^(width * omitted)`. Divide that out first, then multiply by the legal fibre
cardinality. The implementation checks divisibility and uses a wide intermediate
before the checked conversion to the owner's count width.

Selectors used to repair empty environments are deliberately excluded. Such a
selector would count aliases of another environment. Original empty environments
contribute no terms.

`EVENT_LAB_RETRACTION_FACTOR=joint|faces` selects the original or factored count
in one executable. Both prepare identical roots, memory and caches. The existing
scalar/word switch is independent. Every count remains read-only; no conjunction
or observation result is interned. Constructor overhead is measurable even when
`joint` is selected. A comparison to the previous executable would confound that
extra preparation with the algorithm change, so the native pair must use one
new executable.

## Checks and limits

The [finite reference](results/factor-reference.json) checks 18,432 Events across
288 single/environment-indexed domains and 33,408 fibre replacements, using both
minimum and XOR-nearest prefix completion. It retains the coupled-support failure.

The Rust checks compare all four count modes on the same owners and inspect
unchanged nodes, bytes and memory breakdowns. Each of six carriers runs 1,824
finite cases, including every whole-face mask, holes, empty environments,
zero-width state faces and arbitrary copied support. Four carriers additionally
check sixteen 61-coordinate Events each against direct integer formulas.
Both [slab](results/factor-slab-check.json) and
[enum](results/factor-enum-check.json) suites pass. The first compile attempt
exposed a private domain-cardinality accessor; its log/source snapshot are
retained, and the accessor is now crate-visible inside this laboratory.

The four [Lean reports](lean/FaceCounting.lean) prove the finite product,
environment sum, raw/legal fibre replacement and coupled-support boundary.
They are denotational proofs, separate from the Rust correspondence checks.

## Native results and leaf-size control

The [complete measurements](FACTOR-MEASUREMENTS.md) retain 216 configurations and
552 matched comparisons from one executable. The screen, independent repeat,
64-cell screen and focused cutoff repeat pass 102 processes. Separate native
acceptance adds eight, with 24 owned-result cases and twelve symbolic programs.
Every finite output is checked pointwise against original legal worlds, outside
timing. All 48 matched count-mode pairs retain identical nodes and memory.

Five-sample fresh-query medians below use width six, the outer memo, slab storage,
and certified fused products. Packed and dense use their native materialized
controls. Each complete query publishes eighty Events and observes exact counts.
Construction is separate.

| Domain / order | Prefix joint ms | Prefix faces ms | Minimum faces ms | Packed ms | Dense ms |
| --- | ---: | ---: | ---: | ---: | ---: |
| below / bit | 4.076 | 2.476 | 3.949 | 2.126 | 2.912 |
| below / face | 8.045 | 8.071 | 8.657 | 6.758 | 2.914 |
| holes / bit | 7.058 | 2.642 | 14.575 | 7.039 | 3.095 |
| holes / face | 7.538 | 7.405 | 12.783 | 16.034 | 3.142 |
| fibred / bit | 11.719 | 5.208 | 20.602 | 8.402 | 6.406 |
| fibred / face | 16.774 | 15.557 | 22.155 | 25.746 | 6.446 |

On the fibred bit-major query, prefix observation falls from 6.696 to 0.168 ms;
computation remains about 5 ms. Its final arena is 6,485.7 KB, versus packed's
13,322.2 KB and dense's 45,018.8 KB. Its focused fresh range is 4.933–5.385 ms.
Current construction medians are 25.825, 59.507 and 462.959 ms respectively;
these constructors are not inherent lower bounds for their representations.
The grouped layout remains dominated by construction of Event results: reducing
its already short count phase does not yield a comparable complete-query gain.

The smaller 64-cell cutoff loses fresh-query time and retained bytes in all six
initial width-six prefix cases. The five-sample fibred confirmation gives:

| Order | Prefix64 ms / KB | Prefix512 ms / KB |
| --- | ---: | ---: |
| bit-major | 7.418 / 9,227.8 | 5.208 / 6,485.7 |
| face-major | 24.941 / 7,703.7 | 15.557 / 7,091.4 |

That is a query/storage result, not dominance in every phase: face-major
construction is 15.836 ms for prefix64 versus 25.346 ms for prefix512 in that
repeat. Other operations and layouts still need their own comparisons.

No samples are discarded. The earlier packed fibred face-major repeat includes
26.238, 340.179, 99.760, 27.824 and 60.700 ms; host load increased during that
process. The focused repeat is 25.013–26.566 ms. Prefix64's 33.251 ms face-major
sample also remains. The generated tables select the latest supplied process
per exact case and retain links to all earlier raw data.

[Actual ARM64](results/assembly-factor.json) retains the shared observation
dispatcher and its unchanged word contraction. The latter contains vector
Boolean operations, `CNT.16B` and `UDOT`. The dispatcher removes whole factors
before entering that kernel; the gain does not require a new SIMD representation.

## Cardinality is one interpretation

The general identity is a pushforward through a readout. If membership of A is
determined by `u = readout(w)`, partition the original legal worlds by u:

```text
Count_S(A) = sum over legal readouts u:
               membership_A(u) * |{ w in S : readout(w) = u }|
```

Each legal world appears in exactly one fibre. This identity also holds on
coupled support, but its fibre cardinality can vary with u. The implemented
product certificate supplies that cardinality as `|D(e)|^omitted`; it does not
claim that every readout has a constant number of completions. General symbolic
fibre-cardinality functions are a separate, unimplemented contraction strategy.

For the lab's Coup position, thirteen available physical cards give 78 possible
unordered two-card hands for Bob and 55 compatible two-card hands for Cleo
after fixing Bob's hand: 4,290 deals. Counting a Bob-only predicate can
therefore use 55 completions per legal hand. The two hands are still coupled by
card conservation. The current three-face benchmark certifies products of
**complete states**, not independence between the hands inside one state.
Additional evidence about Cleo can make those hand-fibre cardinalities unequal.
The [Coup reference](results/coup-fibre-reference.json) enumerates all 4,290
deals. Restricting to Cleo holding a Duke leaves 19, 10 or 0 completions for a
fixed Bob hand with zero, one or two Dukes. It checks sixteen pushforward counts
before/after that restriction. Internal card-copy handles are then hidden:
Bob's fifteen possible role-hand observations have unequal fibre populations,
and fourteen permit Cleo to hold a Duke. The two-Duke hand forbids it; none
forces it. This is a finite reference for the next readout experiment, not a
native performance result.

No public row weights or inferred probability independence were added. A joint
source law may couple retained and omitted faces through a shared unknown
parameter. Eliminating that face must retain its resulting factor over the
parameter, rather than substituting its integer domain size. The current
shared-parameter spectrum therefore keeps the full original-support traversal.
A polynomial or other semiring interpretation needs its own exact contraction
and performance evidence. The [information-readout workload](PREFIX-READOUTS.md)
is also a separate next experiment; this change only isolates observation cost
in the existing complete relation program.
