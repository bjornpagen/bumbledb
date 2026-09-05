//! The independent tiny history model (P11; chapter 20; audit ASS-002,
//! gates `PROTO-01..-20` model side, CONC-04, G07).
//!
//! One never-reused tenant HEAD changed by compare-and-swap over immutable
//! decisions, retained named receipts, receipt epochs with permanent
//! retirement, `ExactState` witnesses that detect ABA application histories,
//! and Frozen/Deleted authority states. This model calls NO production
//! transition helper — the bench crate has no `bumbledb-log` dependency at
//! all, so the independence is compile-time — and command identity binding
//! uses STRUCTURAL equality of the canonical body, strictly stronger than
//! any digest, so no hash helper is shared either.
//!
//! The exhaustive small-schedule tests below are the adversarial trace
//! corpus: every interleaving of two writers' capture/CAS steps, response
//! loss at each boundary, and the maintenance races chapter 20 names. P12
//! feeds the production machine the same schedules in F3 and compares every
//! client-visible observation (`verify_trace` is the shared checker);
//! comparing final bytes alone is explicitly insufficient.

use std::collections::{BTreeMap, BTreeSet};

/// A model application fact: (relation, value).
pub type Fact = (u16, u64);

/// The exact-state witness: incarnation plus application-data revision.
/// Maintenance, no-ops and rejections do not move it; any net fact change
/// does — even one that later restores identical values (ABA detection).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct StateStamp {
    pub incarnation: u64,
    pub data_rev: u64,
}

/// Command identity: the receipt-epoch scope plus the caller's 128-bit
/// request id. Binding to the body is by STRUCTURAL equality.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CommandId {
    pub epoch: u64,
    pub request: u128,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Condition {
    Unconditional,
    ExactState(StateStamp),
}

/// The owned canonical command: sealed sets, add-wins normalization
/// applied at sealing (one command's tie rule, never cross-command merge).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Command {
    pub id: CommandId,
    pub adds: BTreeSet<Fact>,
    pub removes: BTreeSet<Fact>,
    pub condition: Condition,
}

impl Command {
    /// Seal a command: normalize `(A, D)` to `(A, D \ A)` once.
    #[must_use]
    pub fn seal(
        id: CommandId,
        adds: BTreeSet<Fact>,
        mut removes: BTreeSet<Fact>,
        condition: Condition,
    ) -> Self {
        removes.retain(|fact| !adds.contains(fact));
        Self {
            id,
            adds,
            removes,
            condition,
        }
    }
}

/// The four terminal outcomes — all durable, all receipt-backed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Terminal {
    Committed { changed: bool },
    NoChange,
    PreconditionFailed { observed: StateStamp },
    InvariantRejected,
}

/// Nonterminal refusals — never masquerading as recorded rejections.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The same id was already bound to a DIFFERENT canonical body.
    CommandIdentityConflict,
    /// The id's epoch is closed (older than open, not retired): known ids
    /// resolve, unseen ids refuse execution.
    CommandEpochClosed,
    /// The id's epoch is retired: permanent refusal, outcome no longer
    /// promised — never re-execution.
    ReceiptExpiredUnknown,
    /// The authority is Frozen: unseen commands do not execute (retained
    /// receipts still resolve).
    Frozen,
    /// The authority is Deleted: ordinary access refuses before hydration.
    Deleted,
    /// The bounded CAS budget expired without a terminal outcome.
    OutcomeUnknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lifecycle {
    Active,
    Frozen { operation: u64 },
    Deleted { operation: u64 },
}

/// One immutable decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    pub seq: u64,
    pub command: Command,
    pub before: StateStamp,
    pub after: StateStamp,
    pub outcome: Terminal,
}

/// The authoritative head + store: revision moves on EVERY successful CAS
/// (maintenance included); the decision sequence and state stamp move by
/// their own rules.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    pub revision: u64,
    pub lifecycle: Lifecycle,
    pub decision_seq: u64,
    pub state: StateStamp,
    pub open_epoch: u64,
    pub retired_through: u64,
    pub facts: BTreeSet<Fact>,
    /// Retained receipts: id -> (bound body, terminal outcome, seq).
    pub receipts: BTreeMap<CommandId, (Command, Terminal, u64)>,
    pub decisions: Vec<Decision>,
    /// The invariant the judge enforces: at most this many facts in
    /// relation 0 (a capacity-shaped law, enough to exercise the
    /// rejected-terminal path).
    pub rel0_capacity: Option<u64>,
}

