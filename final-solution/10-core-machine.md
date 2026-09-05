# The core machine: one semantic plan, two physical working regimes

Status: selected target reconciled with the current source failures, 2026-09-05. C1–C4/C7/C8 select interfaces; chapters 60–65 assign 21 executable lanes. No behavior or performance is qualified by this document.

## 1. The decision

Keep the bet small: canonical sets, compiled schema facts, atomic LMDB transactions, Free Join. Do not replace LMDB, add a server engine, invent a storage plugin system, turn the core into a migration framework, or promise multi-writer conflict resolution from set union alone.

The existing Free Join implementation is real and worth preserving. The current work contains real improvements: schema-derived determinant indexes, key-local judgment, relation-specific change versions, exact floating aggregates, spillable distinct/group state, and executor work polling. The convergence pass must finish their composition rather than discard them and begin a different database.

The architectural defect is repeated interpretation. A key is independently reinterpreted by schema validation, the determinant table, the judge, keyed lookup and the query planner. A retained allocation is independently counted by the object, an operation ledger, and a descriptive cache statistic. The shared canonical field-walk repair must replace divergent row validity rules on every consumer. A derived set spills and is then required to become a complete in-memory image again. Remove those discrepancies at the representation boundary.

The final machine has four internal ownership objects, not a hierarchy of services:

1. A sealed schema with a compiled projection/statement table.
2. One LMDB store owner, with ordinary snapshots and one serialized writer.
3. One bounded resident cache owned by that database, plus execution-owned temporary tables.
4. A prepared query that owns its program and bounded reusable scratch, and produces a sealed result.

These names describe responsibilities; they do not authorize a new public framework.

## 2. Compile schema facts once

### The compiled projection

Extend/rehome the existing `DeterminantTable`; do not add a second projection registry beside it. Each distinct useful projection of one relation has one descriptor containing:

- Its relation and projected scalar field positions in a canonical physical order.
- An optional interval field used as an ordered tail within the scalar group.
- The compiled row-reading/encoding instructions and inverse caller-order permutation.
- Its physical key encoding: exact fixed-width bytes or a 128-bit candidate fingerprint.
- The consumers and guaranteed properties: full-row identity, scalar uniqueness, interval disjointness, containment source/target, capacity group, or point-probe eligibility.

Statements refer to projection IDs. Projection IDs are deterministic under the sealed schema and physical format; incidental hash-map iteration must not assign persistent IDs. Identical physical projections share an index even when statement-side selections differ. Store the unfiltered projection and apply each statement's selected-side predicate to the candidate row; do not materialize one selected index for every predicate without measured justification.

Cross-relation containment is positional at the semantic layer. Sharing/reordering physical keys must preserve that mapping with a compiled permutation; sorting field IDs independently must never silently change which source column equals which target column.

Keep compiled uniqueness witnesses small and unforgeable inside the planner. Their license is specific: covered determinant values determine at most one stored fact. It does not imply existence, distinct projected output, or distinct derivations across written union arms. A full stored row is an implicit key under set semantics. An interval point probe does not bind the interval's complete value.

### Which indexes exist

Compile only indexes required by a declared integrity law or a supported public keyed access. Do not eagerly index every field combination.

| Consumer | Required access |
| --- | --- |
| Exact fact membership | One exact full-row comparison after a sufficiently selective candidate lookup |
| Scalar key | Exact determinant group |
| Pointwise key | Scalar group, ordered by interval start/end |
| Tuple containment | Target lookup and source reverse-group lookup for removed targets |
| Pointwise coverage | Ordered source/target group walks |
| Capacity | Target lookup and source group walk |

The missing source-side projections are why current containment and capacity re-read whole relations. Their indexes are not speculative query acceleration; they implement the declared cost of accepted schema laws.

Maintain these indexes in the same LMDB transaction as row membership and relation versions. A physical multimap must be capable of representing conflicting tentative facts until final-state admission judges them. Do not let a unique B-tree overwrite or reject one candidate before the judge sees both.

### Exact keys versus fingerprints

For the 1.0 default, use an exact order-preserving scalar grouping key when its complete encoded width is at most 16 bytes **and** the complete physical key fits LMDB. Otherwise use a fixed 16-byte BLAKE3 fingerprint and compare the complete canonical projection before declaring equality. A wider exact encoding is permitted only for an explicitly selected ordered-access requirement with a measured net benefit; fitting under 511 bytes alone does not make a 400-byte key a good idea. Choose once per compiled projection, not per row or CPU.

