# 11 — Real floats without ambiguous set identity

Status: **normative proposal for the successor; no new float implementation, proof, benchmark, or test run is claimed here**. This is a first-class scalar across the schema, fact codec, query language, indexes, laws, clients, snapshots, and log replay. It is not `bytes<8>` with a conversion example.

## 1. One deliberately chosen binary64 domain

Call the type `F64`. It contains all finite IEEE-754 binary64 values, both infinities, and one NaN. Collapse both zero signs to positive zero and every NaN sign/payload/signaling encoding to the quiet NaN bit pattern `0x7ff8000000000000`.

```text
canonical(bits):
  if exponent=all-ones and fraction!=0: 0x7ff8000000000000
  else if magnitude=0:                 0x0000000000000000
  else:                                bits
```

`F64` is a private `u64` wrapper constructed by this function or a canonical parser. It derives equality/hash from canonical bits, not host `f64::PartialEq`. All semantic equality surfaces agree: fact identity, joins, projections, group keys, selections, schema determinants, literals, parameter sets, and returned answers.

Thus `NaN = NaN` and `-0 = +0` are true **in the database domain**. This is not IEEE's comparison predicate; it is the reflexive equivalence relation a set needs. Applications requiring NaN payloads or signed-zero provenance should store explicit raw bits in a bytes field. They must not receive different hidden answers from an index and a scan.

Relational ordering is total:

```text
-Infinity < negative finite < 0 < positive finite < +Infinity < NaN
```

All `<`, `<=`, `>`, `>=`, min, max, range indexes, and query ordering comparisons use this order. Provide `is_nan` and `is_finite`; do not make an application guess why a NaN passes a total-order range condition. `NaN > +Infinity` is deliberately true. There is no epsilon equality for keys, and no transitive-equivalence claim about “close enough.” Approximate comparisons, if later added, are ordinary predicates, never identity.

For a canonical bit word `b`, the order key is `~b` for negative values and `b ^ 0x8000000000000000` otherwise, encoded big-endian. Canonical NaN sorts last. Plain canonical payload bits and order-key bytes have different purposes and different named helpers. Neither decoder accepts arbitrary NaN payloads as an already canonical wire value.

## 2. Small arithmetic roster, fully specified

The 1.0 scalar roster is unary negation, addition, subtraction, multiplication, division, explicit numeric casts, comparisons, `is_nan`, and `is_finite`. Aggregates are count, sum, mean, min, and max. No sqrt/libm/transcendental library, implicit decimal, SIMD fast-math mode, or approximate aggregate family is required to ship this type.

**Sum and mean are retained product requirements, not optional follow-up work.** Benchmark and improve their exact implementation on application-sized groups and real join inputs; do not replace their semantics with repeated native addition to win a microbenchmark. The performance contract in [40](40-performance-contract.md) includes both, separately from the cost of first-class scalar storage/comparison.

For each arithmetic node:

1. Evaluate its operands as their canonical scalar values.
2. Compute the IEEE binary64 operation with **round to nearest, ties to even**, gradual underflow, and no contraction with adjacent operations.
3. Canonicalize the result before it enters another database value, a comparison, an accumulator input, or a key.

An expression tree is semantically significant. `(a+b)+c` is not freely reassociated into `a+(b+c)`. `a*b+c` does not silently become an FMA. Division by zero, overflow, invalid operations, and NaNs have IEEE results, followed by canonicalization: `1/0 = +Infinity`, `0/0 = NaN`, `Infinity-Infinity = NaN`. Floating overflow is not the integer `Overflow` error.

No general schema `CHECK` framework is needed to require finite stored values. The existing selected containment law can express it: declare an empty closed relation `EmptyFloat(value:F64)`, then require `project(value, select(value in {NaN,+Infinity,-Infinity}, R)) ⊆ EmptyFloat`. The selector is the existing finite literal-set selection and the target is deliberately empty; any nonfinite R fact violates containment. This describes the admitted descriptor shape, not current compilable macro spelling for the new type. Query predicates `is_nan`/`is_finite` alone are not schema guarantees. Application-only validation must not be advertised as an engine-enforced law.

Zero-sign normalization is performed after **each** operation, not only after writing the final result. Therefore `1 / neg(0)` is `+Infinity` under this quotient domain. This cost of losing signed zero is intentional and testable. It is better than letting host intermediate registers and stored values obey different semantics without explanation.

No implicit mixed numeric promotion: `I64`, `U64`, and `F64` are different domains. Provide named casts:

