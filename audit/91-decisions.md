# Decisions that determine what Bumbledb becomes

These are recommended defaults for the next design pass. They are not approval requests, implemented changes, or claims that every alternative is impossible. They make the philosophical ambition testable without turning the database into an unfocused collection of features.

## 1. What is the unit of the product?

**Recommendation:** the tenant capsule: one admitted theory, logical database incarnation, durable history, owned local materialization, and resource/retention policy.

Tenant is the coarse scaling and lifecycle unit. Braids are derived integrity/coordination domains within that unit, not a substitute for tenant isolation or general row sharding. A hot tenant can receive a resident writer; a cold tenant can open on demand. Placement should not change what a published receipt means.

**Deciding evidence:** actual application schema sizes, braid components, working-set distribution, cold-open cost, and writes per hot braid. See ARCH-006 and [40](40-performance.md).

## 2. What does ordinary success mean?

**Recommendation:** hosted ordinary success means a published, identifiable, recoverable receipt. Keep local/provisional acknowledgment as an explicitly different capability, if retained at all.

An ambiguous transport error is an unknown outcome, not a semantic rejection. Cancellation after publication cannot unpublish a command; timeout must preserve the ability to resolve it. Split commands must report successful prefixes even when later infrastructure fails.

**Tradeoff:** stronger ordinary success pays object-store and local durability costs. Hiding those costs by broadening “accepted” transfers the problem to every host. REP-001/004/020 and SDK-001 show why the distinction is load-bearing.

## 3. What can an ordinary read observe?

**Recommendation:** only a published snapshot, with actual vector and health/freshness provenance. Make a speculative preview a different API if applications need it.

Decide how the writer maintains candidates: private candidate engine, pinned published read snapshot, delayed application, or another explicitly isolated design. Evaluate contention/replay and native-lifetime costs. Merely returning a read-only `Db` does not stop it pointing at speculative state.

**Required property:** a rejected candidate never escaped through ordinary reads, even transiently. SDK-014 is the counterexample to beat.

## 4. Are commands blind set effects or conditional application actions?

**Recommendation:** support both explicitly, starting with the current cheap blind effect path and a narrow optional published-state precondition.

The transaction engine can guarantee all declared laws while two successful “decrement” requests collapse into one effect. For commands whose meaning depends on a read, a witness/expected-fact condition should report movement instead of silently reinterpreting the intent. Start braid-local. Cross-braid atomicity requires its own protocol and should not be implied by an aggregate receipt.

**Tradeoff:** a coarse witness may cause avoidable retries, but is easier to specify and validate than prematurely introducing arbitrary dependency tracking. ARCH-001/002 define the gap.

## 5. Who owns command identity and retry deduplication?

**Recommendation:** the application supplies a stable request identity; the SDK provides a clear receipt pattern with payload digest, outcome/reference, incarnation, and explicit retention horizon.

A set fact is idempotent; a repeated user action with a newly allocated entity ID is not. Receipt facts may initially be ordinary keyed relations, provided winner retrieval and stable replayed outcomes are clear. External effects need an outbox/idempotent dispatcher; they are not included automatically in database atomicity.

**Required property:** crash after publish/before response, then retry, produces one intended business effect and the same result identity. See ARCH-003 and OPS-003.

## 6. What makes an old writer unable to publish?

**Recommendation:** retirement authority enforced by the publication substrate/protocol, not an observed floor or elapsed local clock.

The choice among epoch namespaces, tombstones, or another atomic enforcement mechanism should be made with a tiny history model. Include a pause between the final check and PUT and an offline writer older than retention. A writer lifetime policy is only a safety mechanism if stale publication is actually prevented after expiry.

**Tradeoff:** an extra durable marker or epoch transition can cost storage/round trips. That cost must be compared with a provable publication law, not with an unsafe idealized zero-cost check. REP-001 and REP-005 are different manifestations of this boundary.

## 7. Is local state identified by its shape or by its history?

**Recommendation:** by authoritative logical database incarnation, with schema/version as a separate compatibility coordinate.

