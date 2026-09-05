# The small machine we intend to release

Status: binding target for the new 2026-09-05 convergence swarm. Earlier completion labels are withdrawn. This packet is a proposal/handoff, not a passing release or implementation performed by the reviewer.

## Product constitution

Bumbledb is an exact set-semantic relational application database, implemented with LMDB and Free Join. It serves one user/student/tenant at a time. Rust is the computational substrate; TypeScript uses Effect 4 for operations and ordinary synchronous values for schema/query metadata. The optional log layer owns named commands, publication, recovery, retention, backup, and generated migrations. The core has no dependency on the log, S3, or application routing.

The representation-first essay is our method, not permission to invent abstractions. Parse once into a value that preserves what was learned. Make ownership and valid transitions explicit. Compile repeated schema reasoning once. Distinguish genuinely different semantics instead of hiding them behind options. Reject abstractions that add more concepts than mechanisms they remove.

The owner permits breaking pre-1.0 formats and APIs. The next swarm preserves good current implementation rather than resetting the branch. Existing facts and useful implementation are not disposable merely because compatibility is. Do not delete user data, rewrite a production tenant, reset a format identifier onto old bytes, or publish packages as part of an implementation run. The format family, canonical semantics and ABI must unambiguously reject incompatible predecessors before cleanup or mutation.

## Representation first is a release condition

The owner's representation essay is binding engineering method, not an inspirational preface. For each repaired mechanism answer, in the code review rather than a new report file: **what invalid state or repeated interpretation caused this class of bugs; which representation now prevents it; and which old machinery was deleted?** A comment promising an invariant, a guard added at one call site, or another wrapper around the old mechanism is insufficient when ordinary code can still construct the bad state.

Preserve learned facts in types and ownership: raw bytes become checked canonical rows; a sealed theory carries its compiled projections; an owned allocation cannot shed its charge; a published receipt cannot be downgraded by optional decoding; a staged store is not a ready database; a stale capability cannot become a new owner's capability. Parse at the trust boundary, then use the admitted representation internally. A persisted/untrusted input remains a trust boundary—do not erase corruption checks under the slogan “validate once.”

Replace independent mode booleans with a small sum of real states and state-specific payloads. Replace repeated schema switches with compiled descriptors. Share actual primitives, not interfaces implemented twice. Prefer a direct concrete type or existing module over a generic trait/plugin/strategy hierarchy unless the latter removes several real mechanisms. In hot Rust loops, retain compact layouts, borrowed views and monomorphized/batched execution; representation-first does not mean one heap object, dynamic dispatch or Effect per tuple.

Do not flatten essential distinctions: exact equality versus fingerprint routing; local LMDB commit versus remote conditional publication; operation budget versus retained cache ownership; discrete versus dense intervals; query binding grain versus numerical argument equality. Two physical strategies serving one checked contract are not two competing product implementations. Correctness fallbacks, independent test oracles and separately qualified backends survive because they have distinct necessary roles—not because compatibility is protected.

## Hard cutover, no compatibility tail

This is one selected initial-release API and format family. **No pre-1.0 compatibility readers/importers, aliases, deprecation wrappers, dual-write modes, old/new feature switches, version-dispatch matrices or replacement-plus-legacy implementations ship.** Remove obsolete public callbacks/allocator/replica/sync APIs, stale formats, unused dependencies, duplicate codecs and their unsupported examples/tests in the same integrated change. Generated migrations remain fully supported *within the selected new product*; they are not permission to build an old-engine importer.

Unsupported predecessor stores/commands/plans/native packages fail early and leave existing data untouched. Minimal predecessor rejection cases remain valuable; full predecessor runtimes/corpora do not. Reusing a numeric layout counter is allowed only under a new unambiguous format-family identity. Reprovisioning or separately exporting real old tenant data requires explicit owner action, not an automatic startup side effect.

Every worker owns the replacement **and** deletion of its superseded path. No shim is introduced merely to let another lane compile during the swarm. No knowingly obsolete path survives final review as “cleanup later.” Update downstream repository examples directly to the selected API. Scope includes removing dead fixtures/helpers/exhaust directories, while retaining necessary independent expected data beside its consuming tests. Do not target line-count reduction by deleting real safety or semantic evidence.

