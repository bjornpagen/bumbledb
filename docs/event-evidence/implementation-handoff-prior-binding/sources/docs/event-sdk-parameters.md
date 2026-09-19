# Exact regions and partial functions in the TypeScript SDK

`ParameterRegion`, `ParameterDomain`, `ParameterFunction` and `AlgebraicRoot`
expose the existing native univariate solver through owned, lazy, cancellable
operations. They are the parameter layer for family construction and observation.
They are not stored Event fields, source laws or implicit priors. One parameter
name denotes one actual unknown real value throughout the algebra.

```ts
import { Effect } from "effect"
import {
  ExactPolynomial as P, ExactRational as Q,
  ParameterRegion as R, ParameterDomain as D,
  ParameterFunction as F, PolynomialSigns as S
} from "@bjornpagen/bumbledb"

const conditional = Effect.gen(function* () {
  const name = new Uint8Array(32).fill(3)
  const p = yield* P.parameter(name)
  const one = yield* P.constant(yield* Q.fraction(1n))
  const domain = yield* D.new(yield* R.and(
    yield* R.whereSign(name, p, S.nonNegative),
    yield* R.whereSign(name, yield* P.subtract(one, p), S.nonNegative)
  )) // exactly [0,1], including both endpoints

  // Explicit example: two conditionally independent trials sharing bias p.
  // Joint mass p² divided by evidence mass p.
  const result = yield* F.ratio(domain, yield* P.pow(p, 2n), p)
  const quarter = yield* F.at(result, yield* Q.fraction(1n, 4n)) // exact 1/4
  const impossibleEvidence = yield* F.at(result, yield* Q.fraction(0n)) // null
  const possibleEvidence = yield* F.definedOn(result) // exactly (0,1]
  return { result, quarter, impossibleEvidence, possibleEvidence }
})
```

The numerator/evidence expressions in this example are supplied explicitly.
The API does not infer trial independence or construct a measured family. It
retains the distinction that matters to future observations: cancelling `p²/p`
does not define a conditional probability where evidence has zero mass. Likewise,
`p/p` differs from total one, and multiplying it by zero retains its hole.

## Regions and inhabited domains

| Operation | Contract |
| --- | --- |
| `R.full(name)`, `R.empty(name)` | The entire real line or empty set with one captured 32-byte parameter name |
| `R.whereSign(name, polynomial, mask)` | Exact polynomial sign region on that real line |
| `R.apply(mask, a, b)` | All 16 Boolean set functions; bit `((a << 1) \| b)` selects membership |
| `R.and`, `or`, `xor`, `difference`, `complement` | Ordinary set operations, with complement relative to the real line |
| `R.equivalent`, `included`, `isEmpty`, `isFull` | Exact set judgments |
| `R.containsRational`, `containsRoot` | Exact membership at a rational or real algebraic number |
| `R.witness` | Owned rational/algebraic witness, or `null` exactly for an empty set |
| `R.describe` | Captured parameter, ascending exact boundaries and alternating sector/point membership |
| `D.new(region)` | Admit a nonempty region as an ambient domain |
| `D.asRegion(domain)` | Recheck inhabitation and return the underlying region |

`PolynomialSigns` supplies `none=0n`, `negative=1n`, `zero=2n`,
`nonPositive=3n`, `positive=4n`, `nonZero=5n`, `nonNegative=6n`, `any=7n`.
Sign solving accepts only the captured parameter and constants; foreign names
refuse even for empty/full answers or ignored Boolean operands. It does not
silently restrict a real parameter to [0,1].

`ParameterRegionDescription` has `{ parameter, boundaries, membership }`.
For `n` boundaries, membership has `2n+1` Boolean entries: the open sector before
the first root, the root itself, the next open sector, and so on. The first/last
sectors extend to infinity. Thus [0,1) has roots [0,1] and membership
`[false, true, true, false, false]`. Disconnected regions, open endpoints and
irrational singleton sets are preserved exactly.

`RealWitness` is `{ kind: "rational", value: ExactRational }` or
`{ kind: "algebraic", value: AlgebraicRoot }`. An algebraic witness can denote a
rational number; the tag describes its representation, not irrationality.
There is no rational grid approximation. Operational refusal is an error, never
`null`, empty set or an approximate witness.

## Partial functions

| Operation | Contract |
| --- | --- |
| `F.ratio(ambient, numerator, denominator)` | Defined exactly where the denominator is nonzero in the ambient |
| `F.pieces(ambient, pieces)` | Disjoint partial rational pieces; uncovered points remain undefined |
| `F.domain`, `definedOn` | Distinct inhabited ambient domain and possibly empty defined region |
| `F.at(function, rational)` | Exact rational value, or `null` at an undefined/outside point |
| `F.whereSign(function, mask)` | Sign region excluding every hole, even for `any` |
| `F.isNowhereDefined` | Whether the defined region is empty |
| `F.restrict(function, region)` | Restrict defined points while retaining the ambient |
| `F.onDomain(function, domain)` | Explicitly narrow the inhabited ambient; extension refuses |
| `F.add`, `subtract`, `multiply`, `divide` | Pointwise partial arithmetic on compatible ambients; both operands' holes survive |
| `F.equivalent` | Equal defined regions and values; incompatible ambients refuse |
| `F.describe` | Owned ambient, defined union and original guarded expressions for every retained piece |

