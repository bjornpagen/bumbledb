//! The object-layer composition over the deterministic store: verified puts
//! and gets, immutable-conflict refusal, ambiguous-store resolution, and the
//! bounded decision epoch probe (C07 grammar; STORE-05/06 shapes).
//! Verification: `NotRun` (F1 authors, does not execute).

use std::time::Duration;

use bumbledb::work::Resource;
use bumbledb::{ExecutionPolicy, WorkContext};
use bumbledb_log::history::DecisionDigest;
use bumbledb_log::store::mem::{Behavior, MemStore, Op};
use bumbledb_log::store::{
    ConditionalStore as _, ObjectError, ObjectKind, ObjectRef, ReceiveLimits, ReceivingStore,
    TransportContext, TransportObservation, fetch_decision, get_verified, object_digest,
    put_verified,
};

fn work() -> WorkContext {
    ExecutionPolicy {
        input_bytes: 0,
        working_bytes: 1 << 20,
        scratch_bytes: 0,
        result_bytes: 0,
        rows: 0,
        work_units: 1_024,
        timeout: Duration::from_secs(5),
    }
    .start()
    .expect("work")
}

fn transport(work: &WorkContext) -> TransportContext<'_> {
    TransportContext::new(work, ReceiveLimits::capped(1 << 20))
}

#[test]
fn put_verified_resolves_ambiguity_by_content_and_refuses_conflicts() {
    let store = MemStore::new();
    // Applied-but-unacknowledged: resolved by reading identical content back.
    store.fail_next(Op::PutObject, Behavior::IndeterminateApplied);
    let reference = put_verified(&store, "t", 1, ObjectKind::Chunk, b"bytes")
        .expect("ambiguity resolves by content equality");
    let ctx = work();
    assert_eq!(
        get_verified(&store, "t", &reference, transport(&ctx))
            .expect("get")
            .as_bytes(),
        b"bytes"
    );
    // Dropped-and-unacknowledged: the read-back finds nothing — unresolved,
    // never claimed durable.
    let store = MemStore::new();
    store.fail_next(Op::PutObject, Behavior::IndeterminateDropped);
    let unresolved = put_verified(&store, "t", 1, ObjectKind::Chunk, b"bytes");
    assert!(
        matches!(unresolved, Err(ObjectError::Unverified { .. })),
        "{unresolved:?}"
    );
    // A conflicting payload at the same immutable name refuses: plant foreign
    // bytes at the content address, then attempt the honest put ambiguously.
    let store = MemStore::new();
    let honest = ObjectRef::of(1, ObjectKind::Chunk, b"honest");
    store
        .put_object(&honest.key("t"), b"conflicting occupant")
        .expect("planted");
    store.fail_next(Op::PutObject, Behavior::IndeterminateDropped);
    let conflict = put_verified(&store, "t", 1, ObjectKind::Chunk, b"honest");
    assert!(
        matches!(conflict, Err(ObjectError::ImmutableConflict { .. })),
        "creation never overwrites a colliding payload: {conflict:?}"
    );
}

#[test]
fn get_verified_checks_length_and_domain_separated_digest_before_returning() {
    let store = MemStore::new();
    let reference =
        put_verified(&store, "t", 2, ObjectKind::Checkpoint, b"manifest bytes").expect("stored");
    let ctx = work();
    assert_eq!(
        get_verified(&store, "t", &reference, transport(&ctx))
            .expect("verified")
            .as_bytes(),
        b"manifest bytes"
    );
    // Corrupted content: digest refusal.
    assert!(store.corrupt_object(&reference.key("t"), |bytes| bytes[0] ^= 0xff));
    assert!(matches!(
        get_verified(&store, "t", &reference, transport(&ctx)),
        Err(ObjectError::WrongDigest { .. })
    ));
    // Truncated content: length refusal (checked before the digest).
    assert!(store.corrupt_object(&reference.key("t"), |bytes| {
        bytes.truncate(3);
    }));
    assert!(matches!(
        get_verified(&store, "t", &reference, transport(&ctx)),
        Err(ObjectError::WrongLength { .. })
    ));
    // Absent: definite missing, not a transport error.
    let ghost = ObjectRef::of(2, ObjectKind::Chunk, b"never stored");
    assert!(matches!(
        get_verified(&store, "t", &ghost, transport(&ctx)),
        Err(ObjectError::Missing { .. })
    ));
    // A wrong-kind reference to the same bytes is a different address.
    let as_chunk = ObjectRef {
        kind: ObjectKind::Chunk,
        ..reference
    };
    assert!(get_verified(&store, "t", &as_chunk, transport(&ctx)).is_err());
}

