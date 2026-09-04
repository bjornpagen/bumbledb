# 10 — A small semantic engine on LMDB

Status: **proposed Bumbledb 1.0 design, not implemented or verified**. This replaces, rather than incrementally repairs, the affected 0.20.3 contracts. The dated [engine audit](../audit/20-engine-semantics.md), [query audit](../audit/21-query-runtime.md), and their counterexamples remain evidence about the old implementation.

## The decision

Keep LMDB. Keep final-state relational admission. Keep genuine set answers. **Keep Free Join and the carefully measured application hot paths.** The product is a fast per-student/per-user/per-tenant application database: embedded on Apple Silicon, or hosted with a local materialization on Graviton ARM and qualified x86-64 Node hosts. It is not a new analytical warehouse. Remove representation accidents that force applications to understand internal allocation, dictionaries, partial output, or a particular memory-size regime.

The core knows **facts, laws, queries, canonical values, transactions, and ordinary LMDB snapshots**. It does not know S3, HEAD, named-command/history identity, receipt epochs, migration histories, or backup retention. Those belong to `bumbledb-log`, including its local-history adapter. Application-owned entity IDs are ordinary canonical core values, not identities issued or owned by the log. A dependency boundary, not a naming convention, enforces this split.

Public language scope is deliberate: the **core has Rust and TypeScript APIs only; the public C API is dropped**, including its header, packaging and support promise when implementation starts. The **log product is TypeScript-only in 1.0**, backed by one internal Rust implementation. Internal native bindings are not a public C product. The local-history convenience mentioned here is likewise part of that TypeScript log surface. Schemas and queries are direct typed AST values; macros/builders construct the shared validated representation. There is no SQL/text-query parser project.

The point is not to make every number and pointer a new framework. It is to have a few representations whose construction establishes the facts their consumers need.

## 1. Semantic constitution

1. A relation contains a finite set of complete, typed tuples. Repeating the same fact is a no-op; distinct facts remain distinct even if an accelerator hash collides.
2. A transaction proposes a final state. Normalize a collection of insert/delete intentions to a net delta; judge that final state, not arbitrary statement order. A key replacement is not temporarily illegal because its deletion and insertion were spelled in an inconvenient order.
3. Law failure is a domain rejection, not an I/O exception. Cancellation, resource exhaustion, malformed input, and corruption are separate failures. No failed operation returns an apparently complete partial result.
4. Normal readers see a committed LMDB snapshot. A prepared private write is not a readable published state. The log layer decides when an accepted candidate is authorized to become committed locally.
5. Logical value identity is independent of local row numbers, memory addresses, cache ordinals, LMDB page numbers, and dictionary allocation order.
6. A query denotes a set of bindings and then a set of answers. Aggregates fold the distinct binding set, not an implicit bag of join paths. Two entities with the same amount contribute twice only when both identity-bearing bindings remain in that set.
7. Typed nonrecursive relation expressions compose, including aggregate and computed outputs consumed by another query stage. Value creation does not silently make recursion an unbounded programming language: the supported recursive fragment remains positive, projection-only, finite-active-domain linear recursion over frozen inputs, with no aggregation or value creation in its feedback cycle.

These are specifications to implement and challenge, not claims inherited automatically from the current Lean files. [13](13-lean-and-rust.md) records the new proof boundary.

### The delta tie rule is fixed

A command's inserts and deletes are sets. Canonicalize `Delta(add, remove)` to `(add, remove \ add)` and define application to state S as `(S \ remove) ∪ add`: **add wins when the exact same fact occurs on both sides of one atomic command**. Input iterator/call order does not choose the result. Adding a fact already present and removing one absent are ordinary no-ops; the resulting candidate still undergoes all final-state laws.

This is a normalization rule **inside one command**, not last-writer-wins or add-wins conflict resolution between independently published commands. Two commands are applied in their actual local/log order. Generated, dynamic, wire, scratch, and replay paths must use this one normal form.

## 2. One canonical value boundary

