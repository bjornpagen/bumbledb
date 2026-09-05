//! P12 adversarial integration: hostile REMOTE bytes against the landed
//! replay boundary. A hosted materialization consumes decision objects an
//! attacker-controlled backend could substitute; `apply::materialize` is the
//! trust boundary that must refuse every forgery class without committing
//! anything (REP-018 typed object verification, G14 corruption refusal,
//! PROTO-15 recorded-outcome verification, ARCH-004 identity binding).
//!
//! No subsystem packet owns this file's angle: P04 tests its own framing
//! round-trips and P05 tests object-layer digests; this file forges
//! WELL-FRAMED decisions whose recorded claims lie, plus a strict truncation
//! sweep, and asserts the replay evaluator convicts each one with typed
//! errors and zero state movement. Verification: `NotRun` (F2 authors, does
//! not execute).

#[path = "migration_support/mod.rs"]
mod support;

use std::sync::Arc;

use bumbledb::schema::SchemaDescriptor;
use bumbledb::{ChangeSet, Db, Id128, RelationId, Value};

use bumbledb_log::apply::{self, ApplyError};
use bumbledb_log::history::command::{Command, CommandMetadata, UnverifiedOutcome};
use bumbledb_log::history::decision::{DecisionParts, encode_decision};
use bumbledb_log::history::{
    ChangeSummary, CommandId, CommandResult, Condition, DatabaseId, DatabaseIdentity,
    IncarnationId, ReceiptEpoch, RequestId, StateStamp,
};
use bumbledb_log::writer::{LocalHistory, LogError};

use support::{LIMITS, base_schema, db_id, incarnation, op, temp_dir, work};

/// One initialized keyed source: `Note(id: u64, body: string)` with a
/// Functionality key on `id`, genesis authority attached.
fn keyed_history(tag: &str) -> (Arc<Db<SchemaDescriptor>>, LocalHistory<SchemaDescriptor>) {
    let dir = temp_dir(tag).join("db");
    let db = Arc::new(
        Db::create(&dir, base_schema())
            .expect("create store")
            .expect("empty store admits"),
    );
    let history = LocalHistory::create(
        Arc::clone(&db),
        db_id(0xa1),
        incarnation(0xb1),
        op(0xc1),
        LIMITS,
        &work(),
    )
    .expect("local history creates");
    (db, history)
}

fn seal_inserts(
    db: &Db<SchemaDescriptor>,
    identity: DatabaseIdentity,
    request: u8,
    rows: &[(u64, &str)],
) -> Command {
    let mut draft = ChangeSet::builder(db.schema(), work());
    for (id, body) in rows {
        draft
            .insert(
                RelationId(0),
                &[Value::U64(*id), Value::String((*body).into())],
            )
            .expect("insert");
    }
    let changes = draft.finish().expect("draft finishes");
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
    .expect("command seals")
}

fn row_count(db: &Db<SchemaDescriptor>) -> usize {
    let mut count = 0;
    db.read(|read| {
        for row in read.scan(RelationId(0))? {
            row?;
            count += 1;
        }
        Ok(())
    })
    .expect("scan reads");
    count
}

/// Frame one decision claiming `outcome` for `command` directly over the
/// genesis position — the attacker's tool.
fn forge(
    history: &LocalHistory<SchemaDescriptor>,
    identity: DatabaseIdentity,
    command: &Command,
    outcome: UnverifiedOutcome<'_>,
    advance_state: bool,
) -> Vec<u8> {
    let authority = history.authority().expect("authority reads");
    let position = authority.position().expect("live genesis");
    let after = if advance_state {
        StateStamp {
            incarnation: position.state.incarnation,
            data_revision: position.state.data_revision + 1,
        }
    } else {
        position.state
    };
    let canonical_command = command.encode(LIMITS).expect("command encodes");
    encode_decision(
        DecisionParts {
            identity,
            seq: position.decision.seq + 1,
            parent: position.decision,
            before_state: position.state,
            after_state: after,
            canonical_command: &canonical_command,
            outcome,
        },
        LIMITS,
    )
    .expect("decision frames")
}

