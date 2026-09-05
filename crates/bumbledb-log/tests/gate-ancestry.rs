//! F3 review-fix regressions for finding E at the machine level: `AtLeast`
//! ancestry witnesses over BOTH history machines. `LocalHistory::witness`
//! judges retained receipt rows plus the one-time activation evidence;
//! `HostedHistory::witness_ancestor` judges the composed head's root
//! evidence and the protected decision chain, bounded by the catch-up
//! window budget. Pruned or unretained evidence is `WitnessUnavailable` —
//! never a claimed validation, never a sequence-integer comparison
//! (chapter 20 receipts/roots, chapter 30 `AtLeast`, OPS-005).
//!
//! Also pins finding F's machine half: an invariant-rejected receipt from
//! EITHER machine carries canonical evidence that decodes through the ONE
//! core codec (`bumbledb::schema::evidence`) to the same violation set.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use bumbledb::schema::ValidateDescriptor as _;
use bumbledb::{
    ChangeSet, Db, ExecutionPolicy, Id128, RelationId, Theory as _, Value, WorkContext,
};

use bumbledb_log::checkpointer::{
    CheckpointKind, CheckpointOutcome, CheckpointPolicy, publish_checkpoint, read_live_head,
};
use bumbledb_log::history::command::{Command, CommandMetadata, Limits};
use bumbledb_log::history::decision::{
    GenesisProvenance, GenesisRecord, blank_initial_digests, genesis_stamp,
};
use bumbledb_log::history::{
    CommandId, CommandResult, Condition, DatabaseId, DatabaseIdentity, DecisionDigest,
    DecisionStamp, IncarnationId, OperationId, ReceiptEpoch, RequestId, TerminalOutcome,
};
use bumbledb_log::replica::WitnessCheck;
use bumbledb_log::store::mem::MemStore;
use bumbledb_log::writer::{HostedHistory, LocalHistory, SubmitOutcome};

bumbledb::schema! {
    pub GateMini;
    relation Item { a: u64, b: u64 }
    Item(a) -> Item;
}

const LIMITS: Limits = Limits {
    envelope_bytes: 1_000_000,
    change_bytes: 900_000,
    evidence_bytes: 100_000,
    result_bytes: 1_000,
};

static DIR_SEQ: AtomicU64 = AtomicU64::new(0);

