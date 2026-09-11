//! The object-layer composition over the deterministic store: verified puts
//! and gets, immutable-conflict refusal, ambiguous-store resolution, and the
//! bounded decision epoch probe (C07 grammar; STORE-05/06 shapes).

use bumbledb::WorkContext;
use bumbledb_log::store::mem::{Behavior, MemStore, Op};
use bumbledb_log::store::{
    ConditionalStore as _, ObjectError, ObjectKind, ObjectRef, ReceiveLimits, ReceivingStore,
    TransportContext, TransportObservation, fetch_decision_ref, get_verified, put_verified,
};

fn work() -> WorkContext {
    WorkContext::new()
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
            .as_slice(),
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
            .as_slice(),
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
fn fetch_decision_uses_one_authenticated_locator_without_epoch_probing() {
    let store = MemStore::new();
    let body = b"decision bytes stand-in";
    let reference = ObjectRef::of(3, ObjectKind::Decision, body);
    let key = reference.key("t");
    store.put_object(&key, body).expect("stored");
    let ctx = work();
    let bytes = fetch_decision_ref(&store, "t", &reference, transport(&ctx)).expect("found");
    assert_eq!(bytes.as_slice(), body);
    let ghost = ObjectRef {
        epoch: 4,
        ..reference
    };
    let missing = fetch_decision_ref(&store, "t", &ghost, transport(&ctx));
    assert!(matches!(missing, Err(ObjectError::Missing { .. })));
    let probes = store
        .operations()
        .into_iter()
        .filter(|(op, key)| *op == Op::GetObject && key == &ghost.key("t"))
        .count();
    assert_eq!(
        probes, 1,
        "one GET at the authenticated locator, never an epoch scan"
    );
    // Corrupt bytes at the address refuse rather than returning.
    assert!(store.corrupt_object(&key, |bytes| bytes[0] ^= 0xff));
    assert!(matches!(
        fetch_decision_ref(&store, "t", &reference, transport(&ctx)),
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
fn get_verified_retains_only_its_received_buffer_until_the_owner_drops() {
    let store = MemStore::new();
    let ctx = work();
    let reference =
        put_verified(&store, "t", 1, ObjectKind::Chunk, b"owned-payload").expect("stored");
    let body = get_verified(&store, "t", &reference, transport(&ctx)).expect("verified");
    #[cfg(feature = "alloc-counter")]
    let before_drop = bumbledb::alloc_counter::snapshot().absolute.live_bytes;
    #[cfg(feature = "alloc-counter")]
    let capacity = body.capacity() as u64;
    ctx.cancel();
    assert_eq!(body.as_slice(), b"owned-payload");
    drop(body);
    #[cfg(feature = "alloc-counter")]
    assert_eq!(
        bumbledb::alloc_counter::snapshot().absolute.live_bytes,
        before_drop - capacity,
        "drop releases the full capacity; the adapter's request log is independent"
    );
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
fn get_verified_without_cancellation_still_verifies_the_complete_object() {
    let store = MemStore::new();
    let reference = put_verified(&store, "t", 1, ObjectKind::Chunk, b"bytes").expect("stored");
    assert_eq!(
        get_verified(&store, "t", &reference, TransportContext::limited(64)).unwrap(),
        b"bytes"
    );
    let mut wrong = reference;
    wrong.length += 1;
    assert!(matches!(
        get_verified(&store, "t", &wrong, TransportContext::limited(64)),
        Err(ObjectError::WrongLength { .. })
    ));
}