For exact keys, the schema already supplies types and widths: do not repeat per-field type tags, blob lengths, or padding in the index. Encode booleans in one byte, integers and canonical F64 order keys in eight bytes, `Id128` in sixteen, fixed bytes at their declared width, and interval endpoints as the corresponding order words. Unbounded text uses the fingerprint arm. Large fixed projections can also use that arm.

The exact-key safety calculation includes **all** physical prefix, projection ID, interval-tail and row-surrogate bytes. The 16-byte scalar-key crossover separately controls footprint; it is not the backend safety limit. Ordered interval tails exist only where the compiled consumer requires them, regardless of whether the scalar group uses direct bytes or a fingerprint. Query the environment's key limit or use a proven supported-platform bound in one place; do not scatter 400/496/511 as unrelated axioms. Schema validation must not retain the old 496-byte rejection merely because an earlier backend put the whole determinant into an LMDB key.

The interval-group case needs special care. A hashed scalar group may share a fingerprint with a different group. Seek its bucket, exact-check the scalar projection, then apply endpoint ordering to the matching group. If a collision prevents a single naturally ordered exact group walk, filter into the same bounded transient ordered table. Correctness is independent of collision frequency; the ordinary no-collision path remains cheap.

Two durable hash roles remain sufficient:

- `Fingerprint([u8; 16])`: candidate routing, never equality.
- `Digest([u8; 32])`: authoritative content commitment.

Internal 64-bit join hashes remain ordinary exact-checked table routing, not a third durable identity system. `Id128` remains an application identifier. BLAKE3 output truncation saves index bytes, not half the hashing CPU. No production AEGIS switch, configurable hash family, or CPU-dependent persisted bytes is proposed here.

For independent uniform 128-bit fingerprints the birthday approximation is `n(n-1)/2^129`; at one billion inputs this is about `1.47e-21`. Exact comparisons make an accidental collision a performance event rather than incorrect equality. A 256-bit commitment retains approximately 128-bit generic collision resistance. Those are different requirements, not gratuitous casework.

### Membership-index redundancy

Keep row-surrogate storage and the full-fact membership index for this release pass. Do not make index removal another mandatory experiment. A future independently justified simplification may reuse a guaranteed selective exact scalar key for membership, but would have to preserve conflicting candidate rows and full-row comparison.

Do not eliminate membership blindly for a broad pointwise group: replacing a single membership lookup with a long interval walk can be a severe regression. Do not put every row directly under a content hash without comparing the larger references then needed in secondary indexes. Adopt an index-removal rule only when the resulting point lookup, mutation cost and total physical bytes improve on the accepted application workload. If no such rule is convincingly simpler, retain the membership index and record its measured cost.

## 3. Final-state judgment with delta-local work

### The invariant

Every admitted state satisfies the entire sealed schema. A candidate is the parent's canonical set with the normalized delta applied. Judge that final state, not insertion order. Application rows, every physical index, host records, attachment, generation and touched-relation versions commit or abort together.

Incremental judgment is sound only under the lawful-parent premise. Normal create/admit/commit maintains it; the offline verifier evaluates the complete state and cannot use delta-local skipping. Internal construction/adoption APIs must make that premise explicit rather than accepting arbitrary bytes under an innocent-looking store handle. The new UnreadyStore::admit currently calls incremental prepare with an empty delta and thereby skips all laws; replace it with the complete judgment entry, and remove the unready Store accessor.

The reference evaluator remains small, complete, and independent of production access planning. Share scalar/interval denotation and canonical encoding, not the production projection access plan itself. A test that compares an index wrapper with the same underlying judge twice does not independently establish the law.

### Compile the affected schedule

Relations already have adjacency lists to their key, outgoing containment, and capacity statements. Use that compiled schedule instead of walking every schema statement on every commit. Deduplicate affected statement IDs and judge in canonical statement order.

For each affected statement derive exact group keys from the delta and use the compiled projection:

