# Exact arithmetic over observations

The host and query layers compute exact partial numbers from completed
probability and expectation answers. Rust `Number` heads and TypeScript
`NumberExpr` programs retain their original observations in an immutable
derivation, through grouping, joins, stage forwarding and owned result transport.

```rust
let p = ObservationNumber::probability(
    probability.clone(), ObservationComponent::Value, limits, &mut arithmetic,
)?;
let utility = ObservationNumber::expectation(
    expectation.clone(), ObservationComponent::Value, limits, &mut arithmetic,
)?;
let score = p.apply(NumberOp::Multiply, &utility, limits, &mut arithmetic)?;
let positive = score.where_sign(PolynomialSigns::POSITIVE, limits, &mut arithmetic)?;
```

`score.expression()` retains the written operands, including the original Events,
evidence, laws and payoff functions. `positive.predicate().view()` exposes the
exact true, false and undefined cases. `possibly()` asks for a true assignment;
`always()` requires truth on the entire ambient domain, including definedness.
These are explicit numerical judgments. They do not turn a partial answer into
a Boolean by accident.

## Number domain and operators

`PartialNumber` is a fixed optional rational or an owned `ParameterFunction`.
Fixed values have one assignment; `None` means undefined. Parameter functions
retain their inhabited ambient domain and exact defined region. The native
solver remains univariate. This does not restrict the general reference
semantics or close the multivariate/general semialgebraic gate.

All binary operations first validate both complete numerical operands under the
caller's shared exact-arithmetic budget. A fixed number lifts to a constant on
the other operand's parameter domain. A fixed undefined value lifts to a
nowhere-defined function. Two parameter operands require the same named ambient
domain; numerical-domain restriction is explicit through `on_domain`. Distinct
parameter names are never silently equated or assigned a joint prior.

| Operation | Value and defined domain |
| --- | --- |
| Add, subtract, multiply | Exact pointwise result on the intersection of operand defined domains |
| Divide | That intersection with every zero-divisor assignment removed |
| Negate, absolute value | Exact result on the original defined domain |
| Natural power | Original defined domain, including exponent zero |
| Min, max | Pointwise branch selection on the common defined domain, with exact boundary points |
| Numerical equivalence | Equality of partial functions, including their defined domains |
| Sign mask / comparison | Exact true, false and undefined partition |

`p / p` equals one only where p is defined and nonzero. `p - p`, `0 * p`, and
`p ^ 0` keep p's original holes. Arithmetic bit/work limits, capacity refusal and
cancellation remain errors; they never become undefined numerical answers.

Multiplying two measured probabilities computes their numerical product. It
does not assert that this product measures the conjunction. A conjunction must
still be formed as an Event and contracted under the captured joint law.
Likewise, comparing observations from different fixed sources compares their
numbers without merging those sources. The derivation retains both.

## Truth partitions

`NumberPredicate` is immutable and constructed only from checked operations.
Its parameter view partitions the entire ambient domain into three disjoint
regions: holds, fails, undefined. Fixed predicates have the corresponding
`Some(true)`, `Some(false)`, or `None` value.

Negation exchanges holds and fails, retaining undefined. All sixteen `BoolOp4`
operations have a strict partial lift: both predicates must be defined. Even
a constant truth table retains its input holes. This is the ordinary algebra of
partial functions. No short-circuit three-valued policy is inferred. `always`
requires an empty false region and an empty undefined region; nowhere-defined
evidence therefore never makes a universal claim true by vacuity.

Numerical equivalence and pointwise equality are distinct. An undefined partial
value can be numerically equivalent to itself while its equality predicate is
undefined. Neither judgment equates its original Event/provenance with another
observation that happens to have the same values.

## Ownership and bounds

`ObservationNumber` owns an `Arc` derivation whose leaves are exact literals or
completed probability/expectation answers. Selecting Value, Numerator or
EvidenceMass retains the entire leaf observation. Binary, unary, power and
explicit domain-restriction nodes retain their input derivations. A restriction
changes the numerical ambient domain; it does not condition or replace the
original source law. `ObservationPredicate` now owns an inspectable sign/negation/Boolean/domain
derivation alongside the checked partition. Its [portable predicate and explicit
guard API](event-observation-predicates.md) retains every numerical leaf.

Expanded expression size and depth are checked before publishing a node. The
default node limit is 65,536; the depth ceiling is 256 even if a caller requests
more, bounding recursive inspection and destruction. Arithmetic/solver limits
validate operands anew, share the supplied work counter and preserve cancellation.
These limits do not constitute the complete retained-byte policy required by
the proposal. Portable numerical derivations now have the replay transport below;
this does not make them stored schema fields.

## Portable derivations

