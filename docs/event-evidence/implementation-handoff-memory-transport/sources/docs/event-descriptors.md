# Portable finite Event descriptors

The native host API now transports finite maps, face products, relation views,
composition plans and complete fibre squares as inspectable data. `Descriptor`
owns canonical BEVT payloads and authored space names. `AdmittedDescriptor`
owns reconstructed, checked native objects. `EventImport` now retains these
objects and their canonical bytes for [readout and relation query heads](event-queries.md).
Map, face and product imports are implemented; complete SDK transport remains
a separate gate.

```rust
use bumbledb::event::{AdmittedDescriptor, Descriptor, DescriptorLimits};

let limits = DescriptorLimits::default();
let data = Descriptor::capture(&AdmittedDescriptor::Fibre(pair.clone()), limits, &())?;
let bytes = data.to_bytes(limits, &())?;
// `bytes` can outlive every original source, map and arena.
let AdmittedDescriptor::Fibre(restored) = Descriptor::import(&bytes, limits, &())?
    else { unreachable!() };
```

`pair` above is a checked `FibreProduct`. Applications can inspect or edit the
plain fields before admission. Equality on descriptors compares that explicit
representation; it is not a decision procedure for arbitrary equivalence of
different presentations. No host closures, arena pointers or unchecked proof
handles are serialized. All space names, including the result product and the
composition workspace, are supplied explicitly and retained across import.

## Data and admission

| Descriptor | Reconstruction |
| --- | --- |
| `Map` | Decode full source/target markers and ordered Event readouts; check readout arity/context and original target support |
| `Surjective` | Rebuild the map and recompute its exact supported image |
| `Faces` | Rebuild every onto environment map, require one environment, reconstruct the complete legal tuple product |
| `Fibre` | Rebuild its two onto environment maps and complete pair product, then restore endpoint orientation |
| `Relation` | Rebuild the pair descriptor and align the stored Event region to that exact product context |
| `Composition` | Rebuild S×T, T×U and S×U; check equality of actual endpoint/environment maps; reconstruct one shared S,T,U workspace |
| `Square` | Rebuild the product and both readouts; prove their joint map is onto the full compatible pair product |

The source/target markers in a map must be **full** Events. Passing a proper
region cannot silently turn it into a space descriptor. Readouts remain ordered
by target semantic coordinate. Arbitrary Boolean readouts and nonrectangular
supports retain their meaning; physical node order is excluded from capture.

`FibreDescriptor` stores its base environment maps in the original semantic
concatenation order and a separate `reversed` flag. Converse exchanges endpoint
roles while keeping the region coordinates and named product unchanged. Rebuilding
a reversed product by concatenating its exchanged endpoints would change meaning.

Format tags request reconstruction checks; they do not establish certificates.
In particular, two separately onto projections do not establish a complete
square, and equal environment ranges do not establish equal environment maps.
Serialized certificates are not trusted even when the producer was BumbleDB.

## BEDC version 1

This envelope is separate from BEVT. BEDC v1 reconstructs finite structural
certificates; its Event blobs accept unmeasured v1 and fixed-law v2 values.
Measured endpoint laws survive transport, but no map or product thereby gains
a law-preservation certificate. Logical products remain unmeasured. Parameter
solver certificates, programs, strategies and new field types are not encoded.

Every integer below is an unsigned little-endian u64. `blob` means length then
exact payload bytes; `map` means source blob, target blob, readout count and that
many readout blobs. `fibre` means a 32-byte product name, one orientation byte
(`0` or `1`), then its two base environment maps.

The header is four ASCII bytes `BEDC`, version byte `1`, then one kind byte:

| Kind | Body |
| --- | --- |
| 0 Map | map |
| 1 Surjective | map |
| 2 Faces | 32-byte product name, map count, environment maps |
| 3 Fibre | fibre |
| 4 Relation | fibre, region blob |
| 5 Composition | 32-byte workspace name, S×T fibre, T×U fibre, S×U fibre |
| 6 Square | fibre, left map, right map |

Unknown versions/tags, invalid orientation, truncated/oversized lengths and
trailing bytes refuse. `from_bytes` checks this fixed-depth grammar and returns
untrusted data. `admit` performs semantic construction. `import` performs both;
use it when the result will execute. A successful syntax parse alone is not a
validity claim about its contained BEVT data or mathematical roles.

Default limits are 16 MiB for payload/wire bytes and 4,096 logical descriptor
items. An item is the top descriptor, each map/fibre node, and each BEVT blob;
repeated occurrences count separately. Plain-data admission counts BEVT payload,
while wire parsing/encoding additionally bounds the complete envelope. Per-owner
Event limits apply to decoded owners and reconstructed products/workspaces.
These are explicit admission limits, not a global memory quota. Capturing an
Event may need its codec workspace before its byte extent is known. No oversized
or interrupted import publishes a partially admitted object. Parsing, copying,
encoding and semantic reconstruction all poll cancellation.

## Evidence and boundaries

[Descriptors.lean](../crates/bumbledb-event/semantics/Descriptors.lean) has eleven
reference reports. They prove reconstruction from injective **legal** target
codes, preservation of image/pullback/surjectivity, full-product reconstruction
and converse, with counterexamples for erased support, orientation, environment
maps and joint-fibre claims. Ten reports use no axioms; one uses `propext`.
The Rust parser and constructors must satisfy the stated correspondence premises;
the proofs do not verify native bytes or allocation.

Seven native tests cover all seven descriptor kinds; 128 asymmetric relation
regions/orientations with modal comparisons; nonlinear environments and physical
orders; independently owned round trips; forged claims; truncation/tag/extent
refusal; cancellation and resource limits. The database relation fixture now
drops every original source/product owner, imports the separate BEDC descriptor,
reopens stored regions and recovers their typed relation view after Free Join.

Readout query imports accept `Map` and `Surjective`, face imports accept `Fibre`,
and product imports accept `Composition`. Query relation trees retain those
descriptors without serializing callbacks. Fixed-law functions/channels/revisions now use the separate
[BESC v1 envelope](event-source-descriptors.md). SDK descriptors, parameterized
sources and integrated performance remain separate acceptance work. This transport does not change
public v1.3.1 or authorize a release.
