# Exact expectation query heads

Final `Expectation(value, when, given)` heads aggregate ordinary integer columns
or exact ratios of integer columns, owned exact constants, and finite/family
functions into a checked payoff on the evidence. Rust macros, native IR,
Free Join, Node and the TypeScript SDK share this contract. This implements one
M5/M6 integration boundary; the full M0–M8 proposal remains the target.

```rust
let query = bumbledb::query!(Game {
    (action, utility: Expectation(value, when, given), witnesses: Count) |
        Payoff(action, value, when), Evidence(action, given);
});
```

The names stand for application relations and columns. `value` accepts a
body-bound `i64`/`u64` variable or `Ratio(numerator, denominator)`; `when` and
`given` are Events. Event-producing interiors can construct their regions before
this final aggregate. Binary64 is refused rather than silently treated as exact.
Closed vocabulary/reference codes are refused as payoffs: join an explicit
ordinary numeric value mapping first. Both ratio operands must be body-bound
ordinary `i64`/`u64` columns, independently signed or unsigned. This ratio is
exact rational construction; it has no integer rounding mode or binary64 step.

```rust
let query = bumbledb::query!(Game {
    (action, utility: Expectation(Ratio(numerator, denominator), when, given)) |
        Payoff(action, numerator, denominator, when), Evidence(action, given);
});
```

Native IR uses `PayoffExpr::Integer`, `PayoffExpr::Ratio`, or
`PayoffExpr::Imported(PayoffImport)` for the value slot.
Negative divisors and `u64::MAX / -1` retain their exact mathematical values.
Zero denominators refuse, including `0/0` on empty or redundant regions. A
zero-mass evidence observation is distinct from an invalid payoff expression.
## Owned constants and local functions

```rust
let utility = PayoffImport::capture(
    ImportedPayoff::Family(function),
    SourceDescriptorLimits::default(),
    &mut arithmetic,
)?;
let query = bumbledb::query!(Game {
    use payoff utility = &utility;
    (action, mean: Expectation(Payoff(utility), when, given)) |
        Payoff(action, when), Evidence(action, given);
});
```

`ImportedPayoff` accepts `Rational(ExactRational)`, `Finite(FiniteFunction)` and
`Family(FamilyFunction)`. Imports retain source owners and bounded BERA/BESC
presentations; `from_bytes` reconstructs their mathematical values. Every other
BESC role refuses, including a partial parameter-only function. Construct a total
source-owned family function explicitly before importing it. This adds no
persisted field type, callback, implicit source or descriptor version.

Different union arms can supply different imports or ordinary column payoffs.
If any function participates, the head admits a [function cover](event-function-covers.md):
functions must agree at every actual world in their overlap on the evidence.
They need not agree globally. Scalars become total constant functions; when a
family participates, finite functions are promoted to family functions. This
promotion preserves source and actual values and does not choose a prior.

For example, use `p` on `p ≤ 1/2` and `1−p` on `p ≥ 1/2`. Their overlap is the
single boundary point, where both give `1/2`; the query admits the tent-shaped
payoff. Adding payoff `7` at that point refuses, regardless of its prior mass.

## Admission comes before measurement

The remaining nonaggregate head outputs identify each group. Every expectation
head collects its own roster within that group. Repeated rows with the same
numerical value contribute the union of their regions; they do not multiply
utility or probability. Equal positive signed and unsigned integers agree even
when different rule arms supply them. Fraction presentations such as `1/2` and
`2/4` merge by normalized exact value before overlap/coverage admission.

All participating `when`, `given` and imported function operands must align to
one source context.
Empty regions, supplied zeros and later rows after saturation still participate.
One aligned, equal evidence Event is required throughout the group. After
equal-value union, clip the regions to evidence and prove that distinct values
are disjoint and cover it completely. Overlap outside evidence is harmless.
Missing coverage is an error even on zero-mass worlds. Missing values never
default to zero. An empty input has no group.

A structurally valid payoff on zero-mass evidence produces an impossible
observation, not a missing row. A missing law is an error, including for zero
payoffs or empty evidence. For parameter families, the conditional result keeps
its exact positive-evidence domain; no prior or independence is inferred.

For two draws sharing bias `p`, utilities 2 and 8 according to the second draw,
conditioned on the first draw, produce numerator `2p + 6p²`, evidence mass `p`
and conditional function `2 + 6p` on `p > 0`. The endpoint hole survives.

## Representation and Free Join

The complete-binding computed sink gathers inline scratch claims carrying a
logical group token, a tagged four-word payoff payload, both Event keys and
written operand provenance. The payload is either a raw signed ratio or an index
into the collector's owned import registry. Canonical identical presentations
share an import; cumulative encoded import bytes are capped at 16 MiB. The exact group key includes the head ordinal, so several
expectations can share one collector without mixing their rosters. Wide group
keys use the existing exact scratch maps; claims spill at 4,096 entries or about
4 MiB of retained collector data. These are collector thresholds, not a complete
retained-memory policy for the execution registry or output.

The first finish pass aligns all contexts against the canonical minimum source
descriptor of each group. Only after this complete pass can evidence or
partition/function-cover errors terminate admission. Function failures have operand
ordinal 2 and `EventOperandSource::PayoffImport`; they carry the offending BESC
without inventing a variable. The expected source remains a full BEVT marker. Operational refusals never claim a
complete fault set. The second pass requires common evidence and unions equal
raw presentations. Identical imported functions therefore retain their original
function and union of supplied regions, rather than one patch per relational
witness. It retains those claims until final observation admission.

