# Event structure across arenas, layouts and ownership boundaries

An Event representation must remain useful when the database moves a value.
Native rows need exact fact identity, admission needs the same overlap/coverage
algebra as queries, and a returned result must keep its space alive. A fast
temporary arena is insufficient if those boundaries erase dependence or force
every symbolic space into a world enumeration.

This experiment gives all fourteen carriers a common **structural transport
view**, checks its support maps, and restores actual Free Join inputs and outputs
into fresh owners. The public field remains `event` / `Event`. Packet nodes,
maps and owner batches are implementation contracts, not application schema
columns or per-row weights. Production crates are unchanged.

## 1. A shared graph vocabulary without a mandatory resident graph

The implementation is [transfer.rs](src/transfer.rs). Its packet contains:

```rust
struct Packet {
    names: Vec<u64>,        // lab identifiers for semantic binary coordinates
    order: Vec<u32>,        // physical rank, separate from coordinate identity
    nodes: Vec<Node>,       // shared postorder graph
    support: Ref,          // explicitly declared admissible worlds
    roots: Vec<Ref>,        // an ordered batch of event formulas
}
enum Node {
    Split { coords: Vec<u32>, children: Vec<Ref> },
    Table { coords: Vec<u32>, words: Vec<u64> },
}
```

`Split` enumerates disjoint exhaustive assignments of at most six local axes;
each assignment selects a shared continuation. `Table` holds the truth table of
at most twenty local axes. Local axis zero is the low assignment bit. References
0/1 mean false/true; other references name a node and carry one complement bit.
Roots denote formulas restricted to the separately retained support.

Binary and four-way diagrams expose their decisions. Anchored diagrams expose
their representative/polarity and support separately. Packed diagrams expose
prefix decisions and table leaves. Block64 expands its selector partition into
the corresponding exhaustive local child vector. These nine exporters traverse
shared graphs, without enumerating the ambient worlds. Fixed-false MDD padding
is omitted from the semantic coordinates. Explicit finite carriers currently
export one table per distinct region; they do not yet expose run/container views.

The builder interns complete node structures, drops equal-child splits and
constant tables, and traverses deterministically. These are safe structural
reductions. They do not turn every possible packet into a unique representation
of its Boolean function. Different resident decompositions may emit different
packets for the same Event.

The strict binary codec uses magic/version `EVTP\x01`, big-endian integers,
backward node references and an explicit ordered root list. Decoding rejects
truncated/trailing input, duplicate or invalid coordinates, malformed orders,
forward references, invalid shapes and nonzero table padding. Input is capped
at 64 MiB, two million nodes/roots and fewer than 63 coordinates. Limits are
explicit refusals, not alternate semantics. These are lab format choices;
source descriptors and a persisted Event scalar codec remain future work.

## 2. Import is substitution, with a separate support proof

The importer reconstructs target literals, combines cofactors with exact ITE,
and interns results in the destination carrier. Shared packet nodes get a
shared import memo. An omitted axis is existentially eliminated by unioning its
two cofactors. **Negation is pushed into those cofactors before elimination:**
`exists(!E)` is generally different from `!exists(E)`.

An explicit injective coordinate map identifies where each source coordinate
lives in the target. Project target support onto the old coordinates:

```text
P = exists new_coordinates . target_support
Restriction: empty != P <= source_support
Extension:   P = source_support != empty
```

A restriction deliberately removes old possibilities. An extension retains
every old possibility, so inverse-image lifting is a faithful Boolean map.
Both reject a target world outside the source support. Validation runs in a
fresh **full-product proof arena**; proving equality after target-support masking
would hide exactly the source worlds whose loss must be detected.

The explicit proof carriers cap materialized spaces at twenty coordinates.
Symbolic proof carriers can handle the tested forty-coordinate spaces directly.
The source carrier currently also selects the proof carrier, which affects
measured restore cost. There is no automatic verifier-selection policy.

## 3. Three different map guarantees

The [primary-source review](../../research/space-maps.md) separates:

| Guarantee | What it licenses | What it does not license |
| --- | --- | --- |
| Support inclusion, plus totality when claimed | Well-defined lift; a total lift reflects containment and emptiness | Moving arbitrary quantifiers across the map |
| Complete fibres for a specified projection square | Moving that existential elimination across substitution | An arbitrary law interpretation |
| Pushforward equality for a designated joint law | Preserving probabilities of lifted old events | Inventing independence from structural compatibility |

For example, extend X with a copy Y. Legal pairs are `(0,0)` and `(1,1)`.
Every old X survives. Yet hiding X in `X=1` before lifting gives full, while
lifting first and hiding X while retaining Y gives `Y=1`. The copy retains
information. No generic “valid extension” flag can authorize that rewrite.

