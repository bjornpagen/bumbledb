# Binding interfaces — retain the fact, delete the workaround

C1–C9 are the sole authority for cross-lane representations. Product denotation is fixed by [01](01-semantic-contract.md); usage by [30](30-sdk-and-application.md). Type names below describe selected roles; the coordinator settles exact declarations during F0 and owns root exports. Workers may choose local algorithms, not alternate contracts.

A producer is contract-ready only after publishing actual declarations, constructor restrictions, error/cleanup semantics and named consumers. It is not source-integrated until the coordinator traces the adapted production consumers through that owner; execution evidence waits for final qualification. No compatibility shim or dummy success implementation is a handoff.

## C1 — Compiled theory and one scalar grammar

Producers L01 (theory/projections), L15 (TS authoring); consumers L02/L05/L07/L10/L14. The coordinator owns export hubs.

One schema-owned `Arc<CompiledTheory>` contains canonical row layouts, interned physical projection descriptors, positional inverse permutations, affected-law adjacency and narrowly checked optimization witnesses. Statement IDs reference projections; storage and the planner do not interpret raw statements a second time. Compile source and target accesses for containment/capacity, not just keys. Unfiltered shared indexes apply each law’s predicate at consumption. Candidate indexes remain multimaps so conflicts survive until judgment.

Choose exact scalar grouping keys up to 16 encoded bytes when the **whole** key fits the backend; otherwise exact-checked 16-byte BLAKE3 routing. Ordered interval tails are separate from scalar grouping width. No CPU/row-dependent persistent format or generic hash registry. Keep local row IDs and membership indexes.

The scalar runtime AST is one operator/literal grammar parameterized by leaf scope: bound query variables versus symbolic source-field names. Query variables already carry checked schema kinds. A migration `Scalar.field("units")` is deliberately **unresolved**, not a falsely typed expression and not invalid merely because no snapshot is available during authoring. Builders accept it inside arithmetic/casts. Result kind is known only when derivable from known operands; otherwise it stays unresolved. No `field<T>`, arbitrary asserted field kind, or cast that merely asserts a host type.

Use constant-work node construction with cached depth/kind summaries, not recursive re-judgment of the entire tree at every constructor. Known incompatible literal/typed operands refuse; unresolved field expressions must be bound and fully typechecked by the native compiler against the exact source snapshot, even on zero rows, before artifact commit or source freeze. Native execution only accepts compiled scalar programs. One actual shared node/operator builder replaces duplicate QueryNode/MigrationNode rosters; no general VM or JS evaluator.

## C2 — Capacity belongs to allocations and transactions

Producer L03; consumers L02/L04/L05/L06/L07/L10/L13/L14.

Separate cumulative input/work/time, resident cache retention, result backing, fresh delivery and physical scratch capacity. These conservatively bound explicitly owned capacity, not exact RSS, virtual mappings or JS heap behavior.

An allocation owner contains private payload plus inseparable reservation. Borrow slices or transfer the whole owner. Delete owning `values`, `into_values`, `into_parts` escapes that refund while bytes live. Reserve before growth; rollback reservation on allocation failure; admit copy overlap before copying. Clearing length does not release retained capacity. Prepared pools carry their capacity charge across operations. A consumer may not manufacture a new unlimited context to cover an old API.

One RAM-first exact scratch substrate offers early-stoppable fallible visits and ordered fixed-word keys. Write reservations, sequence assignment and ledger mutations commit/abort together with the LMDB transaction, including MapFull retries. Live logical bytes and conservative page/file reservation are distinct; repeated overwrites cannot become lifetime-traffic billing. Enforce the declared disk policy before growth. Exclusive scratch ownership begins before fallible setup and ends after native release.

## C3 — Generation-owned images and complete row sources

Producer L04; consumers L05/L07/L12/L13.

Select an owned `Arc<GenerationState>` with resolver storage and generation identity. Every token-bearing image, retained memo and prepared execution that interprets its words holds that same owner (directly or through its image). A token cannot outlive its resolver. Cache map entries are eviction references, **not allocation charge owners**: the image slab’s charge is inside the shared allocation and refunds only when the last real owner releases it.

