# Event value encoding, version 1

This is the implemented unmeasured finite-space codec in `bumbledb-event`.
It supplies the first part of M2. Database field integration, store compatibility,
registry ownership and measured source descriptors are still required before
the complete persistence gate passes. Version 1 does not encode a probability law.

The format encodes two predicates over one named coordinate environment:
admissible support `S` and membership `S ∩ E`. A decoder's extra raw aliases never
enter membership. Source identity and the number of semantic coordinates are
part of the value. Semantic bit indices are fixed; physical variable order is not.

| Offset | Bytes | Meaning |
| --- | --- | --- |
| 0 | 4 | ASCII `BEVT` |
| 4 | 1 | Version, currently `1` |
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

## Resources and compatibility

Conversion may be exponentially larger than a favorable resident order. Explicit
record, memo and work limits refuse conversion; no heuristic wire identity or
partial result is substituted. Decoding checks lengths before allocation and
supports cooperative cancellation. A failed decode publishes no owner/value.

Older readers refuse unknown versions and measurement kinds. A measured-source
extension must specify its descriptor and compatibility fixtures before native
publication; it cannot reuse the unmeasured tag. Event format versions are
independent of the package release number. This work creates no release or tag.
