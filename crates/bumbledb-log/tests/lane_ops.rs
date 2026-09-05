//! Bounded status fixtures and maintenance cancellation — OPS-TEST-01 and the
//! maintenance half of OPS-TEST-02 (native-runtime shutdown is P06/C09; the
//! obligation here is that bounded work stops typed, keeps durable progress
//! and resumes). Verification: `NotRun` (F1 authors, does not execute).

mod lane_support;

use std::time::Duration;

use bumbledb::ExecutionPolicy;
use bumbledb_log::admin;
use bumbledb_log::checkpointer::{CheckpointKind, CheckpointPolicy, publish_checkpoint};
use bumbledb_log::gc::{GcError, GcPolicy, run_collection};
use bumbledb_log::history::authority::{DeletedReason, FreezeIntent};
use bumbledb_log::inspect::{Condition, render, status_hosted, status_of_head, status_of_local};
use bumbledb_log::store::mem::MemStore;
use lane_support::{HEAD_CAP, LIMITS, Mirror, insert_user, op, work};

fn ckpt_policy() -> CheckpointPolicy {
    CheckpointPolicy {
        chunk_bytes: 4_096,
        head_cap: HEAD_CAP,
        ..CheckpointPolicy::DEFAULT
    }
}

#[test]
fn ops01_status_fixtures_distinguish_every_condition_without_payloads() {
    let store = MemStore::new();
    let mut mirror = Mirror::create("ops-status", &store, "t");
    let identity = mirror.identity;
    // Empty: at the tip with zero data revisions.
    let head = mirror.head();
    let genesis_tip = head.control.live().expect("live").decision;
    assert_eq!(
        status_of_head(&head, Some(genesis_tip)).condition,
        Condition::Empty
    );
    // NotYetHydrated: the head exists, no local stamp.
    assert_eq!(
        status_of_head(&head, None).condition,
        Condition::NotYetHydrated
    );
    // Ready and StaleButValid.
    mirror.submit(&insert_user(mirror.db(), identity, 1, 10));
    let head = mirror.head();
    let tip = head.control.live().expect("live").decision;
    assert_eq!(status_of_head(&head, Some(tip)).condition, Condition::Ready);
    assert_eq!(
        status_of_head(&head, Some(genesis_tip)).condition,
        Condition::StaleButValid
    );
    // Local authority: Ready directly from the attachment.
    let local = status_of_local(&mirror.authority());
    assert_eq!(local.condition, Condition::Ready);
    // Frozen: admission stops, status says so, retained stamps remain.
    admin::freeze_hosted(
        &store,
        "t",
        op(0x01),
        FreezeIntent::Erasure,
        HEAD_CAP,
        &work(),
    )
    .expect("freeze");
    let frozen = status_hosted(&store, "t", Some(tip), HEAD_CAP);
    assert_eq!(frozen.condition, Condition::Frozen);
    assert!(
        frozen.decision.is_some(),
        "a frozen tenant still reports its stamps"
    );
    // Deleted: tombstone, no live stamps invented.
    admin::tombstone_hosted(
        &store,
        "t",
        op(0x02),
        DeletedReason::Erasure,
        HEAD_CAP,
        &work(),
    )
    .expect("tombstone");
    let deleted = status_hosted(&store, "t", None, HEAD_CAP);
    assert_eq!(deleted.condition, Condition::Deleted);
    assert_eq!(deleted.decision, None);
    // Missing: definite absence at another prefix; never created by status.
    let missing = status_hosted(&store, "elsewhere", None, HEAD_CAP);
    assert_eq!(missing.condition, Condition::Missing);
    // Renderings are bounded counters/hex; no payload or credential text.
    for status in [&frozen, &deleted, &missing] {
        let text = render(status);
        for banned in ["AKIA", "secret", "password", "credential", "value-"] {
            assert!(!text.contains(banned), "{banned} in {text}");
        }
    }
}

#[test]
fn ops02_cancelled_maintenance_stops_typed_and_resumes_from_durable_progress() {
    let store = MemStore::new();
    let mut mirror = Mirror::create("ops-cancel", &store, "t");
    let identity = mirror.identity;
    for request in 1..=3u8 {
        mirror.submit(&insert_user(
            mirror.db(),
            identity,
            request,
            u64::from(request),
        ));
    }
    // An exhausted work budget stops the checkpoint with a typed refusal and
    // publishes nothing.
    let starved = ExecutionPolicy {
        input_bytes: 1 << 30,
        working_bytes: 1 << 30,
        scratch_bytes: 1 << 30,
        result_bytes: 1 << 30,
        rows: 1 << 20,
        work_units: 1,
        timeout: Duration::from_secs(600),
    }
    .start()
    .expect("starved budget");
    let refused = publish_checkpoint(
        mirror.db(),
        &store,
        "t",
        LIMITS,
        CheckpointKind::Ordinary,
        &ckpt_policy(),
        &starved,
    );
    assert!(refused.is_err(), "{refused:?}");
    assert!(
        mirror
            .head()
            .recovery
            .expect("recovery")
            .checkpoint
            .is_none(),
        "a cancelled checkpoint published nothing"
    );
    // A fresh budget completes the same maintenance.
    publish_checkpoint(
        mirror.db(),
        &store,
        "t",
        LIMITS,
        CheckpointKind::Ordinary,
        &ckpt_policy(),
        &work(),
    )
    .expect("resumed checkpoint");
    // GC under a starved budget stops typed; durable phase state remains
    // resumable and a fresh budget converges.
    let gc_policy = GcPolicy {
        head_cap: HEAD_CAP,
        ..GcPolicy::DEFAULT
    };
    let starved = ExecutionPolicy {
        input_bytes: 1 << 30,
        working_bytes: 1 << 30,
        scratch_bytes: 1 << 30,
        result_bytes: 1 << 30,
        rows: 1 << 20,
        work_units: 1,
        timeout: Duration::from_secs(600),
    }
    .start()
    .expect("starved budget");
    let stopped = run_collection(&store, "t", op(0x11), LIMITS, &gc_policy, &starved);
    assert!(
        matches!(stopped, Err(GcError::Work(_) | GcError::Checkpoint(_))),
        "cancellation reaches actual maintenance work: {stopped:?}"
    );
    let report = run_collection(&store, "t", op(0x11), LIMITS, &gc_policy, &work())
        .expect("resumed collection");
    assert!(
        report.finished,
        "the interrupted collection resumed to completion"
    );
}
