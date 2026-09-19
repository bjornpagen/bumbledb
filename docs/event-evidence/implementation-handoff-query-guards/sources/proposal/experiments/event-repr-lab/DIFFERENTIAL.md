# Competing Shannon and difference decompositions

This is a standalone representation screen motivated by the
[primary-source review](../../research/difference-decomposition.md). It does not
replace the frozen native Free Join experiments or add a production Event type.

The raw manager fixes one basis per semantic coordinate: Shannon, positive Davio,
negative Davio, or a repeating mixed schedule. All use the same sixteen-byte
record `(exact dependency mask, two signed references)`, the same full-key
interner and the same two-million-record cap. Davio references name a base and
difference; complement flips the base polarity only. Every published raw
function has exact identity within its fixed manager.

The direct coefficient Apply implements all sixteen truth functions via their
Boolean-ring coefficients. A second kernel obtains ordinary cofactors for
nonlinear operations while retaining coefficient XOR. The kernels share their
normal form and constructor; differing intermediate work cannot be confused
with different input functions or more permissive output semantics.

## What is checked

The [current Rust verification](results/difference-cofactor-check.json) passes
50,331,648 binary operation results across all 256 three-bit functions, all six
coordinate orders, four basis schedules and two kernels. It compares exact
canonical IDs against independently constructed truth-table roots. Further
checks cover 98,304 existential projections, 356,352 coordinate/nonlinear maps,
98,304 completed projections on coupled support, complement identity, exact
dependency masks, and 61-coordinate symbolic operations. Projection cache keys
retain root polarity, and target selectors are substituted simultaneously.

These are finite implementation checks, not a universal Rust correctness proof.
[Differential.lean](lean/Differential.lean) supplies the algebraic statements,
including arbitrary-width full-tree roundtrip and uniqueness under any fixed
mixed schedule. Shared-node reduction and Rust arena correspondence remain
separate obligations.

## Shape/work screen

The four-output relation fixture composes a stay-or-increment relation with an
advance-by-two relation, then returns composition, converse, May of a goal, and
All of that goal. It checks every answer against explicit middle-state witnesses.

The Coup fixture uses the retained four-card encoding and 4,290 legal deals.
Thirty-six input predicates are completed onto the first legal deal outside
support. It explicitly replays the retained clover's 4,096 complete bindings,
returning all 64 grouped Events with their common legal-count checksum. It
compares every output on all 65,536 raw codes. This replay is deliberately
labelled: it executes the Event workload, but **does not run Free Join**.

The bilinear fixture uses twenty bits: ten x bits and ten y bits. It builds
`f = parity(x & y)` and `g = parity(x & rotate_left(y,1))`, then returns their
XOR, AND, existential projection of the AND over x0, and universal projection
of the XOR over x0. Every process checks 4,194,304 output answers against an
independent pointwise oracle. Grouping the x coordinates before y deliberately
tests a family where difference coefficients should help.

The first [coefficient-only screen](results/difference-shapes.json) has ten
complete processes and six common node-cap refusals, with no demonstrated
semantic failures. Its original runner recorded nonzero exits as `failed`;
the retained logs identify all six as `RESOURCE_CAP: coefficient nodes`.
The [kernel control](results/difference-kernel-shapes.json) distinguishes those
statuses explicitly and retains phase markers. Timings are not collected.

The [initial sources](results/difference-initial-src/manifest.json), raw logs
and exact source/binary hashes remain frozen. The subsequent control must be
read as a separate build. The standalone bytes estimate retained capacities,
including operation caches, but exclude allocator and temporary/oracle memory.

## Results: normal form and evaluation schedule both matter

The kernel-control screen has **26 completed processes, six node-cap refusals,
and zero observed semantic failures**. All six caps are the coefficient kernel
on Coup. Every capped case constructs and verifies its inputs first, then caps
during grouped query construction. Cofactor Apply completes those same normal
forms, so the first cap is not proof that the final output cannot fit.

Coup's completed cofactor-kernel cases still favor Shannon structurally:

| Basis / order | Input records | Retained records | Output reachable | Retained bytes | Apply misses |
| --- | ---: | ---: | ---: | ---: | ---: |
| Shannon / bit | 5,033 | 93,302 | 8,859 | 12,304,592 | 309,518 |
| Shannon / face | 7,988 | 121,066 | 10,218 | 14,713,040 | 429,412 |
| Positive / bit | 22,632 | 1,291,111 | 48,319 | 200,769,744 | 3,736,034 |
| Positive / face | 13,431 | 965,440 | 26,940 | 125,501,648 | 3,362,352 |
| Negative / bit | 19,771 | 1,571,949 | 51,480 | 200,769,744 | 4,395,624 |
| Negative / face | 16,081 | 919,692 | 40,477 | 125,501,648 | 3,390,961 |
| Mixed / bit | 13,966 | 542,711 | 25,436 | 71,139,536 | 1,516,326 |
| Mixed / face | 9,668 | 415,295 | 16,048 | 53,117,136 | 1,427,605 |

The [bilinear screen](results/difference-bilinear-shapes.json) completes all
sixteen processes. The coefficient-kernel positive basis has much smaller
reachable outputs, but greater retained workspace and Apply work:

| Basis / order | Input records | Retained records | Output reachable | Retained bytes | Apply misses |
| --- | ---: | ---: | ---: | ---: | ---: |
| Shannon / bit | 90 | 750 | 669 | 77,172 | 586 |
| Positive / bit | 32 | 1,064 | 242 | 158,068 | 1,842 |
| Shannon / face | 3,070 | 12,210 | 10,036 | 1,230,084 | 12,310 |
| Positive / face | 31 | 14,426 | 330 | 1,774,852 | 22,537 |

At face order the positive output graph is about 30× smaller; including retained
input roots, its live union is 349 records versus Shannon's 12,209. This could
matter after collection or for transported results, but neither a collector nor
transport savings are measured here. The cofactor kernel gives the same output
census and increases this positive case's workspace to 17,058 records and
41,518 Apply misses. It is a useful Coup workaround, not a universal schedule.

The small relation fixture shows a narrower benefit: positive/bit coefficient
Apply uses 30 output records versus Shannon's 32, but 432 Apply misses versus
122. Do not turn any of these census figures into timing rankings.

## Formal status and next decision

The eleven [difference proofs](lean/Differential.lean) check the equations,
the invalid coefficient-wise projection rewrite, and arbitrary-width full-tree
roundtrip/uniqueness for any fixed mixed basis schedule. Nine reports are
axiom-free; the two tree reports use `propext`. They do not prove arbitrary
node-adaptive basis selection canonical or verify the Rust shared arena.

The subsequent [counting experiment](COUNTING.md) and
[primary-source argument](../../research/compact-counting.md) close an additional
question: a compact sparse-difference graph does not promise an efficient exact
uniform count. The observer needs its own algorithm and cost evidence.

Retain Davio as a specialized competing representation, without promoting it
over the tested Shannon/table carriers. The next independent candidate is
Shannon with compressed canalizing/XOR edge chains. That preserves ordinary
cofactor semantics while testing elementary reductions from λDD; charge the
labels and normalization work explicitly. It is a research direction, not an
implemented or selected winner.

All three structural builds are retained separately: the
[initial snapshot](results/difference-initial-src/manifest.json),
[kernel-control snapshot](results/difference-kernel-src/manifest.json), and
[bilinear snapshot](results/difference-bilinear-src/manifest.json). The current
library additionally contains the counting operations and has its own frozen
check; the old evidence is verified against its original sources.