The core scalar vocabulary is `Bool`, `U64`, `I64`, `F64`, UTF-8 `Text`, and fixed-width `Bytes<N>`. Keep explicit integer domains and nominal application wrappers: account identifiers, timestamps, and money are not interchangeable because they occupy the same number of bytes. Nominal wrappers lower to a structural scalar; their application name is not a new execution kernel.

`F64` is the fully specified binary64 domain in [11](11-floats.md), not a string convention or an untyped eight-byte escape hatch. Text means exact UTF-8 bytes: no implicit locale collation, Unicode normalization, or case folding. Applications can normalize before storage when that is their chosen equality.

There are two public input paths and one internal representation:

```text
generated typed facts ── safe typed field encoder ─┐
                                                 ├─ CheckedDelta
external bytes ── schema-bound canonical parser ──┘
```

`CheckedDelta` owns its bytes, schema identity, and layout. It is the native checked representation of the public **`ChangeSet`**, not a parallel change language or a second sealed payload. Typed Rust/TypeScript change builders and dynamic ingestion converge on that same owned value. Its constructors establish widths, canonical bool/float values, UTF-8, interval bounds, and the absence of dangling internal references. It does not claim the proposed facts satisfy the schema laws; that is the next boundary.

Do not retain a safe extensible trait whose successful `encode` call is treated as a proof about arbitrary output. Custom integrations either emit typed fields through the safe builder or submit bytes to the checked parser. Generated code uses the same field primitives; a proc macro is not a privileged unchecked constructor. Internal raw construction is crate-private and small enough to audit. There is no public `__ground_axiom` bypass.

Wire encodings use explicit tags/lengths and fixed endianness. A versioned canonical parser rejects overlong encodings, trailing fields, wrong lengths, and noncanonical float payloads. Host constructors normalize floats; wire parsers reject alternative encodings, so a signed or hashed command has exactly one byte representation. Encoding and LMDB index order are separate concepts where necessary.

### Generic intervals without a second temporal engine

Retain discrete `Interval<U64>` and `Interval<I64>`, and add **`Interval<F64>`**. All are checked, half-open and nonempty, with a canonical two-word/16-byte endpoint representation. The shared interval algebra needs ordered endpoints, not integer successor arithmetic. Keep its element family sealed; generic does not mean arbitrary host comparators or user-defined numeric kernels.

Integer intervals retain their existing point domain: **represented temporal points exclude the maximum integer**, which is reserved as the ray endpoint. Ordinary integer fact fields still have their full integer domain. State that distinction in public type documentation rather than suggesting every integer can be contained in an interval.

Consequently `[MAX-1, MAX)` is a ray in this language, `ray(MAX)` refuses, and a fixed interval must have positive width and a checked endpoint strictly below `MAX`. A ray's duration refuses instead of returning a finite measure. The ordinary constructor, constant/macro constructor, and dynamic parser enforce the same interval domain. An ordinary scalar membership probe at `MAX` is a well-defined nonmatch, not corruption or a global rejection of that integer value; parameter sets and negation follow that same rule. No public safe unchecked constructor remains.

Float intervals use canonical binary64 endpoints on a **dense numeric line**, with `-Infinity` and `+Infinity` as unbounded endpoints only. NaN is forbidden at either endpoint; signed zero normalizes before validation; `start < end` uses numeric endpoint order. `[-Infinity, +Infinity)` is the whole line. A finite F64 membership probe is embedded exactly; every nonfinite scalar probe returns false. These are continuous numeric ranges, not sets of representable machine floats. This distinction makes `[-Infinity, -MAX_FINITE)` a valid nonempty left ray, and keeps a positive gap between distinct adjacent representable endpoints meaningful even if no machine float lies strictly between them. Full float semantics and examples are in [11](11-floats.md).

Allen relations, overlap, coverage and pack/coalescing use the same ordered-endpoint algorithms after canonical type-specific order encoding. Never feed raw F64 payload bits to an unsigned integer-order kernel. Integer and float intervals do not mix implicitly. Keep integer fixed-width intervals; **do not add `FixedInterval<F64>`**: rounded `start + width` is not an exact fixed-width representation. Float bounded length is a correctly rounded numerical length, not a count of representable points; nonfinite bounds and finite-length overflow have distinct failures.