/// What one writer captured: the exact head coordinates its CAS spends.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct View {
    pub revision: u64,
    pub state: StateStamp,
    pub facts: BTreeSet<Fact>,
}

/// The submission outcome surface: terminal receipts versus explicit
/// nonterminal refusals; a lost RESPONSE is the driver's to model (drop the
/// value, later `resolve`).
pub type Submit = Result<Terminal, Refusal>;

impl Model {
    #[must_use]
    pub fn new(rel0_capacity: Option<u64>) -> Self {
        Self {
            revision: 0,
            lifecycle: Lifecycle::Active,
            decision_seq: 0,
            state: StateStamp {
                incarnation: 1,
                data_rev: 0,
            },
            open_epoch: 1,
            retired_through: 0,
            facts: BTreeSet::new(),
            receipts: BTreeMap::new(),
            decisions: Vec::new(),
            rel0_capacity,
        }
    }

    /// Capture one finite published view — the writer's exact parent.
    #[must_use]
    pub fn capture(&self) -> View {
        View {
            revision: self.revision,
            state: self.state,
            facts: self.facts.clone(),
        }
    }

    /// Receipt lookup PRECEDES admission: retained ids return their
    /// recorded outcome even under Frozen or a closed epoch; a different
    /// body under a known id conflicts; retired epochs refuse permanently.
    fn lookup(&self, command: &Command) -> Option<Submit> {
        if let Some((bound, outcome, _)) = self.receipts.get(&command.id) {
            if bound == command {
                return Some(Ok(*outcome));
            }
            return Some(Err(Refusal::CommandIdentityConflict));
        }
        if command.id.epoch <= self.retired_through {
            return Some(Err(Refusal::ReceiptExpiredUnknown));
        }
        None
    }

    /// One bounded submission: lookup, admission guards, then a bounded
    /// capture/judge/CAS loop (`attempts` bounds the retries; exhaustion is
    /// an explicit `OutcomeUnknown`, never a silent retry forever).
    ///
    /// # Errors
    /// The typed refusal: deleted/frozen authority, a closed command epoch,
    /// or `OutcomeUnknown` when every attempt lost its race.
    pub fn submit(&mut self, command: &Command, attempts: u32) -> Submit {
        for _ in 0..attempts {
            // A tombstoned authority refuses ordinary access BEFORE any
            // hydration or lookup: a Deleted head has no receipt table
            // (chapter 20 — `Deleted` is terminal, not a read-only mode).
            if matches!(self.lifecycle, Lifecycle::Deleted { .. }) {
                return Err(Refusal::Deleted);
            }
            // Receipt lookup precedes the REMAINING admission guards:
            // retained ids resolve while Frozen or epoch-closed.
            if let Some(found) = self.lookup(command) {
                return found;
            }
            if matches!(self.lifecycle, Lifecycle::Frozen { .. }) {
                return Err(Refusal::Frozen);
            }
            if command.id.epoch != self.open_epoch {
                return Err(Refusal::CommandEpochClosed);
            }
            let view = self.capture();
            // A lost CAS race falls through: catch up and rejudge.
            if let Some(outcome) = self.try_cas(&view, command) {
                return Ok(outcome);
            }
        }
        Err(Refusal::OutcomeUnknown)
    }

    /// Judge one sealed command against one captured view — pure.
    fn judge(&self, view: &View, command: &Command) -> (Terminal, BTreeSet<Fact>) {
        if let Condition::ExactState(expected) = command.condition
            && expected != view.state
        {
            return (
                Terminal::PreconditionFailed {
                    observed: view.state,
                },
                view.facts.clone(),
            );
        }
        let mut next: BTreeSet<Fact> = view.facts.difference(&command.removes).copied().collect();
        next.extend(command.adds.iter().copied());
        if let Some(cap) = self.rel0_capacity {
            let rel0 = u64::try_from(next.iter().filter(|(rel, _)| *rel == 0).count())
                .expect("model states are tiny");
            if rel0 > cap {
                return (Terminal::InvariantRejected, view.facts.clone());
            }
        }
        if next == view.facts {
            (Terminal::NoChange, next)
        } else {
            (Terminal::Committed { changed: true }, next)
        }
    }

