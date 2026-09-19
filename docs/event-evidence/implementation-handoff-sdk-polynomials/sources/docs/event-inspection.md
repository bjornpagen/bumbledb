# Native Event diagram inspection

`Event::diagram(control)` captures an immutable inspection snapshot of the
completed membership function **and its original legal support**. Callers can
traverse shared nodes, borrow local truth-table words, implement their own
structural algorithms, and reconstruct the Event in a checked target space.
This closes the finite backend's read-only graph interface in M4. Source laws,
query programs and persistent face/map descriptors retain their separate gates.

```rust
use bumbledb::event::{BoolOp4, DiagramView, Space, SpaceId};

let raw = Space::new(SpaceId([1; 32]), 2, &())?;
let legal = raw.table(3, &[0b1001], &())?; // only 00 and 11
let space = raw.restrict(&legal, &())?;
let event = space.coordinate(0, &())?;
let diagram = event.diagram(&())?;

let view = diagram.root().view();
// Borrowed views hold no manager lock; same-space construction remains safe.
let tautology = event.apply(BoolOp4::OR, &event.complement(), &())?;
assert!(tautology.is_full());
assert!(matches!(view, DiagramView::Table { .. }));
assert_eq!(diagram.rebuild(&space, &())?, event);
```

The corresponding crate example is a compiled doctest. The database relation
consumer also inspects and reconstructs a persisted result after closing the
database, on both resident and cursor query paths.

## What the view means

| API | Contract |
| --- | --- |
| `identity()`, `dimensions()`, `order()` | Named finite context, semantic coordinate extent and captured physical split order |
| `root()` | Completed membership function; includes decoder aliases |
| `support()` | Original legal-code predicate, before completion |
| `DiagramNode::view()` | Constant, borrowed local table, or checked child nodes of a symbolic split |
| `coordinates()` | Exact raw essential-coordinate mask of that node, including decoder-induced dependence |
| `complement()`, `regular()`, `is_complemented()` | Allocation-free signed-node inspection and memoization |
| `records()`, `table_words()` | Reachable snapshot storage, counting shared nodes once; excludes other arena intermediates |
| `contains(world)` | Membership with original-support and coordinate-extent admission |
| `rebuild(target, control)` | Checked same-context reconstruction under the target's working order and decoder |

A table assigns truth bits to the ascending set bits of its coordinate mask.
For local assignment i, read bit `i % 64` of word `i / 64`, then XOR the
`complemented` flag. Padding is zero before complement and never denotes an
assignment. The current carrier's tables have at most nine coordinates/eight
words. A split's children already include its polarity. Traversals may use a
`HashMap<DiagramNode, ...>`: node equality/hash identifies one signed raw node
within one shared snapshot. Independently captured snapshots have separate
node identities, even if their Events have identical meaning.

Every nonconstant node is reachable from at least one of the two roots. Child
references are private, acyclic and owned by the snapshot. No public constructor
accepts unchecked node offsets. A clone shares the immutable graph; borrowed
nodes cannot outlive that graph. The snapshot retains sufficient context to
remain usable after all Event, arena and database owners are dropped.

## Legal support and reconstruction

The membership root denotes `A(rho(code))`, where rho maps raw codes to legal
worlds and fixes legal worlds. A true leaf may describe an illegal decoder alias.
For witnesses, quantification, legal counts or measurement, first gate membership
with original support. `contains` does this and rejects illegal codes.

`coordinates()` describes a physical function. It cannot replace the
membership-FD check performed by `SurjectiveMap::descend`. Likewise, counting
raw assignments cannot replace Event's legal-world count or a designated
probability law.

`rebuild` checks the named source identity and coordinate extent, reconstructs
and compares original support, then reconstructs the membership root and applies
the target's decoder. It checks support for empty/full Events too. It preserves
meaning across different working orders and completion anchors. The public
`Space::table`, `Space::coordinate` and `Event::ite` constructors remain available
for clients building or transforming functions from these views; construction
into a new source context is the caller's explicit semantic decision.

Snapshot node identities and shape are **not canonical persistence**. Physical
order, decoder choices and leaf cutoff remain observable. BEVT v1 retains the
separate canonical support-relative encoding contract.

## Memory, concurrency and refusal

Capture copies each reachable nonconstant record and its local words once. It
does not copy unrelated arena history, expand shared paths to a tree, enumerate
worlds, or change the resident graph. The resulting leaf slices borrow snapshot
storage. This first interface is an owned snapshot with borrowed views; it does
not expose zero-copy arena slabs. Snapshot memory is additional retained storage
whose lifetime ends with its last clone/view owner.

Capture holds the manager lock only while copying. Traversal, table access and
`contains` hold no manager lock. Clients may retain views while constructing
Events in the source manager or another thread without reentrant lock acquisition.

Capture uses the source's record/table/memo/operation limits and cancellation;
reconstruction uses the target's limits. Allocation is fallible. Failure returns
no snapshot or Event; reconstruction may retain canonical target intermediates
under the existing arena policy. Capture cannot mutate source records or words.

## Evidence

[Inspection.lean](../crates/bumbledb-event/semantics/Inspection.lean) adds ten
reports for signed references, constructor correspondence, acyclic graph and
root reconstruction, two-root legal membership, support/decoder gates and
complement propagation. Its algebra correspondence is an explicit premise. It
does not verify Rust indexing, capture, table packing, allocation or mutexes.

Seven native inspection tests cover all four-world supports and predicates,
different working orders, arbitrary twelve-bit support against an independent
bitset, sparse coordinates through bit 61, signs/padding, legal alias refusal,
independent lifetime, same-arena construction with live views, cancellation and
resource failures. A 62-coordinate parity case has 54 reachable nodes; an
external traversal computes its `2^61` satisfying assignments without expanding
all paths. This demonstrates structural access and sharing, not a performance
ranking or a probability calculation.

The [current semantic run](event-evidence/inspection-semantics/check.json) records
95 reports across six modules. The [native qualification](event-evidence/native-inspection-qualification/check.json)
pins sources and logs. The [handoff audit](event-evidence/implementation-handoff-inspection/check.json)
checks current documentation and both native/proposal proof evidence. The full
M0–M8 implementation remains open.