Final review must find one current owner of every semantic fact, resource, authority transition and public operation. A justified physical fallback or independent oracle is named for that role; anything retained only to keep the old design alive is removed. This is how the seven commitments below become a clean cutover rather than another layer of code.

## Seven commitments

### 1. Compile the theory into one reusable machine

A sealed schema produces one `CompiledTheory`: canonical row layouts, normalized projections, law dependencies and lawful optimization witnesses. Storage, admission, point lookup and query planning consume it. They do not each reinterpret the schema.

A projection has semantic identity independent of physical statement numbering. A physical access path has a checked key encoding and an exact equality/order contract. Compile-time selection is `ExactBounded` or `FingerprintBucket`; no CPU- or row-dependent persistent hash choice. Share an index only when equality, order, filters, domain and candidate-state requirements match. One logical law may use several access paths; several laws may use one. Complete judgment establishes lawful state; incremental judgment consumes that premise. An unready populated store cannot acquire it from an empty delta. Do not mistake relation-key uniqueness for a physical unique index that cannot temporarily represent a rejected candidate.

### 2. Make small changes stay local where the law permits

Application mutation, law judgment and cache invalidation are distinct consequences of one candidate delta. Compile affected-law and projection dependencies; skip unrelated laws and reuse unaffected relation versions. Use exact determinant-local work for key laws. Upper capacities and containment can require larger neighborhoods; establish equivalent affected-group algorithms, not an unsupported universal O(delta) claim.

Retain a small streaming reference judge as an independent specification target. The indexed path must agree on admission and meaningful diagnostic evidence. A test must not require the optimized path to scan or spill just because the reference path does.

### 3. Make retention carry its own cost and release

Represent retained decoded values, buffers, images, group state and results with owners that keep allocation capacity and charges together. A caller cannot extract an owning `Vec` and accidentally drop its reservation. Charge before growth; bind refunds to actual release, not the end of a function that leaves reusable capacity alive.

There are operation work/input limits, tenant/runtime retained-cache limits, scratch-disk limits, and caller-owned result/page limits. They are related but not interchangeable. One operation must not invisibly finance an immortal cache, and a later operation must not borrow unaccounted capacity. Detailed accounting policy and transfer rules live in the core and interface chapters.

Use one scratch substrate shared by admission, deduplication, grouping, derived relations and results. Charge ownership is inside shared allocations, not beside cache references; resolver generations are retained by the words they interpret, not detached pin counters. Preserve RAM-first application performance. Spilling is a physical choice, never a change in denotation. Text, derived stages, results and transport must have a bounded path too. Large values may be refused by an explicit per-value/operation policy; total database size has no arbitrary RAM or 32 GiB ceiling.

### 4. Keep Free Join and exploit known facts

Keep the resident COLT/lazy-trie, factorized plan, cover selection and batched probing architecture. Add no second optimizing query language. Prepare once; reuse safe versions and buffers. The same sealed plan/typed operator contracts govern resident and cursor/scratch execution.

Existence-only suffix cancellation, key-backed distinctness, direct key probes and eligible fused folds are conditional optimizations with explicit witnesses. Set storage does not make every projection distinct; aggregates consume distinct bindings, not a set of numerical arguments. Stage error/rounding boundaries are semantic, not incidental materialization choices.

Poll cancellation and work limits at bounded exploration/build quanta even if no result is emitted. Account for COLT construction and retained pools, not only image input or successful output. Select the cursor fallback before a row-position representation overflows; do not widen every hot index just to accommodate a nonresident regime.

### 5. Keep log as publication and lifecycle, not another database

Core owns admitted candidates, pinned snapshots, exact rows/changes, private staged database construction, and atomic opaque adjunct records. Log owns the interpretation of adjuncts: history identity, publication evidence, receipts, recovery roots and migration lineage.

One local LMDB commit is local publication. One successful hosted conditional HEAD replacement is hosted publication. A transport error after dispatch is not proof of nonpublication. Use one state-specific attempt sum so certainty cannot be erased by a generic error conversion. A freely chosen phase field beside a freely chosen outcome is expressly rejected; see C5.

Initialize, restore and migrate reuse staging, validation and publish primitives where contracts match. Source freeze, target activation, and backup retention are genuinely different protocol transitions; do not merge them into a giant configurable workflow engine. Current recovery state and named roots protect exact objects; timestamps and object-list absence are not deletion authority.

### 6. Finish the public product through a real application

