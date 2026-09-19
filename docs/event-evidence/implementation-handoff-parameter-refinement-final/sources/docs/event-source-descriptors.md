# Portable finite functions, channels and revisions

`SourceDescriptor` transports fixed finite scalar functions, conditional channels
and complete mathematical revision receipts. Plain descriptors are inspectable,
untrusted data; `AdmittedSourceDescriptor` owns objects reconstructed by checked
native constructors. These complement the structural [BEDC descriptors](event-descriptors.md).
They do not add schema weight columns or change Event identity.

```rust
use bumbledb::event::{
    AdmittedSourceDescriptor, ArithmeticLimits, ExactArithmetic,
    SourceDescriptor, SourceDescriptorLimits,
};

// `revision` is an owned result of condition, likelihood or jeffrey.
let limits = SourceDescriptorLimits::default();
let mut work = ExactArithmetic::new(ArithmeticLimits::default(), &());
let data = SourceDescriptor::capture(
    &AdmittedSourceDescriptor::Revision(revision), limits, &mut work,
)?;
let bytes = data.to_bytes(limits, &())?;
let AdmittedSourceDescriptor::Revision(restored) =
    SourceDescriptor::import(&bytes, limits, &mut work)?
    else { unreachable!() };
// Old Events translate through restored.revised().unwrap().translation().
// An impossible revision instead retains its prior, inputs and complete cause.
```

A captured function stores its full named space and canonical nonzero rational
cells, with zero as the default. Imported plain presentations may split equal
values into disjoint cells or include explicit zero/empty cells. Construction
validates all their contexts and overlap before simplification. Signed values
remain exact. Equality of plain descriptors compares their representation;
it does not replace function equivalence or validate observable coverage.

A channel stores its complete parent map and conditional density. Import checks
the map, aligns the density into its source arena, and reconstructs nonnegative
unit fibre sums on **every legal parent**, including zero-prior worlds. Merely
normalizing the joint law under one prior would not establish that property.
The imported channel closes under an explicitly supplied compatible prior. Its
parent law does not license hidden information access or assert independence.

A revision stores its full measured prior, mathematical inputs, original receipt
numbers and claimed outcome. Import replays the appropriate constructor:

| Receipt | Checks beyond input admission |
| --- | --- |
| Condition | Recompute evidence mass and posterior, or zero-evidence impossibility |
| Likelihood | Recompute normalizer at the supplied scale and posterior, or zero-likelihood impossibility |
| Jeffrey | Reconstruct the full indexed partition; recompute every old mass; check target normalization; reconstruct the posterior or every unsupported positive-target position |

A successful revision claim includes the posterior's complete named law and
original support. A different normalized posterior fails. Its identity-on-worlds
translation is rebuilt from the result; serialized map handles are unnecessary.
Impossible claims must match the exact cause and complete ordered list of
unsupported cell positions. Empty cells retain their positions. Prior and
posterior Events retain zero-mass structural possibilities.

`DescriptorClaimMismatch` reports disagreement with a recomputed mathematical
claim. Syntax, context, input normalization and operational errors retain their
own error kinds. Cancellation or an exhausted budget never becomes a successful
impossible outcome. Admission returns the recomputed object, never a trusted
wrapper around asserted output bytes.

Likelihood factors that differ by a positive scale can produce the same
posterior but different normalizers. The receipt preserves that distinction.
Jeffrey replacement is a separate receipt kind; targets are never silently
reinterpreted as likelihoods. These are mathematical receipts, **not provider
responses, authentication records or observation deduplication keys**.

## BESC version 1

This envelope leaves BEDC v1, BEVT v1/v2, BELW v1 and BERA v1 unchanged. No new
persisted field kind, package version or release is introduced. Programs,
strategies, parameter families, solver certificates, provider metadata, source
extension objects and owned probability/expectation observations are outside this
version. A checked channel can reconstruct an extension by closing under a prior.
Full SDK and query observation transport remain separate gates.

