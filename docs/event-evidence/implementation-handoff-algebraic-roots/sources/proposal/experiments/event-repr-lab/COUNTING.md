# Sparse difference graphs and exact uniform counting

This standalone experiment follows the [counting argument](../../research/compact-counting.md).
It adds direct sparse-ANF construction and an exact counter to the raw
fixed-basis manager. The counter expands ordinary cofactors and memoizes their
populations. Counts include skipped coordinates and complemented roots. Its
contract is the full raw Boolean cube, not decoder aliases, legal-world counts
or arbitrary correlated source laws.

No timings are reported. None of these processes runs native Free Join.

## Checks and fixtures

The [updated Rust check](results/difference-counting-check.json) preserves all
50,331,648 binary-operation comparisons, 98,304 projections, 356,352 maps and
98,304 completed projections. It adds 12,288 uniform counts across every
three-bit Boolean function, six orders, four basis schedules and two Apply
kernels. Direct sparse construction checks every three-bit ANF under all six
orders (1,536 cases), including duplicate cancellation and exact identity with
independent truth-table construction. Additional counts exercise 61 coordinates,
omitted coordinates, constants and complements.

The [circuit probe](difference-prototype/src/bin/constraint_counting.rs) encodes
AND/NOT gate equations with auxiliary wires, then multiplies each constraint
by a fresh Boolean multiplier. The resulting polynomial has degree at most
three and at most `3g+2` term occurrences for g gates. `Arena::from_anf` builds
the base/difference trie directly without truth-table enumeration or Apply.

The small end-to-end check exhausts all two-input circuits with zero, one or two
AND/NOT gates (unordered AND arguments), selecting every available output wire.
It covers 197 circuit/output choices in 788 manager cases across two orders and
two kernels. Every raw assignment is checked against direct polynomial
evaluation. Circuit evaluation independently supplies the satisfying-input count,
which is checked against the character-sum identity and the diagram counter.

The larger screen uses a fixed seeded family with four inputs and
4/8/12/16/24 gates. Its scalar oracle evaluates all sixteen circuit inputs and
uses the general identity proved in [ConstraintCounting.lean](lean/ConstraintCounting.lean).
It does not enumerate the larger raw cubes. This family has a deliberately
cheap oracle; the observed blow-up is a weakness of this general cofactor
counter, not evidence that every algorithm must struggle on these instances.

## Results

[Raw results](results/difference-counting-shapes.json) retain 21 processes:
the small check plus sixteen complete larger counts and four node-cap refusals.
No semantic failure was observed. Every cap occurs after direct construction
and the input-size checks, during exact counting; the retained logs identify the
common two-million-record cap, not the runner's separate timeout limit.

| Gates | Raw bits | Order | Input records | Retained records after count | Retained bytes after count |
| --- | ---: | --- | ---: | ---: | ---: |
| 4 | 13 | Forward | 17 | 72 | 10,281 |
| 4 | 13 | Reverse | 14 | 71 | 10,281 |
| 8 | 21 | Forward | 31 | 1,054 | 192,529 |
| 8 | 21 | Reverse | 26 | 1,311 | 162,065 |
| 12 | 29 | Forward | 45 | 21,015 | 3,076,473 |
| 12 | 29 | Reverse | 39 | 25,791 | 3,076,473 |
| 16 | 37 | Forward | 59 | 334,441 | 49,218,017 |
| 16 | 37 | Reverse | 51 | 492,543 | 74,449,377 |
| 24 | 53 | Forward | 87 | Node cap | — |
| 24 | 53 | Reverse | 73 | Node cap | — |

The two Apply kernels produce identical counts and storage/work census for
every matched case. This is expected: the counter uses cofactors and XOR, while
their difference concerns nonlinear Apply. Repeating both kernels checks that
the preceding Coup workaround cannot remove this counting behavior by itself.

Record totals include the distinguished terminal record. Bytes estimate retained
arena/interner/operation-cache capacities. They exclude allocator overhead,
temporary vectors and the per-call count memo, which is released before the
final census. These are not peak-memory measurements.

## Decision and boundary

The difference representation's small inputs and the counter's large workspace
are separate observations. Retain both. The inspected literature and explicit
reduction establish the generic worst-case obstacle; these finite examples only
demonstrate one counter's behavior. A specialized gate/constraint-aware observer
could exploit this fixture's structure. Conversion, elimination schedules and
quadratic/parity-specific observers remain possible engineering choices.

The Event contract does not gain a new weight field, a promise of independent
sources, or a weakened notion of equality. Any future difference carrier must
retain exact owner-aware operations and charge for observation as part of its
admission evidence. The current standalone probe is not admitted as a native
carrier.

The [source/tool snapshot](results/difference-counting-src/manifest.json) binds
the updated tests and counting screen to this build. Earlier coefficient,
kernel-control and bilinear snapshots remain unchanged. The central proof suite
then reached 165 reports, including eleven difference-algebra reports and seven
constraint-counting reports. Rust interning and the complexity reduction as a
whole are not formally verified.