A stable token occupies one output binding word. It is constant for all rows of
the logical group, allowing the existing scalar aggregate sink to count/sum the
original bindings alongside several expectations. It also composes with the
existing single Pack head. Existing restrictions on mixing Pack with scalar
folds remain unchanged. Suffix skipping and fused leaf scans stay forbidden:
one witness does not establish payoff coverage or complete operand validation.

Finalization uses one arithmetic budget to normalize **all** payoff rosters,
union equal scalar values, and admit every partition or function cover before
any probability or expectation contraction. The same budget then covers every
contraction; normalization cannot reset it per row or group. Publication is
atomic. Failed internal appends preserve the initialized answer prefix and
discard only the failed append.

## Owned results and SDK

`AnswerValue::Expectation(&ExpectationAnswer)` exposes `given()`, `payoff()`,
`function()` and `value()`. `ExpectationPayoff` distinguishes scalar partitions,
finite covers and family covers. `partition()` and `values()` return `Some` only
for scalar rosters, preserving supplied zeros and empty buckets. `function()`
distinguishes the finite and family carriers. Assembled functions extend by zero
outside evidence; that extension is never additional supplied coverage.

`ExpectationValue::Fixed` retains the signed observation, numerator, evidence
mass and optional exact rational. `Parameter` retains a finite function on a
parameter source; `Family` retains a family function. Both retain the exact raw
functions, partial conditional result and positive-evidence domain. Owners
survive database/snapshot/query release and result paging.

Answer equality retains source, evidence and encoded assembled-function identity;
equal means do not identify different observables. Family encodings preserve
arithmetic presentations and are **not** a canonical quotient by numerical
equivalence. Explicit `FamilyFunction::equivalent` performs the checked numerical
comparison under a work budget. This infallible result identity neither proves
inequivalence nor replaces that solver operation.

```typescript
const observed = query(Game).rule((r) => {
  const p = v(Payoff)
  return r.match(Payoff, p).find({
    action: p.action,
    utility: expectation(p.value, p.when, p.given),
    witnesses: r.count()
  })
})
```

SDK answers discriminate `law: "fixed" | "parameter"` and retain the admitted
`payoffKind: "scalar" | "finite" | "family"`. Scalar answers retain `payoffs`;
function answers retain `patches` with owned regions and functions. All retain
the assembled function, evidence, numerator/mass and result/domain. Native
delivery shares arithmetic and a cumulative 16 MiB observation payload budget
with probability outputs per collection or page. `expectationResult` describes
an imported query output; it is not a stored schema field. Authoring stays pure.
`expectation(payoffRatio(p.numerator, p.denominator), p.when, p.given)` constructs
the same exact ratio head. The owned ratio expression survives description
export/import and query snapshots. Native transport retains the old integer
ordinal spelling and adds the strict `{kind: "ratio", numerator, denominator}`
and `{kind: "imported", bytes}` value forms. Pass an owned `ExactRational`,
`FiniteFunction`, or `FamilyFunction` directly to `expectation` in the SDK.
JS ingress copies bytes; one native admission budget and cumulative byte limit
cover all payoff imports in the query before preparation. Descriptions copy
every operand and reject unknown fields or invalid ordinals; native validation
checks column types and reference roles.

Observation interiors and arithmetic over their query results still refuse
explicitly. This boundary must expand; it does not narrow the proposal.

## Evidence and limits

Tests exercise integer and fractional values over all 256 pairs of four-world
payoff/evidence regions on resident and fallback joins against an independent weighted-world oracle, duplicates,
multiple heads with scalar folds, signed/unsigned union arms with Pack, zero-mass
gaps, overlap, changing evidence, absent groups, missing laws, extreme integers,
wide group keys, forced spill, reaim, cancellation, owned family endpoint holes
and append rollback. SDK checks cover descriptions, ownership, pages and both laws.
Ratio checks additionally cover equal presentations, negative denominators,
extreme signed/unsigned values, foreign empty operands before division errors,
static divisor/refcode rejection, empty `0/0`, cumulative arithmetic limits,
early-admission rollback and partial family results.

Function-import tests enumerate all 64 evidence/patch-mask combinations on a
two-world source on both Free Join paths, with a zero-mass possible world. Further
checks cover arbitrary-size literals, mixed finite/family/scalar arms, exact
parameter-boundary agreement/refusal, grouped import retention across spill and
reaim, reset/owner release, shared admission budgets, malformed/wrong-role imports,
complete imported-operand faults, append rollback, descriptions and paged SDK
cover replay. These test integration; they do not add a performance claim.

`QueryExpectation.lean` supplies thirty-two reference reports for witness unions,
unique coverage, clipping, grouping, missing values, partial conditioning,
presentation invariance, participating definedness and the equivalence between
functional admission and structural coverage plus pointwise agreement. Constant
scalar patches, local replacement, union arms and the unique selected payoff
are connected to the same reference contract.
They do not prove Rust refinement, the scratch collector or transport. No
performance qualification or scalar-SQL observation oracle is claimed.