`ObservationNumberImport::capture(&number, limits, &mut arithmetic)` produces
owned BENO v1 data. `from_bytes` reconstructs the entire numerical expression.
`value()` returns its checked number; `bytes()` exposes the portable encoding.
The immutable import implements equality and hashing by its complete encoded
derivation. Equality ignores Arc sharing and allocation order. It distinguishes
different written expressions, selected observation components, sources and
indexed payoff rosters, even when their partial numerical values agree. Use
numerical equivalence explicitly when that is the intended judgment.

Each probability leaf carries both original BEVT Events, including their law,
support, parameter domain and guards. Each expectation leaf carries the evidence
and its complete scalar partition or finite/family function cover. Indexed empty
cells, zero payoffs, possible zero-mass outcomes and full functions on patches
remain present. In particular, a patch's function and region outside the evidence
are retained. A domain-restriction node changes the number's ambient domain while
keeping the original observation and source domain in its child.

The envelope carries operations and leaves rather than a claimed numerical
result. Admission decodes every source, rechecks partition/cover admission,
contracts the observations and replays every numerical operator. It then
reencodes the result and requires exactly the supplied bytes. This rejects
presentation aliases, including scalar cells that would be clipped by their
evidence during partition admission. Capture goes through the same replay path;
it owns reconstructed source managers with the same portable identities.

All exact decoding, contraction, arithmetic and reencoding share the caller's
`ExactArithmetic`. `ObservationNumberCodecLimits` combines numerical bounds with
`SourceDescriptorLimits`. The descriptor byte bound applies to the whole envelope;
its item bound counts expression nodes, payload blobs and indexed payoff entries.
Nested source codecs also enforce their own bounds. Numerical nodes retain the
expanded-size limit and hard depth ceiling of 256. Both codec traversals use
explicit work stacks, so malformed nesting never recurses on the host stack.
These admission bounds do not claim a complete aggregate retained-memory policy.

BENO v1 uses a prefix expression after `BENO` and version byte `1`. Node tags
0–7 denote literal, probability, expectation, binary operation, negate, absolute
value, natural power and domain restriction. Component tags select value,
numerator or evidence mass. Binary tags select add, subtract, multiply, divide,
min or max in that order. Counts, blob lengths and exponents are little-endian
u32 values; payloads reuse BERA, BEVT, BEPR and BESC. Operands are written left
before right. Every root must consume the whole envelope. These remain query
values, separate from stored schema fields.

## Evidence and remaining integration

Core tests compare fixed and parameter arithmetic with a rational oracle,
including independent poles, division zeros, exact min/max boundaries, powers,
all eight sign masks, all sixteen strict Boolean lifts, irrational roots,
domain/scope refusals, hidden oversized operands, shared arithmetic exhaustion,
function cell limits and cancellation.

Database consumers extract completed staged probability and expectation answers
on resident/cursor paths and retain derivations after owner closure. They check
distinct equal-valued observations, mixed signed arithmetic, zero-mass evidence,
shared-parameter holes, explicit restriction and expression bounds.

`PartialNumbers.lean` gives 26 reference reports for defined-domain
intersection, division exclusions, powers, truth partitions, strict Boolean
lifting, explicit quantification, restriction and provenance retention. It
assumes a mathematical assignment domain; it does not prove the native solver,
Rust arithmetic, concrete derivation representation or query lowering.

Native replay tests cover every arithmetic operator and observation component,
scalar/finite/family payoff leaves, source and component identity, equal values
with different derivations, independence from Arc sharing, indexed empty cells,
zero-mass outcomes, partial shared parameters and explicit domain restriction.
Adversarial cases cover every truncated prefix, unknown tags, trailing bytes,
invalid lengths, wrong source roles, missing laws, mismatched contexts,
partition gaps/overlap/aliases, hidden foreign empty cells, oversized masked
payoffs, source-law bounds, cumulative work, cancellation and depth 256/257.
Database consumers also round-trip composed observations after owner closure.

`NumberReplay.lean` adds eight reference reports for value-stack isolation,
continuation composition, left/right operator order, exactly one root result,
ordered source retention, malformed stack refusal and the distinction between
value equality and source identity. Leaf admission and operations are abstract;
these reports do not verify the Rust byte parser, bounds or nested codecs.

## Numerical queries

For a relation `Trial { id: u64, value: i64, when: event, given: event }`:

```rust
let scored = query!(Numbers {
    interior observed(id, value, p: Probability(when, given)) |
        Trial(id, value, when, given);
    interior scores(id, score: Number(Value(p) * Integer(value))) |
        observed(id, value, p);
    (score, count: Count) | scores(id, score);
});
```

`Probability` finishes before `Value(p)` selects its conditional value.
`Numerator(p)` and `EvidenceMass(p)` select the other exact components.
All three selectors also accept a completed `Expectation`. `Integer(value)`
reads an i64/u64 column exactly; a bare variable inside `Number(...)` reads an
already completed Number. Selection retains the whole original observation.

