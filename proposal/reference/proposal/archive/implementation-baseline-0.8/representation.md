# Event representation for implementation

**Revision 0.8.** The initial general backend is a canonical completed function
with symbolic splits and essential-coordinate tables. This selects an
implementation starting point from the [design review](design-review.md); it
is not a universal performance claim. Dense and packed512 remain comparison
controls. Earlier BDD-pair layouts are preserved in the
[pre-cleanup snapshot](archive/implementation-prep-0.7/representation.md).

## 1. Stable public identity, replaceable internal carrier

```rust
#[repr(C)]
struct EventKey { space: u64, region: u64 }

struct Event { key: EventKey, owner: Arc<EventSpace> }
```

Fields are private. A checked owner resolves each region before kernels run.
The space token is generation checked; root indices are never recycled within
a live arena. Empty and full validate and retain owners exactly like other
values. A standalone owned Event is larger than the two-word resident key.

Within the canonical resident owner, equal keys mean equal admissible regions.
Across owners, equality requires checked alignment/rebasing. Hashes find possible
matches; full canonical contents settle equality. Copying a key keeps the same
source outcome; a fresh draw is a source extension.

The backend boundary exposes checked constants, Boolean construction, equality,
structural witnesses, face mapping, quantified construction, exact legal counts,
read-only traversal and canonical publication. Public callers see the complete
region algebra rather than raw node indices or tuning switches. Relation roles,
map certificates and source laws are owned descriptors over this same carrier.

## 2. Canonical completed function

Fix a decoder `rho` from raw codes onto legal worlds that fixes legal worlds:

```text
stored_A(code) = A(rho(code))
```

The [retraction proofs](experiments/event-repr-lab/lean/Retraction.lean) establish
support-relative identity, complement and Boolean preservation. Its defining
membership FD is `decoded world -> Event membership`. A raw witness is decoded
before publication. Original legal support and the designated law are retained;
raw aliases never add legal states or probability mass.

For a certified product, separate face decoders can reflect semantic face
independence in the physical function. A whole face may contain a correlated
Coup state. Product state copies do not assert independent players. Arbitrary
coupled support uses a general decoder and support-aware fallback; it cannot
borrow product laws without the product certificate.

The raw essential-coordinate mask describes dependence of this completed
Boolean function. It is not a uniquely least logical dependency set on arbitrary
legal support: copied coordinates give counterexamples. Keep logical dependency
certificates distinct from physical masks.

## 3. Initial memory and kernel choices

Use constant references, canonical symbolic splits, and truth tables containing
only exact essential coordinates. The initial cutoff is nine raw coordinates:
a local table needs at most eight 64-bit words. Larger functions split in an
immutable order; equal continuations share nodes. This cutoff is internal and
must not affect source identity, public semantics or persistent fact bytes.

The laboratory's compact record is two words: a dependence mask and a payload
selecting children or an offset into a word slab. Treat its compact child-index
and fewer-than-63-coordinate limits as checked prototype capacities. A production
port must document and test its representable bounds before publication; wider
coordinate sets require explicit storage, never bit truncation or a changed
answer. Semantic coordinate identities are independent of record numbering.

Intern every canonical constructor with full collision checks. Complement is
one region-polarity bit. `BoolOp4` selects all sixteen binary truth functions.
Use the validated direct ITE kernel and staged support-aware projection as the
initial operation schedule. Constructor certificates and structural cardinality
caches remain internal; no cached count may count decoder aliases as worlds.

Prepare projection data from certificates: required support gates, hidden
coordinates, and decoder coordinates requiring repair. Product coverage can
remove gates; locality can remove unaffected repairs. When a certificate is
missing, execute the full exact gated/repair program. Maximal fusion, borrowed
views and prefix-specific decoders remain optional implementations with their
own prerequisites and costs, not public flags.

[Kernel notes](kernels.md) retain the ARM64 evidence. Word/NEON operations act on
aligned local bitplanes. The fifteen pair-occupancy signatures support a separate
predicate-as-data path. A sixteen-entry lookup table must initialize every valid
Event signature; the Allen-specific 0–12 wrapper is insufficient.

## 4. Query execution and temporary memory

Free Join moves ordinary word bindings and retained owners. Event computation
consumes checked operands. Pack keeps group presence separate from the region
fold. A present empty Event remains a result and an absent group remains absent.

Complete-binding and factored-branch schedules share one mathematical contract.
The latter requires a scalar separator certificate, compatible typed operators,
all participating validation and exact multiplicity/presence accounting.
Composition preserves one middle state; residual aggregation uses its intersection
law. Native stable diagnostic descriptors are required by [the proposal](proposal.md).

Representation ranking must be measured after the relevant algebraic rewrites:
[relational Pack](experiments/event-repr-lab/RELATIONAL-PACK.md) changed the preferred
packed layout in a measured workload. Small warm groups can lose to factoring.
The supplied checker certifies a partition, not an optimal-partition search.
The participation-index draft is shelved and supplies no accepted memory result.

## 5. Ownership, publication and memory pressure

Use immutable append-only records with stable references. Transaction-private
scratch can build speculative values. Publication rebases referenced values
into the canonical owner and resolves concurrent interning by full equality.
Only admitted facts and their transitive owner set become visible atomically.

Images, derived stages, spill readers and result batches retain one owner per
referenced space rather than one atomic refcount operation per binding. A result
must survive releasing its input snapshot and executor. Empty/full, relation
views, maps and exact observation outputs obey the same lifetime rule.

The initial reclamation policy releases an arena with its last owner. Bound
scratch and caches through WorkContext; cancellation/resource refusal aborts
publication. Cache eviction cannot invalidate a live result. Long-lived arenas
can retain dead intermediates, so report retained capacity separately from live
answer graphs and process RSS. Compaction, when introduced, needs explicit
translation and cannot recycle IDs visible to existing owners.

## 6. Persistence is a blocking implementation gate

The existing transport lab verifies an ordered batch codec and checked rebasing.
It does not define canonical native per-fact identity or serialize every source
and law descriptor. Freeze a versioned codec during M2 before persistent Event
facts can ship. It must establish:

- semantic space/source identities, named coordinates and admissibility;
- exact region bytes independent of allocation, leaf cutoff, decoder and execution order;
- identical fact identities across supported resident carriers;
- complete owner/law references, corruption checks and bounded resource behavior;
- explicit compatibility/version refusal for older readers.

A deterministic semantic-order graph codec is a concrete candidate. Its potential
reordering blowup is a cost that must be measured and may refuse under limits.
An expression hash, arena ID, heuristic variable order or unverified batch packet
cannot substitute for the required canonical encoding. The persisted format is
not claimed selected or formally verified by the representation experiments.

## 7. Acceptance boundaries

The [implementation plan](implementation-plan.md) assigns the native work and
gates. The [proof matrix](semantics/README.md) separates denotational guarantees
from Rust correspondence, arena canonicality, solver correctness and wire identity.
The laboratory remains the source of raw performance evidence. Changing the
initial backend later requires the same complete operation, publication,
observation and conversion comparisons; it never permits weaker semantics.
