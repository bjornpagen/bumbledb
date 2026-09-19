# Exact expectation query heads

Final `Expectation(value, when, given)` heads aggregate ordinary integer columns
or exact ratios of integer columns and Event columns into a checked payoff on the evidence. Rust macros, native IR,
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

Native IR uses `PayoffExpr::Integer` or `PayoffExpr::Ratio` for the value slot.
Negative divisors and `u64::MAX / -1` retain their exact mathematical values.
Zero denominators refuse, including `0/0` on empty or redundant regions. A
zero-mass evidence observation is distinct from an invalid payoff expression.
Arbitrary-precision rational literals and functional payoff query slots remain
follow-on work; this syntax reads bounded integer columns into arbitrary-precision
rational arithmetic.

The host [function-cover API](event-function-covers.md) now supplies exact local
agreement and structural coverage for finite and parameter-dependent function
patches returned by ordinary queries. It does not yet extend this native head.

## Admission comes before measurement

The remaining nonaggregate head outputs identify each group. Every expectation
head collects its own roster within that group. Repeated rows with the same
numerical value contribute the union of their regions; they do not multiply
utility or probability. Equal positive signed and unsigned integers agree even
when different rule arms supply them. Fraction presentations such as `1/2` and
`2/4` merge by normalized exact value before overlap/coverage admission.

All participating `when` and `given` operands must align to one source context.
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
logical group token, payoff sign/numerator/denominator magnitudes, both Event keys and written
operand provenance. The exact group key includes the head ordinal, so several
expectations can share one collector without mixing their rosters. Wide group
keys use the existing exact scratch maps; claims spill at 4,096 entries or about
4 MiB of retained collector data. These are collector thresholds, not a complete
retained-memory policy for the execution registry or output.

The first finish pass aligns all contexts against the canonical minimum source
descriptor of each group. Only after this complete pass can evidence or
partition errors terminate admission. Operational refusals never claim a
complete fault set. The second pass requires common evidence and unions equal
raw presentations. It retains those claims until final observation admission.

A stable token occupies one output binding word. It is constant for all rows of
the logical group, allowing the existing scalar aggregate sink to count/sum the
original bindings alongside several expectations. It also composes with the
existing single Pack head. Existing restrictions on mixing Pack with scalar
folds remain unchanged. Suffix skipping and fused leaf scans stay forbidden:
one witness does not establish payoff coverage or complete operand validation.

Finalization uses one arithmetic budget to normalize **all** payoff rosters,
union equal exact values, and admit every `EventPartition` before any probability
or expectation contraction. The same budget then covers every contraction;
normalization cannot reset it per row or group. Publication is atomic. Failed internal appends
preserve the initialized answer prefix and discard only the failed append.

## Owned results and SDK

`AnswerValue::Expectation(&ExpectationAnswer)` exposes `given()`, `partition()`,
`values()`, `function()` and `value()`. The retained partition preserves supplied
zero values and empty buckets. The canonical finite function extends by zero
outside evidence for arithmetic; this extension is never a claim that those
worlds had supplied payoffs. Equality retains source, evidence and payoff
identity; two different payoffs with the same mean are not equal observations.

`ExpectationValue::Fixed` retains the original signed observation, numerator,
evidence mass and optional exact rational. `ExpectationValue::Parameter` retains
the corresponding functions, partial conditional result and exact defined
domain. Owners survive database/snapshot/query release and result paging.

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
`payoffs`, source function, evidence, numerator/mass and result/domain. Native
delivery shares arithmetic and a cumulative 16 MiB observation payload budget
with probability outputs per collection or page. `expectationResult` describes
an imported query output; it is not a stored schema field. Authoring stays pure.
`expectation(payoffRatio(p.numerator, p.denominator), p.when, p.given)` constructs
the same exact ratio head. The owned ratio expression survives description
export/import and query snapshots. Native transport retains the old integer
ordinal spelling and adds the strict `{kind: "ratio", numerator, denominator}`
value form. Descriptions copy every operand and reject unknown fields or invalid
ordinals; native validation checks both column types and reference roles.

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

`QueryExpectation.lean` supplies twenty-six reference reports for witness unions,
unique coverage, clipping, grouping, missing values, partial conditioning,
presentation invariance and definedness of every participating payoff expression.
They do not prove Rust refinement, the scratch collector or transport. No
performance qualification or scalar-SQL observation oracle is claimed.