The numerical grammar provides `+`, `-`, `*`, `/`, unary minus, `Min`, `Max`,
`Abs` and `Pow(value, natural_u32)`. Exact integer literals follow the macro's
ordinary i64/u64 spelling; rational constants can be written as division.
`use number name = &import` supplies a checked `ObservationNumberImport` for
`Imported(name)`. `use number_domain name = &domain` supplies a `NumberDomain`
for `OnDomain(expression, name)`. Domain restriction is explicit and preserves
inherited holes; it never changes the original observation's source.

`FindTerm::Number(NumberExpr)` produces a distinct query type. Numbers can be
forwarded through named/imported stages and projection-only recursion, bound
in identity joins and antijoins, grouped, counted and returned alongside new
observations. Stored scalar folds, ordering comparisons, Event operators and
external scalar parameters cannot implicitly consume them. Numerical predicate
and guard/refinement query operators remain a separate unfinished gate.

Grouping uses canonical BENO identity **before** an aggregate sees the row.
Computing a per-row placeholder and replacing it after aggregation would split
one logical group. Grouping only on a rational answer would merge distinct
sources. The execution registry assigns one word to each complete derivation;
Free Join and spilled rows use these ordinary identity tokens. For example,
three scores of 7/2 from sources A, A and B form groups of sizes two and one.

Numerical heads run once for each complete surviving binding, after interval
piece generation and before projection or grouping. Every required producer
finishes before its consumers; a later filter cannot hide its failure. Stage
execution temporarily owns the observation registry and cumulative arithmetic
budget, returning both before observation contraction. Ordinary errors restore
ownership; re-execution clears abandoned state after an unwind. Exact work is
not refunded at a stage boundary. Cancellation, exhaustion and partial outputs
remain query failures rather than invented numeric values.

`AnswerValue::Number` borrows an owned `ObservationNumberImport` in the answer
buffer. Copies and results retained after database closure own their derivations.
Native numerical programs allow at most 65,536 written nodes and depth 256;
macro and Node/SDK authoring use depth 128 and 4,096 nodes. Imported derivations
have their own BENO limits. All numerical execution and observation contraction
share one exact work counter per execution. These bounds are not an aggregate
retained-memory policy.

## TypeScript

```ts
const scored = query(Theory).rule((r) => {
    const c = v(observed)
    return r.match(observed, c).find({
        id: c.id,
        score: NumberExpr.multiply(NumberExpr.value(c.p), NumberExpr.integer(c.value))
    })
})
```

The builder provides `value`, `numerator`, `evidenceMass`, `integer`, `literal`,
`imported`, `add`, `subtract`, `multiply`, `divide`, `min`, `max`, `negate`, `abs`,
`pow` and `onDomain`. `literal` accepts an owned `ExactRational`; `imported`
accepts an owned `ObservationNumber`; `onDomain` takes a `ParameterDomain`.
These are pure constructors. They do not run a second numerical evaluator.

`NumberAnswer` contains `number`, the complete portable derivation, and either
`law: "fixed"` with an exact rational or null, or `law: "parameter"` with a partial
`ParameterFunction`, its defined region and inhabited ambient domain. Use
`ObservationNumber.toBytes`/`fromBytes` to copy envelopes. Pure imports check only
the envelope; native query admission replays every source and operation, including
imports in an unreachable arm. Observation and payoff imports share the query's
worker admission budget. Delivery has one cumulative exact-work/byte budget.

`numberResult` supplies the query-only result descriptor for
`queryFromDescription`. Descriptions, imported variables and result pages retain
the numerical type and own all wire buffers. The engine uses BENO identity; equal
partial values alone do not authorize an equality join on derivations.

## Query evidence and remaining work

Rust consumers cover canonicalization before Count, derivation identity joins
and antijoins, projection unions, imported stages, recursive forwarding, mixed
producer heads, rebind/release/reuse, domain failures before downstream filters,
and forbidden coercions on both resident and forced cursor paths. Internal
checks force spill and failed appends, and test one cumulative budget across
numerical evaluation and observation contraction. Arithmetic-budget tests retain
spent work on normal drop, cancellation and unwind. Macro fixtures pin import,
domain, exponent, operator and excessive-chain diagnostics.

Raw Node checks exercise opaque numerical result replay and malformed shapes,
roles, versions, shared buffers and cyclic expressions. SDK consumers cover all
operators/components, portable imports, grouped derivation identities, typed
stages/descriptions, closed-owner results, partial family holes, incompatible
domains and corrupt imports behind empty bodies.

`NumberQueries.lean` adds ten reference reports for canonical grouping and
counts, staged round trips, counterexamples to fresh-token/value-only grouping,
and cumulative primitive charging across successful and failed stages. Faithful
encoding and abstract primitive costs are explicit premises. These are not
proofs of the Rust compiler, registry, allocator, SIMD kernels or bridge.

Numerical predicates must retain their exact domains; turning a parameter region
into an Event still requires query guard/refinement construction. Multivariate
solving, provider provenance and the other open M0–M8 gates remain in force.
No release or version bump.