Integers are unsigned little-endian u64 values. A `blob` is its byte length followed
by exactly that many bytes. Space markers and regions use canonical BEVT; scalar
blobs use canonical BERA. A `map` uses the unchanged BEDC body grammar: full source
blob, full target blob, readout count and that many readout blobs.

The header is ASCII `BESC`, byte `1`, then a kind byte:

| Kind | Body |
| --- | --- |
| 0 Function | function |
| 1 Kernel | parent map, density function |
| 2 Revision | prior blob, receipt, outcome |

A `function` is a full-space blob, piece count, then `(region blob, value blob)`
pairs. A receipt starts with its own kind byte:

| Receipt kind | Body |
| --- | --- |
| 0 Condition | evidence blob, claimed mass blob |
| 1 Likelihood | function, claimed normalizer blob |
| 2 Jeffrey | cell count, then `(cell blob, claimed old mass blob, target blob)` triples |

An outcome begins with one byte:

| Outcome kind | Body |
| --- | --- |
| 0 Revised | full posterior blob |
| 1 Zero evidence | empty |
| 2 Zero likelihood | empty |
| 3 Unsupported targets | position count, then that many u64 positions |

Unknown versions/tags, overflow, truncation, oversized rosters and trailing bytes
refuse. `from_bytes` only parses this fixed-depth grammar. `to_bytes` checks shape
and extent, not truth. `admit` reconstructs the mathematical objects; `import`
parses and admits. A valid byte envelope can contain a false receipt.

`SourceDescriptorLimits` combines existing descriptor, function, law and partition
limits. The default envelope bound is 16 MiB / 4,096 logical items. Items count
the top descriptor, each function/kernel/revision/map node, each BEVT/BERA blob,
and each unsupported position. Function pieces and Jeffrey triples have no extra
node charge; their blobs count individually. Repeated blobs count separately.
Plain admission bounds payload bytes; parsing/encoding also bound the full wire.
Roster limits are checked before reserving decoded vectors.

The caller's `ExactArithmetic` budget spans the entire admission, including every
embedded BEVT v2 law and BERA scalar and the replayed calculations. Embedded laws
also honor the supplied law-cell/byte limits. Capture uses already canonical law
bytes; reconstructing these laws is admission work. Per-owner graph limits apply
to each decoded owner. These remain operation and shape bounds, **not a complete
aggregate retained-memory policy**. Event capture may require codec workspace
before its eventual byte extent is known.

## Evidence and correspondence

[SourceTransport.lean](../crates/bumbledb-event/semantics/SourceTransport.lean) adds
14 reference reports: scalar and fibre reconstruction, kernel normalization,
likelihood receipt reconstruction, indexed unsupported positions and complete
replay admission. Counterexamples retain zero-prior row failures, likelihood
scale, omitted empty targets and operational failure versus impossibility.
They assume exact decoding on the same legal worlds and correct native replay;
they do not verify Rust arithmetic, codecs, arena alignment or resource accounting.

Ten native tests cover 255 signed functions on every nonempty four-world support,
485 conditioning/likelihood cases, 240 Jeffrey cases, all impossible causes,
reordered independent owners, malformed scalar payloads and every truncated prefix
of six envelopes. Additional tests forge normalized posteriors, receipt numbers,
likelihood scale and incomplete/reordered unsupported lists; exercise arithmetic,
law, shape, envelope and cancellation refusals; and retain a symbolic 62-coordinate
source. A pinned zero-dimensional signed-function fixture fixes BESC v1 bytes.

The persisted Coup consumer reconstructs its Tax channel, writes the revision
envelope to an application-owned sidecar, drops the original owners, replays it,
and uses the recovered translation in resident and cursor queries. An imported
signed payoff still yields `-37/147` after database/query release. This uses the
supplied two-proposition marginal; the complete executable Coup fixture remains M8.
