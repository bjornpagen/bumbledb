# Local functions on Event covers

`FiniteFunction::glue` and `FamilyFunction::glue` assemble local observables on
an evidence Event. Each patch supplies a source-owned total function on a named
region. The regions must cover the evidence, and overlapping patches must give
the same value on their overlap. These are structural obligations, independent
of a probability law. Zero-mass worlds and isolated parameter points count.

This is the functional form of pointwise uniqueness plus union coverage. A
patch need not have one constant value, and functions need not agree outside the
part of their regions that intersects the evidence. Repeated compatible patches
never add or multiply utility. The caller still chooses a law, evidence and any
explicit prior separately when requesting an expectation.

```rust,ignore
let cover = FiniteFunction::glue(
    &given,
    &[
        FunctionPatch { region: left, function: left_payoff },
        FunctionPatch { region: right, function: right_payoff },
    ],
    FunctionLimits::default(),
    &mut work,
)?;
let observation = cover.function().expectation(cover.parent(), &mut work)?;
```

The owned `FunctionCover<F>` retains `parent()`, all aligned original
`patches()` and the assembled `function()`. The function is zero outside the
parent. This zero extension is useful for algebra, but is never treated as
supplied coverage. Supplied zeros, empty patches and regions extending outside
the evidence remain inspectable. Only checked gluing constructs a Rust cover.

## Parameter-dependent agreement

Consider the full parameter domain `0 ≤ p ≤ 1`. One patch supplies `p` where
`p ≤ 1/2`; the other supplies `1-p` where `p ≥ 1/2`. The two functions differ
globally, but agree at their only overlap, `p=1/2`. Gluing produces
`min(p,1-p)` without deleting the shared boundary or assigning it twice.

Changing the second patch's value at that one point makes the cover conflict.
Even a later continuous prior giving the boundary probability zero cannot waive
this check. The exact family solver checks actual parameter worlds, preserving
the original domain and guard structure; gluing does not introduce a prior or
refine away a conflict.

`mask(region)` is also available on both function types. It keeps the source,
support and parameter domain and returns zero outside the region. Its operand
must already be total on its source. It cannot repair a parameter-function hole.

## Admission, ownership and budgets

Every patch region and function must align to the parent source before any
overlap/gap or empty/full shortcut. Empty patches and redundant patches from a
foreign source therefore refuse. The native API reports `FunctionCoverConflict`
or `FunctionCoverGap` for structural failures. Operational failures and source
mismatches remain distinct.

The implementation clips regions to the parent, checks each new overlap against
the assembled function, and adds values only on the fresh region. Event algebra
and existing finite/family function arithmetic perform these operations. No
world enumeration or probabilistic approximation is introduced. One caller's
`ExactArithmetic` covers all nested exact arithmetic. A function budget bounds
the patch roster and outer scans; each nested function/solver operation retains
its own declared limits. This is not a complete aggregate memory policy.

The SDK exposes `FiniteFunction.mask/glue` and `FamilyFunction.mask/glue` as
cancellable worker operations, plus `FunctionPatch<F>`/`FunctionCover<F>` result
types. For example:

```typescript
const cover = yield* FamilyFunction.glue(given, [
  { region: left, function: rising },
  { region: right, function: falling }
])
const answer = yield* FamilyFunction.expectation(cover.function, cover.parent)
```

Inputs are copied before asynchronous work. Original patch and function outputs
are owned, and all output blobs share the worker's 16 MiB budget. Native ingress
replays every region and function under shared arithmetic; plain cover records
are never accepted as trusted certificates. No new descriptor role or wire
version is needed: retained Events and functions use their existing formats.

## Database boundary and evidence

An integration fixture stores patch regions with ordinary scalar keys and Event
containment. After persistence/reopen, both resident and cursor Free Join paths
return the patches. The caller selects separately reconstructed functions and
glues them after releasing database/query owners. Two functions that differ
globally agree locally; the resulting four-world utilities are
`[1/3, 2/3, -2, 0]`, with uniform expectation `-1/4`.

This exercises host gluing of query-produced patches. Native `Expectation`
heads accept integer columns and exact ratios of integer columns. Functional
payoff slots and observation staging/arithmetic remain implementation work.

Core tests compare all 1,024 two-world patch/value/parent combinations with an
independent pointwise oracle, including order reversal. Additional tests cover
parameter-boundary agreement, supplied zeros, zero-mass gaps/conflicts, empty
parents, foreign empty inputs, symbolic 62-coordinate sources, detached owners,
limits and cancellation. Node and SDK tests cover shared ingress budgets,
malformed late operands, replay, copied inputs and late cancellation.

`FunctionCovers.lean` adds fifteen reference reports for coverage, compatibility,
uniqueness, zero extension, mask laws, duplicate/order invariance and impossible
conflicting representations. The proofs do not assume a measure. They do not
verify the Rust gluing algorithm, exact solver, transport or Free Join compiler.
