//! P12 adversarial integration: the PRODUCTION history machine driven
//! through P11's adversarial schedules with the INDEPENDENT model as the
//! oracle and `bumbledb_bench::closure::history_model::verify_trace` as the
//! shared trace checker (chapter 62: "P12 feeds the production machine the
//! same schedules and compares every client-visible observation"; ASS-002,
//! CONC-02/03/04, PROTO-01/02/08/09, ARCH-001/003, G07).
//!
//! Divergence detection is structural: production receipts and model
//! receipts are mapped into ONE event trace under shared command ids, so any
//! disagreement on a terminal outcome is a checker violation, not a bespoke
//! assertion. The model imports no production helper (its independence is
//! P11's compile-time guarantee); this file is the bridge P11 declared but
//! could not wire before the machine landed.
//!
//! Requires the `bumbledb-bench` dev-dependency (P12 hub request to P00).
//! Verification: `NotRun` (F2 authors, does not execute).

mod lane_support;

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use bumbledb::schema::SchemaDescriptor;
use bumbledb::{Db, Id128, RelationId, Value};

use bumbledb_bench::closure::history_model::{
    Command as ModelCommand, CommandId as ModelCommandId, Condition as ModelCondition, Event,
    Model, StateStamp as ModelStamp, Terminal as ModelTerminal, verify_trace,
};

use bumbledb_log::history::command::Command;
use bumbledb_log::history::{
    CommandId, CommandRef, Condition, DatabaseIdentity, RequestId, StateStamp, TerminalOutcome,
    TerminalReceipt,
};
use bumbledb_log::store::mem::{Behavior, MemStore, Op};
use bumbledb_log::writer::{HostedHistory, LocalHistory, ResolveOutcome, SubmitOutcome};

use lane_support::{LIMITS, fresh_db, op, test_identity, work};

/// Map a production command id into the model's id space.
fn model_id(id: CommandId) -> ModelCommandId {
    ModelCommandId {
        epoch: 1,
        request: u128::from_le_bytes(id.request_id.as_core().to_bytes()),
    }
}

/// Map a production state stamp into the model's, relative to the genesis
/// data revision (both machines use one incarnation here).
fn model_stamp(stamp: StateStamp, genesis_revision: u64) -> ModelStamp {
    ModelStamp {
        incarnation: 1,
        data_rev: stamp.data_revision - genesis_revision,
    }
}

/// Map a production terminal outcome into the model's vocabulary.
fn model_outcome(outcome: &TerminalOutcome, genesis_revision: u64) -> ModelTerminal {
    match outcome {
        TerminalOutcome::Committed { .. } => ModelTerminal::Committed { changed: true },
        TerminalOutcome::NoChange { .. } => ModelTerminal::NoChange,
        TerminalOutcome::PreconditionFailed { observed, .. } => ModelTerminal::PreconditionFailed {
            observed: model_stamp(*observed, genesis_revision),
        },
        TerminalOutcome::InvariantRejected { .. } => ModelTerminal::InvariantRejected,
    }
}

fn receipt_event(receipt: &TerminalReceipt, genesis_revision: u64) -> Event {
    Event::Receipt {
        id: model_id(receipt.command.id),
        outcome: model_outcome(&receipt.outcome, genesis_revision),
    }
}

fn seal(
    db: &Db<SchemaDescriptor>,
    identity: DatabaseIdentity,
    request: u8,
    condition: Condition,
    adds: &[u64],
    removes: &[u64],
) -> Command {
    lane_support::command(db, identity, request, condition, |draft| {
        for id in adds {
            draft
                .insert(RelationId(0), &[Value::U64(*id)])
                .expect("insert");
        }
        for id in removes {
            draft
                .delete(RelationId(0), &[Value::U64(*id)])
                .expect("delete");
        }
    })
}

fn decided(outcome: SubmitOutcome) -> TerminalReceipt {
    match outcome {
        SubmitOutcome::Decided { receipt, .. } => receipt,
        other => panic!("expected a decision, got {other:?}"),
    }
}

fn model_command(
    request: u8,
    condition: ModelCondition,
    adds: &[u64],
    removes: &[u64],
) -> ModelCommand {
    ModelCommand::seal(
        ModelCommandId {
            epoch: 1,
            request: u128::from_le_bytes([request; 16]),
        },
        adds.iter().map(|id| (0u16, *id)).collect::<BTreeSet<_>>(),
        removes
            .iter()
            .map(|id| (0u16, *id))
            .collect::<BTreeSet<_>>(),
        condition,
    )
}

