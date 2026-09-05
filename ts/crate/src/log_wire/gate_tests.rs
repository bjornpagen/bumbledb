//! F3 review-fix regressions (findings E and F) over the ACTUAL bridge
//! machine — the same `open_history` / `open_published_snapshot` /
//! `run_history_verb` entries the napi verbs dispatch to, below the JS
//! marshalling only (a napi `Env` cannot exist in a Rust unit test; the
//! JS-side crossing is pinned by `ts-log/test/gate-evidence.test.ts`).
//!
//! Finding E: `AtLeast` must prove exact same-lineage ancestry from retained
//! authoritative evidence — never accept a lower sequence as a floor.
//! Finding F: an invariant-rejected receipt must carry the COMPLETE decoded
//! violation set through the canonical evidence codec — never an empty array.

use std::time::Duration;

use bumbledb::Theory as _;
use bumbledb::work::ExecutionPolicy;
use bumbledb_log::history::decision::{
    GenesisProvenance, GenesisRecord, blank_initial_digests, genesis_stamp,
};
use bumbledb_log::history::{DecisionDigest, DecisionStamp};

use super::*;
use crate::runtime::{CloseReport, Options};

bumbledb::schema! {
    pub GateMini;
    relation Item { a: u64, b: u64 }
    Item(a) -> Item;
}

bumbledb::schema! {
    pub GatePair;
    relation Pair { a: u64, b: u64, c: u64 }
    Pair(a) -> Pair;
    Pair(b) -> Pair;
}

fn options() -> Options {
    Options {
        workers: 2,
        queue_capacity: 8,
        cleanup_capacity: 8,
        owner_capacity: 8,
        native_handle_capacity: 16,
        aggregate_bytes: [64 << 20; 4],
        chunk_bytes: 1 << 20,
        cleanup_timeout: Duration::from_millis(500),
    }
}

fn policy() -> ExecutionPolicy {
    ExecutionPolicy {
        input_bytes: 16 << 20,
        working_bytes: 16 << 20,
        scratch_bytes: 16 << 20,
        result_bytes: 16 << 20,
        rows: 1 << 20,
        work_units: 1 << 30,
        timeout: Duration::from_secs(10),
    }
}

fn unique_dir(tag: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    let seq = SEQ.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "bumbledb-f3-gate-{tag}-{}-{seq}",
        std::process::id()
    ))
}

fn identity_of(descriptor: &bumbledb::SchemaDescriptor, seed: u8) -> DatabaseIdentity {
    DatabaseIdentity {
        database_id: DatabaseId::from_core(bumbledb::Id128::from_bytes([seed; 16])),
        incarnation_id: IncarnationId::from_core(bumbledb::Id128::from_bytes([seed ^ 0xff; 16])),
        schema_id: bumbledb_log::schema_file::schema_id(descriptor).expect("valid schema"),
    }
}

fn open_spec_for(
    descriptor: bumbledb::SchemaDescriptor,
    directory: &std::path::Path,
    create: bool,
    seed: u8,
) -> OpenSpec {
    let identity = identity_of(&descriptor, seed);
    let artifact = bumbledb_log::schema_file::render(&descriptor).into_bytes();
    OpenSpec {
        create,
        directory: directory.to_string_lossy().into_owned(),
        identity,
        backend: BackendSpec::Local,
        discard_mismatched: false,
        creation: create.then(|| {
            (
                OperationId::from_core(bumbledb::Id128::from_bytes([seed.wrapping_add(1); 16])),
                artifact,
            )
        }),
        descriptor,
        attrs: Vec::new(),
        tail_policy: bumbledb_log::manifest::TailPolicy::UNBOUNDED,
    }
}

fn drain_runtime(runtime: &Arc<Runtime>) -> CloseReport {
    let (tx, rx) = std::sync::mpsc::channel();
    runtime.drain(
        None,
        Box::new(move |report| {
            tx.send(report).unwrap();
        }),
    );
    rx.recv_timeout(Duration::from_secs(10))
        .expect("runtime drain")
}

