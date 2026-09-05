//! P11-authored adversarial trace corpus over the independent history
//! model (gates `PROTO-01/-02/-03/-07(model half)/-08/-09/-16`, CONC-04,
//! G07). Authored in F1, executed only after the F3 barrier; in F3 P12
//! replays the same schedules against the production machine and feeds the
//! observations through [`verify_trace`].

use std::collections::BTreeSet;

use super::{
    Command, CommandId, Condition, Event, Fact, Lifecycle, Model, Refusal, StateStamp, Terminal,
    verify_trace,
};

fn id(request: u128) -> CommandId {
    CommandId { epoch: 1, request }
}

fn insert(request: u128, fact: Fact) -> Command {
    Command::seal(
        id(request),
        [fact].into_iter().collect(),
        BTreeSet::new(),
        Condition::Unconditional,
    )
}

fn replace(request: u128, remove: Fact, add: Fact, witness: StateStamp) -> Command {
    Command::seal(
        id(request),
        [add].into_iter().collect(),
        [remove].into_iter().collect(),
        Condition::ExactState(witness),
    )
}

/// Replay the committed decisions in publication order from the empty
/// state: the linearization every client-visible history must equal.
fn linearized(model: &Model) -> BTreeSet<Fact> {
    let mut facts: BTreeSet<Fact> = BTreeSet::new();
    for decision in &model.decisions {
        if matches!(decision.outcome, Terminal::Committed { .. }) {
            facts = facts
                .difference(&decision.command.removes)
                .copied()
                .collect();
            facts.extend(decision.command.adds.iter().copied());
        }
    }
    facts
}

#[test]
fn proto_01_one_successor_per_revision_under_every_interleaving() {
    // Two contenders, each: capture, then CAS; all interleavings of the
    // four steps that preserve each writer's order, plus bounded retry to
    // terminal. Exactly one CAS wins per revision, both requests end
    // terminal, and the final state is the linearization of the decisions.
    let schedules: &[&[usize]] = &[
        &[0, 0, 1, 1],
        &[0, 1, 0, 1],
        &[0, 1, 1, 0],
        &[1, 0, 0, 1],
        &[1, 0, 1, 0],
        &[1, 1, 0, 0],
    ];
    for schedule in schedules {
        let mut model = Model::new(None);
        let commands = [insert(1, (0, 10)), insert(2, (0, 20))];
        let mut views: [Option<super::View>; 2] = [None, None];
        let mut results: [Option<Terminal>; 2] = [None, None];
        let mut events: Vec<Event> = Vec::new();
        for &writer in *schedule {
            if views[writer].is_none() {
                views[writer] = Some(model.capture());
            } else if results[writer].is_none() {
                let view = views[writer].clone().expect("captured");
                let expected = view.revision;
                let won = model.try_cas(&view, &commands[writer]);
                events.push(Event::Cas {
                    expected,
                    won: won.is_some(),
                });
                if let Some(outcome) = won {
                    events.push(Event::Receipt {
                        id: commands[writer].id,
                        outcome,
                    });
                    results[writer] = Some(outcome);
                }
            }
        }
        // Bounded retry for the loser: the SAME immutable command.
        for writer in 0..2 {
            if results[writer].is_none() {
                let outcome = model
                    .submit(&commands[writer], 4)
                    .expect("retry reaches a terminal outcome");
                events.push(Event::Receipt {
                    id: commands[writer].id,
                    outcome,
                });
                results[writer] = Some(outcome);
            }
        }
        verify_trace(&events).expect("one successor per revision");
        assert_eq!(
            model.facts,
            linearized(&model),
            "the final state is the decisions' linearization ({schedule:?})"
        );
        assert_eq!(
            model.facts,
            [(0, 10), (0, 20)].into_iter().collect::<BTreeSet<Fact>>(),
            "both unconditional inserts land ({schedule:?})"
        );
    }
}

#[test]
fn proto_02_same_id_same_body_is_one_stable_outcome() {
    let mut model = Model::new(None);
    let command = insert(7, (0, 1));
    let first = model.submit(&command, 4).expect("terminal");
    // Response lost: the caller retries the identical sealed command —
    // including "on another host": only the id and canonical bytes travel.
    let retried = model.submit(&command, 4).expect("terminal");
    assert_eq!(first, retried, "one named request, one outcome");
    // And resolve() by retained id returns exactly the recorded outcome.
    assert_eq!(model.resolve(command.id), Ok(first));
    // A durable NO-CHANGE command still has a stable receipt.
    let noop = Command::seal(
        id(8),
        [(0, 1)].into_iter().collect(),
        BTreeSet::new(),
        Condition::Unconditional,
    );
    let noop_outcome = model.submit(&noop, 4).expect("terminal");
    assert_eq!(noop_outcome, Terminal::NoChange);
    assert_eq!(model.submit(&noop, 4), Ok(Terminal::NoChange));
}

