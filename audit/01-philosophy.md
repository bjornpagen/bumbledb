# What Bumbledb is trying to become

Audit date: 2026-09-04. This is an architectural assessment and proposal, not a claim that the proposed capabilities already exist.

## The thesis worth protecting

Bumbledb should be a **tenant-sized application database whose declared laws are executable knowledge**. The same law should help an application express its model, reject invalid changes, explain rejection, optimize reads, determine safe coordination boundaries, and restore an intelligible history.

That is a much stronger proposition than “SQLite with a nicer query API,” and much more precise than “the fastest database.” It also gives the project a way to decide what not to build.

The valuable core is the direction of derivation:

1. Define the facts and the laws.
2. Admit only a language whose execution and enforcement obligations are understood.
3. Derive access paths, checks, simplifications, and partition boundaries from those declarations.
4. Make the remaining operational obligations explicit.
5. Measure the complete application path, including the costs the elegant model does not erase.

The current tree contains real implementations of this idea: set-valued relations; final-state judgment; functionality, containment, and capacity; typed schema/query descriptions; virtual closed relations; prepared execution; Free Join; LMDB snapshots; schema-derived braids; a shared Rust protocol grammar; and a large independent-testing estate. These are assets to build on, not reasons for a rewrite.

## Historical orientation, with the staleness boundary explicit

The July 10 Nessie context, *Bumbledb: The Database Is a Theory*, states the original thesis particularly well: “state the sentences, derive the mechanisms — and measure everything the derivation claims.” It also records a deliberately small, read-heavy, single-process design point and explicitly excludes distribution. The later replication research changes that scope substantially.