    /// The conditional replacement: succeeds only against the EXACT
    /// captured revision — the hosted publication linearization point. A
    /// lost CAS returns `None`; the caller re-captures and rejudges the
    /// SAME immutable command.
    pub fn try_cas(&mut self, view: &View, command: &Command) -> Option<Terminal> {
        if view.revision != self.revision {
            return None;
        }
        let (outcome, next_facts) = self.judge(view, command);
        let before = self.state;
        let after = if matches!(outcome, Terminal::Committed { .. }) {
            StateStamp {
                incarnation: before.incarnation,
                data_rev: before.data_rev + 1,
            }
        } else {
            before
        };
        self.revision += 1;
        self.decision_seq += 1;
        self.state = after;
        if matches!(outcome, Terminal::Committed { .. }) {
            self.facts = next_facts;
        }
        self.decisions.push(Decision {
            seq: self.decision_seq,
            command: command.clone(),
            before,
            after,
            outcome,
        });
        self.receipts
            .insert(command.id, (command.clone(), outcome, self.decision_seq));
        Some(outcome)
    }

    /// Resolve a possibly-lost response by id: the recorded terminal
    /// outcome while retained; `ReceiptExpiredUnknown` after retirement;
    /// an explicit point-in-time absence otherwise.
    ///
    /// # Errors
    /// `ReceiptExpiredUnknown` past retirement; `OutcomeUnknown` for a
    /// point-in-time absence (never proof of nonpublication).
    pub fn resolve(&self, id: CommandId) -> Result<Terminal, Refusal> {
        if let Some((_, outcome, _)) = self.receipts.get(&id) {
            return Ok(*outcome);
        }
        if id.epoch <= self.retired_through {
            return Err(Refusal::ReceiptExpiredUnknown);
        }
        // Point-in-time absence: NOT proof of nonpublication forever.
        Err(Refusal::OutcomeUnknown)
    }

    /// Head maintenance: rotate the open receipt epoch (a revision-moving
    /// CAS that does NOT move the state stamp).
    pub fn rotate_epoch(&mut self) {
        self.open_epoch += 1;
        self.revision += 1;
    }

    /// Retire receipts through a closed-epoch prefix: strictly below the
    /// open epoch, monotone, atomically dropping the retired rows.
    ///
    /// # Panics
    /// On a backward or open-epoch retirement — constructor-grade refusal.
    pub fn retire_through(&mut self, epoch: u64) {
        assert!(
            epoch < self.open_epoch && epoch >= self.retired_through,
            "retirement advances monotonically through closed epochs only"
        );
        self.retired_through = epoch;
        self.receipts.retain(|id, _| id.epoch > epoch);
        self.revision += 1;
    }

    /// Freeze the authority (migration intent): revision moves, state does
    /// not; unseen commands refuse while retained receipts resolve.
    pub fn freeze(&mut self, operation: u64) {
        self.lifecycle = Lifecycle::Frozen { operation };
        self.revision += 1;
    }

    /// Tombstone the authority: terminal; ordinary access refuses.
    pub fn delete(&mut self, operation: u64) {
        self.lifecycle = Lifecycle::Deleted { operation };
        self.revision += 1;
    }
}

/// One client-visible observation for the shared trace checker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    /// A CAS was attempted against `expected` and `won`.
    Cas { expected: u64, won: bool },
    /// A terminal receipt was returned for `id`.
    Receipt { id: CommandId, outcome: Terminal },
}

/// The independent trace checker P12 feeds production histories through:
/// at most one winner per expected revision, and one stable terminal
/// outcome per command id.
///
/// # Errors
/// A human-readable violation description.
pub fn verify_trace(events: &[Event]) -> Result<(), String> {
    let mut winners: BTreeSet<u64> = BTreeSet::new();
    let mut outcomes: BTreeMap<CommandId, Terminal> = BTreeMap::new();
    for event in events {
        match event {
            Event::Cas { expected, won } => {
                if *won && !winners.insert(*expected) {
                    return Err(format!(
                        "two successful CAS results against revision {expected}"
                    ));
                }
            }
            Event::Receipt { id, outcome } => {
                if let Some(prior) = outcomes.get(id) {
                    if prior != outcome {
                        return Err(format!(
                            "command {id:?} returned two distinct terminal \
                             outcomes: {prior:?} then {outcome:?}"
                        ));
                    }
                } else {
                    outcomes.insert(*id, *outcome);
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