/// Harness validity: an HONEST decision materializes, and replaying the
/// exact same immutable decision refuses (the chain extends exactly once) —
/// forged refusals below are therefore refusals of the forgery, not of the
/// harness (PROTO-03 idempotent replay boundary).
#[test]
fn an_honest_decision_materializes_once_and_never_twice() {
    let (db, history) = keyed_history("honest");
    let identity = history.identity();
    let command = seal_inserts(&db, identity, 0x11, &[(1, "alpha")]);
    let bytes = forge(
        &history,
        identity,
        &command,
        UnverifiedOutcome::Committed {
            changed: ChangeSummary::new(1, 0).expect("nonzero summary"),
            result: &[],
        },
        true,
    );
    let before = history
        .authority()
        .expect("authority")
        .position()
        .expect("live");
    let advanced = apply::materialize(
        &db,
        &history.authority().expect("authority"),
        &bytes,
        LIMITS,
        &work(),
    )
    .expect("the honest decision applies");
    assert_eq!(row_count(&db), 1, "the honest fact landed");
    assert_eq!(
        advanced.position().expect("live").decision.seq,
        before.decision.seq + 1
    );
    // Replaying the applied decision against the ADVANCED authority refuses:
    // its parent no longer matches; no duplicate facts appear.
    let replay = apply::materialize(&db, &advanced, &bytes, LIMITS, &work());
    assert!(
        matches!(replay, Err(ApplyError::Chain(_))),
        "exact replay refuses: {replay:?}"
    );
    assert_eq!(row_count(&db), 1, "replay committed nothing");
}

/// A well-framed decision recording `Committed` for a command the judge
/// REJECTS (two rows conflicting on the declared key). Replay re-judges at
/// the exact predecessor and convicts the recorded outcome — a hostile
/// backend cannot launder an unlawful state through a signed-looking frame
/// (PROTO-15, ENG-005 boundary, G14).
#[test]
fn a_forged_committed_outcome_for_an_unlawful_command_refuses_whole() {
    let (db, history) = keyed_history("forged-commit");
    let identity = history.identity();
    let unlawful = seal_inserts(&db, identity, 0x22, &[(1, "alpha"), (1, "beta")]);
    let bytes = forge(
        &history,
        identity,
        &unlawful,
        UnverifiedOutcome::Committed {
            changed: ChangeSummary::new(2, 0).expect("nonzero summary"),
            result: &[],
        },
        true,
    );
    let authority = history.authority().expect("authority");
    let refused = apply::materialize(&db, &authority, &bytes, LIMITS, &work());
    assert!(
        matches!(refused, Err(ApplyError::OutcomeMismatch)),
        "the recorded-outcome check convicts the forgery: {refused:?}"
    );
    assert_eq!(
        row_count(&db),
        0,
        "nothing committed — not even the lawful-looking row"
    );
    let after = history
        .authority()
        .expect("authority")
        .position()
        .expect("live");
    assert_eq!(after.decision.seq, 0, "the authority never moved");
}

/// The inverse forgery: a LAWFUL command recorded as `InvariantRejected`
/// with fabricated evidence. Re-judgment commits, the record claims
/// rejection — refused whole; a hostile backend cannot erase a decision's
/// effect by rewriting its outcome (PROTO-15, REP-020 all-or-none spirit).
#[test]
fn a_forged_rejection_of_a_lawful_command_refuses_whole() {
    let (db, history) = keyed_history("forged-reject");
    let identity = history.identity();
    let lawful = seal_inserts(&db, identity, 0x33, &[(3, "gamma")]);
    let bytes = forge(
        &history,
        identity,
        &lawful,
        UnverifiedOutcome::InvariantRejected {
            core_evidence: b"fabricated evidence",
        },
        false,
    );
    let authority = history.authority().expect("authority");
    let refused = apply::materialize(&db, &authority, &bytes, LIMITS, &work());
    assert!(
        matches!(refused, Err(ApplyError::OutcomeMismatch)),
        "the fabricated rejection is convicted: {refused:?}"
    );
    assert_eq!(row_count(&db), 0, "no partial application either way");
}

