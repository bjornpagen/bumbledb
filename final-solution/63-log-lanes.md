# Log execution lanes L08–L11

## Common dispatch preamble — send with every lane

Use exclusive writes in [60](60-cursor-execution.md). Read your source and attached C/D/finding sections first. Preserve the repaired implementation, replacing only broken/duplicate mechanisms. No tests/builds/typechecks/format checks/package hooks/benchmarks/commits during fanout; author tests for the final phase. No sibling/live-data mutations. Send ReadyForIntegration with actual producer and consumer changes, deletions, authored symbols, NotRun and explicit open seams. Cross-file changes go to the named owner; architectural conflicts go to the coordinator, never a compatibility shim.

## L08 — Publication evidence and one coherent local parent

**Read:** C4/C5/C6; LOG-001–006/016/017/021/025/026/029; certainty.rs, admin.rs, writer/hosted.rs::resolve_after_unknown and apply.rs. Own admin/apply/certainty/identities and writer/**. Integration tests: writer_hosted.rs, writer_local.rs, cert-publication.rs, adversarial-admin-identity.rs, adversarial-trace.rs, history_admission.rs, history_command.rs, history_model.rs, conformance_v3.rs. Other history codec tests are L09.

**Outcome:** every authority path preserves strongest evidence and local control/facts/receipts never regress across concurrent apply/retirement. Ready at F0 on C5 and the C6 walker signature.

**Implement:** retain state-specific attempt sums. Encode/admit bytes before actual dispatch; record stable ref beforehand. After dispatch, every cancellation/retry/local/decode failure remains unknown or positively decided. Build covered negative proof from one coherent installed frontier/receipt lookup; retirement produces expired-unprovable. Revalidate identity+decision+control under the same local writer for all control/receipt transitions; discover/prune keys boundedly under that ownership. Same-tip maintenance must work without fake user decisions. Preserve exact historical precondition/evidence replay.

**Outputs:** evidence-bearing Rust results and mutation recovery refs to L10/L14/L17; traversal requests to L09; backend observations required from L11.

**Delete:** arbitrary phase inference, certainty-erasing adapters, out-of-writer authority checks as proof, changed-version-only loss and hidden callback retry.

**Acceptance:** D05/D13/D14/D15 drive published-lost-response, retired receipt, control-only races, local commit/diagnostic failure and a genuinely proved loser through real producers. Keep a small independent history model; do not make it call production decision logic.

**Do not:** declare absence permanent, mint new IDs on retry, conflate host health with publication, or launch a new history protocol.

## L09 — Checked roots, one locator walk, rooted retention

**Read:** C5/C6; LOG-012/013/015/016/018; history/locator.rs, decision codec, manifest read_recovery, gc/local_roots and every fetch_decision caller. Own history/**, manifest.rs, gc.rs and local_roots.rs. Integration tests: history_framing.rs, adversarial-hostile-bytes.rs, gate-ancestry.rs, lane_gc.rs, lane_local_roots.rs, lane_erase.rs, lane_ops.rs.

**Outcome:** every root/frame/traversal agrees on base and tip; GC protects exactly the authenticated closure, with bounded progress. Ready at F0.

**Implement:** checked checkpoint-only/suffix root constructors; full stamp comparison, required tip kind/hash and parent validation. ObjectRef=49, option=1 or50; derive frame sizes once. Preserve initial tip locator, stop before reading older than base and honor n=1 fetch budget. One early-stoppable streaming walker handles source refs; backup relocation uses manifest refs with unchanged historical bytes. Remove epoch probing after all consumers move. Maintain exact HEAD revisions for GC progress, canonical last-completed-key listing and transactional local-root registration/release.

**Outputs:** checked refs, encoded-size helpers and visitor to L08/L10/L11/L14. Explicitly request migration of recovery/checkpointer/backup; an unused strict walker is not delivered.

**Delete:** duplicate walkers, 45/51-byte duplicated formulas, missing-link fallback, oldest-suffix-object-as-tip, receipt/object epoch conflation and opaque-provider-token durable progress.

**Acceptance:** D16 plus G10: checkpoint-only seq7, equal-sequence different-stamp refusal, 2+ link suffix, exact-cap±1 frames, malformed/interior links, no fetch beyond base, relocated backup, crash/resume GC and local root races. Independent expected frame bytes must fail the current extra-tag formula.

**Do not:** retain all ancestors by default, rehome published decisions without protocol authority, or delete from object listing/age alone.

## L10 — One bounded private materializer through every lifecycle

**Read:** C2/C4/C5/C6; LOG-007–010/017/021/022/027; recovery::begin_staged/StagedPopulation/install_judged_store (create_staged was removed during review), restore, backup tail, checkpointer and migration state/executor/hosted/MapSpill::finish. Own those source prefixes and bin/**. Own remaining log integration tests not assigned L08/L09/L11, excluding coordinator helper hubs.

**Outcome:** initialization, cold hydration, migration/resume, backup/restore and inspection share bounded core staging, without a ready partial target or database-sized shadow map. Ready at F0 on L07 stage and L09/L11 traversal/receiving declarations.

**Implement:** finish the new begin_staged/StagedPopulation integration using private UnreadyStore; remove disarm-to-Store/bare-path escape and install_judged_store’s empty-delta incremental judgment. Populate canonical facts/indexes/opaque records in bounded batches; invoke complete admission once for the final target. Stream compiled Map transforms directly into exact staged sets; deduplicate convergent output there. Compile all expressions under verified source/target schemas before iterating, even zero rows. Remove whole-tail/chunk aggregates and MapSpill::finish’s full reconstruction of Rows; keep the spill-backed relation through every consumer. Checkpoint captures one snapshot then validates the exact current suffix; same-tip retirement/rebase is legal. Preserve source freeze, target-ready, activate, cancel-tombstone and explicit route cutover semantics; retry verifies exact operation identity.

**Outputs:** bounded producer/sink and install outcomes to L14/L17; actual checked-root consumers to L09; no per-lifecycle codecs.

**Delete:** MigrationState/CollectedState full-row prerequisites, ready-path restore, manual core header assembly, backup-tail Vec and separate unbounded lifecycle convenience methods.

**Acceptance:** D06/D16/D17/D20/D26 and inherited migration/crash schedules. Valid nonempty-required target must initialize, cold reopen, restore and resume; invalid final key/capacity target never becomes ready. Source/output/tail beyond RAM still complete with sufficient scratch. Fail each install/cutover boundary and verify exact absent-or-complete/recoverable identity.

**Do not:** add online dual-write migration, execute JS callbacks over rows, silently reseed absence or thaw source after an unproved target cancellation.

## L11 — Real bounded transports and kernel exclusion

**Read:** C5/C6/C8; LOG-011/012/014/023/024; store.rs, store/receive.rs, s3/fs/mem/fence. Own store.rs and store/**. Integration tests: lane_b_fs_store.rs, lane_b_mem_store.rs, lane_b_interop.rs, local_ownership.rs and s3_smoke.rs (replace smoke-only assertions with real semantics or remove them).

**Outcome:** no production body receive buffers beyond the admitted envelope; actual backend observations stay truthful; existing kernel exclusion can serve the generator without lock-file guessing. Ready at F0.

**Implement:** contextual receiving as the only production read path for HEAD/objects, with bounded chunks, deadline/cancellation and incremental hash/length verification. Admit copy overlap. Bound provider retry/list work; distinguish missing/denied/bucket/region errors and conditional 412/409/indeterminate outcomes. Keep shared Tokio/credentials, actual credential refresh and canonical paged listing. Expose/reuse existing kernel-held directory fence without importing log policy into core; persistent lock inode is never unlinked as stale recovery. Same-process contention must be covered.

**Outputs:** receiving/store and exclusion primitives to L08/L09/L10/L14. L08 interprets certainty; do not emit a guessed publication verdict from a generic transport error.

**Delete:** legacy whole-body get_object/read_head production methods after callers migrate, per-tenant runtimes, TTL ownership and split body/fence authority. Update fake transports to the same observation contract, not to manufactured success.

**Acceptance:** D17/D28 and real S3/IAM cells at final qualification: changing/unknown length, stalled body, lost ack after commit, conditional conflict/retry, immutable identical/different bytes, pagination, region/denied and credential refresh. OS lock remains held while owner is paused; death releases it without deleting the inode.

**Do not:** run credentialed/live checks during fanout or label emulator behavior real-backend qualification. Missing test credentials is NotRun, never a waived pass.
