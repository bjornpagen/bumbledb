# Factor Pack before enumerating bindings

This rewrite now has an isolated native implementation, independent differential
tests and Lean proofs. It changes the query schedule while preserving each
carrier's representation. The preceding range comparison remains a separate
complete-binding experiment. Production code and its planner are unchanged.

The clover fixture binds a group and owner, then joins three branches whose
remaining scalar variables are disjoint. For one group, let their Event values
be `A[i]`, `B[j]`, and `C[k]`. Its current output is

```text
union over i,j,k: A[i] AND NOT (B[j] OR C[k]).
```

Because every combination of branch rows is a legal scalar join binding, this
is exactly

```text
(union i: A[i]) AND (union j: NOT B[j]) AND (union k: NOT C[k]).
```

All predicates still refer to the same world in the same Event owner. This
factors scalar row enumeration; it does not split the world witness, assume
statistical independence or multiply marginal probabilities. Each branch's
complement remains inside its union. Complementing `union B` would compute a
different query.

For fanout m, the first expression has m³ complete row combinations. The second
has three m-row reductions and two final intersections. This is a structural
count, not a measured runtime prediction. Existing memoization, exact output
counting, input preparation and the cost of each Event operation still matter.

## The necessary certificate

For a general three-way binding relation J, let I₁,I₂,I₃ be its three projections.
The factorization works for every possible assignment of row-local Event values
if and only if

```text
J = I₁ × I₂ × I₃.
```

The [Lean proof](join-factorization/FactorizedPack.lean) establishes that exact
condition, the clover rewrite, a coupled-binding counterexample and the separate
group-existence law. All six reports are axiom-free and recorded in a
[separate proof result](results/factorized-pack-lean.json); they are not yet added
to the 195-report central suite. An independent
[finite oracle](join-factorization/oracle.py) checks all 256 three-Boolean-index
binding relations against all 64 local predicate combinations. Precisely the
28 Cartesian-product relations pass.

This is a join dependency of the binding relation at a fixed group and owner.
The first implementation can derive it from the existing conjunctive query;
it need not add weights or a new schema declaration. Richer dependency-based
rewrites would need their own derivation of the same certificate.

For the existing clover query, the normalized occurrence mappings give the
certificate after group and scope are fixed. The triangle's branches share
additional keys, so the same reduction is not licensed. Cross-branch residuals,
anti-probes or computed validation can also invalidate a putative certificate.
A planner must establish the actual join condition, not infer rectangularity
from common group names or separate nonempty branches.

## An empty Event still has a row

The result needs both a group-presence fact and an Event. A Cartesian branch
product has a row exactly when all three branch row sets are inhabited.
That is independent of whether the resulting Event contains any world.
Three nonempty branches can legitimately produce `Event(empty)` and must still
emit that group. Missing branches produce no joined group unless explicit seeds
supply it. The Lean `present_empty` example isolates this distinction.

A factorized query implementation must therefore preserve an existence bit or
an equivalent ordinary relational witness alongside each Event reduction.
It must resolve and validate every participating owner/field value before
applying algebraic shortcuts. It cannot stop at a full Event and hide a later
validation error. Conversely, an invalid row with no joined companions must
not become an error merely because the new schedule scans its branch.
The unchanged native `ComputedSink` restrictions still apply until this separate
factorization plan has its own proof-carrying execution path.

## Native execution and validation

The [isolated implementation](pack-native-src/pack.rs) uses three single-occurrence
Free Join plans and a normal clover Free Join plan over their summaries. It
accepts only this constructed clover query, whose shared variables are group
and owner, and rejects triangle. It does not claim a general certificate finder.
The single-branch plans retain local row keys: equal Event values on distinct
rows still contribute their original binding multiplicity. Whole duplicate rows
are deduplicated by the engine's ordinary image path.

The temporary representation is a sorted map from `(group, owner)` to three
vectors of Event roots. Once branch presence is known, validate participating
roots, reduce A by union and B/C by intersection, and emit one summary row per
branch. A parallel exact count records the Cartesian multiplicity. Run the
existing final expression over the summary join. Outputs use `Option<Event>`:
`Some(empty)` remains distinct from `None`.

That staging is deliberate. An invalid unmatched row stays unevaluated; a later
invalid participating row is checked even after an accumulator saturates. Both
schedules use the same immutable published-root registry for the fixture owner
and return a sorted, deduplicated set of faults. They do **not** preserve a
first-error order. Two [Lean reports](join-factorization/ParticipatingValidation.lean)
prove participating validity and equality of those unordered fault sets.

The [independent oracle](pack-native-src/pack_checks.rs) deduplicates complete
rows, explicitly enumerates matching triples, and computes output words
directly. Six carriers pass 896 native executions each: all branch-presence
subsets of two rows per branch, irregular multi-group data, duplicate rows,
equal roots on distinct keys, foreign owners, errors hidden behind saturation,
unmatched errors made participating, empty-valued groups and repeated execution.
The broader semantic suite supplies 41 further reports in the same binary.
See [retained correctness evidence](results/pack-native-correctness.json).