#[test]
fn proto_03_same_id_different_body_conflicts_without_execution() {
    let mut model = Model::new(None);
    let original = insert(9, (0, 1));
    model.submit(&original, 4).expect("terminal");
    let facts_before = model.facts.clone();
    let seq_before = model.decision_seq;
    // The same id with different canonical bytes: identity conflict —
    // never a second execution, never a reinterpretation.
    let forged = insert(9, (0, 2));
    assert_eq!(
        model.submit(&forged, 4),
        Err(Refusal::CommandIdentityConflict)
    );
    assert_eq!(model.facts, facts_before, "no second execution");
    assert_eq!(model.decision_seq, seq_before, "no decision published");
}

#[test]
fn proto_08_two_witnessed_decrements_one_precondition_failure() {
    let mut model = Model::new(None);
    model
        .submit(&insert(1, (1, 10)), 4)
        .expect("seed the counter fact");
    let witness = model.state;
    // Two writers read the same state and each submit a witnessed
    // replacement 10 -> 9.
    let w1 = replace(2, (1, 10), (1, 9), witness);
    let w2 = replace(3, (1, 10), (1, 9), witness);
    let first = model.submit(&w1, 4).expect("terminal");
    let second = model.submit(&w2, 4).expect("terminal");
    assert_eq!(first, Terminal::Committed { changed: true });
    assert!(
        matches!(second, Terminal::PreconditionFailed { .. }),
        "the second witnessed decrement records a precondition failure"
    );
    // The BLIND variant documents the different meaning: both blind
    // replacements land as one net effect (set semantics working as
    // specified; the application failed to encode its read dependency).
    let mut blind = Model::new(None);
    blind.submit(&insert(1, (1, 10)), 4).expect("seed");
    let b1 = Command::seal(
        id(2),
        [(1, 9)].into_iter().collect(),
        [(1, 10)].into_iter().collect(),
        Condition::Unconditional,
    );
    let b2 = Command::seal(
        id(3),
        [(1, 9)].into_iter().collect(),
        [(1, 10)].into_iter().collect(),
        Condition::Unconditional,
    );
    assert_eq!(
        blind.submit(&b1, 4),
        Ok(Terminal::Committed { changed: true })
    );
    assert_eq!(blind.submit(&b2, 4), Ok(Terminal::NoChange));
}

#[test]
fn proto_09_state_stamp_moves_exactly_on_net_change() {
    let mut model = Model::new(None);
    let base = model.state;
    // Maintenance does not move the stamp (revision moves).
    let rev = model.revision;
    model.rotate_epoch();
    assert_eq!(model.state, base);
    assert!(model.revision > rev, "maintenance moves the revision");
    // A rejection does not move it.
    let mut capped = Model::new(Some(0));
    let stamp = capped.state;
    assert_eq!(
        capped.submit(&insert(1, (0, 1)), 4),
        Ok(Terminal::InvariantRejected)
    );
    assert_eq!(capped.state, stamp, "a durable rejection moves no stamp");
    // A no-op does not move it.
    let mut m = Model::new(None);
    m.submit(&insert(1, (0, 5)), 4).expect("terminal");
    let stamp = m.state;
    let noop = insert(2, (0, 5));
    assert_eq!(m.submit(&noop, 4), Ok(Terminal::NoChange));
    assert_eq!(m.state, stamp);
    // Change-and-restore MOVES it — the deliberate ABA detection.
    let del = Command::seal(
        CommandId {
            epoch: 1,
            request: 3,
        },
        BTreeSet::new(),
        [(0, 5)].into_iter().collect(),
        Condition::Unconditional,
    );
    let readd = insert(4, (0, 5));
    m.submit(&del, 4).expect("terminal");
    m.submit(&readd, 4).expect("terminal");
    assert_eq!(m.facts, [(0, 5)].into_iter().collect::<BTreeSet<Fact>>());
    assert_ne!(
        m.state, stamp,
        "identical values, different history: the witness must move"
    );
    // A stale witness against the restored-but-moved state fails.
    let stale = replace(5, (0, 5), (0, 6), stamp);
    assert!(matches!(
        m.submit(&stale, 4),
        Ok(Terminal::PreconditionFailed { .. })
    ));
}