fn drain_resource(resource: &Arc<HistoryResource>) -> CloseReport {
    let (tx, rx) = std::sync::mpsc::channel();
    resource.drain(Box::new(move |report| {
        tx.send(report).unwrap();
    }));
    rx.recv_timeout(Duration::from_secs(10))
        .expect("history drain")
}

fn validated(descriptor: &bumbledb::SchemaDescriptor) -> bumbledb::schema::Schema {
    use bumbledb::schema::ValidateDescriptor as _;
    descriptor.clone().validate().expect("valid schema")
}

fn seal_command(
    schema: &bumbledb::schema::Schema,
    identity: DatabaseIdentity,
    request: u8,
    rows: &[&[Value]],
    relation: bumbledb::RelationId,
    work: &WorkContext,
) -> Command {
    let mut draft = bumbledb::ChangeSet::builder(schema, work.clone());
    for row in rows {
        draft.insert(relation, row).expect("insert row");
    }
    let changes = draft.finish().expect("finish change set");
    Command::seal(
        CommandMetadata {
            identity,
            id: CommandId {
                receipt_epoch: ReceiptEpoch::new(1).expect("one"),
                request_id: RequestId::from_core(bumbledb::Id128::from_bytes([request; 16])),
            },
            condition: Condition::Unconditional,
        },
        changes,
        CommandResult::from_canonical_bytes(Vec::new().into_boxed_slice()),
        LIMITS,
        work,
    )
    .expect("seals")
}

/// Submit one sealed command through the ACTUAL verb lane
/// (`run_history_verb`, the native side of `logHistoryCall`).
fn submit_via_verb(
    resource: &Arc<HistoryResource>,
    command: Command,
    work: &WorkContext,
) -> SubmitOwned {
    let reference = command.command_ref();
    let verb = HistoryVerb::Submit {
        command: Arc::new(command),
        reference,
        options: SubmitOptions::DEFAULT,
    };
    match run_history_verb(resource, verb, work) {
        Ok(Output::Machine(MachineOutput::Submit(owned))) => owned,
        other => panic!(
            "submit verb must produce a submit output, got ok={}",
            other.is_ok()
        ),
    }
}

fn resolve_via_verb(
    resource: &Arc<HistoryResource>,
    reference: CommandRef,
    work: &WorkContext,
) -> ResolveOwned {
    match run_history_verb(resource, HistoryVerb::Resolve(reference), work) {
        Ok(Output::Machine(MachineOutput::Resolve(owned))) => owned,
        other => panic!(
            "resolve verb must produce a resolve output, got ok={}",
            other.is_ok()
        ),
    }
}

fn decided_stamp(owned: &SubmitOwned) -> DecisionStamp {
    match owned {
        SubmitOwned::Decided { receipt, .. } => receipt.decision_at,
        _ => panic!("expected a decided receipt"),
    }
}

fn at_least(
    resource: &Arc<HistoryResource>,
    stamp: DecisionStamp,
    work: &WorkContext,
) -> MachineResult<SnapshotOwned> {
    let (kind, lease) = resource.kind_and_lease().expect("live history");
    let outcome = open_published_snapshot(
        resource,
        &kind,
        lease,
        ConsistencySpec::AtLeast(stamp),
        work,
    );
    if let Ok(snapshot) = &outcome {
        begin_snapshot_teardown(&snapshot.session);
    }
    outcome
}