Both TypeScript packages share the same schemas, IDs, queries, changes, readers, results, policy and native runtime. Fixed workers retain snapshots/prepared state as data and return to scheduling after each job; idle sessions do not occupy worker stacks. Delete unused JS-driven write-session ABI instead of carrying its reactor into the new design. Pure declarations remain synchronous. Operations return lazy Effects; pages are bounded Streams from a completed owned result. No Promise/sync/disposal twin, no per-row fibers, and no public raw callback bridge masquerading as a second API.

Internal cross-package imports use a deliberate internal subpath, not duplicated primitives or declarations. A subpath is organization, not a security boundary; runtime authority checks still apply. Source-field scalar expressions stay symbolic until compiled against the verified source snapshot; metadata construction must not reject them just because their kind is not yet known. Generated migration artifacts are deterministic repo-local data; users express ambiguous rename/backfill/drop intent declaratively, never write imperative migration callbacks.

The current application integration is specified in the SDK chapter from the actual sibling consumer, not an imagined historical explanation package. Run a real schema → generate → initialize → write → read → migrate → reopen journey through the native addon. Qualify Next.js/Alchemy deployments on Node runtimes; Edge/browser/mobile are not promised.

### 7. Subtract while proving

Delete dead compatibility surfaces, twin codecs, duplicate caches/registries, stale examples, and runtime tests that establish no useful behavior. Preserve independent oracles, counterexamples, real native boundaries, collision/rollback/crash schedules, and explicit expensive qualification. No implementation folder, new fixtures folder, campaign exhaust or wave-report hierarchy: necessary independent expected data lives beside its consuming test, permanent contracts live in the existing reference docs, and raw qualification reports live with release/CI artifacts.

No smoke-only tests. Each retained test names a plausible wrong behavior that its assertion rejects. An exact model comparison on a tiny corpus is not a smoke test. A large workload that only checks positive time or file existence is not a specification. Fewer tests and fewer lines are welcome outcomes, not targets that override correctness.

## Fixed scope and explicit exclusions

Keep: exact canonical sets; keys/containment/grouped count-weight-duration laws; closed vocabularies; F64 including exact sum/mean and dense float intervals; AST queries and nonrecursive relation composition; restricted positive linear recursion; Rust/core TS; TS-only public log; local and S3 authority; generated migrations/backup/restore; larger-than-memory operation; Apple Silicon first, Graviton and x86 Node qualified.

Do not add: a replacement storage engine, remote page-tree service, general CRDT multiwriter core, cross-tenant transactions, sharding/fleet scheduler, SQL parser, imperative migration scripts, online dual-write migration framework, general CHECK language, weighted bags, temporal occupancy engine, plugin hash registry, compression subsystem, parallel aggregation framework or universal host runtime.

LMDB remains single-writer per environment. Hosted writers may compete for one tenant HEAD; semilattice properties do not remove key/capacity conflicts, deletion order or read-dependent intent. Exact sum-state merge is associative/commutative over disjoint bindings, not idempotent. Preserve these essential distinctions.

## Decision hierarchy and review questions

Implementation begins when the owner gives the orchestrator the goal prompt; authoring this packet did not start it. The constitution and semantic contract outrank subsystem convenience. Interface contracts C1–C9 resolve cross-layer representation; subsystem target chapters preserve product behavior; refreshed findings identify current mechanisms and strict discriminators. If old wording in a retained target chapter conflicts with an explicit C contract, resolve it in favor of that C contract and update the wording before implementing. Never use precedence to silently weaken semantics. A discovered contradiction is reported and resolved in this folder before coding its consumers.

Selected defaults: existing BLAKE3 with 16-byte exact-checked local fingerprints and 32-byte commitments; exact bounded projection keys where useful; no algorithm matrix. Keep row-ID indirection and membership indexes in this pass. A later index-removal proposal requires demonstrated replacement coverage and workload evidence; it is not another mandatory swarm experiment. AEGIS is an optional one-time comparison, not a blocking research program or a second shipped algorithm.

The owner should review the public syntax, declared scope, explicit cutover/downtime, and performance qualification method. The coordinator settles cross-layer declarations before dispatch; workers implement bounded lanes without reinventing architecture. Execution uses at least twelve concurrent workers and the 21-lane graph, not eleven broad accountability essays. No claim that this is a mathematically optimal database, every bug has been found, or all seven commitments already hold is made here.
