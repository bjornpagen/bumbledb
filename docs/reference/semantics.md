# Semantic contract (permanent)

This page is the authoritative product denotation. Physical layout, SDK
spelling and performance work implement these meanings; they do not
narrow them. A symbolic authoring expression is not an admitted program:
the native binder establishes kinds against the verified schema even when
there are no input rows. Lean proves the mathematical laws of this
language; it does not prove LMDB, S3, native ownership or host floating
control merely by naming a file.

## Canonical relations, identities and changes

Each ordinary relation is a set of complete typed canonical tuples.
Closed relations are immutable sealed extensions. Exact canonical value
equality is the logical truth. Membership fingerprints and routing hashes
only find candidates; collisions cannot merge, lose, or delete the wrong
fact. A compact physical row ID is local indirection, not a generated
business identity or a cross-incarnation identifier.

Booleans, integer domains, F64, application Id128, fixed-width bytes,
text and supported interval domains each have one canonical encoding and
checked construction boundary. Invalid UTF-8, malformed arity/type,
alternate already-canonical encodings and unknown tags refuse. Text stays
inline in durable values; an execution token is not a durable dictionary
ID. Id128 is ordinary application-owned data; reuse the same ID across
command retries. Keys, not a hash or UUID issuance theorem, enforce
schema uniqueness.

Within one sealed change set, duplicate exact additions/removals
disappear and addition wins when the same exact fact occurs on both
sides. The proposed state is `(base minus removals) union additions`.
Normalization is order-independent and idempotent. Separate published
commands retain their authoritative order. Reads used to decide a write
require an explicit expected-state witness; blind writes do not acquire
serializable read intent by accident.

A private candidate contains all proposed rows, including mutually
conflicting proposals. Incremental judgment may assume a lawful parent
only when its constructor established that premise. Private staging and
offline verification run complete-state judgment, never empty-delta
incremental skipping. Judgment sees one proposed final state. A rejected
candidate never becomes visible. Diagnostic evidence names genuine
violating statements/rows and is bounded with explicit truncation.

## Laws and measures

Normalize laws to keys, containment and grouped capacity with shared
projection metadata. Count is nonnegative unit weight. Explicit
whole-number weights and supported bounded integer durations are exact
nonnegative measures over distinct matching child facts. Zero weight is a
member with zero contribution, not absence. Whole-group duration sums are
not simultaneous temporal occupancy. Missing-parent vacuity and required
parent existence are separate laws.

Pointwise key/coverage laws over intervals retain their interval
semantics and competing groups. Exact bounded scalar encodings can
accelerate group selection; intervals with the same scalar determinant
are not duplicate keys merely because they share a bucket. Full candidate
conflict representation is required in every physical format.

No float-length/approximate upper-capacity law, arbitrary user predicate
interpreter or weighted-bag engine is introduced. Unsupported
combinations refuse at schema compilation with a clear semantic reason.
Equivalent supported schema spellings normalize identically.

## Query denotation

Queries are typed AST values; no parser. Positive joins, supported
disjunction/union, bound negation, predicates, projection, grouping and
interval operators obey exact set semantics. A full binding can
distinguish two entities with equal numerical amounts; both bindings
contribute. Union deduplication is over the specified tuple grain.

Nonrecursive named stages are typed relations. Names alone do not force
materialization. An aggregate stage exposes final canonical scalars, not
hidden accumulator state. Rewrites preserve binding grain, stage errors
and rounding boundaries. An outer filter cannot hide an error a required
inner stage must report. Keep an independent staged evaluator.

No bindings means no aggregate group, including the global aggregate:
empty input emits no result row. Count and integer folds have explicit
overflow errors; no wraparound or saturation by accident.

Recursion is the selected positive linear, finite-active-domain fragment.
No aggregation, negation, mutual nonlinear feedback or fresh
arithmetic-created values through the recursive cycle. Seen, frontier and
accumulated relations remain exact sets with bounded spill paths.

## F64: one quotient of binary64

Canonicalize every NaN encoding to `0x7ff8000000000000` and both zeros to
positive zero. Equality and hash derive from canonical bits. Relational
order is `-Infinity < negative finite < 0 < positive finite < +Infinity < NaN`.
Min/max use this order. Canonical payload bytes and order-preserving key
bytes are distinct named encodings.

The scalar arithmetic roster is negation, add/subtract/multiply/divide,
explicit numeric casts, comparisons, `is_nan` and `is_finite`. Each node
computes IEEE nearest-even with gradual underflow, then canonicalizes.
No reassociation or implicit FMA. No implicit I64/U64/F64 promotion.
`to_f64_exact` must not lose integer precision. Public numeric APIs must
preserve this behavior under altered host floating control state.

Exact sum accumulates finite inputs as an exact signed integer in units
of 2^-1074. Sum rounds the exact total once. Mean rounds the exact
rational total/count once, never `rounded_sum / count`. Required
discriminators include catastrophic cancellation `{1e16,1,-1e16}`, mean
of two maximum finite values, subnormals/ties, overflow/casts and host
rounding/flush changes. Assert canonical bits, not epsilon proximity.

## Intervals

All intervals are half-open and nonempty. Integer domains remain
discrete. `Interval<F64>` has two canonical F64 endpoints; NaN is
forbidden; signed zeros normalize; strict numeric `start < end` is
required. Infinities are bounds, not members. Adjacent intervals
`[a,b)` and `[b,c)` coalesce. No FixedInterval<F64> or rounded
float-duration capacity is selected.

## Wire and proof boundary

Host TypeScript uses number for F64, bigint for integers, and canonical
lowercase hex for Id128 at the native cell boundary. Generic JSON float
artifacts use canonical-bit tagged data (`{"$f64":"7ff8000000000000"}`).
Exact native/artifact grammar has one authoritative codec.

Semilattice laws justify exact set union/dedup and monotone frontier
reasoning. They do not make all schema-valid states union-closed. Keep
local LMDB single-writer and hosted conditional publication; no free
multiwriter claim follows from set semantics alone.

Permanent Lean/bridge scope (L19 authored; L21 maintains the
qualification binding): `lean/Bumbledb/Bridge.lean`,
`lean/proof-bridge-ledger.md`, `lean/correspondence.md`,
`scripts/lean.sh`, `scripts/spec-census.sh`. Those prove and catalog
current constructors. They do not prove LMDB, S3, native lifetimes, or
host FP control by naming a file. Exact `dyn` counts and wording-ban
census are deleted — not a proof.

Independent oracles for G03/G04/G07 and D04/D05/D19/D26 are
`judge_final_state` (`crates/bumbledb/src/schema/judge.rs`),
`crates/bumbledb-bench/src/naive/successor/staged.rs`, and
`crates/bumbledb-bench/src/closure/history_model.rs`. The production
planner is not an oracle. L20 owns seven executable `C-*` ids in
`bumbledb-bench` `correspondence::OWNED_CASES` (cargo tests, not
`scripts/lean.sh`): `C-D04-collision-bytes`, `C-D19-cancel`,
`C-D19-mean-once`, `C-D19-merge-not-idemp`, `C-G03-mutable-support`,
`C-G03-add-wins`, `C-G03-raw-commute`.

Identity/surface goldens are `python3 scripts/spec-gen.py --check`
against `crates/bumbledb-log/conformance/v3`. That is a wire-byte
fixpoint, not an authority theorem. Census no longer runs those
goldens.

See [behavioral-obligations.md](behavioral-obligations.md) for the `E-*`,
`F-*`, `Q-*` and `P-*` schedules that discriminate this contract.