The query timer charges scans, validation, staging, Event reductions, summary
planning/image/COLT construction, the final join and exact output counts.
Input-image preparation is separate, and both input COLTs are primed. Fresh
means a fresh input-only Event arena; warm retains learned roots and caches but
still rebuilds summaries. Root-vector capacities and COLT retention are reported
separately from Event/memo estimates; none is a peak allocation measurement.
The new validation work in both schedules means absolute latency must not be
compared directly with the earlier carrier-only harness.

## What the native measurements decide

The [main sweep](PACK-NATIVE-MEASUREMENTS.md) has 36 serial processes, six
carriers, both Coup presentations, fanouts 2/4/8 and memo off/on: 144
configurations and 72 matched schedule pairs, with seven samples each. All
fresh pairs improve; 17 warm pairs lose, all at fanout two. For fanout eight,
32,768 complete emissions become 1,536 branch emissions and 64 final bindings.

An [eleven-sample follow-up](PACK-CROSSOVER-MEASUREMENTS.md) independently
shuffles the dense/packed controls, reverses schedule order at fanouts two and
eight, and adds fanout one. All eight fanout-one pairs lose in both fresh and
warm timing. At fanout two all eight fresh pairs improve, while five warm pairs
lose. All eight fanout-eight pairs improve both fresh and warm.

For dense with memo, the fanout-eight repeat gives compact Coup **7.365 →
0.172 ms** fresh and **2.365 → 0.106 ms** warm. The card-coordinate presentation
gives **47.512 → 0.878 ms** fresh and **2.186 → 0.113 ms** warm. The main sweep's
latter complete fresh median was 154.157 ms; retaining that process variation
matters. Both processes show a large gain, but they do not establish one stable
universal speedup. The retained Event/memo estimate for compact dense falls
from 5.22 to 0.24 MB, without changing the resident representation.

Carrier rankings remain workload-dependent: in the main sweep's factored fresh
queries, compact Coup favors dense; card-coordinate Coup favors packed512 with
outer memo off and dense with it on. This is a scheduling result with matched
carriers, not evidence that a new representation dominates them all.

The conclusion is a costed pair of certified plans. Branch cardinalities,
existing operation reuse, carrier costs, validation and summary construction
all contribute. A fixed rule to always factor would be wrong. A generic cost
model still needs skewed groups, sparse participation and varying query shapes;
this uniform-fanout benchmark does not calibrate one by itself.

## The relational form is larger than Boolean products

For typed relation families `R[i]: X → Y` and `S[j]: Y → Z`, row aggregation
can move outside composition:

```text
union i,j: (R[i] ; S[j]) = (union i: R[i]) ; (union j: S[j]).
```

Both sides retain one shared intermediate world y. The row indices can be
eliminated early; that world cannot be replaced with two separate witnesses.
The [relation proof](join-factorization/RelationPack.lean) makes the interfaces
explicit and includes a counterexample to separate witnesses.

For the left residual `R \\ T`, meaning the largest S with `R ; S ⊆ T`, the
corresponding laws reverse one aggregation:

```text
(union i: R[i]) \\ T       = intersection i: (R[i] \\ T)
R \\ (intersection j: T[j]) = intersection j: (R \\ T[j]).
```

In Coup, first union alternative transitions for a player, then compose with
the opponent's possible replies. For a safety obligation, collect all possible
antecedent transitions and intersect the resulting obligations. These are
constructions of further Events, before any probability observation. They
require the same compatible legal faces and support gates as ordinary relation
operations; they do not authorize erasing coupled support or absent groups.
The four relational Lean reports establish denotational laws, not a native
relational Pack implementation. Together with the six original rewrite reports
and two validation reports, these are twelve separate axiom-free reports; the
central 195-report suite remains unchanged.

## Paper boundary and next implementation gate

FAQ, [arXiv:1504.04044v7](https://arxiv.org/abs/1504.04044v7), §1.2 and §2.2,
formulates elimination over factors and uses distributivity to fold common
factors. Here the factors range over Event regions: union and intersection form
the relevant semiring at a fixed owner. This narrow row-local complement
expression becomes a product of three factors before elimination. It does not
license exchanging arbitrary universal, probability, expectation or mixed
aggregates. The [reading/oracle record](results/factorized-pack-oracle.json)
pins the retained primary source.

FAQ expressly assumes commutative multiplication. Relational composition is
ordered and typed, so its arbitrary ordering rules and runtime bounds do not
transfer automatically. Desharnais, Möller and Struth,
[arXiv:cs/0310054v1](https://arxiv.org/abs/cs/0310054), §§2.1, 2.3 and 2.4,
provide the appropriate connection: distributive multiplication need not commute,
and relations and Boolean matrices are concrete models. Their main abstract
results use idempotent semirings; the proposal does not attribute a quantale-only
theorem to those results. The [reading record](results/pack-native-reading.json)
pins both papers and the scope used here.

The remaining planner work is to derive branch separators from a normalized
query, check cross-branch residuals and anti-probes, preserve each operation's
aggregation direction, and cost the complete and factored schedules. A native
relational Pack experiment must additionally retain typed interfaces, one shared
world witness, legal support and group existence. None of these obligations is
discharged by merely having a fast Boolean carrier.
