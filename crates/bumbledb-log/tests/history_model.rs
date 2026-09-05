//! Independent, test-only history specification over four possible facts.
//! It never calls production transition helpers and is NOT a production or
//! durable authority. Guard decisions are cross-checked against production;
//! atomic failure/response-loss schedules await a real core transaction driver.

use std::collections::{BTreeMap, BTreeSet};

use bumbledb::Id128;
use bumbledb_log::history::admission::{AdmissionView, Refusal, Submission};
use bumbledb_log::history::{
    AccessMode, CommandDigest, CommandId, CommandRef, Condition, DatabaseId, DatabaseIdentity,
    DecisionDigest, DecisionStamp, EmptyResult, IncarnationId, ReceiptEpoch, ReceiptPolicy,
    RequestId, SchemaId, StateStamp, TerminalOutcome, TerminalReceipt,
};

type Fact = (u8, u8);

#[derive(Debug, Clone, PartialEq, Eq)]
struct Intent {
    scope: u8,
    epoch: u64,
    request: u8,
    additions: BTreeSet<Fact>,
    removals: BTreeSet<Fact>,
    expected: Option<u64>,
}

impl Intent {
    fn new(
        request: u8,
        additions: impl IntoIterator<Item = Fact>,
        removals: impl IntoIterator<Item = Fact>,
    ) -> Self {
        let additions: BTreeSet<_> = additions.into_iter().collect();
        let removals = removals
            .into_iter()
            .filter(|fact| !additions.contains(fact))
            .collect();
        Self {
            scope: 1,
            epoch: 1,
            request,
            additions,
            removals,
            expected: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Outcome {
    Changed { added: usize, removed: usize },
    NoChange,
    Moved { expected: u64, observed: u64 },
    Rejected { complete_keys: BTreeSet<u8> },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Receipt {
    intent: Intent,
    decision: u64,
    revision: u64,
    outcome: Outcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Denied {
    Identity,
    Deleted,
    Expired,
    Conflict,
    Closed,
    Future,
    Frozen,
    Resources,
    Storage,
    Exhausted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Gate {
    Existing(Receipt),
    Evaluate,
    Moved(u64, u64),
    Denied(Denied),
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Answer {
    Decided(Receipt),
    Denied(Denied),
    Unknown,
}

#[derive(Debug, Clone, Copy)]
enum Fault {
    None,
    BeforeCommit,
    LostReply,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Model {
    facts: BTreeSet<Fact>,
    receipts: BTreeMap<(u64, u8), Receipt>,
    access: u8, // 0 active, 1 frozen, 2 deleted; independent of production enum.
    open: u64,
    retired: u64,
    head: u64,
    decision: u64,
    revision: u64,
}

impl Model {
    fn genesis(facts: BTreeSet<Fact>) -> Self {
        assert!(violating_keys(&facts).is_empty());
        Self {
            facts,
            receipts: BTreeMap::new(),
            access: 0,
            open: 1,
            retired: 0,
            head: 0,
            decision: 0,
            revision: 0,
        }
    }

    fn guard(&self, intent: &Intent) -> Gate {
        if intent.scope != 1 {
            return Gate::Denied(Denied::Identity);
        }
        if self.access == 2 {
            return Gate::Denied(Denied::Deleted);
        }
        if intent.epoch <= self.retired {
            return Gate::Denied(Denied::Expired);
        }
        if intent.epoch > self.open {
            return Gate::Denied(Denied::Future);
        }
        if let Some(receipt) = self.receipts.get(&(intent.epoch, intent.request)) {
            return if receipt.intent == *intent {
                Gate::Existing(receipt.clone())
            } else {
                Gate::Denied(Denied::Conflict)
            };
        }
        if intent.epoch != self.open {
            return Gate::Denied(Denied::Closed);
        }
        if self.access == 1 {
            return Gate::Denied(Denied::Frozen);
        }
        if let Some(expected) = intent.expected
            && expected != self.revision
        {
            return Gate::Moved(expected, self.revision);
        }
        Gate::Evaluate
    }

    fn submit(&mut self, intent: Intent, evidence_cap: usize, fault: Fault) -> Answer {
        let mut candidate = self.facts.clone();
        let outcome = match self.guard(&intent) {
            Gate::Existing(receipt) => return Answer::Decided(receipt),
            Gate::Denied(reason) => return Answer::Denied(reason),
            Gate::Moved(expected, observed) => Outcome::Moved { expected, observed },
            Gate::Evaluate => {
                for fact in &intent.removals {
                    candidate.remove(fact);
                }
                for fact in &intent.additions {
                    candidate.insert(*fact);
                }
                let complete_keys = violating_keys(&candidate);
                if complete_keys.len() > evidence_cap {
                    return Answer::Denied(Denied::Resources);
                }
                if !complete_keys.is_empty() {
                    candidate.clone_from(&self.facts);
                    Outcome::Rejected { complete_keys }
                } else if candidate == self.facts {
                    Outcome::NoChange
                } else {
                    Outcome::Changed {
                        added: candidate.difference(&self.facts).count(),
                        removed: self.facts.difference(&candidate).count(),
                    }
                }
            }
        };
        let (Some(head), Some(decision)) = (self.head.checked_add(1), self.decision.checked_add(1))
        else {
            return Answer::Denied(Denied::Exhausted);
        };
        let Some(revision) = self
            .revision
            .checked_add(u64::from(candidate != self.facts))
        else {
            return Answer::Denied(Denied::Exhausted);
        };
        let receipt_key = (intent.epoch, intent.request);
        let receipt = Receipt {
            intent,
            decision,
            revision,
            outcome,
        };
        if matches!(fault, Fault::BeforeCommit) {
            return Answer::Denied(Denied::Storage);
        }
        // This assignment block is the specification's atomic transition;
        // proving the production LMDB boundary implements it is a later gate.
        self.facts = candidate;
        self.receipts.insert(receipt_key, receipt.clone());
        self.head = head;
        self.decision = decision;
        self.revision = revision;
        if matches!(fault, Fault::LostReply) {
            Answer::Unknown
        } else {
            Answer::Decided(receipt)
        }
    }

    fn maintain(&mut self, open: u64, retired: u64, access: u8) {
        assert!(open >= self.open && retired >= self.retired && retired < open);
        assert!(self.access != 2 || access == 2);
        self.head = self.head.checked_add(1).unwrap();
        self.open = open;
        self.retired = retired;
        self.access = access;
        self.receipts.retain(|(epoch, _), _| *epoch > retired);
    }
}

fn violating_keys(facts: &BTreeSet<Fact>) -> BTreeSet<u8> {
    let mut values: BTreeMap<u8, BTreeSet<u8>> = BTreeMap::new();
    for (key, value) in facts {
        values.entry(*key).or_default().insert(*value);
    }
    values
        .into_iter()
        .filter_map(|(key, values)| (values.len() > 1).then_some(key))
        .collect()
}

fn facts(mask: u8) -> BTreeSet<Fact> {
    (0..4)
        .filter(|index| mask & (1 << index) != 0)
        .map(|index| (index / 2, index % 2))
        .collect()
}

fn decided(answer: Answer) -> Receipt {
    match answer {
        Answer::Decided(receipt) => receipt,
        other => panic!("expected decision, got {other:?}"),
    }
}

#[test]
fn exhaustive_small_sets_establish_add_wins_and_final_state_laws() {
    let mut cases = 0;
    for before in 0_u8..16 {
        if before & 0b0011 == 0b0011 || before & 0b1100 == 0b1100 {
            continue;
        }
        for additions in 0_u8..16 {
            for removals in 0_u8..16 {
                let mut model = Model::genesis(facts(before));
                let receipt = decided(model.submit(
                    Intent::new(1, facts(additions), facts(removals)),
                    2,
                    Fault::None,
                ));
                // Independent bitset equation, not the model's loop evaluator.
                let candidate = (before & !removals) | additions;
                let invalid = candidate & 0b0011 == 0b0011 || candidate & 0b1100 == 0b1100;
                let expected = if invalid { before } else { candidate };
                assert_eq!(model.facts, facts(expected));
                assert_eq!(model.revision, u64::from(before != expected));
                assert_eq!((model.head, model.decision), (1, 1));
                assert_eq!(matches!(receipt.outcome, Outcome::Rejected { .. }), invalid);
                cases += 1;
            }
        }
    }
    assert_eq!(cases, 2304);
}

#[test]
fn all_terminal_outcomes_survive_lost_replies_and_retry_without_reexecution() {
    let mut model = Model::genesis(BTreeSet::new());
    let changed = Intent::new(1, [(0, 0)], []);
    assert_eq!(
        model.submit(changed.clone(), 2, Fault::LostReply),
        Answer::Unknown
    );
    let first = decided(model.submit(changed.clone(), 2, Fault::None));
    assert!(matches!(
        first.outcome,
        Outcome::Changed {
            added: 1,
            removed: 0
        }
    ));
    let unchanged = model.clone();
    assert_eq!(
        decided(model.submit(changed.clone(), 2, Fault::None)),
        first
    );
    assert_eq!(model, unchanged);

    let no_op = Intent::new(2, [(0, 0)], []);
    let no_op_receipt = decided(model.submit(no_op.clone(), 2, Fault::None));
    assert_eq!(no_op_receipt.outcome, Outcome::NoChange);
    let rejected = Intent::new(3, [(0, 1)], []);
    assert_eq!(
        model.submit(rejected.clone(), 2, Fault::LostReply),
        Answer::Unknown
    );
    let rejection = decided(model.submit(rejected.clone(), 2, Fault::None));
    assert_eq!(
        rejection.outcome,
        Outcome::Rejected {
            complete_keys: BTreeSet::from([0])
        }
    );
    let mut conditional = Intent::new(4, [(1, 0)], []);
    conditional.expected = Some(0);
    let moved = decided(model.submit(conditional.clone(), 2, Fault::None));
    assert_eq!(
        moved.outcome,
        Outcome::Moved {
            expected: 0,
            observed: 1
        }
    );
    assert_eq!((model.decision, model.revision), (4, 1));

    decided(model.submit(Intent::new(5, [], [(0, 0)]), 2, Fault::None));
    model.maintain(2, 0, 1); // Epoch closed and authority frozen.
    for (intent, receipt) in [
        (changed, first),
        (no_op, no_op_receipt),
        (rejected, rejection),
        (conditional, moved),
    ] {
        assert_eq!(
            decided(model.submit(intent, 0, Fault::BeforeCommit)),
            receipt
        );
    }
    assert_eq!((model.head, model.decision, model.revision), (6, 5, 2));
}

#[test]
fn aba_is_detected_and_nonterminal_failures_record_nothing() {
    let mut model = Model::genesis(facts(1));
    let initial_facts = model.facts.clone();
    let mut observed_intent = Intent::new(1, [(1, 0)], []);
    observed_intent.expected = Some(0);
    decided(model.submit(Intent::new(2, [], [(0, 0)]), 2, Fault::None));
    decided(model.submit(Intent::new(3, [(0, 0)], []), 2, Fault::None));
    assert_eq!(model.facts, initial_facts);
    assert_eq!(
        decided(model.submit(observed_intent, 2, Fault::None)).outcome,
        Outcome::Moved {
            expected: 0,
            observed: 2
        }
    );

    let invalid = Intent::new(4, [(0, 1), (1, 0), (1, 1)], []);
    let unchanged = model.clone();
    assert_eq!(
        model.submit(invalid.clone(), 1, Fault::None),
        Answer::Denied(Denied::Resources)
    );
    assert_eq!(
        model, unchanged,
        "incomplete diagnostics are not a terminal rejection"
    );
    assert_eq!(
        model.submit(invalid.clone(), 2, Fault::BeforeCommit),
        Answer::Denied(Denied::Storage)
    );
    assert_eq!(
        model, unchanged,
        "no facts/receipt/counter prefix before atomic commit"
    );
    let rejected = decided(model.submit(invalid, 2, Fault::None));
    assert_eq!(
        rejected.outcome,
        Outcome::Rejected {
            complete_keys: BTreeSet::from([0, 1])
        }
    );
    assert_eq!((model.decision, model.revision), (4, 2));
}

#[test]
fn retired_archival_receipts_never_restore_admission_and_conflicts_do_not_replace_intent() {
    let mut model = Model::genesis(BTreeSet::new());
    let intent = Intent::new(1, [(0, 0)], []);
    let original = decided(model.submit(intent.clone(), 2, Fault::None));
    let mut conflict = intent.clone();
    conflict.additions = BTreeSet::from([(0, 1)]);
    assert_eq!(
        model.submit(conflict, 2, Fault::None),
        Answer::Denied(Denied::Conflict)
    );
    let archival = model.clone();
    model.maintain(3, 1, 0);
    assert!(model.receipts.is_empty());
    assert_eq!(
        model.submit(intent.clone(), 2, Fault::None),
        Answer::Denied(Denied::Expired)
    );
    // Even an erroneously supplied historical row cannot resurrect execution.
    model
        .receipts
        .insert((1, 1), archival.receipts[&(1, 1)].clone());
    assert_eq!(model.guard(&intent), Gate::Denied(Denied::Expired));
    assert_eq!(archival.receipts[&(1, 1)], original);
    assert_eq!((model.decision, model.revision), (1, 1));
}

#[test]
fn independent_admission_matrix_matches_the_production_guard() {
    let mut cases = 0;
    for access in 0..3 {
        for open in 1..5 {
            for retired in 0..open {
                for epoch in 1..6 {
                    for stored in 0..3 {
                        for scope in 1..3 {
                            for expected in [None, Some(1), Some(2)] {
                                let mut model = Model::genesis(BTreeSet::new());
                                model.access = access;
                                model.open = open;
                                model.retired = retired;
                                model.decision = 4;
                                model.revision = 2;
                                let mut intent = Intent::new(1, [], []);
                                intent.scope = scope;
                                intent.epoch = epoch;
                                intent.expected = expected;
                                if stored != 0 {
                                    let mut recorded = intent.clone();
                                    if stored == 2 {
                                        recorded.additions.insert((0, 0));
                                    }
                                    model.receipts.insert(
                                        (epoch, 1),
                                        Receipt {
                                            intent: recorded,
                                            decision: 1,
                                            revision: 0,
                                            outcome: Outcome::NoChange,
                                        },
                                    );
                                }
                                compare_guard(&model, &intent, stored);
                                cases += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(cases, 2700);
}

fn compare_guard(model: &Model, intent: &Intent, stored: u8) {
    let incarnation = IncarnationId::from_core(Id128::from_bytes([2; 16]));
    let identity = DatabaseIdentity {
        database_id: DatabaseId::from_core(Id128::from_bytes([1; 16])),
        incarnation_id: incarnation,
        schema_id: SchemaId([3; 32]),
    };
    let view = AdmissionView {
        identity,
        access: match model.access {
            0 => AccessMode::Active,
            1 => AccessMode::Frozen,
            _ => AccessMode::Deleted,
        },
        receipts: ReceiptPolicy::new(ReceiptEpoch::new(model.open).unwrap(), model.retired)
            .unwrap(),
        decision: DecisionStamp {
            seq: model.decision,
            hash: DecisionDigest::from_bytes([4; 32]),
        },
        state: StateStamp {
            incarnation,
            data_revision: model.revision,
        },
    };
    let command = CommandRef {
        identity: DatabaseIdentity {
            database_id: DatabaseId::from_core(Id128::from_bytes([intent.scope; 16])),
            ..identity
        },
        id: CommandId {
            receipt_epoch: ReceiptEpoch::new(intent.epoch).unwrap(),
            request_id: RequestId::from_core(Id128::from_bytes([1; 16])),
        },
        digest: CommandDigest::from_bytes([5; 32]),
    };
    let retained = (stored != 0).then(|| TerminalReceipt {
        command: CommandRef {
            identity,
            digest: CommandDigest::from_bytes([if stored == 2 { 6 } else { 5 }; 32]),
            ..command
        },
        decision_at: DecisionStamp {
            seq: 1,
            hash: DecisionDigest::from_bytes([7; 32]),
        },
        state_at: StateStamp {
            incarnation,
            data_revision: 0,
        },
        outcome: TerminalOutcome::NoChange {
            result: EmptyResult,
        },
    });
    let condition = intent
        .expected
        .map_or(Condition::Unconditional, |data_revision| {
            Condition::ExactState(StateStamp {
                incarnation,
                data_revision,
            })
        });
    let actual = view.submit(command, condition, retained.as_ref());
    let expected = model.guard(intent);
    let matches = match (expected, actual) {
        (Gate::Existing(_), Ok(Submission::AlreadyDecided(_)))
        | (Gate::Evaluate, Ok(Submission::Evaluate)) => true,
        (Gate::Moved(left, right), Ok(Submission::PreconditionFailed { expected, observed })) => {
            left == expected.data_revision && right == observed.data_revision
        }
        (Gate::Denied(left), Err(right)) => matches!(
            (left, right),
            (Denied::Identity, Refusal::IdentityMismatch)
                | (Denied::Deleted, Refusal::DatabaseDeleted)
                | (Denied::Expired, Refusal::ReceiptExpiredUnknown)
                | (Denied::Future, Refusal::CommandEpochNotOpen { .. })
                | (Denied::Conflict, Refusal::CommandIdentityConflict)
                | (Denied::Closed, Refusal::CommandEpochClosed)
                | (Denied::Frozen, Refusal::DatabaseFrozen)
        ),
        _ => false,
    };
    assert!(
        matches,
        "model={model:?}, intent={intent:?}, stored={stored}"
    );
}