#[test]
fn fetch_decision_probes_the_bounded_epoch_window_newest_first() {
    let store = MemStore::new();
    let body = b"decision bytes stand-in";
    let digest = DecisionDigest::from_bytes(object_digest(ObjectKind::Decision, body));
    // Staged under epoch 3 of a [1, 5] window.
    let key = bumbledb_log::store::decision_key("t", 3, &digest);
    store.put_object(&key, body).expect("stored");
    let (epoch, bytes) = fetch_decision(&store, "t", 1, 5, &digest).expect("found");
    assert_eq!(epoch, 3);
    assert_eq!(bytes, body);
    // Absent across the whole window: definite missing after bounded probes.
    let ghost = DecisionDigest::from_bytes([9; 32]);
    let missing = fetch_decision(&store, "t", 1, 5, &ghost);
    assert!(matches!(missing, Err(ObjectError::Missing { .. })));
    let probes = store
        .operations()
        .into_iter()
        .filter(|(op, key)| {
            *op == Op::GetObject && key.contains(&bumbledb_log::store::hex32(ghost.as_bytes()))
        })
        .count();
    assert_eq!(
        probes, 5,
        "exactly the window [floor, ceiling], never a slot scan"
    );
    // Corrupt bytes at the address refuse rather than returning.
    assert!(store.corrupt_object(&key, |bytes| bytes[0] ^= 0xff));
    assert!(matches!(
        fetch_decision(&store, "t", 1, 5, &digest),
        Err(ObjectError::WrongDigest { .. })
    ));
}

#[test]
fn receive_reports_missing_and_cap_without_manufactured_success() {
    let store = MemStore::new();
    store
        .put_object("t/objects/1/chunk/aa", b"0123456789")
        .expect("put");
    let capped = store
        .receive_object(
            "t/objects/1/chunk/aa",
            TransportContext {
                work: None,
                receive: ReceiveLimits::capped(3),
            },
        )
        .expect_err("cap");
    assert_eq!(capped.observation, TransportObservation::Capped);
    let ctx = work();
    let missing = get_verified(
        &store,
        "t",
        &ObjectRef::of(1, ObjectKind::Chunk, b"never-stored"),
        transport(&ctx),
    );
    assert!(
        matches!(missing, Err(ObjectError::Missing { .. })),
        "{missing:?}"
    );
}

#[test]
fn get_verified_keeps_the_receive_charge_until_the_owner_drops() {
    let store = MemStore::new();
    let ctx = work();
    let baseline = ctx.used(Resource::WorkingBytes);
    let reference =
        put_verified(&store, "t", 1, ObjectKind::Chunk, b"charged-payload").expect("stored");
    let body = get_verified(&store, "t", &reference, transport(&ctx)).expect("verified");
    assert!(
        ctx.used(Resource::WorkingBytes) > baseline,
        "get_verified must not refund the reservation before the caller consumes the bytes"
    );
    assert_eq!(body.as_bytes(), b"charged-payload");
    drop(body);
    assert_eq!(ctx.used(Resource::WorkingBytes), baseline);
}

#[test]
fn get_verified_honors_a_tighter_caller_cap_and_returns_no_body() {
    let store = MemStore::new();
    let ctx = work();
    let reference = put_verified(&store, "t", 1, ObjectKind::Chunk, b"0123456789").expect("stored");
    assert!(
        get_verified(
            &store,
            "t",
            &reference,
            TransportContext::new(&ctx, ReceiveLimits::capped(3)),
        )
        .is_err(),
        "a tighter caller cap must not hand out a body"
    );
    let capped = store
        .receive_object(
            &reference.key("t"),
            TransportContext::new(&ctx, ReceiveLimits::capped(3)),
        )
        .expect_err("cap");
    assert_eq!(capped.observation, TransportObservation::Capped);
}

#[test]
fn get_verified_refuses_without_work_context() {
    let store = MemStore::new();
    let reference = put_verified(&store, "t", 1, ObjectKind::Chunk, b"bytes").expect("stored");
    assert!(
        get_verified(&store, "t", &reference, TransportContext::limited(64)).is_err(),
        "an uncharged receive is not an admitted owner"
    );
}