Approximate float capacity weights, float-length capacity measures, locale collation, nullable/three-valued SQL semantics, decimal, and arbitrary user-defined execution kernels are **not** added. Grouped capacity admission retains exact nonnegative integral measures and scalar grouping keys, as specified below. Merely storing a float interval elsewhere in a row does not turn its group total into pointwise temporal occupancy. Integer interval duration laws retain their exact integer meaning. First-class float sum **and mean remain in 1.0**; their measured cost must be optimized within the specified numerical contract, not used to quietly remove them.

### Exact grouped measures, not weighted relations

Keep the existing useful capacity law. For each selected target/parent fact `p`, collect the **distinct complete selected source facts** whose scalar projected key equals `p`'s projected key. Sum their declared measure and check the target's window. The target key/selection requirements remain checked schema premises. A missing selected parent imposes no group window; an existing selected parent with no children has total **zero**. This is distinct from the query aggregate rule, where no input binding creates no answer row.

Use one checked internal measure/window representation:

| Component | Selected finite grammar |
| --- | --- |
| Source measure | `Count` (one per distinct source fact), `U64Field(source_field)`, or `IntegerDuration(source_interval)` |
| Floor | Nonnegative integer literal in the measure's domain |
| Ceiling | No ceiling, a nonnegative literal, or the existing dimension-compatible target-row field/duration bound |
| Grouping | Matching scalar key projections of one selected source and one selected target relation; no interval projection |

Count is the unit-measure case, not a second admission engine. Row weights are actual source `U64` fields; duration is the exact length of a supported bounded integer interval. Retain nonnegative typing and dimension checks, no dependent floor, and explicit refusal of undefined ray duration even when an absent parent would make the group law vacuous. Accumulate exactly in the proved widened integer domain; never wrap or narrow a witness. A zero-weight child still exists: weighted total zero does not prove that the group is empty, so count/containment and weighted measures are not interchangeable without the required premise.

This law measures **whole scalar-key groups**, not the number/weight of intervals simultaneously active at each point. Two overlapping source intervals contribute both complete durations when duration is selected; the law does not union their spans or enforce pointwise occupancy. Source rows may contain intervals as ordinary fields, but interval grouping/coverage-counting, float length weights, implicit joins to fetch a weight, signed weights, and weighted/bag relation semantics are not introduced. Cross-row weight lookup remains an explicit application/schema modeling choice, not a hidden admission query. Ordinary queries may explicitly join relations under their own set semantics.

Simplify authorship without policing harmless spellings in every client. Typed count/measure/window conveniences lower once to the canonical representation; accept an exact window and an equivalent equal-bound range, and normalize other explicitly proved aliases **within that law family**. Keep unit weight and an existence window representable as grouped measure. Do not blindly rewrite it to ordinary containment: containment's target-key admissibility may differ from a many-child capacity group, even when their formulas appear equivalent. Any cross-family rewrite or tautology elimination must preserve denotation, accepted domain and authored statement attribution for diagnostics, including every key premise. Do not build a general equivalence prover or retain separate macro/TypeScript/wire ban tables. Structural/type errors, inverted literal bounds, unsupported dimensions and genuinely different semantics still refuse; surface flexibility never bypasses the checked canonical boundary. Canonical wire data retains one encoding even when host builders accept equivalent forms.

The source-backed baseline is [Weight/Bound/Capacity](../crates/bumbledb-theory/src/schema.rs), [capacity validation](../crates/bumbledb/src/schema/validate.rs), and [the grouped denotation](../lean/Bumbledb/Capacity.lean). Their current spelling restrictions are not immutable semantic laws. Conversely, their scalar-key whole-group meaning must not be broadened by calling the feature “temporal capacity.” Query relation expressions do not become new arbitrary schema assertions; laws still quantify over the selected stored-relation grammar.