- **Scalar key:** only added determinant groups can introduce a conflict. Visit their final membership and compare exact row identities.
- **Pointwise key:** only added scalar groups can introduce overlap. Use the ordered interval projection and the same half-open overlap law. A full group walk is an acceptable initial bound; two-neighbor checks are optional only with an explicit soundness argument and identical diagnostic policy.
- **Tuple containment:** test selected source additions; for selected target removals, first determine whether a satisfying final target remains and then visit affected final source groups. A remove-and-readd with a changed target predicate must not be treated as unconditional re-establishment.
- **Coverage:** selected source additions and target removals define affected scalar groups/windows. Walk matching ordered target spans, merging overlap/adjacency, and inspect only demanded final source spans. A whole touched-group walk is the first implementation; no interval-tree framework is required.
- **Capacity:** source additions/removals and selected target additions/replacements define groups. Resolve the target's own bound, walk selected final source rows, and fold exact nonnegative measures. Deletion can violate a floor; addition can violate a ceiling. A removed target no longer opens a window. Count/weight/duration remain one measured-group law.

For an accepted indexed schema, target work is `O(delta normalization + affected projection seeks + rows in affected groups)`, with index construction/writes paid explicitly. An empty grouping can legitimately mean a whole relation where the language admits it. Long groups genuinely cost long walks. Do not promise constant work for every law, and do not hide an `O(all rows)` pass behind the word “streaming.”

### Bounded traversal and evidence

Replace the `Vec<RowId>` and `Vec<(RowId, Box<[Value]>)>` competitor interfaces with a cursor/visitor borrowing the candidate transaction. Exact-check one borrowed/charged decoded row at a time. Pointwise sort/spans must use the physical ordered projection or bounded scratch; never materialize a complete group before checking its budget.

Rejection names the complete violated-statement set or returns a resource failure. Examples are bounded, honestly truncated evidence—not the complete offender set. Select examples and capacity witness groups in a stable **logical** order, independent of local row IDs, index strategy, spill boundary, cache state and checkpoint re-encoding. A bounded ordered top-k over canonical fact bytes suffices for citations; do not sort whole relations simply to choose four examples.

Canonical evidence must survive replay after checkpoint/restore. Existing row-iteration-dependent evidence is not automatically a portable commitment. Changing evidence selection is a 1.0 format/semantic decision to freeze across core, log, native and model simultaneously; it must not be “fixed” independently by different swarm workers.

The full judge should return source failures, work failures and scratch failures explicitly. Remove `TypeId`/`Any` error-type inspection used to decide whether a generic judge may spill. The scratch capability belongs in the execution context or one explicit argument, not in a special case on an error type.

## 4. Memory is owned, not retrospectively reported

### Three retention lifetimes

Separate operation work/deadline counters from ownership of retained memory:

| Owner | Lifetime and charge |
| --- | --- |
| Execution scratch | From allocation until release or transfer; owned by the running operation |
| Database resident cache | Across operations; bounded by the database cache allowance |
| Completed result/cursor | Until disposal; owned result allowance plus a fresh delivery-operation work policy |

Do not bind an append-only cache to whichever operation happened to mint a text token. Do not refund a reservation while a prepared COLT pool remains allocated. Do not charge an entire existing cached image anew to each reader merely because it is borrowed. Strong ownership pays retention once; an active operation pays for its own exploration, decoding and new allocations.

Use a small charged owner at allocation boundaries (`OwnedBytes`, decoded row owner, slab owner, table owner or equivalent existing types). It must keep allocation and charge inseparable through ordinary moves. `DecodedRow { pub values, private reservation }` plus instructions to call `into_parts()` is not an ownership proof. Read values by slice; transfer the whole owner. No generic allocator ecosystem is required.

Reserve before capacity growth, including the actual vector/table capacity envelope, not just element length after insertion. Fallible growth returns the one resource channel. Estimates are acceptable only if conservative and documented; an estimate smaller than the allocation is not a bound. Page cache and RSS remain OS/host concerns, explicitly distinct from logical native allocation charges.

### The transient table

Keep one execution-owned RAM-to-temporary-LMDB primitive and make its contracts honest:

- RAM updates charge the **net retained size/capacity**, not the entire entry again on every overwrite.
- Reserve scratch growth before committing the corresponding disposable LMDB write; rollback reservations on aborted/retried operations.
- A spill transfers ownership accounting without charging the same destination payload twice as permanent data plus “overlap.” Charge actual overlapping RAM/copy buffers separately.
- Bound iteration, copies, hashing and decoding by byte/work quanta as well as rows.
- Give ordered consumers a compile-time/constructor proof of an inline ordered key shape. Hash-bucket iteration is exact membership but **not logical key order**. Do not describe one as the other.
- Use one execution scratch environment with a small namespaced table roster when several operators spill; do not create an environment and lockfile per seen-set and per insertion-order log unless measurements justify that split.
- Batch disposable writes in bounded transactions. The current individual transaction per scratch row is avoidable overhead; durability is deliberately not the contract of scratch.
- Create the scratch directory exclusively; install cleanup immediately so failed environment setup cannot strand it. Its loss loses only the running query, never authoritative data.

