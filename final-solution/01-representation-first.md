# Representation first, including the machine around the facts

## The design move

The user-supplied essay makes the right demand: when a new case appears, change the representation before extending the trace of the computation. Bumbledb already applies this to set-valued facts, final-state judgment, and relational vocabularies. The audit shows where that discipline stopped too early: ownership, publication, snapshots, and cleanup were represented by observations and flags rather than by the evidence their transitions require.

The successor applies the same method to those boundaries. It does not expand the product into unrelated infrastructure.

## Ask four questions of every representation

1. **What does this value mean?** Its denotation must not depend on which host, query plan, dictionary generation, or retry path produced it.
2. **What evidence permits its creation?** Parsing may establish canonical bytes; admission may establish schema laws; a successful CAS may establish publication. These are different certificates.
3. **How long is that evidence valid?** A canonical owned value stays canonical. An observed remote HEAD may change immediately. A managed borrow token can be revoked; a legal Rust page borrow must remain valid until it ends. Treating these as the same lifetime is a category error.
4. **Which branches disappear?** If a new abstraction does not remove independent state, repeated validation, duplicated mechanisms, or a necessary substrate boundary, it has not earned its place.

Types make a smaller set of states representable inside their trust boundary. They do not make disk failure, thread cancellation, a paused process, or a changed remote object impossible. Those events become explicit inputs to a small transition machine.

## Representation changes that pay for themselves

| Old representation | Hidden combinations or duplicated work | Successor | What becomes impossible or unnecessary |
| --- | --- | --- | --- |
| Mutable row references retained across await | Encoded command differs from locally applied command | Sealed owned canonical command | Caller alias changes after recording cannot alter meaning |
| Applied candidate in the ordinary local DB | A concurrent reader sees a transaction later rejected | Uncommitted LMDB candidate | Dirty speculative visibility through ordinary committed snapshots |
| Vacant next log slot | A collected slot looks newly available | Never-deleted tenant HEAD naming immutable decisions | Successful publication into a hole behind recovery's floor |
| Separate arbitrary writer ID, counter fence and fresh placeholders | Healthy writers depose each other; issuance adds command/replay cases | Application-owned 128-bit IDs sealed with all other command data | Allocation authority, lineage-qualified entity values and issuance-only outcomes |
| Vector plus scalar sum | More total work disguises a regressed component | One tenant history order | Incomparable recovery floors and cross-braid partial command receipts |
| Path plus matching schema/generation | Same-shaped state from another database accepted | Incarnation-bound materialization certificate | Misconfigured cache reuse silently serving foreign facts |
| One shared tenant object with aggregate release count | Double/stale release consumes another borrow | Distinct spent/live borrow capability | One borrower releasing another owner or another generation |
| Public C callbacks and dead raw pointers kept diagnosable forever | Extra product surface, tombstones and retained engines | Delete C; bounded internal Node capabilities and scoped Rust ownership | Public C compatibility/diagnostic machinery; retained Node payloads still require testing |
| Global immortal dictionary | Deleted text persists as live dictionary content | Ordinary canonical text in live tuples | Separate dictionary GC/refcount/latch lifetime mechanism |
| Full relation image is the only query input | Database size becomes a RAM requirement | Warm Free Join plus selective indexed access and bounded disk fallback | “Does not fit RAM” meaning “cannot execute”; no mandate to discard the fast warm path |
| Multiple ad hoc spill paths | Sort/hash/recursion each invent storage and recovery | One temporary LMDB ordered-map representation | A second storage engine hiding inside query execution |
| IEEE native equality mixed with bytes/hashes | NaN and signed zero break equivalence across consumers | One canonical binary64 quotient and total order | Set, key, grouping and codec disagreeing about equality |
| Floating sum of scan order | Query plan is observable arithmetic input | Exact mergeable accumulator plus one rounding | Different plan or spill partitions changing the result |
| Clock fields and equality sentinel | Restart changes deletion policy | Explicit retained roots and GC epoch barrier | Clock coincidence becoming deletion authority |
| Handwritten migration callbacks and coverage/checksum scaffolding | Hidden effects, duplicated schema knowledge, repeated full rebuilds | Schema/type declarations generate canonical plan data and history | A JavaScript migration interpreter, helper-closure purity system and manual coverage lists |