fn protocol_code(fail: &LogFail) -> &'static str {
    match fail {
        LogFail::Protocol { code, .. } => code,
        other => panic!("expected a protocol refusal, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Finding E — exact AtLeast ancestry through the bridge snapshot lane.
// ---------------------------------------------------------------------------

#[test]
fn at_least_proves_exact_ancestry_never_a_sequence_floor() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("e-ancestry");
    std::fs::create_dir_all(&base).unwrap();
    let dir = base.join("tenant");
    let work = policy().start().unwrap();

    let spec = open_spec_for(GateMini.descriptor(), &dir, true, 41);
    let opened = open_history(&runtime, &spec, &work).expect("creates");
    let identity = opened.resource.identity;
    let schema = validated(&spec.descriptor);
    let relation = bumbledb::RelationId(0);

    // Three real committed decisions: seq 1, 2, 3.
    let mut stamps = Vec::new();
    for request in 1u8..=3 {
        let command = seal_command(
            &schema,
            identity,
            request,
            &[&[Value::U64(u64::from(request)), Value::U64(0)]],
            relation,
            &work,
        );
        let owned = submit_via_verb(&opened.resource, command, &work);
        stamps.push(decided_stamp(&owned));
    }
    assert_eq!(stamps[0].seq, 1);
    assert_eq!(stamps[2].seq, 3);

    // A valid RETAINED ancestor accepts, with at-least freshness provenance.
    let snapshot = at_least(&opened.resource, stamps[0], &work).expect("retained ancestor");
    assert!(matches!(
        snapshot.freshness,
        FreshnessOwned::AtLeast { requested } if requested == stamps[0]
    ));
    assert_eq!(snapshot.decision, stamps[2], "the served frame is the tip");

    // THE DEFECT-E REPRO: an OLDER sequence with a wrong hash used to be
    // accepted as a silent sequence floor. It must refuse WrongLineage.
    let forged_old = DecisionStamp {
        seq: 1,
        hash: DecisionDigest::from_bytes([0xEE; 32]),
    };
    assert_ne!(forged_old.hash, stamps[0].hash, "the forgery is real");
    match at_least(&opened.resource, forged_old, &work) {
        Err(fail) => assert_eq!(protocol_code(&fail), "WrongLineage"),
        Ok(_) => panic!("an older wrong-hash stamp must NEVER be accepted"),
    }

    // The same sequence as the tip with a wrong hash refuses.
    let forged_tip = DecisionStamp {
        seq: stamps[2].seq,
        hash: DecisionDigest::from_bytes([0xDD; 32]),
    };
    match at_least(&opened.resource, forged_tip, &work) {
        Err(fail) => assert_eq!(protocol_code(&fail), "WrongLineage"),
        Ok(_) => panic!("a same-seq wrong-hash stamp must refuse"),
    }

    // A foreign database/incarnation's stamp (its genesis) refuses: the
    // activation evidence names THIS lineage's genesis, nothing else.
    let genesis = genesis_stamp(
        &GenesisRecord {
            identity,
            initial_application_digest: blank_initial_digests().0,
            initial_system_digest: blank_initial_digests().1,
            provenance: GenesisProvenance::Create,
        },
        LIMITS.envelope_bytes,
    )
    .expect("genesis stamp");
    let accepted = at_least(&opened.resource, genesis, &work).expect("own genesis is an ancestor");
    assert!(matches!(
        accepted.freshness,
        FreshnessOwned::AtLeast { requested } if requested == genesis
    ));
    let mut foreign_identity = identity;
    foreign_identity.database_id = DatabaseId::from_core(bumbledb::Id128::from_bytes([0x77; 16]));
    let foreign_genesis = genesis_stamp(
        &GenesisRecord {
            identity: foreign_identity,
            initial_application_digest: blank_initial_digests().0,
            initial_system_digest: blank_initial_digests().1,
            provenance: GenesisProvenance::Create,
        },
        LIMITS.envelope_bytes,
    )
    .expect("foreign genesis stamp");
    match at_least(&opened.resource, foreign_genesis, &work) {
        Err(fail) => assert_eq!(protocol_code(&fail), "WrongLineage"),
        Ok(_) => panic!("a foreign database's genesis stamp must refuse"),
    }

    // A future stamp stays the structured NotYetAvailable — never silently
    // downgraded to whatever is materialized.
    let future = DecisionStamp {
        seq: 99,
        hash: DecisionDigest::from_bytes([9; 32]),
    };
    match at_least(&opened.resource, future, &work) {
        Err(LogFail::Structured(StructuredReason::NotYetAvailable {
            requested_seq,
            captured_seq,
            ..
        })) => {
            assert_eq!(requested_seq, 99);
            assert_eq!(captured_seq, 3);
        }
        other => panic!(
            "a future stamp must refuse structured NotYetAvailable, got ok={}",
            other.is_ok()
        ),
    }

    assert_eq!(drain_resource(&opened.resource), CloseReport::Closed);
    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn at_least_with_pruned_evidence_is_witness_unavailable_never_a_claim() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("e-pruned");
    std::fs::create_dir_all(&base).unwrap();
    let dir = base.join("tenant");
    let work = policy().start().unwrap();

    let spec = open_spec_for(GateMini.descriptor(), &dir, true, 43);
    let opened = open_history(&runtime, &spec, &work).expect("creates");
    let identity = opened.resource.identity;
    let schema = validated(&spec.descriptor);
    let relation = bumbledb::RelationId(0);

    let mut stamps = Vec::new();
    for request in 1u8..=2 {
        let command = seal_command(
            &schema,
            identity,
            request,
            &[&[Value::U64(u64::from(request)), Value::U64(0)]],
            relation,
            &work,
        );
        stamps.push(decided_stamp(&submit_via_verb(
            &opened.resource,
            command,
            &work,
        )));
    }
    let valid_old = stamps[0];
    // Retained: the valid old stamp accepts.
    assert!(at_least(&opened.resource, valid_old, &work).is_ok());

    // Rotate the receipt epoch forward and retire epoch 1: exactly the
    // retained receipt rows below the frontier are deleted in ONE
    // transaction (chapter 20's explicit retention policy).
    {
        let (_, lease) = opened.resource.kind_and_lease().expect("live");
        let db = lease.db();
        bumbledb_log::admin::rotate_receipts_local(
            db,
            ReceiptEpoch::new(2).expect("two"),
            LIMITS.envelope_bytes,
            &work,
        )
        .expect("rotates");
        let removed =
            bumbledb_log::admin::retire_receipts_local(db, 1, LIMITS.envelope_bytes, &work)
                .expect("retires");
        assert!(removed >= 2, "the epoch-1 receipt rows are gone");
    }

    // The SAME valid historical stamp is now unwitnessable: explicitly
    // WitnessUnavailable — not accepted (that would be a claimed validation
    // over pruned evidence) and not corruption.
    match at_least(&opened.resource, valid_old, &work) {
        Err(fail) => assert_eq!(protocol_code(&fail), "WitnessUnavailable"),
        Ok(_) => panic!("pruned evidence must never validate a historical stamp"),
    }

    // Genesis evidence (the activation marker) is retained forever: a
    // sequence-zero request still resolves after retirement.
    let genesis = genesis_stamp(
        &GenesisRecord {
            identity,
            initial_application_digest: blank_initial_digests().0,
            initial_system_digest: blank_initial_digests().1,
            provenance: GenesisProvenance::Create,
        },
        LIMITS.envelope_bytes,
    )
    .expect("genesis stamp");
    assert!(at_least(&opened.resource, genesis, &work).is_ok());

    assert_eq!(drain_resource(&opened.resource), CloseReport::Closed);
    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

// ---------------------------------------------------------------------------
// Finding F — real rejection evidence through the receipt lane.
// ---------------------------------------------------------------------------

#[test]
fn rejected_submissions_expose_the_complete_decoded_violation_set() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("f-evidence");
    std::fs::create_dir_all(&base).unwrap();
    let dir = base.join("tenant");
    let work = policy().start().unwrap();

    let spec = open_spec_for(GateMini.descriptor(), &dir, true, 47);
    let opened = open_history(&runtime, &spec, &work).expect("creates");
    let identity = opened.resource.identity;
    let schema = validated(&spec.descriptor);
    let relation = bumbledb::RelationId(0);

    // Two rows with the same key `a` violate `Item(a) -> Item`.
    let command = seal_command(
        &schema,
        identity,
        9,
        &[
            &[Value::U64(1), Value::U64(10)],
            &[Value::U64(1), Value::U64(20)],
        ],
        relation,
        &work,
    );
    let reference = command.command_ref();
    let owned = submit_via_verb(&opened.resource, command, &work);
    let SubmitOwned::Decided {
        receipt,
        violations,
        ..
    } = owned
    else {
        panic!("a violating command still DECIDES (durable rejection)")
    };
    assert!(matches!(
        receipt.outcome,
        TerminalOutcome::InvariantRejected { .. }
    ));
    let decoded = violations.expect("a rejected receipt carries decoded violations — NEVER empty");
    assert_eq!(decoded.rows.len(), 1, "one violated statement");
    assert_eq!(decoded.rows[0].statement, 0);
    assert!(matches!(
        decoded.rows[0].kind,
        bumbledb::StatementKind::Functionality
    ));
    assert!(
        !decoded.rows[0].canonical.is_empty(),
        "the canonical spelling is preserved"
    );
    assert!(
        !decoded.rows[0].facts.is_empty(),
        "bounded example facts are preserved"
    );
    assert_eq!(decoded.truncated.len(), 1);
    assert!(
        !decoded.truncated[0],
        "two examples fit the judge budget; no truncation label"
    );

    // Resolve through the SAME verb lane preserves the full violation set.
    let resolved = resolve_via_verb(&opened.resource, reference, &work);
    match (&resolved.outcome, &resolved.violations) {
        (ResolveOutcome::Found(found), Some(decoded_again)) => {
            assert!(matches!(
                found.outcome,
                TerminalOutcome::InvariantRejected { .. }
            ));
            assert_eq!(decoded_again.rows.len(), 1);
            assert_eq!(decoded_again.rows[0].statement, 0);
            assert_eq!(decoded_again.rows[0].canonical, decoded.rows[0].canonical);
        }
        _ => panic!("resolve must find the rejected receipt WITH violations"),
    }

    // Resolve AFTER REOPEN (a fresh process's view) still decodes the
    // retained evidence — the durable bytes are the one source of truth.
    assert_eq!(drain_resource(&opened.resource), CloseReport::Closed);
    let reopened = open_history(
        &runtime,
        &open_spec_for(GateMini.descriptor(), &dir, false, 47),
        &work,
    )
    .expect("reopens");
    let resolved = resolve_via_verb(&reopened.resource, reference, &work);
    match (&resolved.outcome, &resolved.violations) {
        (ResolveOutcome::Found(_), Some(decoded_again)) => {
            assert_eq!(decoded_again.rows.len(), 1);
            assert_eq!(decoded_again.rows[0].canonical, decoded.rows[0].canonical);
            assert!(!decoded_again.rows[0].facts.is_empty());
        }
        _ => panic!("resolve-after-reopen must preserve violations"),
    }

    assert_eq!(drain_resource(&reopened.resource), CloseReport::Closed);
    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn d13_resolve_keeps_found_when_diagnostic_budget_fails() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("d13-resolve");
    std::fs::create_dir_all(&base).unwrap();
    let dir = base.join("tenant");
    let work = policy().start().unwrap();
    let spec = open_spec_for(GateMini.descriptor(), &dir, true, 73);
    let opened = open_history(&runtime, &spec, &work).expect("creates");
    let schema = validated(&spec.descriptor);
    let command = seal_command(
        &schema,
        opened.resource.identity,
        73,
        &[
            &[Value::U64(1), Value::U64(10)],
            &[Value::U64(1), Value::U64(20)],
        ],
        bumbledb::RelationId(0),
        &work,
    );
    let reference = command.command_ref();
    match submit_via_verb(&opened.resource, command, &work) {
        SubmitOwned::Decided { receipt, .. } => {
            assert!(matches!(
                receipt.outcome,
                TerminalOutcome::InvariantRejected { .. }
            ));
        }
        other => panic!("violating submit still decides, got {other:?}"),
    }
    let starved = ExecutionPolicy {
        input_bytes: 32,
        working_bytes: 32,
        scratch_bytes: 32,
        result_bytes: 32,
        rows: 1,
        work_units: 1,
        timeout: Duration::from_millis(1),
    }
    .start()
    .expect("starved work");
    match run_history_verb(
        &opened.resource,
        HistoryVerb::Resolve(reference),
        &starved,
    ) {
        Ok(Output::Machine(MachineOutput::Resolve(owned))) => match owned.outcome {
            ResolveOutcome::Found(_) => {
                let _ = owned.violations;
            }
            other => panic!("starved diagnostics must not drop Found, got {other:?}"),
        },
        Ok(Output::Machine(MachineOutput::Admin(_))) => {
            panic!("diagnostic failure must not rewrite a found receipt into admin fail");
        }
        other => panic!("resolve must stay a resolve output, got ok={}", other.is_ok()),
    }

    assert_eq!(drain_resource(&opened.resource), CloseReport::Closed);
    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn multiple_statements_and_truncation_labels_survive_the_decode() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("f-multi");
    std::fs::create_dir_all(&base).unwrap();
    let dir = base.join("tenant");
    let work = policy().start().unwrap();

    let spec = open_spec_for(GatePair.descriptor(), &dir, true, 53);
    let opened = open_history(&runtime, &spec, &work).expect("creates");
    let identity = opened.resource.identity;
    let schema = validated(&spec.descriptor);
    let relation = bumbledb::RelationId(0);

    // Eight rows sharing BOTH keys violate BOTH statements, with more
    // offending facts than the judge's bounded example budget (4): the
    // truncation label must survive to the decoded rows.
    let rows: Vec<[Value; 3]> = (0..8u64)
        .map(|index| [Value::U64(1), Value::U64(2), Value::U64(index)])
        .collect();
    let row_refs: Vec<&[Value]> = rows.iter().map(<[Value; 3]>::as_slice).collect();
    let command = seal_command(&schema, identity, 11, &row_refs, relation, &work);
    let owned = submit_via_verb(&opened.resource, command, &work);
    let violations = match owned {
        SubmitOwned::Decided { violations, .. } => {
            violations.expect("rejected receipt carries decoded violations")
        }
        _ => panic!("expected a decided rejection"),
    };
    assert_eq!(
        violations.rows.len(),
        2,
        "the COMPLETE violated-statement set (both FDs), never truncated"
    );
    assert_eq!(violations.rows[0].statement, 0);
    assert_eq!(violations.rows[1].statement, 1);
    for (row, truncated) in violations.rows.iter().zip(&violations.truncated) {
        assert!(
            row.facts.len() <= 5,
            "examples stay bounded by the judge budget"
        );
        assert!(
            *truncated,
            "more offending facts exist than the bounded examples; the label says so"
        );
    }

    assert_eq!(drain_resource(&opened.resource), CloseReport::Closed);
    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn malformed_evidence_bytes_refuse_typed_never_an_empty_rejection() {
    let work = policy().start().unwrap();
    let descriptor = GateMini.descriptor();
    let schema = validated(&descriptor);

    // Garbage bytes: strict decode refuses with the typed Corruption code.
    match decode_rejection_violations(&descriptor, &schema, b"not-evidence", &work) {
        Err(LogFail::Protocol { code, detail }) => {
            assert_eq!(code, "Corruption");
            assert!(detail.contains("malformed rejection evidence"));
        }
        other => panic!(
            "garbage evidence must refuse Corruption, got ok={}",
            other.is_ok()
        ),
    }

    // Grammatical evidence FOREIGN to this schema (a statement id the theory
    // does not have) also refuses typed — never rendered as valid-but-empty.
    let pair = GatePair.descriptor();
    let pair_schema = validated(&pair);
    // Encode real evidence against GatePair citing statement 1, then decode
    // it against GateMini (which has only statement 0).
    let dir = unique_dir("f-foreign");
    std::fs::create_dir_all(&dir).unwrap();
    let db = bumbledb::Db::create(&dir.join("db"), pair.clone())
        .expect("create")
        .expect("admits");
    let mut session = db.integration_writer(&work).expect("writer");
    let mut violating = bumbledb::ChangeSet::builder(&pair_schema, work.clone());
    violating
        .insert(
            bumbledb::RelationId(0),
            &[Value::U64(1), Value::U64(1), Value::U64(1)],
        )
        .unwrap();
    violating
        .insert(
            bumbledb::RelationId(0),
            &[Value::U64(1), Value::U64(1), Value::U64(2)],
        )
        .unwrap();
    let violating = violating.finish().unwrap();
    let rejected = match session.prepare(&violating).expect("prepare") {
        bumbledb::Admission::Rejected(violations) => violations,
        bumbledb::Admission::Accepted(_) => panic!("the double-key rows must reject"),
    };
    let evidence = bumbledb::schema::evidence::encode_violations(
        &pair_schema,
        &rejected,
        LIMITS.evidence_bytes,
        &work,
    )
    .expect("encodes");
    drop(session);
    match decode_rejection_violations(&descriptor, &schema, &evidence, &work) {
        Err(LogFail::Protocol { code, detail }) => {
            assert_eq!(code, "Corruption");
            assert!(detail.contains("does not belong to this schema"));
        }
        other => panic!(
            "foreign-schema evidence must refuse Corruption, got ok={}",
            other.is_ok()
        ),
    }
    let _ = std::fs::remove_dir_all(&dir);
}

// A REFUSED open must synchronously release its installed directory owner:
// before the F3 repair, the foreign-identity refusal returned while the
// registry entry was still draining, so the immediately following
// wrong-incarnation open misreported DirectoryBusy instead of WrongLineage.
#[test]
fn refused_opens_release_the_directory_synchronously() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("probe-lineage");
    std::fs::create_dir_all(&base).unwrap();
    let dir = base.join("tenant");
    let work = policy().start().unwrap();
    let created = open_history(
        &runtime,
        &open_spec_for(GateMini.descriptor(), &dir, true, 3),
        &work,
    )
    .expect("creates");
    assert_eq!(drain_resource(&created.resource), CloseReport::Closed);
    let mut foreign = open_spec_for(GateMini.descriptor(), &dir, false, 3);
    foreign.identity.database_id = DatabaseId::from_core(bumbledb::Id128::from_bytes([9; 16]));
    match open_history(&runtime, &foreign, &work) {
        Err(LogFail::Protocol { code, .. }) => assert_eq!(code, "ForeignIdentity"),
        Err(other) => panic!("foreign got other error: {other:?}"),
        Ok(_) => panic!("foreign accepted"),
    }
    let mut lineage = open_spec_for(GateMini.descriptor(), &dir, false, 3);
    lineage.identity.incarnation_id =
        IncarnationId::from_core(bumbledb::Id128::from_bytes([8; 16]));
    match open_history(&runtime, &lineage, &work) {
        Err(LogFail::Protocol { code, .. }) => assert_eq!(code, "WrongLineage"),
        Err(LogFail::Core(core)) => {
            panic!("the refusal must be typed WrongLineage, got core {core:?}")
        }
        Err(other) => panic!("the refusal must be typed WrongLineage, got {other:?}"),
        Ok(_) => panic!("a different incarnation must refuse"),
    }
    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

// ---------------------------------------------------------------------------
// Publication phase — D certainty contracts on the submit verb lane.
// ---------------------------------------------------------------------------

#[test]
fn submit_owned_carries_publication_phase() {
    use bumbledb_log::certainty::PublicationPhase;

    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("submit-phase");
    std::fs::create_dir_all(&base).unwrap();
    let dir = base.join("tenant");
    let work = policy().start().unwrap();

    let spec = open_spec_for(GateMini.descriptor(), &dir, true, 51);
    let opened = open_history(&runtime, &spec, &work).expect("creates");
    let schema = validated(&spec.descriptor);
    let command = seal_command(
        &schema,
        opened.resource.identity,
        1,
        &[&[Value::U64(1), Value::U64(2)]],
        bumbledb::RelationId(0),
        &work,
    );
    let owned = submit_via_verb(&opened.resource, command, &work);
    match owned {
        SubmitOwned::Decided { phase, .. } => {
            assert_eq!(phase, PublicationPhase::Confirmed);
        }
        other => panic!("expected a decided submit with confirmed phase, got {other:?}"),
    }

    assert_eq!(drain_resource(&opened.resource), CloseReport::Closed);
    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}