Support known fixed-word ordered shapes and arbitrary exact byte-key maps through the same substrate; do not introduce an external-sort framework to avoid making the key contract explicit.

## 5. Resident execution and out-of-core continuation

### Keep the Free Join machine

Preserve binary-to-Free-Join lowering, factoring, cyclic variable-level splitting, lazy COLT construction, smaller eligible cover selection, batched probes and pipelined output. These are real mechanisms, many already in the Free Join paper. They are not a claim of beating that paper in general.

Exploit set/application information where it eliminates work:

- A projection whose remaining suffix cannot alter the answer stops after a witness.
- A proven full-binding distinctness witness removes the binding seen-set.
- Eligible aggregates fold leaves directly without materializing an intermediate binding table.
- Scalar key facts supply at-most-one probes and join cardinality information.
- Unchanged relations reuse images/tries across unrelated writes.

Each optimization carries its precondition and a differential result test plus a deterministic work assertion. Multiple union derivations, projection collapse and point-membership must remain counterexamples to overbroad uniqueness. Set semantics does not eliminate a genuinely large distinct answer.

### Finish cancellation at the actual work sites

Current top-level “explored batch” polling does not bound the time inside forcing a million-entry COLT node, filtering an image, building overlap state, hashing a giant string, finalizing a group, or copying a result. Put polling in those loops at existing batch boundaries. Flush the final partial quantum and checkpoint completion; operations smaller than the polling quantum must not bypass their finite work allowance entirely.

Prepare/reset/select/force must be charged before growth too. The executor ledger must not begin only after view application and selection already performed the expensive work. Keep the ordinary hot loop a cheap decrement/counter branch with a cold refill/check path; do not put an atomic operation or dynamic dispatch on each tuple merely to obtain accounting.

### Cache identity and eviction

Keep the new relation versions. Advance them only on actual row-set changes, in the same transaction as the rows. Receipt-only writes, duplicate adds and absent removes do not invalidate unrelated relation images. The full database generation remains result/history identity, not the universal cache key.

Move shared relation images to a **database-owned bounded cache** rather than one new cache per prepared query. Prepared programs remain separate; they can share a schema/store/relationship-version image while retaining their own selection/trie program state. This matters for an application with dozens of prepared queries over the same student's data.

Retained cache allocations own cache charges inside the shared slab, not in a cache-map entry beside an Arc. Active executions pin strong generation/image handles. Prepared idle memos identify their cache generation/version and do not indefinitely prevent pressure eviction merely because a query once ran. A stale/evicted memo rebuilds safely. Old snapshots keep the correct old image while live; a new version must never mutate an image observed by an old snapshot.

Text tokens are scoped to a live owned cache generation. Generation state owns resolver storage; images retain that generation without creating an ownership cycle. Synchronized rotation detaches cache references and invalidates idle weak memos; it does not reset a resolver still owned by old readers. Never recycle a token while an image or result still interprets it. This is simpler than arbitrary individual-token reclamation. Avoid storing two owned copies of every string when a single shared byte owner plus lookup metadata suffices.

### A complete bounded fallback

Keep one simple disk evaluator as the semantic fallback, but make it actually complete under bounded memory:

1. Check resident eligibility before allocating: estimated charged capacity, source row-position width, schema/plan representation limits and available cache allowance.
2. Use compiled indexes for bound exact-key/group probes in the fallback; otherwise scan the corresponding pinned relation. It remains a nested-loop fallback, not a second universal optimizer.
3. Use bounded exact text symbol storage backed by the execution scratch table (`exact text -> token`, `token -> bytes`) when the working text set does not fit RAM. A bounded read-through cache is optional. The persistent database still stores inline canonical text; no persistent dictionary returns.
4. Define one sealed derived-table source with resident and scratch backing. Interiors, recursion delta/accumulator and consumers read that source. Spilling a producer must not require refilling its complete output into a `RelationImage` before consumption.
5. A resident-growth refusal can restart once on the same pinned snapshot after releasing attempt-owned state. Retain stable semantic inputs and do not retain a failed attempt's entire text working set against the retry. All other failures propagate; no infinite retry planner.