The finite criterion is complete fibres **in each retained target context**,
rather than a witness somewhere in the target. Full fibre products in the
world-relation contract supply the appropriate squares. This is the concrete
Beck–Chevalley condition; it belongs in checked plan descriptors. The independent
[finite checker](../../research/space-maps/checks.py) verifies the criterion
over 765 squares and 12,240 events, including 243 commuting squares.

Similarly, two full-support fair-bit marginals allow joint `P(1,1)=1/4` or
`3/8`. A structural product does not choose that coupling. The measured fibre
product in Pong's paper explicitly chooses conditional independence and assumes
strictly positive distributions. Its hypotheses cannot be obtained by deleting
our admissible zero-mass worlds.

The Rust prototype implements the first row for finite binary coordinate
embeddings. It does not yet implement general certificates for the other two
rows, real-guard substitutions or source-allocation/environment alignment.

## 4. Flat rows with retained owners

```rust
#[repr(C)]
struct EventKey { space: u64, region: u64 } // 16 bytes, Copy
struct Batch<C> {
    owner: Arc<Space<C>>,
    keys: Vec<EventKey>,
}
```

The scope token is process-local, monotone and never reused. A scope owns its
coordinate names, carrier and published-ID set behind a mutex. Resolving a key
requires the correct owner and a published region. A batch retains the owner
once; copying keys does not touch the reference count. Dropping the source
variable leaves the batch usable, and dropping the last batch releases the
arena. These lifetime assertions run in the common checker.

For carriers with one-bit relative complement, publication now registers the
whole complementary pair. Flipping a published key's polarity must resolve
immediately, without a second publication. The owner checks test that property
before creating the complement batch, then test involution and unchanged graph
node counts. `bdd-root` remains the explicit comparison whose complement may
require a new support-masked root; its owned operation uses the carrier method.
The production fast-complement contract cannot be claimed from that baseline.

The lab's `publish` method trusts that its caller obtained IDs from that carrier.
It is not a finished public safe constructor; raw IDs remain an implementation
boundary requiring a stronger production API. This prototype also does not
establish concurrent publication, multi-owner batches, spill, rollback or
within-arena garbage collection. It tests coarse whole-owner reclamation.

## 5. Resident equality, transport equality and persisted identity

These are distinct acceptance criteria:

1. Equal canonical resident IDs in one scope mean equal Events.
2. Same carrier/order/coordinate presentation and ordered roots emit identical
   packet bytes despite different allocation histories. The checks include
   reverse input insertion and unrelated preexisting imports.
3. Different carriers can exchange semantically identical values even when
   their packets differ. Restoring to one fixed canonical destination also
   gives common destination bytes for the tested batch.
4. A production per-Event persisted identity additionally needs a canonical
   logical space/law descriptor, root-specific encoding or equivalent object
   identity rules, versioning and reopen integration. This remains open.

The native experiment uses packed512 in bit-major order as one common wire
candidate. It compares complete restored packet bytes with bytes built from
independent oracle bitsets in a fresh target arena. It reports a fingerprint
for aggregation, but never uses that fingerprint as the equality proof.
Different batch root order is intentionally a different packet; packet identity
alone cannot serve as native fact identity.

No paper or benchmark grants cheap conversion into every selected order.
Polynomial equivalence checking and compact canonical compilation are different
properties. A fixed wire order is a cost to measure, not an algebraic free pass.

## 6. Actual Free Join replay

[transfer_bench.rs](src/transfer_bench.rs) first runs the full relational query:
real canonical image/COLT/Free Join, grouped relations, closure, converse,
residuals, May and Must. It exports 24 input roots plus 80 result roots. Each
trial restores into a fresh owned scope and checks every region against the
direct fixture bitsets. Target cases are the same carrier in bit-major and
face-major order, plus common packed512/bit-major for other carriers.
The source query timing excludes cardinality and law observation; its results
are the eighty full Event regions, not scalar summaries.

Destination native rows carry the **new actual scope token** and restored
region IDs. The same Free Join program runs again, and its exact output IDs
must match the restored output roots. This checks that imported Events remain
operands of the full algebra, beyond preserving counts or serialized shape.
The destination evaluator currently clones the restored carrier, and the
expected final roots have already been imported. This does not yet implement
publication of novel query results into a shared native owner or transaction.

Record separate timings for source query, export/encode, strict decode, target
setup, and checked restore/publication. Destination replay and oracle/byte
comparisons stay outside timing. Target setup constructs the fixture support in
the destination presentation; it is more than owner allocation. Restore includes
support proof. The first implementation uses recursive Shannon rebuilding of table leaves.
Thus conversion timings measure this baseline implementation, including its
algorithmic costs, rather than an intrinsic lower bound for each format.

