# Current log findings — authority and lifecycle

Paths are relative to `crates/bumbledb-log/src/` unless explicit. Source-only review; every LOG-001–029 obligation survives in [50](50-audit-closure-matrix.md). Use C4–C6 and C8, not old packet numbers in comments.

## LOG-001/026/029 — Preserve the new certainty sum; finish its transitions

`certainty.rs` now derives phase from state-specific variants and removes from_outcome. Native known-receipt diagnostic-health handling has also been repaired. These are improvements, not reasons to rebuild the outcome vocabulary.

Remaining schedule: `admin.rs` must finish encoding and byte admission before setting dispatched. Failure before any authority call is not unknown. After an Indeterminate request, a deadline on the next iteration cannot produce NotStarted. Known rejected receipt plus failed diagnostic work stays Decided with incomplete detail.

L08 owns the transition placement, L14 the actual wire conversion, L17 Effect Cause/health. Delete phase inference from broad errors and certainty-to-legacy adapters where still used. D13 drives real producers; scripted decoder arms are insufficient. Source repair plus remaining transition review; do not claim execution evidence.

## LOG-002/003/004/017/021 — Same writer, full parent identity

`apply.rs` and the moved retirement comparison improve writer coherence. Review `admin.rs::apply_hosted_retirement_locally`: a captured decision alone does not protect a newer control revision at that same decision, and receipt keys assembled outside the writer must be revalidated.

Counterexample: control advances without a fact decision between remote capture and local transition; old retirement must not overwrite the newer access/activation state. Local prune cannot use a different generation’s receipt plan.

L08 checks identity/decision/control under the same writer, uses bounded receipt visitors and supports same-tip retirement. L09 local-root/GC transitions preserve monotonic control; L10 checkpoint consumes the captured root. D14. Preserve repaired writer discipline; classification is required race closure, not every old precheck still present.

## LOG-005 — Coverage and absence are one proof

`writer/hosted.rs::resolve_after_unknown` now has a retirement check. The final implementation must obtain covered interval and absent receipt from one coherent frontier, not a remote pre-catch-up capture plus a later local lookup.

A command can publish, lose its response, then have its receipt retired before resolve. A changed version is compatible with this attempt winning. Return expired-unprovable/unknown with its original ref unless a matching receipt or covered loss proof exists.

L08 constructs the proof value; L14/L17 preserve it. D15 includes retirement between capture/lookup and a genuine covered loser. Delete changed-version-only loss paths. Source-observed partial repair; coherence must be demonstrated.

## LOG-013 — Valid checkpoint root now fails strict validation

`history/locator.rs::validate_tip_locator` permits None only when tip.seq==0; `manifest.rs::read_recovery` invokes it without the base. `checkpointer.rs` produces base==tip with None after an ordinary checkpoint at a nonzero sequence.

That valid head cannot decode under the new validator. Additionally `validate_suffix` overwrites tip_object on every backward step, potentially retaining the oldest fetched suffix object as the tip.

L09 implements C6’s checked checkpoint-only/suffix root, full-stamp comparisons and one walker. L10 updates checkpoint/restore callers. Delete sequence-zero-as-no-suffix and overwritten initial-tip logic. D16 includes sequence 7 checkpoint-only, multi-decision suffix and read-after-publish. Source-confirmed contradictory producer/decoder contracts.

## LOG-013/018 — Codec length and traversal still disagree

`history/decision.rs::encode_decision` currently sizes a present parent as 1 + 1 + OBJECT_REF_ENCODED_LEN, while encoding writes one tag plus the 49-byte reference: one byte overcount, not the old four-byte undercount. Its new test repeats the same wrong arithmetic. Debug length assertion fails; a tight valid cap can refuse unnecessarily.

`history/locator.rs` and `store.rs` both implement chain walking; recovery still uses the old walker and checkpoint/backup still call epoch-probing fetch_decision. The new walk’s decrement-before-zero check also needs exact n=1 handling.

L09 owns codec/walker; L08/L10 consume it. Delete duplicate length arithmetic, required-link probing and whole-tail-return walkers. D16 independent frame bytes, cap±1, fetch-count limits, malformed links, checkpoint stop and relocated backup. Do not qualify the codec with an expectation derived from its own formula.

## LOG-007/008/009/010/022/027 — Streaming must reach every caller

Latest source recheck: create_staged has been replaced with begin_staged/StagedPopulation, and restore now starts private staging. Preserve that improvement. However recovery::install_judged_store calls incremental prepare(empty), repeating CORE-016, while StagedPopulation::open_db uses UnreadyStore::disarm to return a queryable Db plus a bare cleanup path. The readiness/cleanup owner is still not retained by construction.

MapSpill has also landed in migration/state.rs, but MapSpill::finish scans all scratch rows back into the in-memory Rows/BTreeMap and next.relations owns that whole result. It spills then rematerializes, rather than supplying a bounded row source. read_backup_tail still retains the tail.

L10 replaces these with C4 private staged population and C2/C6 bounded transforms, logical export and tail visitation. L14 removes native Vec<Vec<u8>>/whole-object aggregates. Use core codecs/evaluator/index population, not handmade ChangeSet headers or a log-owned database. D06/D17/D20/D26 cover initialization, nonempty-required cold reopen, migration, invalid target, backup and fresh-lineage restore with source/output beyond RAM.

Delete remaining ready-path/population escapes, disarm-to-bare-path ownership and full-state shadow maps/MapSpill::finish rematerialization as production prerequisites. Small RAM optimization is only a bounded strategy of the same representation.

## LOG-011/012/014/023/024 — Receiving wrapper is progress, not universal enforcement

`store/receive.rs` and shared transport runtime exist. Audit every remaining `ConditionalStore`/S3 body call: old get_object/read_head whole-body methods still exist, and a post-read bounded wrapper does not constrain their peak.

L11 makes contextual receiving the only production path, including HEAD, retries and unknown content lengths. L09 uses canonical bounded listing progress; L08 alone classifies publication certainty. D17/G08 require real S3/IAM evidence for missing versus denied, 412/409, retry/lost response, immutable conflict, pagination, redirects and credential renewal. Mocks qualify neither provider semantics nor hardware.

Delete alternate unbounded production methods after consumers move; keep essential backend observations distinct.

## Retained lifecycle obligations

Local root transactions, scoped identities, cancellation tombstones, activation binding, current control projection, origin matching, finite open-tail defaults and ownership through cleanup remain mandatory. Their absence from this shorter defect list is not closure. L08/L09/L10/L11/L14/L17 have explicit lanes; use LOG rows and 220 permanent schedules rather than constructing a second log architecture.

No novel storage engine, CRDT core, network framework, backup service or automatic application migration is selected.
