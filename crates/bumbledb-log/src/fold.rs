//! The one state transition: an entry at a position turns a head into the
//! next head plus the receipts and committed changes it records. Every
//! replica, the machine and the reference fold in the tests share it.

use bumbledb::ChangeSet;

use crate::entry::{Body, Entry, Genesis, Migration, Thaw, Verdict};
use crate::head::{Head, Ledger, Mode, Rejection};
use crate::ids::{Millis, Seq};
use crate::receipt::{Delta, Receipt};

pub struct Folded<'e> {
    pub head: Head,
    pub receipts: Vec<Receipt>,
    pub commits: Vec<(&'e ChangeSet, Delta)>,
}

/// An entry that cannot follow the head: Genesis anywhere but first, or
/// anything else first.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Misplaced;

/// Fold the entry at `seq`, whose object the store stamped at `at`, onto
/// `head` (`None` before Genesis).
/// # Errors
/// [`Misplaced`] entries.
pub fn fold<'e>(
    head: Option<&Head>,
    seq: Seq,
    entry: &'e Entry,
    at: Millis,
) -> Result<Folded<'e>, Misplaced> {
    let head = match (head, &entry.body) {
        (None, Body::Genesis(genesis)) if seq == Seq::GENESIS => {
            return Ok(Folded {
                head: genesis_head(genesis),
                receipts: Vec::new(),
                commits: Vec::new(),
            });
        }
        (Some(head), body) if seq == head.seq.next() && !matches!(body, Body::Genesis(_)) => head,
        _ => return Err(Misplaced),
    };
    let mut next = Head {
        seq,
        ..head.clone()
    };
    let mut receipts = Vec::new();
    let mut commits = Vec::new();
    match &entry.body {
        Body::Genesis(_) => return Err(Misplaced),
        Body::Commands(decided) => {
            for decided in decided {
                if let Verdict::Committed { changes, delta } = &decided.verdict {
                    next.revision = next.revision.next();
                    commits.push((changes, *delta));
                }
                receipts.push(Receipt {
                    command: decided.command,
                    seq,
                    revision: next.revision,
                    outcome: decided.verdict.outcome(),
                });
            }
        }
        Body::Freeze(freeze) => {
            next.mode = Mode::Frozen {
                freeze: freeze.clone(),
                since: at,
            };
        }
        Body::Migration(migration) => next = migrated_head(head, seq, migration),
        Body::Thaw(Thaw::Lifted) => next.mode = Mode::Open,
        Body::Thaw(Thaw::Rejected(rejection)) => {
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

/// The head a migration entry at `seq` produces from `head`.
#[must_use]
pub fn migrated_head(head: &Head, seq: Seq, migration: &Migration) -> Head {
    let mut ledger = head.ledger.clone();
    ledger.applied.push(migration.migration.clone());
    Head {
        database: head.database,
        seq,
        revision: head.revision.next(),
        schema: migration.schema,
        ledger,
        mode: Mode::Open,
    }
}

fn genesis_head(genesis: &Genesis) -> Head {
    Head {
        database: genesis.database,
        seq: Seq::GENESIS,
        revision: crate::ids::Revision(0),
        schema: genesis.schema,
        ledger: Ledger {
            applied: vec![genesis.initial.clone()],
            rejected: Vec::new(),
        },
        mode: Mode::Open,
    }
}