- `to_f64`: correctly rounded conversion of an integer; documents that integers beyond 2^53 may lose precision.
- `to_f64_exact`: refusal if conversion loses information.
- `to_i64_exact` / `to_u64_exact`: finite, integral, in-range only; NaN, infinities, fractions, and boundary overflow refuse.

A later truncating or saturating cast must be named separately. Never reuse Rust's saturating `as` behavior as an undocumented database law.

Arithmetic-producing expressions belong to typed **nonrecursive relation-expression stages**, including a derived stage whose outputs another nonrecursive stage consumes. This replaces the old terminal-output-only restriction; it does not put partial arithmetic directly into relational filters. Each stage preserves the operand tree, canonicalization and error contract in [12](12-query-execution.md). Frozen finite nonrecursive outputs may be inputs to a recursive query, but the recursive feedback cycle itself cannot aggregate or create values: `x+1` cannot manufacture a new recursive value on each round.

## 3. The embedding process does not own our rounding contract

Canonical NaN alone does not make native arithmetic deterministic. Other native code in a Rust/Node process can change rounding mode; x86 MXCSR can enable FTZ/DAZ; ARM FPCR can flush subnormals. Dropping the public C API does not remove these embedding risks.

Use one small **numerical execution guard** around a whole numerical engine operation, not per tuple. On supported x86-64 and ARM64 targets it saves the calling thread's relevant floating control/status state, installs round-to-nearest-even with gradual underflow and nontrapping IEEE behavior, and restores the saved state on every normal/error/unwind exit. Include architecture-specific rounding and flush-control bits; do not assume an ARM register layout is x86 MXCSR with renamed fields. No host callback is invoked while this guard is active.

Hosted workers likewise establish this environment at entry and restore/verify it at operation boundaries. A worker's default at thread creation is not sufficient evidence after foreign native libraries execute. Constructors from raw bits perform integer normalization and do not need to execute a signaling NaN as a floating operation.

The guard must be paired with compiler constraints: no `fast-math`, no implicit reassociation, no implicit FMA, and no unsupported minimum CPU feature. Verify optimized code and cross-architecture bit results. `volatile` is not a substitute for a numerical specification. Direct `F64` host arithmetic, if publicly exposed, must enter the same guard; merely constructing an `F64` from an application-computed `f64` preserves the supplied bits but cannot retroactively guarantee how the application calculated them.

Unsupported numeric environments must fail platform qualification or use an independently validated software implementation of this **small** operation set. Do not invent a whole software math library to avoid auditing two control-register guards. Uncoordinated signal handlers or foreign code that mutates the thread's floating state during an engine operation fall outside the safe embedding contract; document that boundary.

## 4. Sum and mean are deterministic operations on a set

Repeated native `f64` addition is order-dependent. A query planner, a different hash-table iteration order, a spill to disk, or an ARM/x86 difference must not change the meaning of `sum`. “Use a stable sort first” would make a specific execution order part of numerical semantics and require extra sorting. Kahan summation reduces error but does not establish order-independent results.

Choose **exact accumulation followed by one rounding**:

- Every finite binary64 value is an integer multiple of 2^-1074. Accumulate the signed integer exactly.
- Represent the numerical total as exactly one case: `Finite(exact_integer) | PositiveInfinity | NegativeInfinity | NaN`. Do not retain independent flags admitting redundant combinations.
- Keep an exact binding count alongside that sum type, so cardinality-overflow behavior does not depend on when a nonfinite value appears. Empty accumulator is a separate identity; a nonempty accumulator has a nonzero count. Nonfinite states do not retain irrelevant finite limbs.
- Merging adds counts and combines the numerical sum cases by the small commutative table below.
- `sum` rounds the final exact finite total to binary64 once, ties-to-even, then canonicalizes.
- `mean` rounds the exact rational `(finite total × 2^-1074) / count` once. Do not implement it as `rounded_sum / count`; finite mean must not overflow just because the intermediate rounded sum would.

For up to `u64::MAX` contributing bindings, a signed 2,176-bit finite accumulator (34 64-bit limbs) is sufficient: a scaled single finite binary64 magnitude is <2^2098 and the count adds <64 bits. Prove that bound and carry/sign arithmetic; do not accept the limb count from this paragraph as a proof. Include the canonical sum-case tag/count/version in the internal scratch encoding. An attempted count beyond the representable limit returns explicit `CardinalityOverflow` regardless of numerical sum case; it is not a database-size policy cap.

