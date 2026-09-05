//! Engine-backed log-machine bridge tests (C10 `LogNative` / RUN / FFI).
//! Authored in F1, NEVER run here; F3 executes them. They drive the real
//! machine below the N-API layer: real runtime registry, real kernel
//! fences, real LMDB materializations, real `LocalHistory` — no scripted
//! double exists on this side of the wire.

use std::time::Duration;

use bumbledb::Theory as _;
use bumbledb::work::ExecutionPolicy;

use super::*;
use crate::runtime::{CloseReport, Options};

bumbledb::schema! {
    pub Mini;
    relation Item { a: u64, b: u64 }
    Item(a) -> Item;
}

bumbledb::schema! {
    pub Other;
    relation Row { x: u64 }
    Row(x) -> Row;
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
        "bumbledb-p06-log-{tag}-{}-{seq}",
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

fn open_spec(directory: &std::path::Path, create: bool, seed: u8) -> OpenSpec {
    let descriptor = Mini.descriptor();
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

#[test]
fn the_protocol_roster_pins_ts_log_codes_exactly() {
    // logErrorCodes() must equal ts-log/src/codes.ts in ORDER and spelling:
    // the TS roster test compares the arrays exactly, so this Rust twin
    // reads the committed TS source and extracts its string literals.
    let committed = include_str!("../../../../ts-log/src/codes.ts");
    let mut spelled = Vec::new();
    for line in committed.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix('"')
            && let Some(code) = rest.strip_suffix("\",").or_else(|| rest.strip_suffix('"'))
        {
            spelled.push(code.to_string());
        }
    }
    assert_eq!(
        spelled,
        PROTOCOL_CODES
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        "ts-log/src/codes.ts and the native speller drifted"
    );
    // Wave-D roster pin: exactly 33 rows, `MaterializationStale` directly
    // after `MaintenanceRequired` (the two rows landed together in P04R's
    // identities.rs; the TS count/adjacency test pins the same order).
    assert_eq!(PROTOCOL_CODES.len(), 33, "the wave-D roster is 33 rows");
    let maintenance = PROTOCOL_CODES
        .iter()
        .position(|code| *code == "MaintenanceRequired")
        .expect("MaintenanceRequired is rostered");
    assert_eq!(
        PROTOCOL_CODES[maintenance + 1],
        "MaterializationStale",
        "the maintenance/retention rows are adjacent"
    );
}

#[test]
fn the_result_codec_is_the_core_authority_and_id128_crosses_as_hex() {
    let work = policy().start().unwrap();
    let entries = vec![
        ("beta".to_string(), Value::U64(7)),
        ("alpha".to_string(), Value::String("hi".into())),
        ("gamma".to_string(), Value::Bool(true)),
    ];
    let bytes = encode_result_record(&entries, &work).expect("encodes");
    // ONE codec: the bridge bytes ARE the core canonical bytes. The command
    // digest covers them, so the deleted local twin (u64 count, no kind
    // byte, tag 8 refused) was a real C12 defect — this pin keeps the
    // family from ever splitting again.
    let borrowed: Vec<(&str, &Value)> = entries
        .iter()
        .map(|(name, value)| (name.as_str(), value))
        .collect();
    let core = bumbledb::canonical::result::encode_result(&borrowed, LIMITS.result_bytes, &work)
        .expect("the core codec encodes");
    assert_eq!(bytes, core, "the bridge and the core spell ONE byte layout");
    // JS object key order can never change the canonical record.
    let reversed: Vec<_> = entries.iter().cloned().rev().collect();
    assert_eq!(
        bytes,
        encode_result_record(&reversed, &work).expect("encodes")
    );
    let decoded = decode_result_record(&bytes, &work).expect("decodes");
    assert_eq!(decoded.len(), 3);
    assert_eq!(
        decoded[0].0.as_ref(),
        "alpha",
        "entries decode in canonical order"
    );
    // Tag 8 (Id128, Rust-sealed commands) decodes through the re-pointed
    // codec instead of refusing as Corruption; the JS crossing spells it as
    // canonical 32-lowercase-hex text (the recorded wave-D decision — the
    // only spelling the TS CommandScalar can carry).
    let id = bumbledb::Id128::from_bytes([0xAB; 16]);
    let cell = Value::Id128(id);
    let with_id = bumbledb::canonical::result::encode_result(
        &[("entity", &cell)],
        LIMITS.result_bytes,
        &work,
    )
    .expect("encodes");
    let decoded = decode_result_record(&with_id, &work).expect("tag 8 decodes, never Corruption");
    assert_eq!(decoded[0].1, Value::Id128(id));
    let text = hex16(id);
    assert_eq!(text.len(), 32);
    assert_eq!(text, text.to_lowercase(), "canonical lowercase hex");
    // Duplicate keys refuse; truncation refuses; empty is the empty record.
    let duplicate = vec![
        ("k".to_string(), Value::U64(1)),
        ("k".to_string(), Value::U64(2)),
    ];
    assert!(encode_result_record(&duplicate, &work).is_err());
    assert!(decode_result_record(&bytes[..bytes.len() - 1], &work).is_err());
    assert!(decode_result_record(&[], &work).expect("empty").is_empty());
}

#[test]
fn structured_protocol_reasons_carry_their_declared_fields() {
    // `fail_of_log` is exhaustive over P04R's LogError (compile-checked);
    // the two wave-D arms map to the STRUCTURED frame payloads
    // ts-log/src/errors.ts declares.
    match fail_of_log(LogError::MaintenanceRequired {
        count: 7,
        bytes: 4096,
    }) {
        LogFail::Structured(StructuredReason::MaintenanceRequired { count, bytes, .. }) => {
            assert_eq!((count, bytes), (7, 4096), "the C08 payload crosses intact");
        }
        other => panic!("MaintenanceRequired must be structured, got {other:?}"),
    }
    match fail_of_log(LogError::MaterializationStale) {
        LogFail::Protocol { code, detail } => {
            assert_eq!(code, "MaterializationStale");
            assert!(
                detail.contains("reopen"),
                "the detail IS the hydration routing (reopen/recovery guidance)"
            );
        }
        other => panic!("MaterializationStale must carry its detail, got {other:?}"),
    }
}

#[test]
fn local_create_open_and_identity_refusals() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("create-open");
    std::fs::create_dir_all(&base).unwrap();
    let dir = base.join("tenant");
    let work = policy().start().unwrap();

    // Open before create: DatabaseMissing, never an empty replacement.
    match open_history(&runtime, &open_spec(&dir, false, 3), &work) {
        Err(LogFail::Protocol { code, .. }) => assert_eq!(code, "DatabaseMissing"),
        other => panic!(
            "open of a missing tenant must refuse, got {other:?}",
            other = other.is_ok()
        ),
    }

    // Create with the checked canonical artifact.
    let created = open_history(&runtime, &open_spec(&dir, true, 3), &work).expect("creates");
    let identity = created.resource.identity;
    assert_eq!(drain_resource(&created.resource), CloseReport::Closed);

    // Reopen sees the same identity.
    let reopened = open_history(&runtime, &open_spec(&dir, false, 3), &work).expect("reopens");
    assert_eq!(reopened.resource.identity, identity);
    assert_eq!(drain_resource(&reopened.resource), CloseReport::Closed);

    // A different databaseId is ForeignIdentity; a different incarnation is
    // WrongLineage — both refuse before serving any data.
    let mut foreign = open_spec(&dir, false, 3);
    foreign.identity.database_id = DatabaseId::from_core(bumbledb::Id128::from_bytes([9; 16]));
    match open_history(&runtime, &foreign, &work) {
        Err(LogFail::Protocol { code, .. }) => assert_eq!(code, "ForeignIdentity"),
        _ => panic!("a foreign database id must refuse"),
    }
    let mut lineage = open_spec(&dir, false, 3);
    lineage.identity.incarnation_id =
        IncarnationId::from_core(bumbledb::Id128::from_bytes([8; 16]));
    match open_history(&runtime, &lineage, &work) {
        Err(LogFail::Protocol { code, .. }) => assert_eq!(code, "WrongLineage"),
        _ => panic!("a different incarnation must refuse"),
    }

    // A wrong-schema open refuses through the engine's own fingerprint
    // check (a core failure, never a silent adoption).
    let mut wrong = open_spec(&dir, false, 3);
    wrong.descriptor = Other.descriptor();
    wrong.identity.schema_id =
        bumbledb_log::schema_file::schema_id(&wrong.descriptor).expect("valid");
    assert!(open_history(&runtime, &wrong, &work).is_err());

    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn create_retry_completes_and_strangers_refuse() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("create-retry");
    std::fs::create_dir_all(&base).unwrap();
    let dir = base.join("tenant");
    let work = policy().start().unwrap();

    let created = open_history(&runtime, &open_spec(&dir, true, 4), &work).expect("creates");
    assert_eq!(drain_resource(&created.resource), CloseReport::Closed);

    // A retry of the SAME creation (lost response) validates the stable
    // identity and completes instead of adopting a stranger.
    let retried = open_history(&runtime, &open_spec(&dir, true, 4), &work)
        .expect("a matching creation retry completes");
    assert_eq!(drain_resource(&retried.resource), CloseReport::Closed);

    // A DIFFERENT identity creating over the existing authority refuses.
    let stranger = open_spec(&dir, true, 5);
    match open_history(&runtime, &stranger, &work) {
        Err(LogFail::Protocol { code, .. }) => {
            assert!(
                code == "AuthorityExists" || code == "ForeignIdentity",
                "creation over existing authority refuses, got {code}"
            );
        }
        _ => panic!("creation over an unrelated database must refuse"),
    }

    // A creation without the checked artifact refuses before any write.
    let missing = base.join("second");
    let mut artifactless = open_spec(&missing, true, 6);
    artifactless.creation = Some((
        OperationId::from_core(bumbledb::Id128::from_bytes([6; 16])),
        Vec::new(),
    ));
    match open_history(&runtime, &artifactless, &work) {
        Err(LogFail::Protocol { code, .. }) => assert_eq!(code, "UnsupportedArtifact"),
        _ => panic!("an empty creation artifact must refuse"),
    }

    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn submit_decides_and_identity_mismatch_is_not_submitted() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("submit");
    std::fs::create_dir_all(&base).unwrap();
    let dir = base.join("tenant");
    let work = policy().start().unwrap();

    let opened = open_history(&runtime, &open_spec(&dir, true, 7), &work).expect("creates");
    let (kind, lease) = opened.resource.kind_and_lease().expect("live history");
    drop(lease);

    // Seal one empty command against the exact identity: LocalHistory
    // decides all four terminal outcomes in one LMDB transaction; an empty
    // change set decides NoChange.
    let descriptor = Mini.descriptor();
    let schema = {
        use bumbledb::schema::ValidateDescriptor as _;
        descriptor.validate().expect("valid schema")
    };
    let changes = bumbledb::ChangeSet::builder(&schema, work.clone())
        .finish()
        .expect("empty change set");
    let metadata = CommandMetadata {
        identity: opened.resource.identity,
        id: CommandId {
            receipt_epoch: bumbledb_log::history::ReceiptEpoch::new(1).expect("one"),
            request_id: RequestId::from_core(bumbledb::Id128::from_bytes([21; 16])),
        },
        condition: Condition::Unconditional,
    };
    let command = Command::seal(
        metadata,
        changes.clone(),
        CommandResult::from_canonical_bytes(Vec::new().into_boxed_slice()),
        LIMITS,
        &work,
    )
    .expect("seals");
    match kind.submit_with(&command, SubmitOptions::DEFAULT, &work) {
        SubmitOutcome::Decided { receipt, .. } => {
            assert_eq!(receipt.command.identity, opened.resource.identity);
        }
        other => panic!("an in-scope command decides, got {other:?}"),
    }

    // A command sealed for a FOREIGN identity is not-submitted with a
    // typed identity refusal — never a forged receipt.
    let mut foreign_scope = opened.resource.identity;
    foreign_scope.database_id = DatabaseId::from_core(bumbledb::Id128::from_bytes([99; 16]));
    let foreign = Command::seal(
        CommandMetadata {
            identity: foreign_scope,
            id: CommandId {
                receipt_epoch: bumbledb_log::history::ReceiptEpoch::new(1).expect("one"),
                request_id: RequestId::from_core(bumbledb::Id128::from_bytes([22; 16])),
            },
            condition: Condition::Unconditional,
        },
        changes,
        CommandResult::from_canonical_bytes(Vec::new().into_boxed_slice()),
        LIMITS,
        &work,
    )
    .expect("seals");
    match kind.submit_with(&foreign, SubmitOptions::DEFAULT, &work) {
        SubmitOutcome::NotSubmitted { error, .. } => {
            assert!(matches!(
                fail_of_log(error),
                LogFail::Protocol {
                    code: "ForeignIdentity",
                    ..
                }
            ));
        }
        other => panic!("a foreign-scope command is not-submitted, got {other:?}"),
    }

    assert_eq!(drain_resource(&opened.resource), CloseReport::Closed);
    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn published_snapshots_pin_provenance_and_consistency_refusals_are_typed() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("snapshot");
    std::fs::create_dir_all(&base).unwrap();
    let dir = base.join("tenant");
    let work = policy().start().unwrap();

    let opened = open_history(&runtime, &open_spec(&dir, true, 11), &work).expect("creates");
    let (kind, lease) = opened.resource.kind_and_lease().expect("live");
    let snapshot = open_published_snapshot(
        &opened.resource,
        &kind,
        lease,
        ConsistencySpec::Latest,
        &work,
    )
    .expect("a local snapshot is latest by construction");
    assert_eq!(snapshot.identity, opened.resource.identity);
    assert!(matches!(snapshot.freshness, FreshnessOwned::Latest));
    // The pinned session drains honestly.
    let (tx, rx) = std::sync::mpsc::channel();
    snapshot.session.drain(Box::new(move |report| {
        tx.send(report).unwrap();
    }));
    assert_eq!(
        rx.recv_timeout(Duration::from_secs(10)).expect("drain"),
        CloseReport::Closed
    );

    // An at-least consistency ahead of the local stamp refuses typed —
    // never a stale read dressed as fresh.
    let (kind, lease) = opened.resource.kind_and_lease().expect("live");
    let ahead = ConsistencySpec::AtLeast(bumbledb_log::history::DecisionStamp {
        seq: 999,
        hash: bumbledb_log::history::DecisionDigest::from_bytes([1; 32]),
    });
    match open_published_snapshot(&opened.resource, &kind, lease, ahead, &work) {
        Err(LogFail::Structured(StructuredReason::NotYetAvailable {
            requested_seq,
            captured_seq,
            ..
        })) => {
            // The structured frame carries the TS schema's declared fields
            // (requestedSeq/capturedSeq cross as BigInt on the wire).
            assert_eq!(requested_seq, 999);
            assert!(captured_seq < 999);
        }
        _ => panic!("an unreachable at-least stamp must refuse with structured NotYetAvailable"),
    }

    assert_eq!(drain_resource(&opened.resource), CloseReport::Closed);
    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn per_call_submit_options_cross_verbatim_and_local_accepts_and_ignores_them() {
    // The pure wire→machine mapping (P06.md defect 8, P04.md hub request 5):
    // absent fields are the machine defaults; present values cross VERBATIM —
    // the hosted machine clamps to its own attempt/backoff bounds
    // (`effective_attempts`, `MAX_BACKOFF`), and the bridge never re-clamps
    // beyond the wire's u32 type bounds.
    assert_eq!(submit_options_of(None, None, None), SubmitOptions::DEFAULT);
    assert_eq!(
        submit_options_of(Some(3), Some(10), Some(35)),
        SubmitOptions {
            attempts: Some(3),
            backoff_base: Some(Duration::from_millis(10)),
            backoff_cap: Some(Duration::from_millis(35)),
        }
    );
    // A hostile-wide wire request still crosses verbatim: clamping is the
    // machine's judgment (P04R2's in-module clamp tests own that half).
    assert_eq!(
        submit_options_of(Some(0xffff), Some(3_600_000), Some(u32::MAX)),
        SubmitOptions {
            attempts: Some(0xffff),
            backoff_base: Some(Duration::from_millis(3_600_000)),
            backoff_cap: Some(Duration::from_millis(u64::from(u32::MAX))),
        }
    );

    // LocalHistory accepts and ignores the options (the recorded P04R2
    // contract: one LMDB transaction, no CAS loop to bound) — a narrowed
    // per-call budget still decides.
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("submit-options");
    std::fs::create_dir_all(&base).unwrap();
    let dir = base.join("tenant");
    let work = policy().start().unwrap();

    let opened = open_history(&runtime, &open_spec(&dir, true, 17), &work).expect("creates");
    let (kind, lease) = opened.resource.kind_and_lease().expect("live history");
    drop(lease);

    let descriptor = Mini.descriptor();
    let schema = {
        use bumbledb::schema::ValidateDescriptor as _;
        descriptor.validate().expect("valid schema")
    };
    let changes = bumbledb::ChangeSet::builder(&schema, work.clone())
        .finish()
        .expect("empty change set");
    let command = Command::seal(
        CommandMetadata {
            identity: opened.resource.identity,
            id: CommandId {
                receipt_epoch: bumbledb_log::history::ReceiptEpoch::new(1).expect("one"),
                request_id: RequestId::from_core(bumbledb::Id128::from_bytes([31; 16])),
            },
            condition: Condition::Unconditional,
        },
        changes,
        CommandResult::from_canonical_bytes(Vec::new().into_boxed_slice()),
        LIMITS,
        &work,
    )
    .expect("seals");
    let narrowed = submit_options_of(Some(1), Some(1), Some(1));
    match kind.submit_with(&command, narrowed, &work) {
        SubmitOutcome::Decided { receipt, .. } => {
            assert_eq!(receipt.command.identity, opened.resource.identity);
        }
        other => panic!("local submit ignores per-call options and decides, got {other:?}"),
    }

    assert_eq!(drain_resource(&opened.resource), CloseReport::Closed);
    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn latest_reads_judge_one_catch_up_then_refuse_typed() {
    // The hosted `consistency: latest` lane's post-catch-up judgment (P06.md
    // defect 4): after the ONE `HostedHistory::catch_up` attempt and the ONE
    // frame retake, a stamp at (or past) the reached tip is latest; a stamp
    // still behind it refuses the structured `NotYetAvailable` with the
    // fresh numbers — never a second attempt or a hidden repair loop.
    let stamp = |seq: u64, fill: u8| bumbledb_log::history::DecisionStamp {
        seq,
        hash: bumbledb_log::history::DecisionDigest::from_bytes([fill; 32]),
    };

    assert!(latest_reached(stamp(7, 1), stamp(7, 1)).is_ok());
    // A retaken frame AHEAD of the reached tip (a racing local commit that
    // itself followed a head advance) is fresher, never refused.
    assert!(latest_reached(stamp(7, 1), stamp(9, 2)).is_ok());

    match latest_reached(stamp(7, 1), stamp(4, 3)) {
        Err(LogFail::Structured(StructuredReason::NotYetAvailable {
            requested_seq,
            captured_seq,
            ..
        })) => {
            assert_eq!(requested_seq, 7, "the reached tip is what was requested");
            assert_eq!(
                captured_seq, 4,
                "the retaken local stamp is what was captured"
            );
        }
        other => {
            panic!("a still-stale retake must refuse structured NotYetAvailable, got {other:?}")
        }
    }

    // A diverging hash at the caught-up height is the materialization
    // disagreeing with the chain it just applied: Corruption, never a forged
    // freshness claim.
    match latest_reached(stamp(7, 1), stamp(7, 9)) {
        Err(LogFail::Protocol { code, .. }) => assert_eq!(code, "Corruption"),
        other => panic!("a diverging caught-up hash must refuse Corruption, got {other:?}"),
    }

    // The stale-cache boundary stays typed through the same lane:
    // `catch_up`'s MaterializationStale maps to its rostered code (recovery
    // hydration is native-owned on the next open — the routing detail).
    match fail_of_log(LogError::MaterializationStale) {
        LogFail::Protocol { code, .. } => assert_eq!(code, "MaterializationStale"),
        other => panic!("MaterializationStale must keep its typed reason, got {other:?}"),
    }
}

#[test]
fn closed_history_refuses_and_close_joins_idempotently() {
    let runtime = Runtime::start(options()).unwrap();
    let base = unique_dir("close");
    std::fs::create_dir_all(&base).unwrap();
    let dir = base.join("tenant");
    let work = policy().start().unwrap();

    let opened = open_history(&runtime, &open_spec(&dir, true, 13), &work).expect("creates");
    assert_eq!(drain_resource(&opened.resource), CloseReport::Closed);
    // The spent capability refuses further machine access, typed.
    assert!(matches!(
        opened.resource.kind_and_lease(),
        Err(RuntimeError::ClosedHandle)
    ));
    // A second close joins the finished drain (double release is harmless).
    assert_eq!(drain_resource(&opened.resource), CloseReport::Closed);
    // The directory is reusable by a successor after teardown.
    let reopened = open_history(&runtime, &open_spec(&dir, false, 13), &work).expect("reopens");
    assert_eq!(drain_resource(&reopened.resource), CloseReport::Closed);

    assert_eq!(drain_runtime(&runtime), CloseReport::Closed);
    let _ = std::fs::remove_dir_all(&base);
}

#[test]
fn planned_target_incarnations_are_deterministic_and_operation_scoped() {
    let a = OperationId::from_core(bumbledb::Id128::from_bytes([1; 16]));
    let b = OperationId::from_core(bumbledb::Id128::from_bytes([2; 16]));
    assert_eq!(
        planned_target_incarnation(a),
        planned_target_incarnation(a),
        "a retry of the same operation resumes the same target"
    );
    assert_ne!(
        planned_target_incarnation(a),
        planned_target_incarnation(b),
        "distinct operations never share a planned target"
    );
}