fn temp_dir(tag: &str) -> PathBuf {
    let seq = DIR_SEQ.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let path = std::env::temp_dir().join(format!(
        "bdb-gate-ancestry-{tag}-{}-{nanos}-{seq}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("create test root");
    path
}

fn policy() -> ExecutionPolicy {
    ExecutionPolicy {
        input_bytes: 100_000_000,
        working_bytes: 100_000_000,
        scratch_bytes: 100_000_000,
        result_bytes: 100_000_000,
        rows: 10_000_000,
        work_units: 1_000_000_000,
        timeout: Duration::from_secs(600),
    }
}

fn work() -> WorkContext {
    policy().start().expect("work budget starts")
}

fn fresh_db(tag: &str) -> Arc<Db<bumbledb::SchemaDescriptor>> {
    let dir = temp_dir(tag).join("db");
    Arc::new(
        Db::create(&dir, GateMini.descriptor(), work())
            .expect("create store")
            .expect("empty store admits"),
    )
}

fn identity(db: &Db<bumbledb::SchemaDescriptor>, seed: u8) -> DatabaseIdentity {
    DatabaseIdentity {
        database_id: DatabaseId::from_core(Id128::from_bytes([seed; 16])),
        incarnation_id: IncarnationId::from_core(Id128::from_bytes([seed ^ 0xff; 16])),
        schema_id: bumbledb::schema::fingerprint::fingerprint(db.schema()),
    }
}

fn command(
    db: &Db<bumbledb::SchemaDescriptor>,
    identity: DatabaseIdentity,
    request: u8,
    rows: &[(u64, u64)],
) -> Command {
    let mut draft = ChangeSet::builder(db.schema(), work());
    for (a, b) in rows {
        draft
            .insert(RelationId(0), &[Value::U64(*a), Value::U64(*b)])
            .expect("insert");
    }
    let changes = draft.finish().expect("finish");
    Command::seal(
        CommandMetadata {
            identity,
            id: CommandId {
                receipt_epoch: ReceiptEpoch::INITIAL,
                request_id: RequestId::from_core(Id128::from_bytes([request; 16])),
            },
            condition: Condition::Unconditional,
        },
        changes,
        CommandResult::empty(),
        LIMITS,
        &work(),
    )
    .expect("seals")
}

fn decided(outcome: SubmitOutcome) -> bumbledb_log::history::TerminalReceipt {
    match outcome {
        SubmitOutcome::Decided { receipt, .. } => receipt,
        other => panic!("expected a decided receipt, got {other:?}"),
    }
}

fn own_genesis(identity: DatabaseIdentity) -> DecisionStamp {
    genesis_stamp(
        &GenesisRecord {
            identity,
            initial_application_digest: blank_initial_digests().0,
            initial_system_digest: blank_initial_digests().1,
            provenance: GenesisProvenance::Create,
        },
        LIMITS.envelope_bytes,
    )
    .expect("genesis stamp")
}

// ---------------------------------------------------------------------------
// LocalHistory: receipt-row + activation evidence.
// ---------------------------------------------------------------------------

#[test]
fn local_witness_judges_retained_evidence_never_sequence_integers() {
    let db = fresh_db("local-witness");
    let identity = identity(&db, 0x11);
    let history = LocalHistory::create(
        Arc::clone(&db),
        identity.database_id,
        identity.incarnation_id,
        OperationId::from_core(Id128::from_bytes([1; 16])),
        LIMITS,
        &work(),
    )
    .expect("creates");

    let mut stamps = Vec::new();
    for request in 1u8..=3 {
        let receipt = decided(history.submit(
            &command(&db, identity, request, &[(u64::from(request), 0)]),
            &work(),
        ));
        stamps.push(receipt.decision_at);
    }

    // A retained past stamp proves ancestry; a forged hash at the SAME
    // height refutes it (this is the defect-E repro: the old bridge
    // accepted any lower sequence without evidence).
    assert_eq!(
        history.witness(stamps[0], &work()).expect("witness runs"),
        WitnessCheck::Ancestor
    );
    let forged = DecisionStamp {
        seq: stamps[0].seq,
        hash: DecisionDigest::from_bytes([0xEE; 32]),
    };
    assert_eq!(
        history.witness(forged, &work()).expect("witness runs"),
        WitnessCheck::NotAncestor
    );

    // Sequence zero is judged against the one-time activation evidence.
    assert_eq!(
        history
            .witness(own_genesis(identity), &work())
            .expect("witness runs"),
        WitnessCheck::Ancestor
    );
    let mut foreign = identity;
    foreign.incarnation_id = IncarnationId::from_core(Id128::from_bytes([0x99; 16]));
    assert_eq!(
        history
            .witness(own_genesis(foreign), &work())
            .expect("witness runs"),
        WitnessCheck::NotAncestor,
        "a foreign incarnation's genesis is not this lineage"
    );

    // An unretained height (no receipt row survived) is Unavailable, never
    // assumed: rotate the epoch and retire epoch 1's rows.
    bumbledb_log::admin::rotate_receipts_local(
        &db,
        ReceiptEpoch::new(2).expect("two"),
        LIMITS.envelope_bytes,
        &work(),
    )
    .expect("rotates");
    let removed =
        bumbledb_log::admin::retire_receipts_local(&db, 1, LIMITS.envelope_bytes, &work())
            .expect("retires");
    assert!(removed >= 3, "the epoch-1 receipt rows are deleted");
    assert_eq!(
        history.witness(stamps[0], &work()).expect("witness runs"),
        WitnessCheck::Unavailable,
        "pruned evidence is EXPLICITLY unavailable — not accepted, not corrupt"
    );
    // Activation evidence survives retirement.
    assert_eq!(
        history
            .witness(own_genesis(identity), &work())
            .expect("witness runs"),
        WitnessCheck::Ancestor
    );
}

// ---------------------------------------------------------------------------
// HostedHistory: composed-head root evidence + bounded chain walk.
// ---------------------------------------------------------------------------

fn hosted<'a>(
    tag: &str,
    seed: u8,
    store: &'a MemStore,
) -> (
    Arc<Db<bumbledb::SchemaDescriptor>>,
    DatabaseIdentity,
    HostedHistory<bumbledb::SchemaDescriptor, &'a MemStore>,
) {
    let db = fresh_db(tag);
    let scope = identity(&db, seed);
    let history = HostedHistory::create(
        Arc::clone(&db),
        store,
        "t".to_string(),
        1,
        scope.database_id,
        scope.incarnation_id,
        OperationId::from_core(Id128::from_bytes([seed.wrapping_add(1); 16])),
        LIMITS,
        &work(),
    )
    .expect("creates");
    (db, scope, history)
}

#[test]
fn hosted_witness_walks_the_verified_chain_from_the_captured_tip() {
    let store = MemStore::new();
    let (db, identity, history) = hosted("hosted-witness", 0x21, &store);

    let mut stamps = Vec::new();
    for request in 1u8..=5 {
        let receipt = decided(history.submit(
            &command(&db, identity, request, &[(u64::from(request), 0)]),
            &work(),
        ));
        stamps.push(receipt.decision_at);
    }
    let tip = stamps[4];

    // Every retained ancestor proves; a forged hash at a retained height
    // refutes; the genesis is judged from the activation evidence.
    for stamp in &stamps[..4] {
        assert_eq!(
            history
                .witness_ancestor(tip, *stamp, &work())
                .expect("witness runs"),
            WitnessCheck::Ancestor,
            "seq {} is an ancestor",
            stamp.seq
        );
    }
    let forged = DecisionStamp {
        seq: 2,
        hash: DecisionDigest::from_bytes([0xEE; 32]),
    };
    assert_eq!(
        history
            .witness_ancestor(tip, forged, &work())
            .expect("witness runs"),
        WitnessCheck::NotAncestor
    );
    assert_eq!(
        history
            .witness_ancestor(tip, own_genesis(identity), &work())
            .expect("witness runs"),
        WitnessCheck::Ancestor
    );
    let mut foreign = identity;
    foreign.database_id = DatabaseId::from_core(Id128::from_bytes([0x77; 16]));
    assert_eq!(
        history
            .witness_ancestor(tip, own_genesis(foreign), &work())
            .expect("witness runs"),
        WitnessCheck::NotAncestor
    );

    // A future coordinate is never witnessed from retained evidence.
    let future = DecisionStamp {
        seq: 99,
        hash: DecisionDigest::from_bytes([9; 32]),
    };
    assert_eq!(
        history
            .witness_ancestor(tip, future, &work())
            .expect("witness runs"),
        WitnessCheck::Unavailable
    );
}

#[test]
fn hosted_witness_respects_the_checkpoint_base_and_the_walk_budget() {
    let store = MemStore::new();
    let (db, identity, history) = hosted("hosted-pruned", 0x31, &store);

    // Decisions 1..=3, then a checkpoint at the tip (base = seq 3), then
    // decisions 4..=8 in the protected tail (3, 8].
    let mut stamps = Vec::new();
    for request in 1u8..=3 {
        stamps.push(
            decided(history.submit(
                &command(&db, identity, request, &[(u64::from(request), 0)]),
                &work(),
            ))
            .decision_at,
        );
    }
    let checkpoint_policy = CheckpointPolicy {
        chunk_bytes: 4_096,
        head_cap: LIMITS.envelope_bytes,
        ..CheckpointPolicy::DEFAULT
    };
    let outcome = publish_checkpoint(
        &db,
        &store,
        "t",
        LIMITS,
        CheckpointKind::Ordinary,
        &checkpoint_policy,
        &work(),
    )
    .expect("checkpoint publishes");
    assert!(matches!(outcome, CheckpointOutcome::Published { .. }));
    for request in 4u8..=8 {
        stamps.push(
            decided(history.submit(
                &command(&db, identity, request, &[(u64::from(request), 0)]),
                &work(),
            ))
            .decision_at,
        );
    }
    let tip = stamps[7];
    let (head, _) = read_live_head(&store, "t", LIMITS.envelope_bytes).expect("head reads");
    let recovery = head.recovery.expect("recovery root");
    assert_eq!(recovery.base, stamps[2], "the checkpoint base is seq 3");

    // The base stamp itself is retained ROOT evidence.
    assert_eq!(
        history
            .witness_ancestor(tip, stamps[2], &work())
            .expect("witness runs"),
        WitnessCheck::Ancestor
    );
    let forged_base = DecisionStamp {
        seq: stamps[2].seq,
        hash: DecisionDigest::from_bytes([0xEE; 32]),
    };
    assert_eq!(
        history
            .witness_ancestor(tip, forged_base, &work())
            .expect("witness runs"),
        WitnessCheck::NotAncestor
    );

    // A VALID historical stamp strictly below the base is pruned evidence:
    // explicitly Unavailable — not accepted, not corruption.
    assert_eq!(
        history
            .witness_ancestor(tip, stamps[0], &work())
            .expect("witness runs"),
        WitnessCheck::Unavailable
    );

    // Stamps inside the protected tail (base, tip] still prove.
    assert_eq!(
        history
            .witness_ancestor(tip, stamps[4], &work())
            .expect("witness runs"),
        WitnessCheck::Ancestor
    );

    // The walk budget is respected: a machine bounded to 2 steps cannot
    // establish a witness 4 steps below the tip — Unavailable, never an
    // unbounded traversal and never a claim.
    let bounded = HostedHistory::open(Arc::clone(&db), &store, "t".to_string(), LIMITS, &work())
        .expect("reopens")
        .with_catch_up_bound(2);
    assert_eq!(
        bounded
            .witness_ancestor(tip, stamps[4], &work())
            .expect("witness runs"),
        WitnessCheck::Unavailable,
        "budget exhaustion is explicit unavailability"
    );
    // The same bounded machine still answers near witnesses.
    assert_eq!(
        bounded
            .witness_ancestor(tip, stamps[6], &work())
            .expect("witness runs"),
        WitnessCheck::Ancestor
    );
}

// ---------------------------------------------------------------------------
// Finding F (machine half): both machines record canonical evidence that
// decodes to the same complete violation set through the ONE core codec.
// ---------------------------------------------------------------------------

fn decoded_statements(receipt: &bumbledb_log::history::TerminalReceipt) -> Vec<(u16, usize)> {
    let TerminalOutcome::InvariantRejected { evidence } = &receipt.outcome else {
        panic!("expected an invariant-rejected receipt");
    };
    let decoded = bumbledb::schema::evidence::decode(evidence.as_bytes(), LIMITS.evidence_bytes)
        .expect("canonical evidence decodes");
    let schema = GateMini.descriptor().validate().expect("valid schema");
    let violations = decoded
        .to_violations(&schema, &work())
        .expect("evidence belongs to the schema");
    let rendered = bumbledb::render_rejection(&GateMini.descriptor(), &violations);
    rendered
        .iter()
        .map(|violation| (violation.statement().0, violation.facts().len()))
        .collect()
}

#[test]
fn local_and_hosted_rejections_decode_to_the_same_violation_set() {
    // The same violating command (two rows sharing key `a`) through BOTH
    // machines: each records nonempty canonical evidence, and the decoded
    // violation sets agree — identical behavior via LocalHistory and
    // HostedHistory, never an empty rejection.
    let rows = [(7u64, 1u64), (7, 2)];

    let local_db = fresh_db("f-local");
    let local_identity = identity(&local_db, 0x41);
    let local = LocalHistory::create(
        Arc::clone(&local_db),
        local_identity.database_id,
        local_identity.incarnation_id,
        OperationId::from_core(Id128::from_bytes([2; 16])),
        LIMITS,
        &work(),
    )
    .expect("creates");
    let local_receipt =
        decided(local.submit(&command(&local_db, local_identity, 9, &rows), &work()));

    let hosted_store = MemStore::new();
    let (hosted_db, hosted_identity, hosted) = hosted("f-hosted", 0x51, &hosted_store);
    let hosted_receipt =
        decided(hosted.submit(&command(&hosted_db, hosted_identity, 9, &rows), &work()));

    let local_set = decoded_statements(&local_receipt);
    let hosted_set = decoded_statements(&hosted_receipt);
    assert!(!local_set.is_empty(), "the violation set is never empty");
    assert_eq!(local_set, hosted_set, "one command, one decoded verdict");
    assert_eq!(local_set[0].0, 0, "statement 0 (`Item(a) -> Item`)");
    assert!(local_set[0].1 >= 1, "bounded example facts are present");

    // Resolve returns the SAME retained evidence on both machines.
    let resolved = local
        .resolve(local_receipt.command, &work())
        .expect("resolves");
    match resolved {
        bumbledb_log::writer::ResolveOutcome::Found(found) => {
            assert_eq!(decoded_statements(&found), local_set);
        }
        other => panic!("expected the retained receipt, got {other:?}"),
    }
    let resolved = hosted
        .resolve(hosted_receipt.command, &work())
        .expect("resolves");
    match resolved {
        bumbledb_log::writer::ResolveOutcome::Found(found) => {
            assert_eq!(decoded_statements(&found), hosted_set);
        }
        other => panic!("expected the retained receipt, got {other:?}"),
    }
}

#[test]
fn hosted_catch_up_walks_authenticated_parent_locators_only() {
    use bumbledb_log::history::locator::{
        ChainVisitor, OBJECT_REF_WIRE_BYTES, walk_decision_chain,
    };
    use bumbledb_log::store::{fetch_decision_ref, ObjectRef};

    assert_eq!(OBJECT_REF_WIRE_BYTES, 49);

    let store = MemStore::new();
    let db = fresh_db("loc-walk");
    let identity = identity(&db, 0x21);
    let hosted = HostedHistory::create(
        Arc::clone(&db),
        &store,
        "tenants/loc-walk".to_string(),
        1,
        identity.database_id,
        identity.incarnation_id,
        OperationId::from_core(Id128::from_bytes([0x22; 16])),
        LIMITS,
        &work(),
    )
    .expect("creates");
    let receipt = decided(hosted.submit(
        &command(&db, identity, 1, &[(1, 1)]),
        &work(),
    ));
    let _ = receipt;
    let (head, _) =
        read_live_head(&store, "tenants/loc-walk", LIMITS.envelope_bytes).expect("head");
    let recovery = head.recovery.expect("recovery");
    let tip_object = recovery.tip_object.expect("tip locator");
    let bytes = fetch_decision_ref(&store, "tenants/loc-walk", &tip_object).expect("one get");
    let envelope =
        bumbledb_log::history::decision::decode_decision(&bytes, LIMITS).expect("decodes");
    assert_eq!(envelope.stamp(), recovery.tip);
    let mut budget = 8;
    struct Count(usize);
    impl ChainVisitor for Count {
        type Error = bumbledb_log::store::ObjectError;
        fn visit(
            &mut self,
            _stamp: DecisionStamp,
            _bytes: &[u8],
            _reference: ObjectRef,
        ) -> Result<bool, Self::Error> {
            self.0 += 1;
            Ok(true)
        }
    }
    let mut count = Count(0);
    walk_decision_chain(
        &store,
        "tenants/loc-walk",
        recovery.tip,
        recovery.base,
        Some(tip_object),
        LIMITS,
        &mut budget,
        &mut count,
    )
    .expect("locator chain");
    assert_eq!(count.0, 1);
}

#[test]
fn checkpoint_only_and_suffix_walk_never_fetch_older_than_base() {
    use bumbledb_log::history::locator::{ChainVisitor, walk_decision_chain};
    use bumbledb_log::manifest::RecoveryRoot;
    use bumbledb_log::store::mem::Op;
    use bumbledb_log::store::{fetch_decision_ref, ObjectRef};

    let store = MemStore::new();
    let (db, identity, history) = hosted("ck-seq7", 0x71, &store);
    let mut stamps = Vec::new();
    for request in 1u8..=7 {
        stamps.push(
            decided(history.submit(
                &command(&db, identity, request, &[(u64::from(request), 0)]),
                &work(),
            ))
            .decision_at,
        );
    }
    let checkpoint_policy = CheckpointPolicy {
        chunk_bytes: 4_096,
        head_cap: LIMITS.envelope_bytes,
        ..CheckpointPolicy::DEFAULT
    };
    publish_checkpoint(
        &db,
        &store,
        "t",
        LIMITS,
        CheckpointKind::Ordinary,
        &checkpoint_policy,
        &work(),
    )
    .expect("checkpoint at seq 7");
    let (head, _) = read_live_head(&store, "t", LIMITS.envelope_bytes).expect("head");
    let recovery = head.recovery.expect("recovery");
    assert_eq!(recovery.base, stamps[6]);
    assert_eq!(recovery.tip, stamps[6]);
    assert!(recovery.tip_object.is_none(), "checkpoint-only has no tip locator");
    assert_eq!(
        RecoveryRoot::checkpoint_only(
            recovery.checkpoint,
            recovery.base,
            recovery.tail_bytes,
            recovery.epoch_floor,
        )
        .tip,
        recovery.tip
    );
    assert!(
        RecoveryRoot::suffix(
            recovery.checkpoint,
            recovery.base,
            recovery.tip,
            ObjectRef {
                epoch: 1,
                kind: bumbledb_log::store::ObjectKind::Decision,
                digest: *recovery.tip.hash.as_bytes(),
                length: 8,
            },
            0,
            recovery.epoch_floor,
        )
        .is_err(),
        "suffix refuses base == tip"
    );

    for request in 8u8..=9 {
        stamps.push(
            decided(history.submit(
                &command(&db, identity, request, &[(u64::from(request), 0)]),
                &work(),
            ))
            .decision_at,
        );
    }
    let (head, _) = read_live_head(&store, "t", LIMITS.envelope_bytes).expect("head after suffix");
    let recovery = head.recovery.expect("suffix recovery");
    assert_ne!(recovery.base, recovery.tip);
    let tip_object = recovery.tip_object.expect("2+ link suffix locator");
    let before = store.operations();
    struct Count(usize);
    impl ChainVisitor for Count {
        type Error = bumbledb_log::store::ObjectError;
        fn visit(
            &mut self,
            _stamp: DecisionStamp,
            _bytes: &[u8],
            _reference: ObjectRef,
        ) -> Result<bool, Self::Error> {
            self.0 += 1;
            Ok(true)
        }
    }
    let mut count = Count(0);
    let mut budget = 2;
    walk_decision_chain(
        &store,
        "t",
        recovery.tip,
        recovery.base,
        Some(tip_object),
        LIMITS,
        &mut budget,
        &mut count,
    )
    .expect("two-link suffix");
    assert_eq!(count.0, 2);
    let fetched: Vec<_> = store
        .operations()
        .into_iter()
        .skip(before.len())
        .filter(|(op, _)| *op == Op::GetObject)
        .map(|(_, key)| key)
        .collect();
    assert_eq!(fetched.len(), 2);
    if let Some(checkpoint) = recovery.checkpoint {
        let bytes = fetch_decision_ref(&store, "t", &tip_object).expect("tip present");
        let _ = bytes;
        let _ = checkpoint;
    }
    let _ = identity;
}