| Merge numerical totals | Finite(b) | +Infinity | -Infinity | NaN |
| --- | --- | --- | --- | --- |
| Finite(a) | Finite(a+b), exact | +Infinity | -Infinity | NaN |
| +Infinity | +Infinity | +Infinity | NaN | NaN |
| -Infinity | -Infinity | NaN | -Infinity | NaN |
| NaN | NaN | NaN | NaN | NaN |

Prove this canonical merge is associative and commutative (with the empty identity), including count/error behavior. It combines **disjoint partitions of the already distinct binding set**. It is not idempotent: merging the same finite partial state twice doubles its contribution/count, and the accumulator alone carries no binding provenance capable of detecting that replay. Exact set deduplication must therefore happen before accumulation. There is no numerical state meaning “NaN and both infinities and an obsolete finite sum.”

This approximately few-hundred-byte state per float sum/mean group is a real memory cost: 34 limbs alone are 272 bytes per finite group before count, key and table overhead. Use compact no-group/one-group paths and existing constant-group batch specialization where the binding proof permits; share one exact total/count when the same binding/argument requests both sum and mean. A query that never requests these aggregates pays no accumulator cost. Charge group capacity before allocation; temporary LMDB is the bounded overflow path, not the default route for a student's small dashboard. Do not add parallel aggregation merely because the merge operation supports it. The sibling benchmark's fast exact **u128 integer** sums are useful evidence for the existing integer kernels, not evidence that a 2,176-bit F64 accumulator has their throughput.

Special cases are fixed, not implementation-dependent:

| Inputs in a nonempty group | Sum | Mean |
| --- | --- | --- |
| Any NaN | NaN | NaN |
| Both infinity signs | NaN | NaN |
| Only positive infinity, with any finite values | +Infinity | +Infinity |
| Only negative infinity, with any finite values | -Infinity | -Infinity |
| Finite values | Once-rounded exact total | Once-rounded exact rational average |
| Exact zero total | +0 | +0 |

Min/max select by the total database order: min of `{1,NaN}` is 1; max is NaN. They do not imitate a host `fmin` NaN-elision rule.

Preserve the existing set-query group rule: **no binding means no group**. A global aggregate over empty input emits an empty answer set, not a fabricated zero/NaN row. All above cases concern nonempty groups. A user wanting a default zero models the default explicitly at the application boundary.

Distinct binding semantics matter: two binding tuples `(entityA, amount=1)` and `(entityB, amount=1)` contribute two amounts; projecting away identity before the aggregate can intentionally leave one distinct input tuple. The optimizer must not deduplicate only numeric arguments when the binding vocabulary still distinguishes them.

An aggregate-derived relation exposes its **final canonical scalar values**, not hidden exact accumulator states. A downstream mean of per-course means averages those once-rounded group means; it is not generally the global mean of the original bindings. Likewise a sum of subgroup sums can differ from one sum over their original union because each subgroup boundary rounded. Naming, inlining or fusing stages cannot erase that rounding boundary, distinct-row grain or an upstream numerical error. Sharing exact sum/count state is permitted within one aggregate stage with the same input binding set and argument; carrying an unrounded state through a public derived scalar would change its meaning.

## 5. All surfaces, not a kernel-only feature

Required in the same release: Rust schema macro and dynamic descriptor; typed/dynamic fact input; literal and parameter validation; closed relation constants; equality/range/negative atoms; keys, containment selections and joins; answer decoding; direct key probes; naive oracle; query explain; logical export; log codec; core Rust/TypeScript values; packaged artifacts and declarations. The public C surface is deleted. The public log API is TypeScript-only; its one internal Rust codec/runtime participates in conformance testing without creating another public SDK. These are typed AST/codec paths, not a new textual query parser.

TypeScript uses `number` and integer domains use `bigint`. Wire/HTTP representations encode the canonical 64-bit payload explicitly; JSON's conversion of NaN/infinity to `null` is not a valid float codec. Freeze the generic JSON form as `{"$f64":"7ff8000000000000"}`: exactly sixteen lowercase hexadecimal digits containing canonical binary64 payload bits, with the same form for finite values. The shared codec rejects noncanonical payloads/alternate spellings. Host NaN payload preservation is not promised.

Application-owned entity IDs remain nominal 128-bit bytes, not numbers; a request ID must likewise never be converted through `number`. No fresh-ID placeholder exists. Schema identity includes the float-domain/encoding version. Old clients or stores do not guess the meaning of a formerly unknown type tag.

### `Interval<F64>`: continuous ranges, compact ordered bounds