These are commitments to remove failure classes, not an excuse to implement all conceivable optimizations. The detailed chapters must name the simplest working mechanism for each row.

## One backend, two execution regimes

LMDB already supplies durable ordered maps, page-backed access, atomic writes, and stable read transactions. Bumbledb should use those properties rather than insist that every relational operation rebuild an independent in-memory database first.

The product's fast path is warm Free Join and selective indexed access over bounded working sets. A complete disk-native path is the correctness baseline and ordinary fallback, not a reason to replace all warm kernels with row-at-a-time cursors or temporary writes. Both consume the same admitted query semantics and produce the same canonical answers. Temporary oversized sets can move through one LMDB-backed scratch representation. Database size is not itself an error condition. Measure a deletion of an existing fast path as seriously as a proposed optimization.

Map capacity, file allocation, page-cache residency, process RSS, and query scratch are different quantities. A large sparse virtual mapping is not a demand to allocate that amount of RAM. Elastic map growth is ordinary backend lifecycle work; it must be coordinated with live transactions according to LMDB's rules, not implemented as an arbitrary user-facing maximum size.

“Just works beyond memory” does not mean pretending an exhausted physical disk or address space exists. It means the program takes its ordinary storage path, returns exact answers, and gets slower as it performs more I/O—without crossing an artificial database-size cliff.

## Scope discipline: the engine remains the engine

The core `bumbledb` is responsible for canonical values, an admitted theory, exact facts, final-state judgment, query evaluation, and LMDB state. It exposes ordinary coherent storage/snapshot primitives only where another layer needs them.

The optional `bumbledb-log` owns history, command identity, materialization binding, remote publication, retained restore points, backup and schema migration. LocalHistory commits directly in LMDB; HostedHistory uses S3 authority. LocalHistory does not emulate the hosted object store. Neither use case requires a fleet orchestration product.

The client layers own host-language ergonomics, not another semantic parser or protocol. The existing schema/query SDK already constructs data and lowers it directly; schema evolution follows that same architecture. Users author TypeScript schema values, from which tools generate canonical schema snapshots, migration plans and checked history. Declarative intent resolves ambiguity; arbitrary migration callbacks are not part of the product. The log's only public SDK is TypeScript, and its native executor runs generated plan data. The proof model owns independent statements of semantics, not a generated restatement of every implementation branch. Tests own independent expectations, not snapshots of whatever the implementation happens to emit.

## Cost is part of the representation

Choosing canonical NaNs removes equality ambiguity but deliberately loses NaN payload identity. Choosing exact floating reductions costs more than unchecked native addition but removes order dependence. Choosing a single tenant authority removes vector and split semantics but imposes tenant-wide publication contention. Choosing raw text removes dictionary lifetime machinery but can cost space for repeated labels. These are explicit engineering trades, not free theorems.

Measure them at the intended per-user application boundary. The sibling M2 Max ledger requires regime, antagonist, machine-code inspection and stamped evidence; use that method without importing its numbers to Graviton or x86. A smaller function that creates more work in every host is not a smaller system. A larger data value that deletes three recovery protocols may be an excellent bargain, but a 28-byte ID that introduces issuance machinery is not justified when a 16-byte application ID suffices. A microbenchmark win that requires preserving a broken visibility contract is not a win.

## What the essay does not license

- Fewer `if` tokens is not a correctness criterion. Essential alternatives still need a total interpretation.
- A half-open interval convention reduces boundary ambiguity; it does not prove arbitrary indexing code has no off-by-one bugs.
- A smart constructor proves only what it actually checks, for the values it owns. It cannot validate tomorrow's mutable array or fence tomorrow's remote write.
- A table-driven machine is useful when its state/event algebra is smaller and clearer. A universal interpreter framework would defeat the purpose.
- A new sum type should encode a real distinction. Do not create a public type, adapter, crate, or service for every paragraph of this document.

The standard is ruthless but practical: **the data model should explain the code that remains**.
