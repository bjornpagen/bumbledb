# Event value encoding, versions 1 and 2

These are the implemented finite-space codecs in `bumbledb-event`: version 1
for structural spaces and version 2 for exact fixed rational joint laws.
It is integrated into native database fields, bindings and results. The complete
M2 gate still requires SDK qualification, retained-memory policy and the
descriptors introduced by parameterized source milestones. Separate [BEDC v1](event-descriptors.md) carries native
finite structural descriptors and rechecks their certificates. BEVT version 1
does not encode a probability law or a
continuous parameter presentation. The [implementation ledger](event-implementation.md)
records the supported slice and remaining gates.

Version 1 encodes two predicates over one named coordinate environment:
admissible support `S` and membership `S ∩ E`. A decoder's extra raw aliases never
enter membership. Source identity and the number of semantic coordinates are
part of the value. Semantic bit indices are fixed; physical variable order is not.

| Offset | Bytes | Meaning |
| --- | --- | --- |
| 0 | 4 | ASCII `BEVT` |
| 4 | 1 | Version `1` |
| 5 | 1 | Raw coordinate count, 0–62 |
| 6 | 1 | Measurement kind, `0` for unmeasured |
| 7 | 1 | Reserved, must be zero |
| 8 | 32 | Named source-environment identity |
| 40 | 4 | Node count, little endian |
| 44 | 4 | Support root reference, little endian |
| 48 | 4 | Membership root reference, little endian |
| 52 | 9 per node | Coordinate byte, low reference, high reference |

References `0` and `1` mean false and true. Each node gets index `i`, starting
at one; its regular reference is `2*i`. XOR with one complements a reference.
The low child of every regular node is regular. The two children differ, both
refer to prior nodes or constants, and their coordinates are strictly greater
than the parent's coordinate. Duplicate nodes are forbidden.

Nodes appear in depth-first low-before-high postorder, first for the support
root and then for the membership root, sharing previously visited nodes. All
nodes must be reachable from these roots. Support is nonempty and membership
is included in support. Readers reject truncated, trailing, unreduced, unordered,
unreachable, out-of-support and unknown-version data. Corrupt payloads that
instead form another valid value require the enclosing store's integrity checks;
this codec does not invent a checksum guarantee.

## Identity argument

At each essential semantic coordinate, the two Shannon cofactors uniquely
determine the Boolean function. Removing equal children and orienting the low
child selects one regular node and one polarity. Induction on the remaining
coordinates establishes a unique reduced graph for a fixed semantic order.
The specified forest traversal then gives one sequence of indices and bytes.

Support-masking makes this graph independent of the resident completion decoder.
Fixed semantic order makes it independent of the working order. Reconstructing
Shannon cofactors from denotations makes it independent of the local-table cutoff
or resident node layout. Equal canonical bytes therefore identify equal named
support and region predicates; distinct regions in the same support cannot share
bytes. This is the encoder's mathematical argument, not a Lean-to-Rust refinement
proof. Exhaustive finite and cross-order fixtures exercise the implemented path.

Decoding rebuilds through the same checked canonical constructors. It chooses a
legal anchor and completes membership outside support. Re-encoding must recover
the input bytes. Decoding into another working order has the same requirement.
Resident tokens remain process-local; cross-owner operations require alignment.

The [Persistence semantics](../crates/bumbledb-event/semantics/Persistence.lean)
prove equality of source, original support and supported membership at a fixed
semantic coordinate universe. The source type must include the concrete source
descriptor when instantiating that model; the theorem does not erase coordinate
meaning or authorize comparing dimensions. Registered-key equality additionally
assumes canonical meanings, admitted values and the resolver roundtrip invariant.

## Version 2: fixed finite law envelope

A measured Event has bytes `BEVT`, version byte `2`, a length-prefixed **version
1 structural Event**, then a length-prefixed **BELW v1 law**. Both lengths are
unsigned little-endian u64. No extra fields or trailing bytes are permitted.
The inner Event supplies the source identity, dimensions, support and membership.
Nested measured Events are forbidden; a version-1 reader refuses this envelope.

BELW v1 starts with `BELW`, byte `1`, and a little-endian u64 piece count. Each
piece contains two u64-length-prefixed blobs: a BERA v1 rational density, then a
BEVT v1 region. Every region has the same named structural context as the Event.
Regions are nonempty and disjoint; densities are strictly positive and distinct.
Pieces are sorted lexicographically by their BERA bytes. Legal worlds outside
all pieces have zero mass. The count-weighted total must equal exactly one.

BERA v1 is `BERA`, version byte `1`, sign byte `0` or `1`, then numerator and
denominator magnitudes, each prefixed by its little-endian u64 byte length.
Magnitudes are minimal unsigned big-endian integers; the denominator is positive
and coprime to the numerator. Zero is an empty numerator, sign zero, denominator
one. Negative zero, leading zeros, unreduced ratios and trailing data refuse.
Densities additionally refuse negative values; the arithmetic codec permits them.

Canonical law construction unions all regions of equal density and omits zero
or empty pieces. Thus the wire partition is determined by the per-world density
function, not input ordering or authored partitioning. Decoding reconstructs the
checked source and compares its canonical law bytes. BEVT v2 identity includes
the designated law even for empty/full Events. Its graph/payload parsers and this
canonicalization remain tested Rust, not a Lean extraction proof.

The [pinned measured fixture](../crates/bumbledb/tests/fixtures/event-v2-source.hex)
has source `[78; 32]`, two fully legal coordinates and region coordinate zero.
Its correlated law gives worlds `00` and `11` mass `1/2` each. Worlds `01` and
`10` remain legal with zero mass. The fixture pins the complete v2 value envelope;
the v1 row fixture separately pins the unchanged database field framing.
See [exact finite sources](event-sources.md) for construction and observations.

## Native row and consumer envelopes

Canonical rows retain their existing big-endian u16 arity. An Event field is
tag **10**, followed by a **big-endian u64 byte length**, followed by one complete
BEVT payload. The payload's own graph integers remain little endian as specified
above. The [pinned row fixture](../crates/bumbledb/tests/fixtures/event-v1-row.hex)
describes one full-support, two-coordinate source `[42; 32]` and membership in
coordinate zero. It pins the row envelope and graph bytes together.

The structural schema type also uses new tag **10**. Existing type tags and
the schema fingerprint family label are unchanged. An old reader refuses the
unknown field/type tag; it must not reinterpret the field as an interval/blob.
The resident layout is two ordinary words with registry ownership, never the
interval `WordPair` comparator. The canonical record is variable length.

The log JSON spelling is `{"event":"<canonical BEVT lowercase hex>"}` and the
schema spelling is `event`. The low-level Node wire uses `Uint8Array` with tag
`event` for typed literals/parameters, and the declared field type for row cells.
The bridge validates through the shared decoder and owns the resulting value;
byte buffers are not trusted resident handles. Worker result conversion encodes
under its WorkContext. High-level SDK constructors and operation APIs remain
separate implementation work. Log command results retain their scalar-only gate.

## Resources and compatibility

Conversion may be exponentially larger than a favorable resident order. Explicit
record, memo and work limits refuse conversion; no heuristic wire identity or
partial result is substituted. Decoding checks lengths before allocation and
supports cooperative cancellation. A failed decode publishes no owner/value.

Older readers refuse unknown versions and measurement kinds. BEVT v2 is the
explicit fixed rational source extension; it does not reuse the unmeasured tag.
Parameterized law formats remain a separate versioned admission obligation. Event format versions are
independent of the package release number. This work creates no release or tag.