Add this to the existing sealed interval element family; do not create a separate float temporal engine. The public value has two canonical F64 endpoints and a distinct schema/codec tag. The canonical payload is **16 bytes**, not a variable-size endpoint tree. Constructors normalize signed zero and reject NaN, then require strict numeric `start < end`. Wire parsing additionally rejects noncanonical endpoint bits. `-Infinity` is legal only as a lower bound and `+Infinity` only as an upper bound; strict ordering enforces those placements. Equal endpoints, including `[-0,+0)`, refuse.

Its denotation is the half-open interval on a **dense numeric line**, with finite binary64 endpoints embedded as their exact rational values. Real-line language is intuitive; exact rational endpoint/order models suffice for the supported proofs and do not require enumerating real numbers. Infinity denotes a missing bound, not a point in the interval. In particular:

| Value or operation | Meaning |
| --- | --- |
| `[0,1)` | All numeric positions from zero inclusive to one exclusive; not a count of binary64 encodings |
| `[a,nextUp(a))`, finite ordered bounds | A valid positive-width interval; no successor arithmetic enters the interval algorithm |
| `[-Infinity,-MAX_FINITE)` | A nonempty left ray even though no finite representable F64 query point lies inside it |
| `[-Infinity,+Infinity)` | The complete numeric line |
| `contains(p)` | Exact comparison after embedding a finite F64 point; false for NaN and either infinity |
| `[a,b)` plus `[b,c)` | Adjacent, so pack coalesces to `[a,c)` |
| End `b`, next start `nextUp(b)` | A real gap; never coalesce merely because the bounds are adjacent machine floats |

This denotation avoids an otherwise hidden contradiction: if points meant *only representable finite floats*, `start < end` would admit the empty set `[-Infinity,-MAX_FINITE)`. Do not mix the dense model with that discrete model in Lean, membership, coverage or test oracles. Integer intervals retain their existing discrete point domain; generic endpoint-order algorithms can serve both without claiming their measures are identical.

The ordered execution words use the non-NaN F64 order-key mapping. Allen's thirteen relations, overlap, intersection, selected containment/coverage, and pack/coalescing depend only on exact endpoint order and reuse the existing scalar/SIMD kernel structure. Empty intersections produce no interval fact. Float endpoints are never compared with an epsilon, and no integer/float interval coercion is implicit. The generic interval value itself need not expose a public lexicographic `Ord`; a physical index order is not another interval predicate.

For a bounded float interval, `length` computes the exact endpoint difference rounded once to canonical F64 under the numerical guard. A rounded overflow to infinity is `MeasureOverflow`; e.g. `[-MAX_FINITE,+MAX_FINITE)` is bounded but its F64 length overflows. Either nonfinite bound gives `UnboundedMeasure`. These are different errors. Length is not a number of representable points, and is not silently narrowed to `u64`. **No `FixedInterval<F64>` or float-width schema compression ships:** rounded addition can collapse a positive requested width at a large start, and does not establish constant exact length. Applications supply two checked bounds.

Capacity remains chapter 10's exact nonnegative **whole scalar-key group** measure. A row may contain a float interval, but neither that field nor the new interval algebra adds pointwise temporal occupancy, interval grouping projections or simultaneous weighted coverage. Existing bounded integer-duration weights remain exact; float-length/approximate capacity is refused at schema validation, not quietly judged with rounding. Useful continuous query ranges and grouped admission are separate capabilities.

## 6. Optimizer law table

| Candidate rewrite | 1.0 policy | Reason/test |
| --- | --- | --- |
| Canonicalize an already canonical F64 | Allowed | Idempotence |
| Replace equal canonical literals / reorder relational total-order comparisons | Allowed with typed order law | Equality and order agree |
| Reorder bindings fed into exact sum/mean, or merge exact accumulators | Allowed over disjoint deduplicated binding partitions | Exact integer/rational denotation; no early rounding or repeated partial-state contribution |
| Replace `sum` with repeated native `+` | Forbidden | `{1e16,1,-1e16}` distinguishes results |
| Deduplicate equal aggregate arguments when full bindings differ | Forbidden without binding witness | Changes set-of-bindings multiplicity |
| `(a+b)+c → a+(b+c)` | Forbidden | Rounding counterexample |
| `a*b+c → fma(a,b,c)` | Forbidden | One rounding versus two |
| `x-x → 0` | Forbidden without finite-domain proof | Infinity and NaN |
| `x/x → 1` | Forbidden without nonzero finite-domain proof | Zero, infinities, NaN |
| `x*0 → 0` | Forbidden without finite-domain proof | Infinity and NaN |
| Push float aggregate through join/union | Only with binding-equivalence proof | Both numerical and set-input meanings matter |
| Fuse aggregate-derived stages or push consumer predicates into them | Only with stage-denotation and error/rounding equivalence | A rounded derived scalar is not an exact partial accumulator; filtering a consumer cannot hide an upstream required error |

