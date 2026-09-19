# Factor relational computations through a query separator

This extends the [first native Pack experiment](FACTORIZED-PACK.md) from Boolean
combinations to composition and left residual. The new tree is isolated under
`relation-pack-src/`; the preceding source trees, executable and evidence remain
frozen. Native correctness now passes 11,904 relational executions across six
carriers, plus 12,288 separator-check cases and the preceding semantic suites.
The [main comparison](RELATIONAL-PACK-MEASUREMENTS.md) also passes 48 serial
processes and 384 configurations. The six-process smoke phase is retained
separately; its single-sample timings do not select a schedule or carrier.

## The two programs

At a fixed scalar group and Event owner, let the two branches supply binary
relations `R[i]` and `S[j]` on compatible legal state faces.

| Requested grouped result | Left reduction | Right reduction | Final operation |
| --- | --- | --- | --- |
| `union i,j: Compose(R[i],S[j])` | union of R | union of S | Compose |
| `intersection i,j: LeftResidual(R[i],S[j])` | union of R | intersection of S | LeftResidual |

The second row computes the largest continuation satisfying every row-pair
obligation. It is an intersection aggregate with its own contract, not an
alternative implementation of the first row's union aggregate. A union of
residuals does not license the second rewrite.

In Coup terms, the first program joins possible actions with possible replies
to construct their reachable outcomes. The second program collects transitions
and required overall behavior, then constructs the permissible continuations.
Neither program chooses a strategy, observes a probability or assumes independent
players. A useful available continuation is the residual intersected with
actual availability, with its domain checked separately.

Composition keeps one shared intermediate state. Source parameters are also
shared: a transition possible only in environment zero cannot compose with a
transition possible only in environment one. The environment remains a free
coordinate; the relational kernel eliminates only the named state face.

## Where the scalar certificate comes from

The [certificate checker](relation-pack-src/separator.rs) consumes the actual
`NormalizedQuery` used to build the native two-branch plan. Given a proposed
occurrence partition and separator variables, it verifies:

- Both blocks are nonempty, disjoint and cover the occurrence roster exactly.
- Their shared variables are exactly the separator.
- Occurrences are pure positive stored relations, without point bindings or
  local filters; global residuals, anti-probes and dead-query markers are absent.

Fixing the separator then lets one branch assignment be combined with any
assignment from the other. The [Lean proof](join-factorization/QuerySeparator.lean)
constructs that combined assignment and proves it preserves the keys and both
local predicates. It also proves the double residual aggregate and retains an
environment counterexample. All six final reports are axiom-free. The initial
proof checked successfully but four reports used propositional extensionality;
its source and reports are retained under `results/query-separator-initial/`.

This is a conservative sufficient certificate. The checker validates a proposed
partition; it does not yet search for optimal separators or use schema FDs to
admit otherwise coupled variable sets. Rejected local filters may be safe after
an incidence analysis. A cross-branch condition needs its own treatment. The
earlier exact rectangularity theorem remains the semantic criterion; this
syntactic check is one way to establish it.

## The native data path

The [executor](relation-pack-src/relation_pack.rs) compares two schedules in one
binary. The complete path emits every scalar pair, performs the typed operation,
then folds the result. The factored path scans each branch using actual Free Join,
stages roots by `(group, owner)`, validates participating rows and relation roles,
reduces the branches, and constructs another ordinary Free Join over summaries.

For G populated groups and fanout f, complete execution emits Gf² bindings;
factored execution emits 2Gf branch rows and G final bindings. It creates 2G
summary rows. Both report Gf² original bindings. Whole duplicate rows are removed
by the normal image builder; distinct row keys carrying equal roots still count.

The per-group result is `Option<Event>`. A present empty relation remains a row;
missing branches produce no group. Residual folds do not turn absent groups into
full relations. All participating roots are checked even after saturation,
including the requirement that a binary relation not depend on the scratch face.
Unmatched invalid roots remain unevaluated. Diagnostics are unordered fault sets.

The checked legal relation layer supplies owner-stamped values, role admission,
support gates, face maps, composition and residuals. Input rows use the fixture's
stable owner label; a fresh benchmark arena explicitly resolves and stamps the
published roots into its own checked owner. This is still a lab binding adapter,
not persisted Event fields or a production owner registry.

The timer charges participating validation, role admission, root staging,
branch reductions, summary planning/images/COLTs, final joining, typed operations
and exact output counts. Input setup and cloning the input arena are separate;
input COLTs are primed in both schedules. Warm runs retain learned Event roots
and memos, while rebuilding summaries. Reported staging bytes sum vector
capacities used across the query, including short-lived resolved-role vectors;
they are neither retained bytes nor peak allocation. Map nodes, planner state
and images have additional costs. The Event/memo and COLT estimates stay separate.

## Independent checks and workload boundary

The [oracle](relation-pack-src/relation_pack_checks.rs) explicitly enumerates
deduplicated matching row pairs and computes each composition/residual using
state matrices. It retains one intermediate-state witness and one environment.
Its oracle cache shares identical pair computations but does not apply the
factorized query rewrite.
The [correctness record](results/relation-pack-correctness.json) retains 54
reports: six new carrier reports at 1,984 native executions each, one separator
report, and 47 reports from the preceding suites in the same executable.