The first full sweep exposed a severe cost in that generic table rebuilding:
all five finite-source processes hit the 60-second limit, with partial verified
results retained in [the baseline](TRANSPORT-BASELINE.md). The next implementation
adds direct table constructors behind a same-binary `direct`/`recursive` control.
Complete-coordinate tables use the existing exact bit-axis permutation and
canonical `import`. Finite carriers can broadcast a local table over their
explicit presentation. Packed carriers can import a table wholly inside their
tail directly as a leaf. Other shapes retain exact recursive fallback.

The shortcut requires every table coordinate to survive the map. If the map
eliminates an axis, polarity still enters the cofactors before existential
union. The [shared check](results/transfer-table-check.json) compares both paths
and an independent explicit-world oracle in 192 cases per carrier, including
1-, 5-, 129- and 256-world presentations, constrained support, reversed maps,
local tables, empty coordinate lists and hidden axes. That is 2,688 new cases.
Native timing and verification require their own compiled revision.

The next `words` control addresses the structure of those tables directly.
For a complete coordinate roster, permute the input bitplane once into the
manager's physical order, recursively split word slices, and intern whole table
leaves. For a table contained in the tail, broadcast absent axes by repeating
words (or repeating a subword pattern), then apply the exact local permutation.
The scalar gather-based `direct` path remains in the same executable as a
control. Both end at the same canonical node constructors and support seal.

The index invariant is simple. A table with axis roster `c` means
`E(w) = table[sum_i bit(w,c[i])*2^i]`. If axis i moves to physical position
`p[i]`, the destination bitplane at assignment d reads the source at
`sum_i bit(d,p[i])*2^i`. `Permutation::dense` implements that exact permutation.
Splitting the reordered array on its high bits now corresponds exactly to the
manager's successive low/high decisions. For a local table, repeating its
`2^k`-cell pattern first makes the added axes irrelevant; the same permutation
then places the retained axes correctly. Finally support sealing and anchored
orientation produce the manager's original canonical Event identity.

This is a use of the table's algebraic structure: broadcast adds irrelevant
coordinates, and a permutation changes only their positions. Neither changes
the source coordinates' meaning or introduces independence assumptions.
The [wide-table check](results/transfer-wide-check.json) adds 96 packed cases over
4,096 worlds, three physical orders, full/local/cross-cut tables and complements.
All three import algorithms must return the independently imported oracle's
exact canonical ID. The earlier 2,688 small cases now also compare all three.

A separate `--transfer-setup product` control uses the declared full-product
constructor for the target space. Symbolic carriers can represent that support
directly; rebuilding a known constant support from a fixture bitplane is not
an unavoidable cost of source construction. The `fixture` control retains the
earlier setup path, and the report keeps them separate.
Here `product` means the fixture's unconstrained binary product. A general full
fibre product of constrained endpoint spaces still retains each endpoint's
support, and may require genuine support construction.

The [matched native controls](REPORT.md#structural-transport-tests-the-representation-boundary)
confirm the word path's effect while retaining every fallback and support check.
[The ARM64 snapshot](results/assembly-transfer-words.json) records whole-leaf
vector loads/stores and vector Boolean instructions in bit-axis permutation.
It is tied to the word revision's native binary and source hashes.

[Generated measurements](TRANSPORT-MEASUREMENTS.md) retain sample ranges and
incomplete process outcomes. The [baseline](results/build-transfer.json),
[direct-table](results/build-transfer-tables.json) and
[word-table](results/build-transfer-words.json) source/build snapshots tie native
evidence to each compiled revision. [Native word-revision verification](results/transfer-word-native-verification.json)
passes all three import modes, including the updated ownership and wide-table checks.

The shared checks passed 784 finite source/target cases, 3,825 support-map cases
and 81 symbolic source/target pairs at forty coordinates. Symbolic packets are
1,037–2,043 bytes for the tested equality family over `2^40` ambient assignments;
that family and its favorable equality-pair orders do not establish universal
compression. [Native verification](results/transfer-native-verification.json)
also retains the Boolean, relational, exact-observation and dependency bridge
regressions.

## Reproduce

```sh
python3 proposal/experiments/event-repr-lab/run.py --no-build --lane transport --layout bit-major --trials 5 --timeout 60 --output my-transfer.json
python3 proposal/experiments/event-repr-lab/transfers.py my-transfer.json
python3 proposal/research/space-maps/checks.py
```

Use an unused output name. Build once if needed before timing; never compile
or run another timing sweep concurrently. The next production decision needs
native per-fact persistence and realistic lifetime/update costs alongside the
existing dependency and full-relation evidence.
