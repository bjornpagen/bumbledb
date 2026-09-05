# Semantics that the physical rewrite must preserve

This rewritten packet preserves the selected semantic decisions; no finding in this pass authorizes weakening them. C1–C9 implement these meanings; physical simplification must not narrow them. It is normative target behavior, not verification evidence. Every physical optimization and SDK spelling must implement this contract. A symbolic authoring expression is not yet an admitted program: the native binder establishes its kinds against the verified schema even when there are no input rows. The core chapter identifies current gaps; the proof bridge must refer to current implementations, not deleted commit machinery.

## Canonical relations, identities and changes

Each ordinary relation is a set of complete typed canonical tuples. Closed relations are immutable sealed extensions. Exact canonical value equality is the logical truth. Membership fingerprints and routing hashes only find candidates; collisions cannot merge, lose, or delete the wrong fact. A compact physical row ID is local indirection, not a generated business identity or a cross-incarnation identifier.

Booleans, integer domains, F64, application Id128, fixed-width bytes, text and supported interval domains each have one canonical encoding and checked construction boundary. Invalid UTF-8, malformed arity/type, alternate already-canonical encodings and unknown tags refuse. Text stays inline in durable values; an execution token is not a durable dictionary ID. Id128 is ordinary application-owned data; reuse the same ID across command retries. Keys, not a hash or UUID issuance theorem, enforce schema uniqueness.

Within one sealed change set, duplicate exact additions/removals disappear and addition wins when the same exact fact occurs on both sides. The proposed state is `(base minus removals) union additions`. Normalization is order-independent and idempotent. Separate published commands retain their authoritative order. Reads used to decide a write require an explicit expected-state witness; blind writes do not acquire serializable read intent by accident.

A private candidate contains all proposed rows, including mutually conflicting proposals. Incremental judgment may assume a lawful parent only when its constructor established that premise. Private staging and offline verification run complete-state judgment, never empty-delta incremental skipping. Judgment sees one proposed final state. A rejected candidate never becomes visible. Diagnostic evidence names genuine violating statements/rows and is bounded with explicit truncation; do not fabricate a first successful row or arbitrary hash cohabitant as a violator. Receipt-only metadata for a rejected log command is committed under the same writer parent discipline.

## Laws and measures

Normalize laws to keys, containment and grouped capacity with shared projection metadata. Count is nonnegative unit weight. Explicit whole-number weights and supported bounded integer durations are exact nonnegative measures over distinct matching child facts. Zero weight is a member with zero contribution, not absence. Whole-group duration sums are not simultaneous temporal occupancy. Missing-parent vacuity and required parent existence are separate laws; add containment when the model requires existence.

Pointwise key/coverage laws over intervals retain their interval semantics and competing groups. Exact bounded scalar encodings can accelerate group selection; intervals with the same scalar determinant are not duplicate keys merely because they share a bucket. Full candidate conflict representation is required in every physical format.

No float-length/approximate upper-capacity law, arbitrary user predicate interpreter or weighted-bag engine is introduced. Unsupported combinations refuse at schema compilation with a clear semantic reason, not several language-specific spelling bans. Equivalent supported schema spellings normalize identically.

## Query denotation and composition

Queries are typed AST values; no parser. Positive joins, supported disjunction/union, bound negation, predicates, projection, grouping and interval operators obey exact set semantics. A full binding can distinguish two entities with equal numerical amounts; both bindings contribute. A prior projection may intentionally remove that distinction. Union deduplication is over the specified tuple grain, not whatever partial key is convenient for a sink.

Nonrecursive named stages are typed relations. Names alone do not force materialization. Computed and aggregate outputs may feed later stages. An aggregate stage exposes final canonical scalars, not hidden accumulator state. Rewrites preserve binding grain, stage errors and rounding boundaries. An outer filter cannot hide an error a required inner stage must report. Keep an independent staged evaluator to arbitrate fusion/pushdown changes.

No bindings means no aggregate group, including the global aggregate: empty input emits no result row. Applications model default zero explicitly. Count and integer folds have explicit overflow errors; no wraparound or saturation by accident.

Recursion is the selected positive linear, finite-active-domain fragment. No aggregation, negation, mutual nonlinear feedback or fresh arithmetic-created values through the recursive cycle. Frozen finite nonrecursive predecessor results may feed it. Seen/frontier/accumulated relations remain exact sets with bounded spill paths. Remove stale arbitrary branch limits only by supplying the promised bounded Boolean/stage representation, not by eagerly expanding a larger Cartesian product of disjuncts.

## F64: one quotient of binary64

Canonicalize every NaN encoding to `0x7ff8000000000000` and both zeros to positive zero. All other binary64 bits retain their value. Equality and hash derive from canonical bits. Thus database NaN equals itself; signed zero has no hidden identity. Applications needing raw payloads store bytes.

