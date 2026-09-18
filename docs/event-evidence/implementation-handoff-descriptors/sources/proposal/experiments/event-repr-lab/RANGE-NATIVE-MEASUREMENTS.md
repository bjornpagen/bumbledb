# Native comparison: grouped selectors do not win Coup

The separate native build passes the [optimized correctness run](results/range-native-correctness.json)
and all twelve [native admission processes](results/range-native-admission.json).
The [join comparison](results/range-native-join.json) passes fourteen serial
processes, retaining 56 configurations and seven raw samples per configuration.
The [analysis](results/range-native-comparison.json) compares eight query cases
against six controls; the [auditor](range_native_audit.py) verifies 48 comparisons.

The three matched range configurations use the same anchored support convention,
constructor, packet format and exact counting/source-law adapters. Only the
record normal form and step/group kernels differ. Existing baseline modules are
copied unchanged. Every candidate runs in the same optimized ARM64 executable,
with the same scalar rows and independently checked grouped Event outputs.

## Fresh and repeated queries

Fresh query medians for clover with outer Event-operation memo enabled:

| Carrier | Compact Coup, ms | Card-coordinate Coup, ms |
| --- | ---: | ---: |
| Range plain Shannon | 27.883 | 34.113 |
| Range records, one-bit kernel | 27.261 | 34.391 |
| Range records, grouped kernel | 29.691 | 35.837 |
| packed512 | 3.410 | 11.667 |
| dense | 0.890 | 7.969 |
| essential512, derived/slab | 29.573 | 78.008 |
| retraction512, local-needed | 30.380 | 90.289 |

Grouped/control fresh ratios versus the same range records with one-bit
operations range from 1.0003 to 1.136 across all eight scenarios/shapes/memo
cases. None of these medians favors grouping. Against packed512, grouping is
3.07–11.24 times the measured fresh time; against dense, 1.32–45.80 times.
These are finite same-binary measurements, not universal complexity claims.

For the two clover/memo cases above, grouped repeated-query medians are
0.745 / 1.084 ms, versus one-bit range's 0.737 / 1.096 ms, packed512's
0.191 / 0.302 ms, and dense's 0.160 / 0.166 ms. The timer includes exact output
counts, so even a warm Event-operation cache does not remove all carrier work.
The seven samples per configuration come from one process; small differences
are descriptive rather than a significance claim.

Group compression reduces the final retained node count from 103,502 to 101,380
on compact Coup and from 121,398 to 118,851 on card-coordinate Coup in those
clover cases. Estimated bytes barely change: 12,617,216 → 12,612,608 and
15,169,584 → 15,163,952. These estimates include arena/operation caches and the
outer Event memo. They are not peak process memory. Fewer retained nodes did
not translate into lower measured fresh latency.

## What was included and checked

The existing native timer includes actual Free Join execution, Event expression
construction, per-group union and exact output counts. Input construction,
query preparation, cloning the fresh input arena, and independent output-oracle
checks remain outside it and are separately recorded where the existing harness
provides fields. The native path still consumes every complete scalar binding.

The compact encoding has 4,290 admissible worlds. The card-coordinate encoding
uses 65,536 raw codes for the same 4,290 legal deals. Native range controls use
support masking/anchoring; the earlier standalone range replay used completed
predicates. Its state counts must not be substituted for these native measurements.
Native correctness also checks ordinary dependency-query routing. The twelve
admission processes supply 69 verified rows covering owner computation, symbolic
relation construction, packet transfer and exact source-law contraction. Their
single-sample times are acceptance diagnostics, not this ranking.

The executable SHA256 is
`58b53e8ec12f592e81004d2e10bfbb55d77cddfba238a19a68f953542d08d739`.
The [build record](results/build-range-native.json) and
[source/executable snapshot](results/range-native-src/manifest.json) pin it.
No compilation, Lean run, profiling or other laboratory benchmark ran alongside
the serial measurement processes. The original native laboratory module tree,
previous executable, and production sources remain unchanged.

## Decision and next discriminating experiment

Do not promote sparse shared-exit selectors to the default Event carrier or
add their tag to the essential representation on this evidence. Retain the
implementation, proof, exact source-law adapter and strength fixture as a
completed competing experiment. It demonstrates a useful algebraic pattern,
not a broadly superior query representation. Since grouped fresh queries lose
to their matched step control in this sweep, the planned wider algebra timing
sweep was not needed to select the next experiment. Native algebraic acceptance
has passed; wider performance remains unmeasured.

The next higher-priority candidate is [factorized Pack](FACTORIZED-PACK.md).
Its complete Cartesian-binding criterion and group-presence requirement are
proved. It could remove scalar binding enumeration before any carrier does
work. It now needs a native execution experiment using actual Free Join/COLT
structures, with complete-binding controls and explicit checks for missing
branches, empty Events and validation errors. No speedup from that unimplemented
path is claimed here.
