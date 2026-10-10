//! The one state transition: an entry at a position turns a head into the
//! next head plus the receipts and committed changes it decides there. Every
//! replica, the machine and the reference fold in the tests share it.

use bumbledb::{ChangeSet, SchemaFingerprint};

use crate::entry::{Body, Entry, Genesis, MigrationId, Proposal, Thaw};
use crate::head::{Head, Ledger, Mode, Rejection};
use crate::ids::{Millis, Revision, Seq};
use crate::receipt::{Delta, Outcome, Receipt};

pub struct Folded {
    pub head: Head,
    pub receipts: Vec<Receipt>,
    pub commits: Vec<(ChangeSet, Delta)>,
}

/// An entry that cannot follow the head: Genesis anywhere but first,
/// anything else first, or a batch judged at or after its own position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Misplaced;

/// What an entry does where it landed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Standing {
    /// Nothing: a batch on a frozen head or judged at another schema, or a
    /// freeze on a frozen head or for a migration already applied or
    /// rejected. The head only moves to its position.
    Void,
    /// What it records: a batch landed right after the head it was judged
    /// at, or any other entry.
    Fresh,
    /// A batch landed past the head it was judged at: its commands are
    /// judged again where it landed.
    Rebased,
}

/// How `body` stands at `seq` on `head` (`None` before Genesis).
/// # Errors
/// [`Misplaced`] entries.
pub fn standing(head: Option<&Head>, seq: Seq, body: &Body) -> Result<Standing, Misplaced> {
    let head = match (head, body) {
        (None, Body::Genesis(_)) if seq == Seq::GENESIS => return Ok(Standing::Fresh),
        (Some(head), body) if seq == head.seq.next() && !matches!(body, Body::Genesis(_)) => head,
        _ => return Err(Misplaced),
    };
    Ok(match body {
        Body::Commands(batch) if batch.base >= seq => return Err(Misplaced),
        Body::Commands(batch) if head.mode != Mode::Open || batch.schema != head.schema => {
            Standing::Void
        }
        Body::Commands(batch) if batch.base == head.seq => Standing::Fresh,
        Body::Commands(_) => Standing::Rebased,
        Body::Freeze(freeze)
            if head.mode != Mode::Open || head.ledger.decided(&freeze.migration) =>
        {
            Standing::Void
        }
        Body::Genesis(_) | Body::Freeze(_) | Body::Migration(_) | Body::Thaw(_) => Standing::Fresh,
    })
}

/// Fold the entry at `seq`, whose object the store stamped at `at`, onto
/// `head` (`None` before Genesis). `decided` holds the commands a batch's
/// standing decides here, in order, each with the outcome that holds here:
/// as recorded when fresh, as judged here when rebased. Nothing else reads it.
/// # Errors
/// [`Misplaced`] entries.
pub fn fold(
    head: Option<&Head>,
    seq: Seq,
    entry: &Entry,
    at: Millis,
    decided: &[Proposal],
) -> Result<Folded, Misplaced> {
    let standing = standing(head, seq, &entry.body)?;
    let mut next = match (head, &entry.body) {
        (Some(head), _) => Head {
            seq,
            ..head.clone()
        },
        (None, Body::Genesis(genesis)) => genesis_head(genesis),
        (None, _) => return Err(Misplaced),
    };
    let mut receipts = Vec::new();
    let mut commits = Vec::new();
    match (&entry.body, standing) {
        (Body::Genesis(_), _) | (_, Standing::Void) => {}
        (Body::Commands(_), _) => {
            for Proposal { command, outcome } in decided {
                if let Outcome::Committed(delta) = outcome {
                    next.revision = next.revision.next();
                    commits.push((command.changes().clone(), *delta));
                }
                receipts.push(Receipt {
                    command: command.reference(),
                    seq,
                    revision: next.revision,
                    outcome: outcome.clone(),
                });
            }
        }
        (Body::Freeze(freeze), _) => {
            next.mode = Mode::Frozen {
                freeze: freeze.clone(),
                since: at,
            };
        }
        (Body::Migration(migration), _) => {
            next = migrated_head(&next, seq, &migration.migration, migration.schema);
        }
        (Body::Thaw(Thaw::Lifted), _) => next.mode = Mode::Open,
        (Body::Thaw(Thaw::Rejected(rejection)), _) => {
            next.ledger.rejected.push(Rejection::clone(rejection));
            next.mode = Mode::Open;
        }
    }
    Ok(Folded {
        head: next,
        receipts,
        commits,
    })
}

/// The head that applying `migration` at `seq` produces from `head`.
#[must_use]
pub fn migrated_head(
    head: &Head,
    seq: Seq,
    migration: &MigrationId,
    schema: SchemaFingerprint,
) -> Head {
    let mut ledger = head.ledger.clone();
    ledger.applied.push(migration.clone());
    Head {
        database: head.database,
        seq,
        revision: head.revision.next(),
        schema,
        ledger,
        mode: Mode::Open,
    }
}

fn genesis_head(genesis: &Genesis) -> Head {
    Head {
        database: genesis.database,
        seq: Seq::GENESIS,
        revision: Revision(0),
        schema: genesis.schema,
        ledger: Ledger {
            applied: vec![genesis.initial.clone()],
            rejected: Vec::new(),
        },
        mode: Mode::Open,
    }
}