## 3. Store tuples, not allocation history

### Default physical representation

Use ordinary LMDB named databases or namespaced keys for rows, membership, schema-declared indexes, and small core metadata. Rows contain canonical scalar payloads and inline variable-length text in the LMDB value. Physical row IDs are local surrogates only.

Remove the default global text dictionary. A deleted row should not leave an independently live dictionary entry forever. Repeated low-cardinality application vocabulary can use a closed relation and its ordinary identifier; that already expresses the semantic distinction between a symbol and prose. This costs repeated text bytes and may lose compression on some workloads. Measure it; do not build refcounted interning, a second collector, and dictionary-epoch cache invalidation before proving those mechanisms necessary.

LMDB MVCC naturally retains the previous row version while an old snapshot needs it. Deleting facts does not promise physical secure erasure of freed pages, filesystem copies, or log/backup history. A clean logical export contains only live rows; policy for using it as a backup, erasure rebuild, or migration is entirely in `bumbledb-log`.

### Exact equality despite finite hash keys

The current `fact_hash` explicitly makes hash equality fact equality. 1.0 removes that logical axiom. Hashes select candidate buckets; full canonical tuple bytes decide equality. The selected persisted local fingerprint is **16 bytes**, the first 16 bytes of a domain-separated BLAKE3 digest of canonical input; a membership index can use `(relation, fingerprint, local-row-id)` and obtain candidate row values before exact comparison. The same exactness rule applies to hashed large determinants, grouping, and deduplication. Temporary fixed-word tables may retain their measured 64-bit routing hash with full-key checks. Authoritative schema/command/remote-object integrity uses its separate 32-byte digest. [41](41-storage-and-hashing.md) owns the pre-format fingerprint benchmark, precise role separation and collision argument; there is no public hash-algorithm registry.

Read LMDB's actual maximum key size at open; `heed 0.22.1` documents the usual limit as 511 bytes. Never put arbitrary text or a complete large tuple into a key and assume it fits. Fixed scalar indexes use order-preserving bounded keys. Large determinants use exact-checked hash buckets. A text range predicate without a suitable bounded index has a correct scan fallback. That is slower, not an excuse for a wrong answer or a hidden field truncation.

Canonical logical export orders by relation and the versioned tuple fingerprint, then full canonical bytes within a collision bucket. Physical row IDs never enter its logical identity. For a deliberately adversarial enormous collision bucket, a repeated bounded-memory minimum scan is an acceptable slow fallback; no unbounded in-memory collision list is required. A future faster collision sorter must preserve this behavior. Cryptographic snapshot integrity still assumes the chosen 32-byte digest's collision resistance; **logical tuple equality does not**.

No row-store rewrite, page Merkle tree, second persistent storage engine, compression service, or automatic text-index subsystem is part of this proposal.

## 4. Admission must describe the proposal, not a failed landing

Build the proposed final relation view from the committed snapshot plus the normalized delta. Judge all relevant laws over that view. A physical uniqueness index is an accelerator and an installation target, not the definition of which proposed facts exist.

During judgment, tentative determinant entries are a multimap or a scratch relation. This permits evidence for all conflicting proposals even when a physical unique index could not install the first one. Once the candidate is admitted, its unique indexes can be installed without semantic ambiguity. The simple reference path may inspect the full affected relation; incremental checks may substitute only when equivalence is established.

For a completed semantic rejection, return the complete set of violated statement IDs with a bounded, explicitly labeled number of example facts per statement. Do not promise every pair of conflicting facts. If the budget expires before all statements are judged, return `ResourceExhausted`, not a falsely complete rejection. Diagnostic order is canonical by statement ID; fact examples may use a specified bounded selection order.

Core API sketch, illustrative rather than compilable current API:

```rust
fn parse_delta(schema: &Schema, input: &[u8]) -> Result<CheckedDelta, InputError>;
fn prepare_write<'a>(
    owner: &'a mut WriteOwner,
    delta: &CheckedDelta,
    ctx: &mut WorkContext,
) -> Result<Admission<PreparedWrite<'a>>, EngineError>;

impl<'env> PreparedWrite<'env> {
    fn seal(self, host_changes: HostChanges) -> Result<SealedWrite<'env>, StorageError>;
    fn abort(self);
}
impl SealedWrite<'_> {
    fn commit(self) -> Result<CoreCommit, StorageError>;
    fn abort(self);
}
```

`PreparedWrite` owns the uncommitted LMDB write transaction and its admission evidence. It is not clonable and exposes no public committed read capability. Core ordinary writes prepare, seal with no host changes, and immediately commit. The log layer can hold the candidate on its owning worker across its bounded publication attempt; see below. No application callback runs inside that interval.

The sole attachment mechanism is a narrow integration-only opaque host-record namespace in the same LMDB transaction: bounded keys, owned values, bounded per-command changes, and snapshot iteration. It is **not** a public key/value product or extensibility framework. Core does not interpret these records as a command, receipt, schema law, or backup. Normal application facts/queries cannot read or mutate this namespace. The log owns its grammar and uses small metadata records plus individually keyed receipt records; no forever-growing metadata blob is needed.

`seal(HostChanges)` exists for a specific ordering requirement: the log can compute a decision hash only **after** application admission determines its outcome, but must store the receipt containing that hash in the **same** private LMDB transaction before remote publication. Sealing may change only opaque host records, never application facts or indexes, so it cannot invalidate the admission evidence. A sealed write is then commit/abort only. A read snapshot obtains facts, attachment, and host records from the same transaction.

For a domain rejection, the application candidate aborts. The log retains its exclusive writer-session ownership, prepares an empty application delta against the unchanged committed parent, seals the rejection receipt/metadata, and uses that private transaction for publication. There is no gap in which an unrelated local writer may change the base. This is a concrete two-case flow, not a general transaction amendment API.

## 5. Identity and snapshots without a history framework

The core has a persistent `CoreStoreId`, a per-open `EnvironmentId`, and a checked monotone local `Generation`. Generation advances on any durable core transaction that changes facts or attachment; it is not a hosted decision sequence or an application-state witness. Environment identity protects borrowed plans/caches from a different native environment even if it opened a copy of the same store.

`OwnedSnapshot` owns one real LMDB read transaction. It provides its generation, core identity, and attachment **from that transaction**, and bounded cursors over that same view. A core export/copy helper consumes this snapshot; it never opens a second source view midway. This closes ENG-003 by construction.

The log wraps it in its own certificate containing `DatabaseId`, `IncarnationId`, `DecisionStamp`, `StateStamp`, and system-state evidence. A renamed bucket, wrong cache, rollback, or migration is resolved by that layer before it exposes the materialization. The core does not infer a database's authoritative identity from a path or schema hash.

**Delete database-generated identity machinery.** There is no `reserve<u64>`, `FreshRef`, sequence-derived 28-byte entity ID, abort-burn counter, or fresh-result expansion. The normal application ID is a nominal canonical `Bytes<16>`/128-bit value, chosen once by the application or SDK convenience before sealing a command and reused unchanged across retries. It is distinct from the command's request identity, even though both can be 128 bits. Its bits carry no database sequence/incarnation authority, and migration/restore preserve them as application data. Ordinary schemas may still choose other scalar key domains. A key law detects conflicting facts; neither the core nor a UUID helper proves a collision mathematically impossible. The log owns request identity and receipt semantics, not entity birth.

## 6. LMDB larger than RAM is a first-class case

Remove the fixed `MAP_SIZE = 32 << 30` policy. A mapped address range is not resident RAM, and an on-disk database is not a query-memory allocation. Support 64-bit environments with an elastic map, sized from actual allocated pages plus headroom and grown geometrically/page-aligned. The exact growth factor is an implementation tuning parameter, not a semantic database-size limit.

Do not pre-touch the map or eagerly load the database to prove it fits memory. The operating system pages LMDB data; query-owned scratch and caches have separately controlled resident budgets. The database can be much larger than RAM. On sparse-capable filesystems, virtual reservation and allocated disk blocks are separately reported. On other supported filesystems, growth must respect actual allocation behavior rather than assuming sparse files are free.

