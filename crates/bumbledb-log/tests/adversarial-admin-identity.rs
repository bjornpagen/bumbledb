//! F3 review finding A — admin operations must validate the intended
//! database identity at the authority boundary (REP-011 / SDK-016 /
//! ARCH-004; gates G10/G11/G14).
//!
//! The defect: admin verbs selected an open database by DIRECTORY alone and
//! a hosted authority by PREFIX alone; no comparison of the requested
//! binding's database/incarnation/schema identity against the loaded
//! authority record preceded mutation. These regressions pin the gates the
//! fix installed: `admin::verify_local_identity` (warm and cold local
//! materializations) and `admin::verify_hosted_identity` (hosted prefix
//! selection), each refusing with a typed [`AdminError::Identity`] BEFORE
//! any tenant-state change — asserted here as byte-unchanged facts,
//! receipts, roots and authority on the unintended tenant.

mod lane_support;

use std::sync::Arc;

use bumbledb::schema::SchemaDescriptor;
use bumbledb::{Db, Id128, RelationId, Value};
use bumbledb_log::admin::{
    AdminError, apply_hosted_retirement_locally, capture_local_parent, local_authority,
    rotate_receipts_hosted, rotate_receipts_local, verify_hosted_identity, verify_local_identity,
};
use bumbledb_log::certainty::AdminCertainty;
use bumbledb_log::history::{
    DatabaseId, DatabaseIdentity, HeadRevision, IncarnationId, ReceiptEpoch, SchemaId,
};
use bumbledb_log::manifest::{HeadRecord, encode_head};
use bumbledb_log::store::fs::FsStore;
use bumbledb_log::store::mem::MemStore;
use bumbledb_log::store::{
    BackendError, ConditionalOutcome, ObservedError, ReceiveLimits, ReceivedHead, ReceivingStore,
    TransportContext, TransportObservation, head_key, objects_prefix,
};
use bumbledb_log::writer::{LocalHistory, SubmitOutcome};
use lane_support::{HEAD_CAP, LIMITS, fresh_db, insert_user, op, temp_dir, work};

/// Submit must actually decide — a silently refused fixture write would make
/// every byte-unchanged assertion vacuous.
fn must_decide(outcome: SubmitOutcome) {
    match outcome {
        SubmitOutcome::Decided { .. } => {}
        other => panic!("fixture submit decides, got {other:?}"),
    }
}

/// One hosted tenant under `prefix` with an explicitly chosen identity
/// (`lane_support`'s `Mirror` pins one fixed identity; these gates need
/// distinct tenants).
fn create_hosted<B>(
    tag: &str,
    backend: &B,
    prefix: &str,
    db_seed: u8,
    inc_seed: u8,
) -> (
    Arc<Db<SchemaDescriptor>>,
    LocalHistory<SchemaDescriptor>,
    DatabaseIdentity,
)
where
    B: ReceivingStore,
    B::Error: BackendError + ObservedError,
{
    let db = fresh_db(tag);
    let identity = DatabaseIdentity {
        database_id: DatabaseId::from_core(Id128::from_bytes([db_seed; 16])),
        incarnation_id: IncarnationId::from_core(Id128::from_bytes([inc_seed; 16])),
        schema_id: bumbledb::schema::fingerprint::fingerprint(db.schema()),
    };
    let history = LocalHistory::create(
        Arc::clone(&db),
        identity.database_id,
        identity.incarnation_id,
        op(0xc3),
        LIMITS,
        &work(),
    )
    .expect("local history creates");
    let authority = history.authority().expect("authority reads");
    let record = HeadRecord::genesis(authority, 0).expect("genesis head");
    let body = encode_head(&record, HEAD_CAP).expect("head encodes");
    match backend.create_head(&head_key(prefix), &body) {
        Ok(ConditionalOutcome::Published { .. }) => {}
        other => panic!("genesis head publish: {other:?}"),
    }
    (db, history, identity)
}

/// Every byte the hosted tenant under `prefix` owns: the head body plus all
/// content-addressed objects, sorted by key.
fn snapshot_prefix<B>(store: &B, prefix: &str) -> Vec<(String, Vec<u8>)>
where
    B: ReceivingStore,
    B::Error: BackendError + ObservedError,
{
    let owned = work();
    let ctx = TransportContext::new(&owned, ReceiveLimits::capped(HEAD_CAP as u64));
    let head = match store
        .receive_head(&head_key(prefix), ctx)
        .expect("bounded head receive")
    {
        ReceivedHead::Present { body, .. } => {
            let bytes = body.as_bytes().to_vec();
            drop(body);
            bytes
        }
        ReceivedHead::Absent => Vec::new(),
    };
    let mut rows = vec![(head_key(prefix), head)];
    let mut keys = Vec::new();
    let mut after: Option<Box<[u8]>> = None;
    loop {
        let page = store
            .list_objects(&objects_prefix(prefix), after.as_deref())
            .expect("listing");
        keys.extend(page.keys);
        match page.next {
            Some(next) => after = Some(next),
            None => break,
        }
    }
    keys.sort();
    for key in keys {
        let body = match store.receive_object(&key, ctx) {
            Ok(bytes) => {
                let copied = bytes.as_bytes().to_vec();
                drop(bytes);
                copied
            }
            Err(error) => match error.observation() {
                TransportObservation::Missing => Vec::new(),
                other => panic!("object {key} receive: {other:?}"),
            },
        };
        rows.push((key, body));
    }
    rows
}