Replace detached lease counters/check-then-advance with one synchronized generation acquisition/rotation protocol and strong ownership. Rotation detaches old memo entries; old readers retain exact old meanings. All still-live generations count against the same cache allowance; bounded legitimate pins may refuse new resident admission, then eligible queries use the nonresident path. Never keep all old generations forever, and never reset a resolver beneath live words.

Idle prepared memo caches use weak/versioned handles and rebuild safely; they are not permanent implicit pins. Closed-relation and numeric images are charged too. Text is inline durably; nonresident execution uses the same exact scratch resolver or canonical bounded row source, not the resident interner with a new name.

A sealed derived relation is `Resident | Scratch` with the same typed cursor/seek semantics. Positive, negative and recursive consumers all accept it; no full resident reconstruction. Small stages stay resident. Switch physical regime before u32 row-position overflow. Resolve backing/strategy outside per-scalar hot loops.

## C4 — Lawful state, unready state and installation

Producers L02 (judgment/evidence), L07 (store/read ownership); consumers L08/L10/L12/L14.

Complete judgment and incremental judgment are distinct entry points. Incremental judgment requires a lawful-parent capability established by checked create/open/previous admitted commit. An `UnreadyStore` cannot supply that premise. Its terminal `admit` invokes the complete production judge over the populated final state, **never an empty ChangeSet through incremental prepare**. A custom pass-through judge, unchecked population callback or bare Store cannot mint the lawful-parent capability. Only the built-in complete judge or a correctly premised admitted transition establishes it; trusted persisted open relies on that selected format’s admission invariant. The offline verifier also uses complete judgment.

An unready owner exposes only bounded population/index/opaque-adjunct operations and final admission; no ordinary Store/Db/snapshot accessor or disarm-to-(Store,PathBuf) escapes it. A handoff transfers an admitted cleanup owner, never a bare path whose lifetime is only a comment. This is storage substrate, not a core migration API. All legal nonempty-required theories can initialize/hydrate/restore/migrate. The same compiled indexes are built during population; readiness proves the entire final state.

`Unready → Admitted → InstallOutcome` owns the precise staging identity throughout. Install uses no-clobber publication after required commit/sync/close, distinguishes not-installed from installed-but-settlement-failed by actual publication evidence, and carries a resumable identity. Checking `dest.exists()` does not prove this attempt installed it. Cleanup may only remove its owned unpublished staging identity. Fresh adoption consumes a private capability or checks all metadata under exclusion; zero facts is insufficient.

Core also supplies an owned pinned read object: owned LMDB `OwnedSnapshot`, shared schema/closed/cache owners and provenance, with a short borrowed read frame per operation. Existing `OwnedSnapshot` is Send and !Sync; do not invent a cross-thread lifetime with unsafe Send. Worker-affine prepared state may remain !Send on its worker. Explicit operation work is passed to each frame, not stored as the snapshot’s lifetime deadline. Synchronous Rust writer scopes may remain internal; no JS-driven writer transaction is required.

Canonical diagnostics select bounded citations by logical bytes **before truncation**, independent of row IDs/insertion/physical strategy. Completed rejection has all violated statement IDs plus honest example truncation; resource failure is not rejection. Preserve exact log replay evidence comparison.

## C5 — Evidence-bearing attempts and coherent negative proof

Producer L08; consumers L10/L11/L14/L17.

Keep the repaired state-specific certainty sums. Prepare/encode/admit request bytes before actual authoritative dispatch. Transition into uncertainty at the dispatch boundary, not before encoding and not after awaiting the response. Stable command/admin recovery identity exists first. No independently settable phase/outcome pair; phase is derived.

Known receipts survive failed diagnostic decoding, local settlement, Effect interruption and finalization. Attach unhealthy/incomplete detail without downgrading evidence. Unknown stays resolvable under the original identity. A subsequent predispatch refusal says nothing about an older attempt.

A negative proof combines consumed conditional version with a **coherent retained-coverage frontier and absence lookup**. Capture/check them in one owned local snapshot after the applicable authority state is installed, or revalidate before use. A remote captured retirement floor plus a later unrelated local receipt lookup is not one proof. Retirement means expired-unprovable, not loss.

