# Core execution lanes L01–L07

## Common dispatch preamble — send with every lane

Implement only your exclusive files from [60](60-cursor-execution.md), preserving current useful work. Read the named source, attached C contracts and D discriminators. This is a clean cutover: replace the real production path and delete its predecessor, not an optional wrapper. Do not edit another lane’s files; send that owner a precise call/signature change. Root exports/manifests/shared helpers belong to the coordinator.

No tests, builds, typechecks, package hooks, lint/format checks, benchmarks or commits during the swarm. Author the discriminators now. Report ReadyForIntegration with changed paths, actual declarations/callers/deletions, authored test symbols, verification NotRun and every open seam. Escalate source evidence contradicting a fixed contract; do not invent a shim, weaken a limit or label required work future tightening.

## L01 — Compile the actual access machine

**Read:** C1/C4; CORE-002/011/012/022/025; Schema::shared_compiled_theory, schema/compiled.rs, storage/store/det_index.rs and schema positional law definitions. Exclusive writes are those two source files, with adjacent inline tests. schema.rs is a coordinator hub; storage/store/tests belongs to L07.

**Outcome:** all useful key/containment/capacity accesses and narrow optimization witnesses have deterministic interned descriptors consumed by storage, judge and query. Source-ready at F0; do not wait for L02/L05.

**Implement:** extend the existing shared theory, not another registry. Emit source reverse and target lookup projections for containment, source groups and targets for capacities, including interval ordered tails. Preserve cross-relation positional mapping with inverse permutations. Share only physically/semantically compatible indexes. Use exact scalar grouping up to 16 bytes subject to the full key limit; otherwise 16-byte fingerprint plus exact comparison. Keep candidate multiplicity until judgment. Exhaustion of projection IDs is explicit. Export descriptor-based row visits and lawful scalar/full-row distinctness witnesses.

**Outputs:** declarations and actual index-emission behavior to L02/L05/L07; exact changed call shapes to the coordinator for schema.rs. L07 updates persistent rows/tests and L05 consumes compiled witnesses; request both, do not declare them optional.

**Delete:** raw-statement determinant recompile, duplicate physical projection IDs, broad “key implies distinct” flags and redundant serialization of known schema tags.

**Acceptance:** D04/D10 on persisted roster/key bytes, reordered cross-relation columns, conflicting tentative rows, forced hash collisions and increasing unrelated groups. Inspect that planner and judge call the compiled owner. A test of 1+2+8+8 alone is not proof of a compact index. Never sacrifice full-row collision checks for measured speed.

**Handoff:** identify actual projection/witness consumers still to adapt, not merely a list of new types.

## L02 — Complete admission and local judgment are different proofs