Cases cover every subset of two rows per branch, irregular groups, whole-row
duplicates, equal roots on distinct row keys, unknown roots, foreign owner
labels, published roots with an invalid relation role, and unmatched bad rows
made participating. Both programs, schedules, memo settings, coordinate layouts
and repeated execution are checked. Dedicated cases make both operands nonempty
while composition is empty because the middle states or environments disagree.

The timing fixture is a structured transition/obligation workload on two legal
state domains selected by a shared environment. It uses 16 groups, finite state
codes with holes and two coordinate layouts. It is not a full Coup simulator.
Each legal state can semantically stand for a correlated world; the fixture
only tests this bounded abstract state space and does not choose a sufficient
information state for every Coup strategy.

## Native results and the layout reversal

The [main comparison](RELATIONAL-PACK-MEASUREMENTS.md) retains seven samples for
each of 384 configurations: six carriers, two state widths, two layouts, two
fanouts, two typed programs, two memo settings and two schedules. All 192 fresh
schedule pairs improve. All 96 warm fanout-eight pairs also improve. At fanout
two, every memo-enabled warm pair loses to staging, while every memo-disabled
pair improves. The algebra licenses the rewrite; it does not eliminate the need
to cost it.

The [independently shuffled repeat](RELATIONAL-PACK-REPEAT.md) reverses schedule
order for dense and packed512 at width five, retaining eleven samples in 64
configurations. All 32 fresh pairs improve. The same eight small warm pairs
with outer memo lose; all remaining warm pairs improve.

For dense, bit-major, fanout eight and memo enabled, the repeated composition
query improves **2.523 → 0.463 ms** fresh and **0.0750 → 0.0268 ms** warm.
The residual query improves **4.513 → 0.608 ms** fresh and **0.0797 → 0.0272 ms**
warm. The main sweep's retained Event/memo estimates fall from 6.92 to 3.31 MB
for composition and 9.74 to 3.51 MB for the residual. At this fanout, 1,024
complete emissions become 256 branch emissions and 16 final bindings.

The representation consequence is more interesting than a single speedup.
For packed512 composition at width five/fanout eight with memo, the repeat gives:

| Schedule | Face-major | Bit-major |
| --- | ---: | ---: |
| Complete pairs, fresh | 39.502 ms | 5.048 ms |
| Factored branches, fresh | 0.526 ms | 0.995 ms |

The preferred layout reverses. Branch reduction changes the functions that the
carrier must construct, so a representation ranking taken before that rewrite
can pick the wrong working order afterward. These are measured layout-specific
workloads, not a claim that face-major is generally preferable. Stable Event
identity and explicit face maps should permit the planner to choose a working
representation without changing the mathematical value.

Dense is the strongest fresh control in almost all of this bounded fixture.
The face-major residual at width five/fanout eight with memo is effectively a
close dense/packed comparison in the repeat: 0.586 versus 0.581 ms. This does not
justify selecting a universal carrier, especially for symbolic spaces that
cannot be enumerated. The scheduling result survives across all six carriers.

## Literature boundary

FAQ [arXiv:1504.04044v7](https://arxiv.org/abs/1504.04044), §1.2 and §5.1.2,
ties factor scopes to variable incidence and folds factors by distributivity.
The new separator proof establishes the relevant local-assignment premise.
FAQ's commutative-multiplication assumption does not permit reversing relational
composition. The ordered relation laws are proved separately, using concrete
typed predicates and the relation-algebra interpretation described by
Desharnais, Möller and Struth [arXiv:cs/0310054v1](https://arxiv.org/abs/cs/0310054).

Free Join [primary source](https://arxiv.org/pdf/2301.10841),
§4.4, explicitly connects trie union/product structure with factorized databases
and compressed output. This experiment instead reduces typed Event factors
before expanding their scalar combinations. It uses the existing executor for
all scans and joins but does not claim the production executor performs arbitrary
semiring aggregation or has acquired suffix-skipping capabilities.

## Shelved follow-up: participation index

Research closed before this competitor was validated. The
[draft status](PARTICIPATION-STATUS.md) records the cancelled build; the following
is a retained experimental hypothesis, not a measured result or required next task.

The staged vectors retain every branch root until participation is known. A
competing execution structure can retain a key/participation index instead:

1. Scan the right branch for its `(group, owner)` roster and exact row counts,
   without resolving Event values.
2. Scan the left branch, reducing only keys present on the right and recording
   left presence/counts and all participating faults.
3. Scan the right branch again, reducing only keys with a left witness.
4. Join the two reduced summaries as before.

This trades an additional native branch scan for eliminating the root vectors.
At uniform fanout it emits `3Gf + G` rows versus staging's `2Gf + G`; its state
scales with groups rather than all staged roots. Every participating input must
still be resolved, including after a fold saturates. A group with a fault can
skip arithmetic after failure, but not further fault collection. The comparison
must retain group presence, multiplicity, checked roles and unordered errors.

This is a proposed competitor, not a measured improvement. It also does not
grant the ordinary sink permission to skip suffixes: an eventual COLT prefix
reduction would require a separate capability backed by this participation and
operator certificate. Measure the three-scan structure before changing the
executor or declaring the current staging allocation intrinsic to Event.