Represent such conditions in typed operator metadata/witnesses used by validation and optimization. Do not build a universal algebraic theorem engine or rely on a blacklist of source strings. The reference evaluator retains the original typed expression tree and group bindings.

## 7. Blocking float test obligations

All are **future acceptance obligations, not tests executed in this proposal**. Root release gates must include them across supported architectures and fresh artifacts.

| Gate | Required independent assertion |
| --- | --- |
| `F-CANON` | Canonicalization idempotence; exhaustive sign/exponent boundary classes plus random 64-bit patterns; every NaN class and both zeros normalize; parser refuses alternative wire forms |
| `F-GOLDEN` | Checked-in bit/byte fixtures for ±0, ±1, smallest/largest subnormal, smallest normal, largest finite, ±infinity, multiple signaling/quiet NaNs, exact/inexact integer boundaries |
| `F-ORDER` | Total-order antisymmetry/transitivity/equality consistency; byte order equals logical order; scan, point/range index, sort/comparison and membership agree |
| `F-ARITH` | Compare +,-,*,/,negation/casts against an independent correctly rounded integer/rational or established software IEEE reference; include tie/overflow/underflow/subnormal/cancellation boundaries |
| `F-ENV` | Set every supported nondefault rounding mode and FTZ/DAZ/FPCR flush setting before entry; operation bits remain specified; host environment restored on success, error, cancellation and unwind |
| `F-AGG` | All permutations and disjoint partitions/merge trees give identical exact sum/mean bits; overlapping/replayed binding inputs deduplicate before accumulation; negative fixture shows partial-state merge itself is not idempotent; RAM/scratch and finite-mean-overflow cases agree |
| `F-SET` | Equal NaNs/zeros deduplicate consistently; different entity bindings with same amount contribute separately; projection-before-group versus naming-only grain, union/negation and aggregate-derived consumers agree with the independent staged evaluator |
| `F-OPT-NEG` | Each forbidden rewrite above has a minimal counterexample that actually distinguishes it; optimized engine matches unoptimized evaluator, including staged subgroup rounding, mean-of-means versus global mean, and consumer filters that would hide an upstream cast/measure/aggregate error |
| `F-CROSS` | Identical scalar/interval/aggregate bit fixtures on Apple Silicon macOS, qualified Graviton Linux ARM64 and Linux x86-64 through fresh Rust/Node builds; log command/export fixtures compare the internal Rust codec/runtime with its public TypeScript surface; no C artifact |
| `F-WIRE` | Log replay and checkpoint roundtrip preserve canonical facts/receipts; old tag/version refuses; JS mutable buffer or JSON special-value loss cannot enter a sealed command |
| `F-RESOURCE` | Tiny group budgets force RAM→LMDB transition during float accumulation; no early rounding, missed group, uncharged accumulator, or partial result on disk-full/cancel |
| `F-PROOF` | Lean representation/equality/order and exact-accumulator/merge/rounding lemmas complete; implementation linkage independently tested as described in 13 |
| `F-INTERVAL` | Dense endpoint oracle versus constructor/codec/index/Allen/pack/coverage paths; ±0, NaN refusal, both infinity bounds, adjacent representable endpoints, `[-Infinity,-MAX_FINITE)`, finite-length overflow, nonfinite membership false; no FixedInterval<F64>, float-length capacity or accidental pointwise-occupancy admission escape |

Golden arithmetic should include `{1e16,1,-1e16}` summing to exactly 1 before rounding, `{MAX_FINITE,MAX_FINITE}` whose mean is `MAX_FINITE` despite sum overflow, and `{MIN_SUBNORMAL,MIN_SUBNORMAL}`. Cross-architecture comparison is bitwise, never an epsilon assertion. Decimal source spellings in tests must be paired with exact expected bit patterns to avoid a shared parser masking an arithmetic bug.

No claim of deterministic numerical execution is earned until the altered-host-environment, optimizer-negative, independent differential, disk-path, and packaged cross-architecture gates all pass. “We canonicalized NaN” is only the first representation invariant.