/// P11's witnessed-decrement/ABA schedule (chapter 02 CONC-02/03, ARCH-001,
/// PROTO-08/09) executed against BOTH machines, merged into one trace: the
/// checker convicts any outcome divergence under the shared command ids.
#[test]
#[expect(
    clippy::too_many_lines,
    reason = "one merged adversarial schedule; the trace must stay a single \
              linear script for the divergence checker"
)]
fn the_witnessed_and_aba_schedule_agrees_with_the_independent_model() {
    let db = fresh_db("trace-witness");
    let identity = test_identity(&db);
    let history = LocalHistory::create(
        Arc::clone(&db),
        identity.database_id,
        identity.incarnation_id,
        op(0xc3),
        LIMITS,
        &work(),
    )
    .expect("local history creates");
    let identity = history.identity();
    let genesis_state = history
        .authority()
        .expect("authority")
        .position()
        .expect("live genesis")
        .state;
    let genesis_revision = genesis_state.data_revision;

    let mut model = Model::new(None);
    let mut trace: Vec<Event> = Vec::new();
    let genesis_model_stamp = model.capture().state;

    // Two writers witnessed the SAME state; only one witnessed change enacts.
    let w1 = seal(
        &db,
        identity,
        0x01,
        Condition::ExactState(genesis_state),
        &[10],
        &[],
    );
    let w2 = seal(
        &db,
        identity,
        0x02,
        Condition::ExactState(genesis_state),
        &[20],
        &[],
    );
    let m1 = model_command(
        0x01,
        ModelCondition::ExactState(genesis_model_stamp),
        &[10],
        &[],
    );
    let m2 = model_command(
        0x02,
        ModelCondition::ExactState(genesis_model_stamp),
        &[20],
        &[],
    );

    let r1 = decided(history.submit(&w1, &work()));
    trace.push(receipt_event(&r1, genesis_revision));
    trace.push(Event::Receipt {
        id: model_id(w1.command_ref().id),
        outcome: model.submit(&m1, 4).expect("model decides w1"),
    });

    let r2 = decided(history.submit(&w2, &work()));
    assert!(
        matches!(r2.outcome, TerminalOutcome::PreconditionFailed { .. }),
        "the second witnessed change reports the explicit precondition failure"
    );
    trace.push(receipt_event(&r2, genesis_revision));
    trace.push(Event::Receipt {
        id: model_id(w2.command_ref().id),
        outcome: model.submit(&m2, 4).expect("model decides w2"),
    });

    // ABA: change and restore the exact prior facts; the stamp still moved,
    // so a witness captured before the excursion refuses afterward.
    let mid_state = history
        .authority()
        .expect("authority")
        .position()
        .expect("live")
        .state;
    let mid_model = model.capture().state;
    let away = seal(&db, identity, 0x03, Condition::Unconditional, &[], &[10]);
    let back = seal(&db, identity, 0x04, Condition::Unconditional, &[10], &[]);
    trace.push(receipt_event(
        &decided(history.submit(&away, &work())),
        genesis_revision,
    ));
    trace.push(Event::Receipt {
        id: model_id(away.command_ref().id),
        outcome: model
            .submit(
                &model_command(0x03, ModelCondition::Unconditional, &[], &[10]),
                4,
            )
            .expect("model decides away"),
    });
    trace.push(receipt_event(
        &decided(history.submit(&back, &work())),
        genesis_revision,
    ));
    trace.push(Event::Receipt {
        id: model_id(back.command_ref().id),
        outcome: model
            .submit(
                &model_command(0x04, ModelCondition::Unconditional, &[10], &[]),
                4,
            )
            .expect("model decides back"),
    });

    let stale = seal(
        &db,
        identity,
        0x05,
        Condition::ExactState(mid_state),
        &[30],
        &[],
    );
    let stale_receipt = decided(history.submit(&stale, &work()));
    assert!(
        matches!(
            stale_receipt.outcome,
            TerminalOutcome::PreconditionFailed { .. }
        ),
        "an identical-facts restore still moved the witness (ABA detection)"
    );
    trace.push(receipt_event(&stale_receipt, genesis_revision));
    trace.push(Event::Receipt {
        id: model_id(stale.command_ref().id),
        outcome: model
            .submit(
                &model_command(0x05, ModelCondition::ExactState(mid_model), &[30], &[]),
                4,
            )
            .expect("model decides stale"),
    });

    // Retries return the retained receipts — replay both machines and merge.
    trace.push(receipt_event(
        &decided(history.submit(&w1, &work())),
        genesis_revision,
    ));
    trace.push(Event::Receipt {
        id: model_id(w1.command_ref().id),
        outcome: model.submit(&m1, 4).expect("model replays w1"),
    });

    verify_trace(&trace).expect("production and model outcomes agree per command id");
}