Relational order is `-Infinity < negative finite < 0 < positive finite < +Infinity < NaN`. Min/max use this order; they do not silently ignore NaN. Equality, scans, ordered keys, joins, groups and literals agree. Canonical payload bytes and order-preserving key bytes are distinct named encodings; the latter complements negative bit words and flips the sign bit for nonnegative values, in big-endian order. Decode each only under its declared encoding.

The scalar arithmetic roster is negation, add/subtract/multiply/divide, explicit numeric casts, comparisons, `is_nan` and `is_finite`. Each node computes IEEE nearest-even with gradual underflow, then canonicalizes before its result is consumed. No reassociation or implicit FMA. Consequently `1 / neg(0)` is positive infinity in this quotient. Overflow/nonfinite arithmetic has canonical IEEE outcomes, not the integer overflow error.

No implicit I64/U64/F64 promotion. Rounded `to_f64` may lose integer precision; `to_f64_exact` must not. Exact integer casts require finite integral in-range values. Fractions, infinities, NaN and out-of-range values refuse. Public numeric APIs and worker entry points must preserve this behavior under altered host floating control state; one operation-level guard saves/sets/restores rounding/flush/trap state, with compiler settings and architecture-specific verification. No per-tuple guard, fast-math, or reliance on a thread's initial environment.

### Exact sum and mean

Accumulate finite inputs as an exact signed integer in units of 2^-1074, with an exact binding count. Numerical state is exactly one of finite total, positive infinity, negative infinity, or NaN; nonfinite cases do not retain meaningless finite state. Count overflow is checked regardless of numerical case.

Sum rounds the exact total once. Mean rounds the exact rational total/count once, never `rounded_sum / count`. An empty accumulator is distinct from a nonempty zero total. The finite count bound implies a finite limb bound; prove the selected representation rather than trusting a magic limb count. The previous 34×64-bit proposal is a candidate justified by a <2^2098 scaled single magnitude and <64 count bits, not evidence that carry/sign code is correct.

NaN absorbs; opposite infinities yield NaN; a single infinity sign absorbs finite values; exact zero yields positive zero. Merge of disjoint binding partitions is associative/commutative with the empty identity, not idempotent. Deduplicate bindings before accumulation. Share sum/count for sum+mean on the same argument and binding grain within one stage, not across exposed rounded stage boundaries. Charge group capacity, including exact accumulator limbs, before growth; spill without early rounding.

Required discriminators include catastrophic cancellation `{1e16,1,-1e16}`; mean of two maximum finite values; subnormals/ties; overflow/casts; duplicate arguments under distinct entities; subgroup sum vs global sum; and host rounding/flush changes. Assert canonical bits, not epsilon proximity or a self-derived golden.

## Intervals

All intervals are half-open and nonempty. Integer domains remain discrete, retaining the selected maximum-word upper ray endpoint; overflow and the unrepresentable maximum-start ray refuse. A maximum-word point is a defined nonmatch for that interval representation. Fixed-width integer schemas must validate exact declared width on every decode path, including resident image construction.

`Interval<F64>` has two canonical F64 endpoints (16 payload bytes) and denotes a dense numeric interval. NaN is forbidden; signed zeros normalize; strict numeric `start < end` is required. Negative infinity can be the lower unbounded endpoint, positive infinity the upper; infinities are bounds, not members. A nonfinite queried point returns false.

`[a,nextUp(a))` is a positive-width interval. `[-Infinity,-MAX_FINITE)` is nonempty even though no finite representable binary64 query point lies inside. Adjacent intervals `[a,b)` and `[b,c)` coalesce; a gap from `b` to `nextUp(b)` remains a gap. Use endpoint-order algorithms without pretending the domain is a sequence of machine floats.

Allen relations, intersection, containment, coverage and pack/coalescing share the checked endpoint representation. Bounded float length is the once-rounded exact difference; rounded infinity is `MeasureOverflow`, while an infinite bound is `UnboundedMeasure`. No FixedInterval<F64> or rounded float-duration capacity is selected.

## Wire and proof boundary

Host TypeScript uses number for F64, bigint for integers, and canonical lowercase hex for Id128 at the native cell boundary. Generic JSON float artifacts use canonical-bit tagged data (`{"$f64":"7ff8000000000000"}`); ordinary JSON's NaN→null conversion is invalid. Exact native/artifact grammar has one authoritative codec, not hand-maintained twins.

Lean proves the mathematical laws of this admitted language. It does not prove LMDB, S3, native ownership, floating control registers or the production Rust implementation merely by mentioning a filename. Map each theorem premise to current representation and empirical refinement tests; identify trusted substrate assumptions. Remove deleted-path/cosmetic census checks and replace them with meaningful correspondence, not a new table of unverified claims.

Semilattice laws justify exact set union/dedup and monotone frontier reasoning. They do not make all schema-valid states union-closed: two individually valid keyed rows may conflict on union, upper bounds may be exceeded, and deletes require order. Keep local LMDB single-writer and hosted conditional publication; no "free multiwriter" claim follows from set semantics alone.