All local fact/control/receipt transitions validate identity, decision and control revision under the same writer. Receipt key discovery/pruning belongs to that coherent transaction or a revalidated bounded plan. Later control at the same decision is still later; do not compare decision alone. Source freeze/target ready/activation/cancellation remain distinct protocol transitions, not one configurable workflow engine.

## C6 — Checked roots, exact walking and bounded receiving

Producers L09 (codec/traversal), L11 (transport); consumers L08/L10/L14.

A checked recovery root has two semantic cases: checkpoint-only `base == tip` with no tip locator, or suffix `base != tip` with a checked tip DecisionRef. Comparison is the complete stamp, not sequence alone. Encode the selected wire format once; constructors and decoding enforce the same condition. Nonzero checkpoint-only roots are normal.

ObjectRef bytes are exactly 8 epoch + 1 kind + 32 digest + 8 length = 49. An option adds **one** tag: absent 1, present 50. Parent-bearing frame sizing derives from the codec owner. A parent locator, when present, binds kind/digest to its parent stamp. Walk stops at captured base before fetching another parent; a missing required interior locator is corruption. A budget of n admits at most n fetches, including n=1. Preserve the initial tip locator while walking backward.

One verified walker/visitor replaces epoch-probing and duplicate chain walkers in recovery, checkpoint validation, GC, backup and witness verification. It streams bounded records rather than returning the entire tail. Backup relocation consumes the manifest’s ordered relocated refs and validates unchanged historical decision commitments; it does not rewrite parent bytes or follow source-location refs.

All receiving APIs carry WorkContext/deadline, byte cap and output ownership; receive and hash chunks incrementally. HEAD is capped during receive too. A length header/stat check is not a receiving bound. Local filesystem and S3 retain truthful missing/conflict/indeterminate/denied observations; only L08 interprets publication proof. One shared transport runtime/credential provider; no per-tenant executor. Durable listing progress is the last fully processed canonical key, not an opaque provider token.

Hydration, Map transforms, restore, backup, tail replay and receipt maintenance stay bounded through core, Rust log and addon callers. A streaming producer with a whole-array consumer fails C6.

## C7 — Fixed workers own resources as data

Producer L12; consumers L13/L14/L16/L17; core read producer L07.

Replace session-long stack reactors with one event loop per configured worker and a worker-local resource table. A snapshot is an owned pinned read object plus prepared state in that table. Many idle snapshots may share one worker within resource limits. One bounded job temporarily borrows its resource, executes, publishes its outcome, then returns to the worker scheduler. No worker waits for a job or readiness handshake scheduled onto the same pool. Opening captures/registers the snapshot and completes immediately; closing reclaims its table entry after in-flight jobs drain.

Keep affinity for genuinely !Send prepared state by routing capabilities to their owning worker. Do not park a worker just to maintain a read transaction lifetime. Delete unused JS-driven WriterSession/HostWrite/open-write/insert/delete/finish ABI and declarations; selected core apply and log submit execute the complete native transaction in one bounded job. Audit actual consumers before deletion; migrate any real one to the sealed command/apply path, not a shim.

The runtime’s short-held route/admission metadata may be shared; heavy payloads/charges belong to the worker table. A capability binds runtime identity, worker route, kind, ID and generation. No payload work, decoding, I/O, destruction or callback under a runtime-global mutex. IDs do not require permanent revoked tombstones: absence or generation mismatch refuses. No strong ownership charge on a JS token whose reachability controls native close.

Admission reserves resource/queue capacity before table insertion; failure drops/refunds under a cleanup owner. Resource states are live/busy/closing with explicit in-flight ownership; close revokes admission and schedules exactly one drain. Control cleanup is an already-owned obligation, not ordinary rejectable work: use per-owner close state plus worker wakeup and coalesced drain, bounded by admitted owners, so QueueFull cannot strand it. Service it at normal job boundaries; cancellation bounds the active job. No promise of instant preemption of uncooperative foreign code.