/// Two concurrent writers over ONE hosted authority: every returned receipt
/// is terminal and stable, the duplicate command decides exactly once across
/// both threads, and the decision sequence has one winner per revision
/// (PROTO-01/02, CONC-04, REP-015 bounded outcome under contention).
#[test]
fn contended_hosted_writers_produce_a_lawful_receipt_trace() {
    let db = fresh_db("trace-contend");
    let identity = test_identity(&db);
    let history = HostedHistory::create(
        Arc::clone(&db),
        MemStore::new(),
        "t".to_string(),
        0,
        identity.database_id,
        identity.incarnation_id,
        op(0xc3),
        LIMITS,
        &work(),
    )
    .expect("hosted history creates");
    let identity = history.identity();

    let receipts: Mutex<Vec<TerminalReceipt>> = Mutex::new(Vec::new());
    std::thread::scope(|scope| {
        for (base, offset) in [(0x10u8, 100u64), (0x20u8, 200u64)] {
            let history = &history;
            let db = &db;
            let receipts = &receipts;
            scope.spawn(move || {
                for step in 0..4u8 {
                    let command = seal(
                        db,
                        identity,
                        base + step,
                        Condition::Unconditional,
                        &[offset + u64::from(step)],
                        &[],
                    );
                    let receipt = decided(history.submit(&command, &work()));
                    receipts.lock().expect("receipts lock").push(receipt);
                }
                // Both threads race the SAME duplicate command.
                let duplicate = seal(db, identity, 0x30, Condition::Unconditional, &[300], &[]);
                let receipt = decided(history.submit(&duplicate, &work()));
                receipts.lock().expect("receipts lock").push(receipt);
            });
        }
    });

    let receipts = receipts.into_inner().expect("receipts");
    assert_eq!(
        receipts.len(),
        10,
        "every submission reached a terminal decision"
    );

    // Receipt trace: one stable outcome per command id across both threads.
    let mut trace: Vec<Event> = receipts.iter().map(|r| receipt_event(r, 0)).collect();
    // CAS trace: distinct decisions claim distinct sequence slots exactly
    // once; retained-retry receipts share their winner's slot, so emit one
    // win per distinct sequence.
    let mut seen: BTreeSet<u64> = BTreeSet::new();
    for receipt in &receipts {
        if seen.insert(receipt.decision_at.seq) {
            trace.push(Event::Cas {
                expected: receipt.decision_at.seq - 1,
                won: true,
            });
        }
    }
    verify_trace(&trace).expect("one winner per revision, one outcome per id");

    // The duplicate decided exactly once: both returned receipts carry the
    // same decision stamp.
    let duplicate_request = RequestId::from_core(Id128::from_bytes([0x30; 16]));
    let duplicates: Vec<&TerminalReceipt> = receipts
        .iter()
        .filter(|r| r.command.id.request_id == duplicate_request)
        .collect();
    assert_eq!(duplicates.len(), 2);
    assert_eq!(duplicates[0].decision_at, duplicates[1].decision_at);
    assert_eq!(
        model_outcome(&duplicates[0].outcome, 0),
        model_outcome(&duplicates[1].outcome, 0)
    );
}

/// A lost CAS response (applied but reported indeterminate) never forks the
/// outcome: this invocation either resolves to the durable decision or
/// reports `OutcomeUnknown` with a retained ref that later resolves to the
/// SAME receipt a retry returns (PROTO-04/05 certainty family, SDK-001).
#[test]
fn a_lost_cas_response_resolves_to_one_stable_outcome() {
    let db = fresh_db("trace-lost");
    let identity = test_identity(&db);
    // One-shot fault, armed per verb: `create` publishes through CreateHead,
    // so the armed ReplaceHead fault waits for the first decision CAS.
    let store = MemStore::new();
    store.fail_next(Op::ReplaceHead, Behavior::IndeterminateApplied);
    let history = HostedHistory::create(
        Arc::clone(&db),
        store,
        "t".to_string(),
        0,
        identity.database_id,
        identity.incarnation_id,
        op(0xc3),
        LIMITS,
        &work(),
    )
    .expect("hosted history creates");
    let identity = history.identity();
    let command = seal(&db, identity, 0x40, Condition::Unconditional, &[400], &[]);
    let reference: CommandRef = command.command_ref();

    let mut trace: Vec<Event> = Vec::new();
    match history.submit(&command, &work()) {
        SubmitOutcome::Decided { receipt, .. } => {
            // The machine resolved the applied-but-unacknowledged CAS itself.
            trace.push(receipt_event(&receipt, 0));
        }
        SubmitOutcome::OutcomeUnknown { .. } => {
            // Honest uncertainty; the retained ref must resolve durably below.
        }
        SubmitOutcome::NotSubmitted { error, .. } => {
            panic!("an applied CAS is never NotSubmitted: {error:?}")
        }
    }
    match history.resolve(reference, &work()).expect("resolve runs") {
        ResolveOutcome::Found(receipt) => {
            assert!(
                matches!(receipt.outcome, TerminalOutcome::Committed { .. }),
                "the applied decision is Committed"
            );
            trace.push(receipt_event(&receipt, 0));
        }
        other => panic!("the applied decision resolves Found: {other:?}"),
    }
    // A retry returns the SAME retained receipt; the merged trace stays
    // lawful (no forked outcome anywhere on this command id).
    let retry = decided(history.submit(&command, &work()));
    trace.push(receipt_event(&retry, 0));
    verify_trace(&trace).expect("the lost response produced exactly one outcome");
}