Historical source: [https://nessielabs.com/n/f9c6a22d-ddf8-4228-aee9-669d3f564d05](https://nessielabs.com/n/f9c6a22d-ddf8-4228-aee9-669d3f564d05).

The beginning and recent tail of the later design conversation were also consulted for chronology, not exhaustively read: [https://nessielabs.com/n/2a057eaa-af11-4141-9189-4723bee748a5](https://nessielabs.com/n/2a057eaa-af11-4141-9189-4723bee748a5). Historical summaries and assistant proposals are not treated as current specifications or as proof of owner decisions.

Current code takes precedence for this audit. Important changes since that context include recursion, capacity, admitted heap instances, append-aware image reuse, the braided log, and the one-reader consolidation. Conversely, the elaborate footprint/escrow optimization described in `docs/research/replication-prior-art/THESIS.md` is not the current writer algorithm. Current non-identical slot losses discard/reopen and rejudge recorded operations (`crates/bumbledb-log/src/writer/loss.rs:47`, `writer/discipline.rs:56`). There is no current L6–L8 footprint implementation to credit with performance or safety.

## The five strongest ideas

### 1. Facts have set identity; occurrences need explicit identity

A duplicate fact is not another occurrence. This removes an enormous amount of implicit bag behavior and makes replay of the same concrete operations useful.

The application must still distinguish “Alice likes this article” from “Alice clicked this article twice.” The first is naturally a set fact. The second needs an occurrence identity or an explicitly aggregated quantity. Set semantics does not make money transfers, email sends, purchases, or user commands idempotent by itself.

**Product consequence:** teach identity modeling as a first-class part of the database, not an appendix. Distinguish fact identity, entity identity, request identity, writer identity, and database incarnation. Several of the most serious log findings come from collapsing two of these coordinates.

### 2. Integrity belongs to the final observable state

Checking the transaction's final state makes cluster creation, replacement, discriminated unions, and temporal models far more natural. It lets the planner rely on facts the checker guarantees.

But “final” must refer to the right visibility boundary. A candidate applied to a writer's local LMDB is not yet a published transaction. Exposing that candidate to unrelated requests breaks the useful meaning of final-state judgment even when the candidate satisfies every declared law. See the SDK dirty-read finding in `30-sdk-hosting.md`.

**Product consequence:** the law must span admission, visibility, durability, and recovery. A correct local judgment cannot compensate for the wrong publication boundary.

### 3. Constraints are reusable knowledge

The engine can spend an accepted key or containment on query simplification, not just validation. Closed relations let the schema carry constants that are folded earlier. A dependency graph can identify independent integrity domains.

This is the strongest defensible differentiation. It is more interesting than adding twenty unrelated features. Make the reuse inspectable: which law justified this plan, which statements couple these relations, which projection creates a hot contention domain?

**Product consequence:** a schema explanation should expose a coordination and cost model alongside its logical model. “This foreign key joins these two write domains” is information an application designer needs before production.

### 4. A bounded language can buy predictable behavior

Rejecting shapes the implementation cannot support faithfully is legitimate. There is no obligation to recreate all of SQL, its type system, or its operational conventions.

However, undecidability of general dependency implication does not imply that richer finite-instance validation is impossible, or that sound incomplete reasoning is impossible. The present restrictions are an engineering/product choice about comprehensibility, enforcement cost, and supported inference. State that choice accurately.

Similarly, Allen masks are a complete coordinate system for the qualitative relation between the supported nonempty intervals. They are not a complete temporal query language: distances, calendars, time zones, recurrence, and business-day arithmetic are separate questions.

**Product consequence:** every refusal should say what it protects, the supported modeling alternative, and the workload evidence that would reopen it. “Forbidden by philosophy” is not enough once a real customer workflow requires it.

### 5. The log is durable truth; local state is a materialization

This is the right direction for cheap per-tenant hosting. It allows workers to be replaceable and makes embedded reads the common path.

The word “disposable” needs a precise qualification: a replica is disposable only when its acknowledged obligations are durably recoverable elsewhere. A local-pending acknowledgment, a sole copy of a pending batch, or an unclosed native handle is not disposable merely because it lives under `/tmp`.

**Product consequence:** recovery and deletion are part of the protocol, not maintenance around it. Checkpoints, leases, command receipts, retention, and database incarnations must be represented with the same care as rows.

## Where the philosophy currently overreaches

### “Make illegal states unrepresentable” is local, not transitive

A private Rust field can eliminate an invalid in-memory construction path. It does not eliminate a stale process, a filesystem rename after lease expiry, a JavaScript alias, or an ambiguous HTTP response. A type named `Fenced` is not evidence that the storage substrate atomically enforced its fence.

Every representation claim needs a boundary:

| Claim | What establishes it | What it does not establish |
| --- | --- | --- |
| A value passed a parser | Parser semantics and owned result | External bytes remain unchanged afterward |
| A candidate satisfies schema laws | Judgment over the exact candidate | It is durably published or isolated from readers |
| A log object exists | Conditional object publication and identity check | It belongs above the current retention floor |
| A lease has a newer token | Acquisition protocol | An older holder cannot still mutate the resource |
| Two braids are independent | Declared-law locality | Arbitrary application read dependencies are independent |
| A checkpoint has a larger sum | Scalar arithmetic | It contains every component of the previous checkpoint |

The recurring lesson is not “types failed.” It is “the proof was about a smaller object than the claim.”

### Removing vocabulary is not the same as removing a problem

Calling migration “ETL” does not remove cutover, validation, rollback, or client coexistence. Calling coordination a slot does not remove contention. Calling a backup a checkpoint does not establish a restore window. Calling an error a refusal does not tell a caller whether it may retry safely.

Keep the compact internal vocabulary, but translate it into ordinary application responsibilities. A database earns simplicity when the host needs less bespoke machinery, not when the host receives responsibilities under new names.

### One owner does not mean one explanation

The one-reader consolidation is a strong maintenance move. One grammar should have one production implementation where feasible. That does not imply there should be only one explanation of its behavior, or that independent reference models are unwanted duplication.

Code, model, and operational guide serve different readers. Repeated implementation logic is dangerous; independently expressed expected behavior is a testing asset. An architecture paragraph that explains why a state transition is safe is not automatically an unwanted second specification.

Some source comments are visibly truncated fragments after prior cleanup, and current public documentation still describes retired APIs/protocols. Optimizing line count or banning words while allowing these contradictions weakens the project's original evidence-first philosophy.

### Local optimality can create host-level complexity

Zero allocation after warm-up, synchronous fixed kernels, no JIT, compact byte grammars, and no LIST on the hot path are useful constraints. None is automatically the best end-to-end product rule.

- Unlimited retained high-water buffers can conflict with thousands of tenants.
- A synchronous query can conflict with event-loop deadlines.
- A no-LIST hot path need not forbid an authenticated administrative inventory or disaster-recovery tool.
- A single writer lock can simplify correctness while creating tenant tail latency.
- A schema fingerprint that changes for every structural revision simplifies identity while requiring a migration control plane.

Preserve the original aim—less total complexity—and evaluate these constraints against that aim, not against themselves.

## The logical conclusion for per-tenant applications

The natural unit is a **tenant capsule**:

- One explicit database incarnation and schema version.
- A durable history namespace and a verifiable checkpoint root.
- A local materialization with deterministic ownership and release.
- A published-read boundary, optional session minimum, and bounded work.
- A command-receipt namespace for retryable application actions.
- An operational policy for backup, retention, erasure, migration, and recovery.

This is not a recommendation for a giant server or a broad SQL layer. It is a small set of missing contracts around an unusually expressive core. The capsule may live entirely on a local machine or be managed by a host over S3. The same logical API should carry different, explicit durability capabilities rather than pretending the failure domains are identical.

## Proposed design constitution

1. **A successful published receipt is a permanent obligation.** A later failure may make it unavailable; it must not silently become a different history.
2. **Only published state is ordinary readable state.** Speculation, if offered, has a visibly different capability.
3. **A retry answers a named command.** Fact idempotence is useful but not substituted for command identity.
4. **A certificate names its premises.** Schema validity, session order, lease authority, and resource budget are distinct certificates.
5. **Cleanup proves non-necessity before deletion.** Reachability, retention, in-flight readers, and epochs are protocol inputs.
6. **Every owned resource has a deterministic lifetime.** GC may backstop it; GC timing is not the budget mechanism.
7. **Performance claims describe the complete regime.** Hardware, workload, warmth, write mix, publication semantics, tail latency, and memory are part of the number.
8. **A gate earns its place by finding a failure class.** Name checks and goldens complement—not replace—independent models and interleaving tests.
9. **Application semantics outrank internal elegance.** When a problem moves into every host, the engine has not necessarily simplified it.
10. **The evidence outlives the campaign.** Keep findings, counterexamples, decisions, and regression references even after a fix lands.

## What not to do next

Do not restart the engine. Do not begin with a new consensus system, universal transaction language, JIT, custom B-tree, or position-level braid algorithm. Do not resurrect the old footprint fast path before the simpler current protocol is demonstrably safe.

First establish a reliable semantic and durability envelope. Then choose a few actual application workloads and make this system excellent at them. The strongest version of Bumbledb is not the one with the loudest theorem; it is the one where the theorem's premises survive an ordinary bad Tuesday in production.