Retain compact `u32` resident positions for hot paths, but prove eligibility before entering them. More than `u32::MAX` rows or pool positions routes to bounded execution, not a debug assertion, truncation or panic. LMDB's larger-than-memory support must not conceal another process-level 32-bit ceiling.

### Derived stages and recursion

Keep the finite derived-stage DAG, including aggregate/computed producer stages, and the existing safe linear-recursion restriction. Do not replace them with unrestricted Datalog or SQL CTE machinery.

A stage seals its exact set and surfaces its producer errors before consumption, even if a later filter would discard everything. Its resident/scratch backing is an implementation choice. Count actual distinct emitted stage tuples for any explicit tuple allowance; an upper-bound hint that double-counts a group spilled twice must not become a semantic cardinality fact.

Recursive accumulation is set union over a finite domain; delta-only rounds avoid repeated derivations. Store seen, delta and accumulated tables in the same bounded substrate. Work/round/tuple refusals are explicit resource outcomes, not partial answers or universal dataset-size axioms. Revisit the current `10_000_000` tuple and `65_536` round defaults as policy, not inherited proofs that all app data fits.

### Complete results and delivery

One result builder resolves and appends values into bounded RAM pages, crossing to scratch during construction. Charge actual byte capacity before append, including large strings; a row-count quantum alone cannot bound a single giant row. The builder publishes `CompleteResult` only after successful query execution, aggregate finalization and backing completion.

`collect` and `next_page` are separate consuming/copying work: require an operation context and output byte allowance, not merely a row cap. The sealed result's retained charge does not pay for an additional full collection or native-to-JS copy. A predelivery admission failure leaves the cursor unchanged for a permitted retry; a terminal backing/transport failure explicitly closes it. No failed call silently skips a page. Native batching needs one delivery ticket across every internal row: reserve/copy/register the complete native output, then commit cursor position. A nonfitting next row ends an already nonempty page; it must not throw away consumed prior rows. Abandonment disposes the backing once.

Do not retain a compatibility path that first builds the complete answer in RAM and only then decides it should have spilled. Even direct keyed results share the same builder; a pointwise key can yield a large row, and ordinary full-result copying is not inherently bounded.

## 6. Numerical and semantic boundaries

Keep `F64`'s canonical identity: one NaN, one zero; host constructors normalize, strict wire/storage parsing rejects noncanonical bits. Payload bits and order keys are distinct encodings. Preserve exact sum/mean using the current wide accumulator and one final rounding; the accumulator merge is associative for partitioning but **not idempotent**, so duplicates must be removed under the correct binding/union semantics first. NaN/infinities and all exact-cast/error rules stay identical across resident, spilled, derived and native paths.

Keep float intervals as dense half-open ranges with canonical endpoints, NaN refused and infinity only as an unbounded end. Finite floating probes use the finite-point guard. Endpoint ordering, overlap, adjacency and coverage reuse the interval algebra. Floating interval length and integer duration weights are not interchangeable: keep float capacity/`FixedInterval<F64>` refused unless the semantic contract explicitly changes them.

Replace duplicated permissive image parsing with a compiled canonical row walker feeding either borrowed scalar values or column words. Validate once on ingress or at the trusted persisted-read boundary, then use proof-carrying trusted rows internally; do not parse every intermediate word through a general-purpose value object in the hot join loop. The shared walker must check declared fixed-interval width and reject a NaN **at either float endpoint**. Corruption is a refusal, not normalization.

Numerical operation guards remain whole-operation guards where host floating execution needs them, not per tuple. No host callback runs under a temporarily changed floating environment. Cross-platform bits, exceptional results and restoration of the host state are release obligations, not a comment that the operation is deterministic.

## 7. LMDB ownership and publication

Elastic virtual map growth is the correct backend. A large mapping is not resident memory; report virtual extent, file bytes, live pages and allocated blocks separately. Keep geometric, page-aligned growth and no product-level 32 GiB cap.

Growth requires no live transaction in the process. Do not invalidate borrowed pages to resize. Do not wait approximately a year when an embedded caller is itself holding the read lease that blocks its nested write. Specify a short bounded resize-wait policy distinct from the compute budget, return a typed retryable blocked-by-readers diagnosis, and retain the owned candidate for a safe retry. Higher layers control session scope; read-modify-write should use the proper writer snapshot rather than a long public read lease around an unrelated writer.

Opening with an explicit map ceiling below existing populated data must have one honest behavior: reject the configuration. Do not silently exceed the purported host limit while the comment claims refusal.