**Read:** C1/C2/C4; CORE-016/021 and LOG-025; SchemaJudge::judge, CandidateState, complete and delta-local judges, offender selection. Own schema/judge.rs, schema/judge/**, schema/evidence.rs and storage/store/judge_bridge.rs, including their inline tests.

**Outcome:** unready imports cannot enter incremental judgment; eligible ordinary changes judge only affected groups; canonical rejection bytes survive physical remint. Ready at F0 on L01 descriptors and L03 charged-row declarations.

**Implement:** expose a complete-final-state judge for staging/verifier and an incremental entry requiring a lawful parent. No empty-delta shortcut. Consume charged decoded row views without Box extraction. Enumerate affected groups from actual net delta/compiled adjacency; implement keys, selected containment/coverage and floor/ceiling capacity correctly. Use ordered storage or bounded scratch for large groups. Select citations by canonical fact/group bytes before bounded top-k truncation; return every violated statement only if judgment completes.

**Outputs:** full and incremental entry contracts to L07; witness semantics to L08/L14; scratch requirements to L03. Preserve the independent full reference judge, sharing denotation but not optimized access planning.

**Delete:** empty-delta-as-full-validation, sequence/row-ID-selected citations, unbounded competitor Vecs, error-TypeId scratch choice and raw schema access planning.

**Acceptance:** D04/D05/D26. Invalid populated stage with zero delta must reject; valid nonempty-required state admits. Remint offender IDs and cross the citation limit without changing receipt bytes. Floor violations from removal and selected target replacement must match the independent final-state model. Resource refusal is not an invariant rejection.

**Do not:** restore a full scan for every incremental law merely to pass a test whose old expectation required spill. Escalate an unsupported locality claim rather than silently narrowing language.

## L03 — Own capacity and scratch transactions

**Read:** C2; CORE-003/004/005/009/020/024; canonical DecodedRow, work/owners.rs, exec/scratch.rs and capability/f3c accounting modules. Own the L03 prefixes in chapter 60, excluding work/cache.rs (L04).

**Outcome:** real decoded rows, buffers and scratch entries retain their charges until release/transfer, and failed/MapFull transactions leave no accounting mutation behind. Ready at F0, independent of cache internals.

**Implement:** private payload+reservation owners with borrowed access and whole-owner transfer. Reserve conservative actual capacity before growth; rollback on allocation failure. Supply admitted copy/transfer for image/result/native consumers. Transaction-local scratch pending writes include byte deltas and token/bucket sequence; commit once, abort entirely. Enforce logical and physical policy without charging every overwrite forever. Support ordered fixed-word claims plus exact arbitrary keys with forced collisions and early-stoppable visitors. Setup must acquire exclusive temporary identity and cleanup before fallible initialization; close native resources before unlink.

**Outputs:** actual owner/visitor/key declarations to L02/L04/L05/L06/L10/L13. Chase all production DecodedRow extractions via their owners; a new wrapper with only tests is incomplete.

**Delete:** public owning values/into_values/into_parts that detach charge, unused equivalent wrappers, speculative/manual reservation counters and reflective scratch injection.

**Acceptance:** D01/D03 before-allocation refusal, capacity retained after clear, overlapping transfer charges, MapFull retry, failed reservation, equal-size overwrite, shrink/reuse and colliding wide keys. Repeated failed setup must not leave or delete unowned directories. Authored perturbation sensitivity must distinguish charging twice from once.

**Do not:** promise exact RSS, introduce a custom allocator ecosystem or let arbitrary error type decide storage strategy.

## L04 — Cache generations carry meanings and bytes

**Read:** C2/C3; CORE-006/013; ImageCache::trim/admit_image, Cached, RelationSlot::Closed, GenerationLeaseRegistry, TextInterner, image build/canon/bind/nonresident. Own image.rs, image/** and work/cache.rs.

**Outcome:** an image or token cannot outlive its charge or resolver; bounded nonresident text is actually constructible. Ready at F0 on charged owner declarations.

**Implement:** shared generation object owns resolver; image owner holds generation and slab charge. Synchronize acquire/rotate under one protocol. Detach obsolete map references without refunding live allocations. Make idle memos weak/versioned; legitimately pinned old generations remain charged. Closed/numeric images obey the same ledger. Integrate admission before build/growth; avoid duplicate full string ownership. Supply bounded scratch token↔canonical text resolution and exact generation-aware comparison for L05. Do not add unsafe Send around Rc-held bytes; select shareable ownership where data really crosses workers, otherwise worker-local ownership.

**Outputs:** generation/image/resolver handles to L05/L07/L12/L13. Source review must find real acquisition at every retained token consumer. L05 adapts prepared memos; L07 attaches one database cache.

**Delete:** map-entry-only charge, detached pin counters and missing-Drop lease family, resetting shared interner beneath live images, unused alternate resolver, per-prepare database caches.

**Acceptance:** D01/D02/D29: two queries, old/new snapshots, concurrent trim/admit, pinned cache pressure, numeric/closed images and text beyond RAM. Dropping cache membership while an image is retained must not refund it. Rotation does not alias old/new tokens. L05’s real fallback uses nonresident resolution.

**Do not:** preserve all generations permanently, automatically pin idle prepared queries forever, or count the same slab once per reader.

## L05 — One bounded query machine through final delivery

**Read:** C1–C4/C8; CORE-004/007/008/010/022; prepared/fallback/reach/derived/result, exec/run/ledger, COLT and planner. Own L05’s prefixes, excluding scratch and sinks. All tests under those prefixes belong to you.

**Outcome:** resident Free Join remains fast, every nonresident/derived/recursive path stays bounded, and completed result delivery has an atomic ticket. Ready at F0; implement against L01/L03/L04 declarations before their internals complete.

**Implement:** consume compiled key/distinct/existence witnesses with explicit semantic premises; preserve factorization/COLT/batched probes. Propagate Continue/Stop/Error through fallback. Reserve before image/selection/COLT/pool growth, retain reusable charges, poll no-output work and flush the last quantum. Select fallback before u32 position overflow; permit at most one clean resident-to-disk restart on the same pinned snapshot.

Make sealed Resident|Scratch sources usable by positive/negative stages and linear recursive seen/frontier/accumulation. No refill-all conversion. Preserve stage errors, binding grain and rounding boundaries; small stages stay resident. Result builder seals only after full success; delivery ticket previews/adopts bounded rows and commits position explicitly after native output registration. It may rescan/retry bounded work but cannot consume undelivered rows or lose output charges.

**Outputs:** source/index calls to L01/L04/L07, sink contract to L06 and result ticket/owned output to L13. Tell L13 exact commit/abort and terminal-error rules.

**Delete:** raw-schema planner interpretation, full-scan key fallback, ignored sink stop, full recursive image resurrection, post-hoc result collection/admission and unlimited ordinary execution twins.

**Acceptance:** D07–D12/D25: independent resident/staged/scratch answers and errors, actual source-visit counts, sub-quantum work refusal, >RAM text recursion, correct row-position regime, fresh delivery work, two-row page cap boundary and retained capacity after operation exit.

**Do not:** invent another optimizer, per-tuple atomics/boxing or a public cursor; force spill on tiny stages; suppress stage errors behind a later filter.

## L06 — Pack and aggregate banks remain bounded

**Read:** C2/C3; CORE-005/023; exec/sink/aggregate/spill.rs, grouping/projection banks and float aggregate state. Own exec/sink.rs and exec/sink/**, including tests.

**Outcome:** Pack yields exact maximal intervals across arbitrary flushes without reconstructing all claims/groups in memory; all sink capacity uses L03 owners. Ready at F0 on ordered scratch declarations.

**Implement:** retain explicit checked narrow/wide mode. For wide groups, exact scratch-backed group→stable token and token→group tables; ordered (token,start,end) claims. Stream one token group through endpoint coalescing; fetch its canonical group header boundedly. No complete sort is needed to compare unrelated groups when output is a set. If an external order is required, use the existing bounded ordered substrate, not an all-rows Vec. Keep RAM-first narrow/small groups. Canonical F64 endpoint order and set binding grain survive spills; exact sum/count state is not rounded early.

**Outputs:** stable sink Finish/Continue/Stop/Error behavior and accounting to L05; required exact key shape to L03.

**Delete:** pack_wide_tokens Vec, resident exact group BTreeMap, all_claims/global final sort, per-claim token issuance and 0xFE payload-based mode inference. Remove only redundant sink code whose behavior has a surviving owner.

**Acceptance:** D01/D11/D19: reverse [10,20) then [0,15) over flushes → [0,20), interleaved groups, adjacency/gaps/duplicate claims, leading-0xFE narrow data, forced collisions and group metadata beyond RAM. Use independent interval sweep, not only resident sink code. Peak capacity and work stay within the documented bound.

**Do not:** disable legal wide grouping, allocate a group header per spilled claim, or make all small queries use LMDB.

## L07 — Core ownership, lawful staging and explicit Rust API

**Read:** C1/C2/C3/C4; CORE-014/015/016/017 and public specimens in chapter 30. Own api/db.rs, api/db/**, storage/store.rs and storage/store/** except L01 det_index and L02 judge_bridge. Own crates/bumbledb/tests/** and store/tests/**; ask coordinator before changing shared manifests/macros.

**Outcome:** public core operations have explicit work; snapshots are owned data; readiness/full judgment and no-clobber install are real. Ready at F0; use L02 judgment declarations while they are implemented.

**Implement:** refactor ReadInstance’s borrowed Db/closure-only lifetime into owned snapshot plus ephemeral borrowed frame over existing OwnedSnapshot and shared schema/cache. Pass fresh work to each operation. Keep actual transaction coherence and drop order. Do not manufacture unsafe Send/Sync or reopen a new snapshot for metadata.

UnreadyStore privately owns population/index/adjunct access; remove its ordinary Store and disarm-to-(Store,PathBuf) escape; transfer an admitted cleanup owner rather than a bare staging path. Full final-state admission calls L02’s complete judge, never empty delta incremental prepare. Protect cleanup from first fallible setup; install no-clobber and return exact installed identity after settlement failure. Fresh adoption verifies all metadata. Reuse initialization internals. Bound resize blocked by a caller’s own pinned reader; honor explicit ceiling, page alignment and >32 GiB support.

**Outputs:** owned-read/frame to L05/L12, full-stage API to L10/L14, typed public Rust calls to L18, projection maintenance to L01. Update affected integration tests directly rather than wrapping removed APIs.

**Delete:** ordinary unready Store accessor, ready destination population, decorative freshness tokens, unlimited work twins and duplicated environment initialization.

**Acceptance:** D06/D07/D26 plus store snapshot/resize/freshness schedules. Invalid populated target rejects even with no delta; valid nonempty-required target survives install/reopen; two installers never overwrite; failure after rename retains this attempt’s evidence without claiming an unrelated existing destination.

**Do not:** put migrations, backup policy or S3 in core, keep caller-facing JS transaction callbacks, or invalidate live LMDB borrowed pages to resize.