### One owner, safe resize

Choose the existing useful restriction: one process owns a local materialization, protected by a kernel-held lifetime lock. Threads share one native environment; a second process opening that directory refuses before recovery or mutation. Multiple hosted replicas use different directories. This avoids inventing an interprocess map-remapping protocol.

An environment-wide transaction gate counts all read/write transactions and borrowed page views. Resizing requires exclusive gate access and zero live transactions; stop admitting new transactions, wait or return a bounded `ResizeBlockedByReaders`, then call `heed::Env::resize` in its one audited unsafe wrapper. **A writer mutex alone is insufficient.** The installed heed documentation explicitly requires no active transactions; the library does not check this condition.

Large initial virtual headroom makes resize uncommon, not optional. No raw mmap-backed pointer survives a snapshot/transaction or crosses a resize. A long application-held snapshot may block growth: expose its age/owner and let the caller release it. Do not silently invalidate a Rust borrow to meet a deadline. On close, drain transactions, drop the environment, then release the ownership lock.

If shared-process-directory support is ever introduced, every participant must join the map-size protocol and handle `MDB_MAP_RESIZED`. Calling resize in one process while another retains unregistered views is unsupported, not made safe by a metadata generation check. A 1.0 multiprocess refusal test is mandatory.

### Map-full and storage failure

For `MDB_MAP_FULL` during a private candidate: abort that LMDB transaction, obtain exclusive resize access, grow subject to real filesystem/address-space availability, and reapply the same owned canonical delta. Do not re-invoke application code or partially commit. Bound attempts by actual progress: a failed/no-growth resize returns a typed failure instead of looping.

Perform page-producing candidate writes before the remote CAS. If a later local commit fails **after** a confirmed remote publication, the log must retain/report the published decision and mark its materialization unhealthy; a local failure cannot turn that decision into a domain rejection. Recovery replays it into a repaired materialization. If publication is unknown, abort the private local transaction and retain the log's immutable attempt capsule for resolution. The core merely reports its local error.

`ENOSPC`, quota exhaustion, map-address reservation failure, reader-slot exhaustion, corrupt pages, and map-size exhaustion are distinct diagnostics. A host may impose explicit tenant disk quotas; that is not a built-in 32 GiB database ceiling. Free-space checks reserve useful headroom but cannot prove a concurrent filesystem write will succeed. The actual commit result remains authoritative.

Native LMDB files are local physical storage tied to their supported page/word/endian/platform envelope. They are authoritative for core/LocalHistory and disposable materializations only for HostedHistory. Canonical streamed rows are the portable interchange boundary used by the log. Do not promise that a raw `.mdb` copied across arbitrary architectures will open correctly.

## 7. Private candidate hosting

The selected hosted path keeps an uncommitted `RwTxn` on its owning worker in the bounded shared executor while the log's conditional HEAD publication is in flight. Existing ordinary `RoTxn`s continue to see committed state; new ordinary readers also open committed snapshots. Never use heed's special nested-read-of-write facility as an ordinary read surface.

The worker does not move the transaction through an arbitrary async executor. It owns it until commit/abort; network completion is delivered as data. Per-attempt deadlines and cancellation bound waiting. On a CAS loss, abort, catch up, and rejudge the same immutable command. On an unresolvable response within the budget, abort local candidate and hand control back with an explicit unknown publication outcome. The durable log owns retry evidence.

This holds LMDB's one-writer slot across one bounded network attempt. That is a real throughput tradeoff, accepted for the simpler per-tenant total-order design. It does **not** clone a database per candidate, expose speculative state, or serialize unrelated tenants on this tenant's LMDB writer lock. Shared worker/transport capacity can still queue unrelated tenants; budget and measure that contention using chapter 31's existing fair admission. No per-tenant thread or candidate-overlay subsystem is added in 1.0.

## 8. Audit disposition and acceptance obligations