/// Every byte the local tenant owns that admin verbs can touch: all host
/// records (receipts, roots registry, binding), the authority attachment,
/// and the application facts of the one test relation.
#[allow(clippy::type_complexity)]
fn snapshot_local(
    db: &Db<SchemaDescriptor>,
) -> (Vec<(Vec<u8>, Vec<u8>)>, Option<Vec<u8>>, Vec<Vec<Value>>) {
    let mut records = Vec::new();
    let mut attachment = None;
    let mut facts = Vec::new();
    db.read(work(), |read| {
        read.integration_host_scan(b"", &mut |key: &[u8], value: &[u8]| {
            records.push((key.to_vec(), value.to_vec()));
            Ok(())
        })
        .expect("host scan");
        attachment = read
            .integration_host_attachment()
            .expect("attachment reads")
            .map(<[u8]>::to_vec);
        for row in read.scan(RelationId(0)).expect("facts scan") {
            facts.push(row.expect("fact row"));
        }
        Ok(())
    })
    .expect("read");
    (records, attachment, facts)
}

fn assert_identity_refusal(result: Result<HeadRecord, AdminError>, dimension: &str) {
    match result {
        Err(AdminError::Identity(mismatch)) => assert_eq!(
            mismatch.dimension(),
            dimension,
            "the refusal names the exact disagreeing identity dimension"
        ),
        other => panic!("expected a typed identity refusal, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Hosted prefix selection is not validation: the gate compares identities.
// ---------------------------------------------------------------------------

#[test]
fn hosted_gate_refuses_every_foreign_identity_dimension_and_bytes_are_unchanged() {
    let store = MemStore::new();
    let (db, history, identity) = create_hosted("hg-dims", &store, "t", 0x11, 0x22);
    must_decide(history.submit(&insert_user(&db, identity, 1, 10), &work()));
    let before = snapshot_prefix(&store, "t");

    // Same schema, different DATABASE (the same-schema-tenant confusion).
    let mut foreign_db = identity;
    foreign_db.database_id = DatabaseId::from_core(Id128::from_bytes([0x99; 16]));
    assert_identity_refusal(
        verify_hosted_identity(&store, "t", foreign_db, HEAD_CAP, &work()),
        "database",
    );

    // Same database NAME, different INCARNATION (rebirth / stale binding).
    let mut stale = identity;
    stale.incarnation_id = IncarnationId::from_core(Id128::from_bytes([0x77; 16]));
    assert_identity_refusal(
        verify_hosted_identity(&store, "t", stale, HEAD_CAP, &work()),
        "incarnation",
    );

    // Same database and incarnation, different SCHEMA.
    let mut wrong_schema = identity;
    wrong_schema.schema_id = SchemaId([0x55; 32]);
    assert_identity_refusal(
        verify_hosted_identity(&store, "t", wrong_schema, HEAD_CAP, &work()),
        "schema",
    );

    assert_eq!(
        snapshot_prefix(&store, "t"),
        before,
        "three refused gates left the tenant's head and objects byte-unchanged"
    );

    // The exact identity passes and returns the loaded authority head.
    let head = verify_hosted_identity(&store, "t", identity, HEAD_CAP, &work()).expect("exact identity");
    assert_eq!(head.control.identity, identity);
}

fn valid_identity_wrong_prefix_refuses<B>(store: &B, tag_a: &str, tag_b: &str)
where
    B: ReceivingStore,
    B::Error: BackendError + ObservedError,
{
    // Two tenants on ONE store: identity A is fully valid — it is just not
    // the tenant living under prefix "b".
    let (_db_a, _history_a, identity_a) = create_hosted(tag_a, store, "a", 0x31, 0x32);
    let (db_b, history_b, identity_b) = create_hosted(tag_b, store, "b", 0x41, 0x42);
    must_decide(history_b.submit(&insert_user(&db_b, identity_b, 1, 10), &work()));
    let before_b = snapshot_prefix(store, "b");

    // The admin boundary: the gate runs BEFORE the mutation is dispatched.
    assert_identity_refusal(
        verify_hosted_identity(store, "b", identity_a, HEAD_CAP, &work()),
        "database",
    );
    assert_eq!(
        snapshot_prefix(store, "b"),
        before_b,
        "the unintended tenant's bytes are unchanged after the refusal"
    );

    // The same request aimed at ITS OWN tenant validates and then mutates —
    // the gate blocks only cross-tenant aim, never legitimate maintenance.
    verify_hosted_identity(store, "a", identity_a, HEAD_CAP, &work()).expect("own tenant validates");
    assert!(
        matches!(
            rotate_receipts_hosted(
                store,
                "a",
                ReceiptEpoch::new(2).expect("epoch"),
                HEAD_CAP,
                &work(),
            ),
            AdminCertainty::Completed { .. }
        ),
        "own-tenant rotation proceeds after the gate"
    );
    assert_eq!(
        snapshot_prefix(store, "b"),
        before_b,
        "tenant b remains byte-unchanged while tenant a is maintained"
    );
}

#[test]
fn hosted_gate_valid_identity_wrong_prefix_refuses_on_mem_store() {
    let store = MemStore::new();
    valid_identity_wrong_prefix_refuses(&store, "hg-mem-a", "hg-mem-b");
}

#[test]
fn hosted_gate_valid_identity_wrong_prefix_refuses_on_fs_store() {
    let root = temp_dir("hg-fs-store");
    let store = FsStore::new(root.to_string_lossy().into_owned());
    valid_identity_wrong_prefix_refuses(&store, "hg-fs-a", "hg-fs-b");
}

// ---------------------------------------------------------------------------
// Local materializations: holding a directory is not validation either.
// ---------------------------------------------------------------------------

#[test]
fn local_gate_refuses_foreign_identities_and_the_tenant_stays_byte_unchanged() {
    let db = fresh_db("lg-local");
    let identity = DatabaseIdentity {
        database_id: DatabaseId::from_core(Id128::from_bytes([0x61; 16])),
        incarnation_id: IncarnationId::from_core(Id128::from_bytes([0x62; 16])),
        schema_id: bumbledb::schema::fingerprint::fingerprint(db.schema()),
    };
    let history = LocalHistory::create(
        Arc::clone(&db),
        identity.database_id,
        identity.incarnation_id,
        op(0xc4),
        LIMITS,
        &work(),
    )
    .expect("creates");
    must_decide(history.submit(&insert_user(&db, identity, 1, 10), &work()));
    must_decide(history.submit(&insert_user(&db, identity, 2, 20), &work()));
    let before = snapshot_local(&db);

    // Same schema, different database: another tenant's directory paired
    // with a perfectly valid — but foreign — identity.
    let mut foreign = identity;
    foreign.database_id = DatabaseId::from_core(Id128::from_bytes([0x98; 16]));
    match verify_local_identity(&db, foreign, LIMITS.envelope_bytes) {
        Err(AdminError::Identity(mismatch)) => assert_eq!(mismatch.dimension(), "database"),
        other => panic!("expected identity refusal, got {other:?}"),
    }

    // Stale binding: the old incarnation after a restore/migration reborn
    // this database under a new one.
    let mut stale = identity;
    stale.incarnation_id = IncarnationId::from_core(Id128::from_bytes([0x63; 16]));
    match verify_local_identity(&db, stale, LIMITS.envelope_bytes) {
        Err(AdminError::Identity(mismatch)) => assert_eq!(mismatch.dimension(), "incarnation"),
        other => panic!("expected identity refusal, got {other:?}"),
    }

    // Wrong schema identity.
    let mut wrong_schema = identity;
    wrong_schema.schema_id = SchemaId([0x54; 32]);
    match verify_local_identity(&db, wrong_schema, LIMITS.envelope_bytes) {
        Err(AdminError::Identity(mismatch)) => assert_eq!(mismatch.dimension(), "schema"),
        other => panic!("expected identity refusal, got {other:?}"),
    }

    assert_eq!(
        snapshot_local(&db),
        before,
        "facts, receipts, roots registry and authority are byte-unchanged \
         after every refusal"
    );

    // The exact identity passes, returning the loaded authority record, and
    // legitimate maintenance still proceeds afterwards.
    let authority =
        verify_local_identity(&db, identity, LIMITS.envelope_bytes).expect("exact identity");
    assert_eq!(authority.identity, identity);
    rotate_receipts_local(
        &db,
        ReceiptEpoch::new(2).expect("epoch"),
        LIMITS.envelope_bytes,
        &work(),
    )
    .expect("own-tenant rotation proceeds after the gate");
}

// ---------------------------------------------------------------------------
// Stale binding after restore/migration: the reborn incarnation refuses the
// old binding.
//
// NOTE: the intended fixture drove the REAL backup→restore chain
// (`restore_writable_with_tail`), but that chain is currently red on a
// cross-lane defect independent of this finding: P05's own
// `lane_backup_restore::backup01_05_...` fails identically with
// `Recovery(Host(KeysNotStrictlyOrdered))` (unsorted/duplicate system-record
// keys reaching the host seal). Until that lane repairs it, this regression
// models exactly the identity state a restore/migration produces — the same
// database reborn under a NEW incarnation, holding the same facts — and pins
// the admin gate's stale-binding refusal against it. The restore internals
// themselves are that lane's coverage, not this gate's.
// ---------------------------------------------------------------------------

#[test]
fn stale_binding_after_reincarnation_refuses_against_the_new_incarnation() {
    // The pre-restore lineage: database D, incarnation I1.
    let database = DatabaseId::from_core(Id128::from_bytes([0x71; 16]));
    let old_incarnation = IncarnationId::from_core(Id128::from_bytes([0x72; 16]));
    let new_incarnation = IncarnationId::from_core(Id128::from_bytes([0xdd; 16]));

    // The post-restore/post-migration materialization: the SAME database,
    // reborn under incarnation I2 in its own directory.
    let db = fresh_db("sb-reborn");
    let history = LocalHistory::create(
        Arc::clone(&db),
        database,
        new_incarnation,
        op(0x0f),
        LIMITS,
        &work(),
    )
    .expect("reborn incarnation creates");
    let new_identity = history.identity();
    must_decide(history.submit(&insert_user(&db, new_identity, 1, 10), &work()));
    must_decide(history.submit(&insert_user(&db, new_identity, 2, 20), &work()));

    // The stale binding: everything matches except the incarnation.
    let stale = DatabaseIdentity {
        database_id: database,
        incarnation_id: old_incarnation,
        schema_id: new_identity.schema_id,
    };
    let before = snapshot_local(&db);
    match verify_local_identity(&db, stale, LIMITS.envelope_bytes) {
        Err(AdminError::Identity(mismatch)) => {
            assert_eq!(mismatch.dimension(), "incarnation");
            assert_eq!(mismatch.actual.incarnation_id, new_incarnation);
            assert_eq!(mismatch.expected.incarnation_id, old_incarnation);
        }
        other => panic!("expected stale-binding refusal, got {other:?}"),
    }
    assert_eq!(
        snapshot_local(&db),
        before,
        "the reborn tenant is byte-unchanged after the stale-binding refusal"
    );

    // The NEW binding validates against the reborn authority.
    let authority = verify_local_identity(&db, new_identity, LIMITS.envelope_bytes)
        .expect("new incarnation validates");
    assert_eq!(authority.identity, new_identity);
}

// ---------------------------------------------------------------------------
// Retirement apply validates the materialized parent under the writer.
// ---------------------------------------------------------------------------

#[test]
fn hosted_retirement_apply_refuses_stale_captured_decision() {
    let store = MemStore::new();
    let (db, history, identity) = create_hosted("retire-parent", &store, "r", 0x51, 0x52);
    must_decide(history.submit(&insert_user(&db, identity, 1, 10), &work()));
    let authority = local_authority(&db, LIMITS.envelope_bytes).expect("local authority");
    let mut captured = capture_local_parent(&authority).expect("live parent");
    captured.decision.seq = captured.decision.seq.saturating_sub(1);
    let new_control = authority.retire_receipts(1).expect("retire frontier");
    let outcome = apply_hosted_retirement_locally(
        &db,
        &new_control,
        captured,
        1,
        LIMITS.envelope_bytes,
        &work(),
    );
    assert!(
        matches!(outcome, Err(AdminError::Corruption(_))),
        "stale captured decision must refuse: {outcome:?}"
    );
}

#[test]
fn hosted_retirement_apply_refuses_newer_control_at_the_same_decision() {
    let store = MemStore::new();
    let (db, history, identity) = create_hosted("retire-rev", &store, "r", 0x61, 0x62);
    must_decide(history.submit(&insert_user(&db, identity, 1, 10), &work()));
    let authority = local_authority(&db, LIMITS.envelope_bytes).expect("local authority");
    let mut captured = capture_local_parent(&authority).expect("live parent");
    captured.revision = HeadRevision(captured.revision.0.saturating_sub(1));
    let new_control = authority.retire_receipts(1).expect("retire frontier");
    let outcome = apply_hosted_retirement_locally(
        &db,
        &new_control,
        captured,
        1,
        LIMITS.envelope_bytes,
        &work(),
    );
    assert!(
        matches!(outcome, Err(AdminError::Corruption(_))),
        "stale captured revision at the same decision must refuse: {outcome:?}"
    );
}