Workers wake for all job/control sources. One-worker configuration must open/read/close; more idle handles than workers must not exhaust threads. With multiple workers, a slow owner must not globally serialize unrelated resource use. Repeated close joins one operation; parent snapshot survives child-session close. Closed means resources and directory fences actually released, not zeroed counters. Abandoned output/callback failures retain a drain owner.

## C8 — Transactional delivery and Effect-only authoring

Producers L05/L13/L15/L16/L17, with L14 native migration/repository ownership.

Delivery’s atomicity boundary is one public pull, not one internal page. The core cursor supports a short-lived delivery ticket/checkpoint: inspect/size and copy bounded rows under admitted overlap, then commit the cursor position once the complete native output owner is registered. Predelivery resource refusal aborts the ticket and returns no rows without advancement. A next row that does not fit an otherwise valid nonempty page ends that page successfully; it is not an error that discards prior rows. Oversized first row refuses unchanged. Terminal scratch/storage failure closes the cursor explicitly. Never publish partial evaluation as a complete result.

Queued output retains its conversion charge until JS transfer or native cleanup. Collection uses the same admitted owner contract. Independent maxBytes/pageBytes intersect work.resultBytes; fresh delivery work does not inherit expired execution work. No second public raw cursor API; TS exposes a scoped one-shot Stream.

Pure schemas/query/scalar metadata is synchronous and addon-free. Operations are lazy pinned Effect 4.0.0-rc.112; read installed documentation before lifecycle edits. Scope every intermediate acquisition before the next interruptible step; preserve Cause and stable publication references. No public Promise/sync/disposal twin, SDK runPromise, superbuilders/errors or per-row Effect/proxy/getter. Ordinary stable-shape rows and bounded batches keep V8 overhead at the boundary. Core primitives are literally imported by log via the intentional exact-version internal subpath.

Generated migrations require the base and all intermediate verified snapshots, ordered plans and fully compiled mappings before writing a new manifest or freezing data. Resolve symbolic fields under C1; no handwritten imperative transforms or optional validation on empty input.

Repository generation uses the existing native kernel-held directory exclusion primitive, exposed only through the internal codec/runtime seam, over a persistent lock-file inode. Never unlink/replace that lock file as stale recovery; process death releases the OS lock. Same-process duplicate generation must also refuse/serialize. Close retains lock ownership until I/O settles. Delete PID liveness, stale-file deletion and token-guess logic. No shell flock dependency, lease timeout, package-level lock framework or new runtime.

Read through one open file descriptor in bounded chunks with fatal UTF-8 and aggregate limits; no stat→readFile race. Publish immutable uniquely owned plan/snapshot artifacts no-clobber, then atomically durably replace the sole authoritative manifest. Derived index/contract outputs are repairable. Ownership begins before any interruptible side effect and persists until canceled I/O is joined; do not release a lock while an uninterruptible promise keeps mutating. Matching recorded content is verified, never overwritten.

## C9 — Qualification is of the final input, not task labels

Producer L21; L19/L20/all lanes contribute, coordinator accepts.

Preserve 68 audit IDs, 220 child behaviors, 78 prior-review IDs, D01–D29 and all required target/backend cells. Implementation acceptance is producer + actual consumers + deletion + discriminators + composed review. No mandatory seam becomes “future tightening.”

No tests/builds/typechecks/package hooks/benchmarks during fanout. Author tests now. After source integration and writer freeze, transfer permanent contracts/checklist to maintained docs, retire only the packet/root prompt, and capture the post-retirement candidate. Final checks include repairs and affected reruns until qualified.

Use the existing small evidence mechanism: recomputed candidate membership/path/kind/mode/content/link targets, specs, artifacts and report hashes, actual platform/backend/cases/skips. Reject arbitrary digests/subsets, stale or duplicate cells and nonempty garbage evidence. No self-hash circularity: non-input result records are declared exclusions, permanent contracts are inputs.

Missing real S3/IAM/Graviton or other advertised evidence remains unqualified. Finish independent work, report exact prerequisites, and stop invented waves. No package publication or live-data mutation is authorized by this packet. The one final integrated commit/push follows explicit execution authorization and verified candidate equality.