Fresh populated-store publication uses a staging directory: create, populate/judge, durable commit, close/fsync as required, then publish the destination. `Db::from_instance` must not create the final empty destination first and leave it visible after population failure.

An owned pinned read contains the existing OwnedSnapshot plus shared schema/cache owners. A short borrowed read frame is constructed for each fresh-work operation. Native workers store snapshots and prepared state as data, not parked read-closure reactors; Rust writer scopes remain synchronous internal operations. No unsafe Send retrofit or JS-driven write transaction is required.

Snapshot adoption requires a fresh-destination capability or complete relevant metadata emptiness checks. “No application rows” does not imply “fresh”: an empty-data log can still hold receipts, attachment and generation. Never overlay copied host records onto such a destination.

Core owns atomic store/copy/admission primitives. Log owns migration policy, backup manifests, replay, retention and hosted publication. Reuse core row/codec/staging machinery; no public Rust log product is introduced.

## 8. Proof and performance obligations

Lean proves mathematical statements under declared hypotheses; it does not prove that a similarly named Rust function now implements them. Rebuild the bridge against actual current symbols, especially deleted `storage/commit/*` paths, and attach each proof premise to an observable refinement test. Keep the complete reference judgment and independent query model independent of production access choices.

Priority evidence is behavior that kills a known wrong implementation:

- Conflicting tentative rows, replacements, selected target removal/re-establishment, floors/ceilings and interval coverage agree with an independent final-state model.
- Exact-key and fingerprint projections, including forced collisions, give identical facts and verdicts.
- Small indexed writes touch only the required groups; unrelated writes rebuild no untouched images.
- Tiny working allowances with sufficient scratch complete genuinely beyond-RAM text/derived/result cases; a genuinely insufficient allowance refuses before forbidden growth and publishes nothing.
- A long no-result query, COLT force, image filter, wide-row decode and page copy all stop within the documented polling bounds.
- Cancellation/rejection/map growth/failed publication leave committed state and ownership coherent.
- Float results are bit-identical across orderings/partitions/backings/platforms, and malformed rows are refused at every trusted reader.

No smoke test is required for these contracts. Small adversarial inputs expose each property; separate large-storage and cross-platform qualification demonstrates the scale regime. Keep independent expected data beside the test that owns it, not in a new `fixtures/`, implementation, exhaust, or report hierarchy. A test must not require unnecessary scratch usage merely to prove that scratch exists. This is one final convergence pass with one reviewed outcome, not an invitation to recreate repeated audit/report waves.

Use the historical M2 Max work in sibling `bumblebench` as hypotheses and measurement discipline, not universal CPU constants: batching depends on cache residency; tag-gated miss probes can outperform unconditional SIMD; bounds-check structure and vector layout affect generated code; fsync changes the clock/core regime; prefetch can help pressured hits and hurt already-saturated loops. Measure whole mutation-then-query behavior with matching durability and stamped machine/toolchain. Requalify on Apple Silicon, ARM Graviton and x86 Linux; keep platform-specific branches only where a concrete kernel earns them.

For every retained magic number identify whether it is a representation bound, host policy, or measured crossover. Keep one location and a reason. The full candidate's storage amplification versus SQLite must count canonical row payload, row keys, membership, every compiled projection, B-tree/page overhead and history separately. Shorter hashes alone cannot explain or fix that total.

## 9. Implementation order and deletion outcome

The scheduling authority is [chapter 60](60-cursor-execution.md): L01–L21 proceed through at least twelve concurrent execution workers and contract-ready handoffs, not a core-first waterfall. Freeze logical evidence/numeric meanings and the actual shared declarations early; producers and consumers then work together. Native/SDK, transport, staged storage and release tooling do not wait for the entire query engine. Resolve reciprocal ownership concerns in one coordinator-owned seam decision.

Do not freeze an incorrect `Vec`/uncharged API to avoid coordinating workers. Delete superseded paths as each replacement is integrated; transfer permanent documentation before proposal retirement, then qualify the post-retirement candidate as chapter 90 requires. There is no cleanup-after-qualified-inputs shortcut.

Delete duplicate row validators, `Any`-selected spill policy, unbounded competitor materializers, stale universal-generation cache assumptions, dead proof citations, and obsolete smoke/shape-only tests. Collapse overlapping tables and per-prepared relation caches where the ownership design replaces them. The success criterion is fewer independent rules with explicit ownership and explainable work—not fewer lines at the cost of an unbounded or incorrect execution path.