Validate identity before adopting local rows, pending commands, scratch, or a transferred session vector. Same schema and same generation are not enough. Exact recovery of the same history can preserve incarnation; rewriting/rolling back into a different logical lineage should deliberately change it. Names and prefixes may change without necessarily changing the logical database, but that requires an explicit migration mapping.

**Required property:** misconfigured cache reuse refuses or safely reseeds before returning another tenant's facts. See SDK-016, REP-011, ARCH-004.

## 8. Who owns time, retention, and deletion?

**Recommendation:** explicit durable metadata and a conservative reachable-history model. Treat deletion as a protocol action needing evidence.

Choose what the retention promise means: every vector reachable within a time horizon, named restore points, or another precise policy. Specify clock source/tolerance, lagging clients, in-flight publications, blobs, and independent backups. A checkpoint's own age alone may not prove it can be removed if it is the necessary base for a target within the window.

**Required property:** restart, clock jumps, frequent publication, and failed deletion do not silently narrow the documented restore envelope. See REP-002/003/007/008/013/019 and OPS-002.

## 9. Is zero allocation a global goal or a steady-state optimization?

**Recommendation:** a qualified steady-state property, subordinate to bounded total resources and correctness.

Copying mutable command input is necessary ownership work. A warmed prepared query can retain useful buffers, but a thousand cold tenants should not each retain their historical high-water allocation indefinitely. Expose trimming/disposal and admission accounting. Cheap median queries do not excuse an unbounded join or a native engine that never closes.

**Tradeoff:** memory-pressure reclamation can make the next query allocate. That is often the right result for a fleet host. Measure the tradeoff, preserve the useful fast lane, and avoid making its preconditions invisible. See QRY-002, SDK-007/011/013 and PERF-002.

## 10. Is text an interned symbol or arbitrary application content?

**Recommendation:** make the lifetime assumptions explicit before expanding workloads. Interned repeated vocabulary and replacement-heavy private text have different storage/erasure needs.

Keep the append-only dictionary where its economics fit. Decide whether other text needs a separate representation or a new-incarnation live-data rebuild that reclaims unreachable values. Define erasure across logs/checkpoints/backups as well as live rows. Do not promise user-level deletion through whole-tenant key destruction.

**Deciding evidence:** unique-text churn, live-to-historical dictionary ratio, privacy obligations, and prepare/cache identity across reclamation. ENG-006 supplies the concrete retention observation.

## 11. How much machinery should be shared between Rust and TypeScript?

**Recommendation:** immediately share an independent schedule corpus and public outcome/visibility contract; evaluate a pure transition kernel after urgent repairs.

One Rust byte grammar is already a strong improvement. A pure machine can make state transitions single-owner while keeping filesystem/network/timer effects natural to each host. It should reduce independent proof obligations, not force JavaScript I/O into a blocking Rust runtime or create a sprawling framework.

**Required evidence:** both drivers produce equivalent histories under the same modeled outcomes and recoveries. Decide whether kernel extraction is worth its migration cost only after the expected state machine is clear. See ARCH-005 and ASS-002.

## 12. What should “verified” and “fast” mean publicly?

**Recommendation:** name premises and regimes rather than using those adjectives as blanket assurances.

The Lean work is meaningful abstract evidence; the current bridge is empirical and the durability mechanism is outside its proof scope. Fix the closed-relation premise mapping without throwing away correct independence. Keep differential oracles independent of production implementation. Measure application paths with cold starts, mixed writes, maintenance, contention, memory, and tail latency—not only selected warm medians.

**Required evidence:** readers can trace a public guarantee to its model premise, implementation boundary, and regression or workload result. Preserve historical claims with their measured version, and preserve findings even after closure. See [50](50-assurance.md) and [90](90-evidence.md).

## A product boundary worth defending

Bumbledb does not need to become a universal analytical warehouse, a global cross-tenant transaction system, or a full SQL compatibility engine to fulfill this vision. It needs to make a small number of ambitious promises unusually well: exact set meaning within the supported language, reusable declared laws, intelligible rejected changes, fast tenant-local execution, and a history whose acknowledged facts remain trustworthy.

That boundary leaves plenty of hard work. It also gives the project a coherent answer to feature pressure: does this change strengthen those promises for actual applications, or add another mechanism the host must learn to distrust?