Each `ParameterFunctionPiece` is `{ numerator, denominator, defined }`. Its
effective region is the intersection of the ambient, the supplied region and
nonzero denominator. All inputs are admitted before empty pieces are dropped;
overlap of effective pieces refuses, even when the values agree. An empty roster
is a nowhere-defined function with an inhabited ambient. No missing piece means
zero. This differs deliberately from total `FiniteFunction`/`FamilyFunction`.

Descriptions reconstruct through `F.pieces(description.ambient, description.pieces)`.
They retain numerator/denominator expressions and inherited holes. Encoded BESC
identity is descriptor identity; use `equivalent` for numerical partial-function
equality. Evaluation at irrational arguments is not exposed as a scalar result;
exact sign regions and algebraic membership already support exact judgments there.

## Numerical algebraic roots

`AlgebraicRoot.isolate(name, polynomial)` returns every distinct real root in
ascending order. Multiplicities do not duplicate roots; the zero polynomial
refuses because it has infinitely many roots. `fromInterval(name, polynomial,
lower, upper)` checks exactly one root in an open isolating interval and requires
nonroot endpoints. Equal endpoints instead require that exact rational to be a
root. Neither function guesses a nearby solution.

`compare` and `compareRational` return -1, 0 or 1 as JavaScript numbers.
`rationalBetween(a, b)` requires `a < b` and returns a strict rational separator.
`describe` returns `{ parameter, polynomial, lower, upper }` with exact coefficients
and bounds. `sign(root, polynomial)` evaluates only the sign, without rounding.

BEAR is a **numerical** identity: canonical minimal polynomial plus root ordinal.
It does not capture a source name. The `parameter` in its description is its
canonical formal indeterminate; `sign` requires that indeterminate (or a constant).
An authored expression can explicitly use `ExactPolynomial.substitute` to replace
its variable by the described formal parameter. Other variables still refuse.
Numeric root comparison and region membership do not require matching formal
names. The BEPR region separately retains the actual captured source parameter.

## Transport, work bounds and qualification

All four types provide pure `fromBytes`, copied `toBytes`, ownership predicates
and native `validate`. BEPR v1 carries regions/domains, BEAR v1 carries roots,
and BESC v2 role 0 carries partial parameter functions. Existing BESC v1 finite
roles are unchanged. Version and role are checked together so v2 role 0 cannot
masquerade as a finite function. A domain's pure import checks only its envelope;
each native use checks inhabitation. Wrappers and descriptions remain owned after
runtime release. Byte arrays in descriptions are independent mutable copies;
records and arrays are frozen. Pure import/Effect construction load no addon.

The bridge performs shape checks and copying. Native constructors own solving,
arithmetic, domain admission and normalization. One `ExactArithmetic` budget spans
every nested operand decode, computation and output encoding. Requests have a
16 MiB combined byte limit and at most 16,384 input blobs. Authored functions
allow at most 4,093 pieces, within the existing 4,096-item BESC descriptor limit.
The native polynomial, root, cell, descriptor and traversal limits still apply.
Combined root-list and function-description encoded blobs are bounded to 16 MiB;
region descriptions fit the canonical BEPR bound. Root descriptions separately
bound their polynomial/endpoints to 16 MiB plus the fixed 32-byte name. JS object
overhead and aggregate retained memory are outside these operation bounds.

Seven SDK tests cover all 16 set operators, all 8 function sign masks, disconnected
sets, open endpoints, irrational singletons, exact roots, typed role separation,
partial arithmetic, reconstruction, malformed data, input-copy boundaries and
late cancelled-result reclamation. The persisted consumer stores a BESC descriptor
in an ordinary string field, retrieves it through a relational join after reopen,
and evaluates it after every database/runtime owner is released. This demonstrates
portable host functions; it does not claim function-valued query observation heads.
Three native bridge tests check nested shared arithmetic exhaustion, cancellation,
role confusion, unused malformed pieces and combined output limits.

Existing [parameter](event-parameter-domains.md),
[root](event-algebraic-roots.md), [family function](event-family-functions.md)
and [transport](event-source-descriptors.md) tests/proofs supply the denotational
contracts. This SDK boundary adds integration evidence, not Rust refinement proofs.
[Live family construction and owned observations](event-sdk-families.md) now
extend this parameter layer. Complete family dynamics consumers, prior integration,
multivariate solving and all remaining M0–M8 gates stay open. No release or version change.