All entries below are **proposed gates; not executed for this successor**. Every fixture must fail the old faulty behavior where applicable and pass the new contract, using independent expected values.

`E-DELTA` requires all permutations and repetitions of add/remove operations, the same exact fact on both sides, present/absent base membership, replacements under multiple laws, and two separately ordered commands. Normalization is idempotent, same-command order does not matter, and separate-command order retains its specified meaning.

| Audit IDs | Representation decision | Required executable obligation |
| --- | --- | --- |
| ENG-001 | Closed typed value constructors, explicit temporal point-domain contract | `E-VALUE`: downstream invalid bool/interval/width/float constructors cannot produce admitted malformed rows; macro constants use the same check; MAX/ray/fixed-width boundary fixtures |
| ENG-002 | Typed encoders or checked external bytes, no trusted safe raw trait | `E-CODEC`: downstream custom encoder attempts wrong relation/width/padding/text reference; reject before LMDB mutation; generated/dynamic/wire equality |
| ENG-003; REP-014; PERF-004 | One owned RO snapshot plus same-transaction attachment | `E-SNAPSHOT`: deterministic pause during export plus concurrent commits; all rows, generation, attachment, and log wrapper certificate name one snapshot |
| ENG-004/007; REP-004/006; SDK-015 | Remove all database-generated entity IDs; application-owned values are sealed once | `E-NO-RESERVE`: old reserve/FreshRef API absent; retries/lost publication/migration preserve chosen 128-bit entity bytes; no hidden allocator write or fresh-result path |
| ENG-005 | Judge a proposed multimap before unique index installation; one canonical grouped-measure law | `E-ADMIT`: original two refused fresh-key rows sharing email plus permutations report both statement IDs; compare naive/disk judgments; count/u64/duration windows, many-child unit-existence windows, empty-parent versus empty-child groups, zero-weight children, overlapping duration totals, ray/dimension refusals and equivalent-builder normalization preserve exact verdicts/accepted domain/statement attribution |
| ENG-006 | Inline text; live-row export; no global append-only dictionary | `E-TEXT`: unique text churn, delete/export/reopen contains no live deleted-text entry; old pinned snapshots still correct; no secure-erasure claim |
| ENG-008 | Durable defaults; unsafe bench policy absent from normal production API | `E-DURABILITY`: downstream production build cannot select benchmark no-sync; power-loss/process-failure matrix names actual acknowledged guarantee |
| REP-016; SDK-008/014 | Read capability excludes mutation and private candidate | `E-VISIBILITY`: read while a candidate later wins/loses/rejects/errors; reads contain only committed facts |
| SDK-016; ARCH-004 | Core snapshot identity plus log-owned authoritative identity binding | `E-ORIGIN`: core foreign-environment plan refuses; log wrong-prefix/equal-generation cache gate runs before any read or pending recovery |
| PERF-001/002; QRY-002 | No compulsory whole-relation image, elastic maps and bounded scratch | `E-LARGE`: >32 GiB logical/data-file fixture where feasible; working set >RAM; same answers with hot/cold pages, bounded cache and no arbitrary size refusal |
| ASS-001/003 | Models rewritten to actual mutable support and current boundaries | `E-BRIDGE`: theorem premises mapped explicitly; current examples compile against packaged API; historical claims labeled |

Additional required tests: forced identical hashes with unequal very long tuples; LMDB key-size boundary; all scalar index orderings; metadata-only and no-op transactions; physical row-ID exhaustion; repeated map growth with pinned readers; cancellation waiting for resize; reopen after `MAP_FULL`/disk-full; process death before/after local commit; lock-release after close; 64-bit Linux x86-64/ARM64 and macOS ARM64; independent fresh restore of a >RAM materialization. True power-loss qualification is a separate authorized hardware/filesystem gate, not inferred from process exit.

The design earns its complexity only if these paths converge on the same canonical tuple, snapshot, and transaction representations. No storage-size, erasure, proof, or performance claim is accepted because an explanatory comment says it is true.