/// A decision framed under a FOREIGN identity (same schema, different
/// database) presented to this tenant's materialization: refused before any
/// command evaluation (ARCH-004, REP-011/SDK-016 cross-tenant boundary).
#[test]
fn a_foreign_identity_decision_refuses_before_evaluation() {
    let (db, history) = keyed_history("foreign");
    let foreign = DatabaseIdentity {
        database_id: DatabaseId::from_core(Id128::from_bytes([0xee; 16])),
        incarnation_id: IncarnationId::from_core(Id128::from_bytes([0xef; 16])),
        schema_id: history.identity().schema_id,
    };
    let command = seal_inserts(&db, foreign, 0x44, &[(4, "delta")]);
    let bytes = forge(
        &history,
        foreign,
        &command,
        UnverifiedOutcome::Committed {
            changed: ChangeSummary::new(1, 0).expect("nonzero summary"),
            result: &[],
        },
        true,
    );
    let authority = history.authority().expect("authority");
    let refused = apply::materialize(&db, &authority, &bytes, LIMITS, &work());
    assert!(
        matches!(refused, Err(ApplyError::Command(LogError::Identity))),
        "foreign identity refuses typed: {refused:?}"
    );
    assert_eq!(row_count(&db), 0);
}

/// A decision that does not extend the exact local parent (wrong parent
/// stamp / skipped sequence) refuses as a chain error — an attacker cannot
/// splice history around a retained boundary (REP-008, REP-018).
#[test]
fn a_decision_off_the_exact_parent_refuses_as_a_chain_break() {
    let (db, history) = keyed_history("splice");
    let identity = history.identity();
    let command = seal_inserts(&db, identity, 0x55, &[(5, "epsilon")]);
    let authority = history.authority().expect("authority");
    let position = authority.position().expect("live genesis");
    let canonical_command = command.encode(LIMITS).expect("command encodes");
    // Claim a parent two steps ahead of the actual local tip.
    let forged_parent = bumbledb_log::history::DecisionStamp {
        seq: position.decision.seq + 2,
        hash: position.decision.hash,
    };
    let bytes = encode_decision(
        DecisionParts {
            identity,
            seq: forged_parent.seq + 1,
            parent: forged_parent,
            before_state: position.state,
            after_state: StateStamp {
                incarnation: position.state.incarnation,
                data_revision: position.state.data_revision + 1,
            },
            canonical_command: &canonical_command,
            outcome: UnverifiedOutcome::Committed {
                changed: ChangeSummary::new(1, 0).expect("nonzero summary"),
                result: &[],
            },
        },
        LIMITS,
    )
    .expect("decision frames");
    let refused = apply::materialize(&db, &authority, &bytes, LIMITS, &work());
    assert!(
        matches!(refused, Err(ApplyError::Chain(_))),
        "spliced parent refuses: {refused:?}"
    );
    assert_eq!(row_count(&db), 0);
}

/// Byte-level hostility: every strict prefix of a valid decision frame and a
/// trailing-byte extension refuse with typed errors — never a panic, never a
/// partial application (REP-018 truncation/overshoot, G14 fuzz floor).
#[test]
fn truncated_and_padded_decision_frames_refuse_without_state_movement() {
    let (db, history) = keyed_history("truncate");
    let identity = history.identity();
    let command = seal_inserts(&db, identity, 0x66, &[(6, "zeta")]);
    let bytes = forge(
        &history,
        identity,
        &command,
        UnverifiedOutcome::Committed {
            changed: ChangeSummary::new(1, 0).expect("nonzero summary"),
            result: &[],
        },
        true,
    );
    let authority = history.authority().expect("authority");
    for len in 0..bytes.len() {
        let refused = apply::materialize(&db, &authority, &bytes[..len], LIMITS, &work());
        assert!(refused.is_err(), "strict prefix of {len} bytes must refuse");
    }
    let mut padded = bytes.clone();
    padded.push(0);
    assert!(
        apply::materialize(&db, &authority, &padded, LIMITS, &work()).is_err(),
        "a trailing byte refuses (no-trailing-bytes framing rule)"
    );
    assert_eq!(row_count(&db), 0, "the hostile sweep committed nothing");
    assert_eq!(
        history
            .authority()
            .expect("authority")
            .position()
            .expect("live")
            .decision
            .seq,
        0,
        "the authority never moved under the sweep"
    );
}