#[test]
fn proto_16_rotation_freeze_and_retirement() {
    let mut model = Model::new(None);
    let retained = insert(1, (0, 1));
    let outcome = model.submit(&retained, 4).expect("terminal");
    // Rotate: epoch 1 closes, epoch 2 opens.
    model.rotate_epoch();
    // Known epoch-1 id still resolves with the recorded outcome.
    assert_eq!(model.submit(&retained, 4), Ok(outcome));
    // An UNSEEN epoch-1 command does not execute.
    let unseen_old = insert(99, (0, 9));
    assert_eq!(
        model.submit(&unseen_old, 4),
        Err(Refusal::CommandEpochClosed)
    );
    // Frozen: retained ids resolve, unseen commands refuse.
    model.freeze(77);
    assert_eq!(model.submit(&retained, 4), Ok(outcome));
    let unseen_new = Command::seal(
        CommandId {
            epoch: 2,
            request: 5,
        },
        [(0, 5)].into_iter().collect(),
        BTreeSet::new(),
        Condition::Unconditional,
    );
    assert_eq!(model.submit(&unseen_new, 4), Err(Refusal::Frozen));
    assert!(matches!(
        model.lifecycle,
        Lifecycle::Frozen { operation: 77 }
    ));
    // Retire epoch 1: permanent refusal, resolution no longer promised —
    // and never re-execution.
    model.retire_through(1);
    assert_eq!(
        model.submit(&retained, 4),
        Err(Refusal::ReceiptExpiredUnknown)
    );
    assert_eq!(
        model.resolve(retained.id),
        Err(Refusal::ReceiptExpiredUnknown)
    );
    // The retained fact itself is untouched: retirement is receipt
    // policy, not data policy.
    assert!(model.facts.contains(&(0, 1)));
}

#[test]
#[should_panic(expected = "retirement advances monotonically")]
fn retirement_cannot_touch_the_open_epoch() {
    let mut model = Model::new(None);
    model.retire_through(1); // open epoch is 1: refused loudly
}

#[test]
fn deleted_refuses_ordinary_access() {
    let mut model = Model::new(None);
    let known = insert(1, (0, 1));
    model.submit(&known, 4).expect("terminal");
    model.delete(42);
    // Unseen and KNOWN ids alike refuse against a tombstone: a Deleted
    // head is terminal with no receipt table — refusal precedes any
    // hydration or lookup (unlike Frozen, where retained ids resolve).
    assert_eq!(model.submit(&insert(2, (0, 2)), 4), Err(Refusal::Deleted));
    assert_eq!(model.submit(&known, 4), Err(Refusal::Deleted));
    assert!(matches!(
        model.lifecycle,
        Lifecycle::Deleted { operation: 42 }
    ));
}

#[test]
fn bounded_attempts_return_outcome_unknown_not_a_lie() {
    let mut model = Model::new(None);
    // Zero attempts: the budget expires before any capture — an explicit
    // unknown, never a fabricated NotSubmitted-as-rejection.
    assert_eq!(
        model.submit(&insert(1, (0, 1)), 0),
        Err(Refusal::OutcomeUnknown)
    );
    // And resolve on the never-dispatched id is a point-in-time absence.
    assert_eq!(model.resolve(id(1)), Err(Refusal::OutcomeUnknown));
}

#[test]
fn trace_checker_rejects_double_winners_and_forked_outcomes() {
    assert!(
        verify_trace(&[
            Event::Cas {
                expected: 0,
                won: true
            },
            Event::Cas {
                expected: 0,
                won: true
            },
        ])
        .is_err()
    );
    assert!(
        verify_trace(&[
            Event::Receipt {
                id: id(1),
                outcome: Terminal::NoChange
            },
            Event::Receipt {
                id: id(1),
                outcome: Terminal::Committed { changed: true }
            },
        ])
        .is_err()
    );
    assert!(
        verify_trace(&[
            Event::Cas {
                expected: 0,
                won: true
            },
            Event::Cas {
                expected: 0,
                won: false
            },
            Event::Cas {
                expected: 1,
                won: true
            },
        ])
        .is_ok()
    );
}
