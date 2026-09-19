# Owned numerical predicates and explicit Event guards

`ObservationPredicate` retains an exact true/false/undefined partition and the
entire calculation that produced it. Fixed observations yield an optional truth
value. Shared-parameter observations yield three exact regions covering one
inhabited ambient domain. No prior is introduced over those assignments.

This native host API consumes query-produced numbers. Dedicated predicate query
heads, staged predicate variables and Node/SDK predicate builders remain open.

## Algebra and identity

`number.where_sign(signs, limits, work)` retains the selected number and sign
mask. `left.compare(right, signs, limits, work)` retains their numerical
difference and tests its sign. ZERO means equality, POSITIVE means greater than,
NON_NEGATIVE means greater than or equal, and all eight sign masks are available.

The resulting predicate supports `negate`, all sixteen `BoolOp4` operations,
explicit `on_domain`, and `equivalent`. The owned expression exposes Sign,
Negate, Binary and OnDomain nodes. Every written operand remains reachable,
including redundant branches and operands of a constant truth table.

Boolean operations are strict partial operations. Both inputs must be defined;
even `TRUE(a,b)` retains their holes. Negation exchanges true and false while
keeping undefined unchanged. Consequently, complementing only the true region
does **not** compute predicate negation when undefined assignments exist.

`predicate().possibly()` requires a true assignment. `always()` requires true
at every ambient assignment, including definedness everywhere. `is_total()`
tests whether undefined is empty. `equivalent` compares all three regions; it
does not identify original sources or evidence. Fixed predicates lift to a
parameter operand's domain. Two parameter operands require the same named
ambient domain. `on_domain` explicitly restricts that domain and keeps the
original observations in its child derivation.

## Crossing into Events

```rust
let comparison = advantage.where_sign(
    PolynomialSigns::POSITIVE, number_limits, &mut work,
)?;
let refined = comparison.refine(
    presentation_id, &source, event_limits, source_limits, &mut work,
)?;
let cases = refined.events();
let profitable = cases.holds();
let unprofitable = cases.fails();
let unknown = cases.undefined();
let carried_claim = refined.refinement().lift(&claim, work.control())?;
```

The three returned Events are disjoint and cover the complete refined source.
The result retains the predicate and source. The explicit presentation identity
names a deterministic guard extension of the same actual worlds and designated
law. It does not condition, renormalize or bind a prior. Existing Event values
move through the returned ordinary `ParameterRefinement`; exact descent refuses
when an added guard is essential.

True and false distinguish all three cases; undefined is their ambient
complement. A total predicate needs only the true distinction. Already
representable cases add no guard. Exact irrational points and open boundaries
remain separate where their membership differs. Possible zero-mass outcomes
remain actual worlds on every truth region.

`comparison.events(&source, source_limits, work)` uses only existing guards. It
refuses if any region cuts a current logical cell. A parameter predicate must
have exactly the source's named ambient domain: silently applying a narrower
predicate to a wider source would invent truth outside its domain. Fixed
predicates can be interpreted as constant regions on any explicitly supplied
admitted source; their numerical origins remain distinct and retained.
`refine` requires a parameter source. Constant predicates on a finite source use
`events` directly.

These Events fit the existing dependency language:

```rust
relation Decision { game: u64 }
relation Truth { game: u64, case: u64, when: event }
Decision(game) -> Decision;
Decision(game, true) -> Decision;
Truth(game, case) -> Truth;
Truth(game, when) -> Truth;
Truth(game) <= Decision(game);
Decision(game, true) == Truth(game, when);
```

The scalar `case` labels true, false or undefined. The Event key prohibits
overlapping cases; containment requires complete coverage. Omitting a nonempty
undefined region fails admission. Joins and subsequent Event expressions use
these fields normally, including after persistence and owner closure.

## Portable replay and limits

`ObservationPredicateImport::capture` and `from_bytes` reconstruct independent
owned observations and replay every arithmetic, sign and Boolean operation.
BENP v1 contains no trusted truth partition. Equality and hashing use complete
encoded derivation identity, independent of Arc sharing and allocation order.
Double negation can preserve the truth partition while remaining a different
written derivation.

The prefix `BENP` and version byte `1` are followed by predicate tags:

| Tag | Payload |
| --- | --- |
| 0 | Sign-mask byte, then an embedded BENO numerical node (without its envelope) |
| 1 | Negated predicate |
| 2 | `BoolOp4` byte, then left and right predicates |
| 3 | Length-prefixed BEPR inhabited domain, then restricted predicate |

One `ObservationNumberCodecLimits` budget counts predicate and numerical nodes
together. Its descriptor byte/item bounds cover the complete stream, including
embedded observation payloads. Numerical leaf codecs retain their source limits.
Every exact operation uses the caller's shared arithmetic counter. Both tree
walks use explicit stacks; combined depth has a hard ceiling of 256, including
numeric nodes below a Sign. Canonical reencoding rejects presentation aliases.
These are admission limits, not a complete aggregate retained-memory policy.

## Evidence and scope

Native tests cover every fixed partial truth pair under all sixteen operations,
all sign masks, negation, portable identity, source distinction, domain
restriction/refusal, disjoint endpoint holes, exact irrational boundaries,
guard lifting, repeated refinement, zero-mass worlds, malformed streams,
combined numerical/predicate depth and size, cancellation and shared work.
Replay rechecks original numerical leaves even when their final values cancel.

`event_predicate_guards` runs query arithmetic on both Free Join paths, retains
and replays its comparison after owner closure, persists all three Event cases,
checks missing/overlapping partition refusals, then queries the stored regions
after reopening. This is a host-mediated consumer, not a predicate query opcode.

Nineteen `PredicateGuards.lean` reports establish exact truth coding,
world-preserving refinement, Boolean lifting, finite contraction preservation,
zero-weight possibility retention and the old-cell factorization requirement.
They do not verify the Rust solver, codec, allocator or query engine.

Predicate/region query staging, captured refinement instructions, Node/SDK
consumers, multivariate solving and every other open M0–M8 gate remain required.
No release, tag or version bump.
