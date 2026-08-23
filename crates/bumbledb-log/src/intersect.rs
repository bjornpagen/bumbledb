//! The pairwise loser decision: subsumed, disjoint, or conflict. The
//! winner's footprint is recomputed from its ops — never read from the
//! carried section, so an understating writer cannot steer losers onto
//! the republish path. Disjoint is strict: no shared key of any
//! existential class, commute cells included (L6's hypothesis is false
//! without excluding them), and every shared capacity parent passes
//! the evaporation-widened interval test; everything else re-judges.

use std::collections::BTreeMap;

use bumbledb::schema::StatementId;

use crate::codec::Op;
use crate::footprint::{
    CapacityKey, CapacityProfile, Entry, FootprintError, Vocabulary, capacity_profiles, footprint,
};

/// The base-state quantities the interval test prices a shared parent
/// group against. The caller supplies them (and can re-run
/// [`capacity_cell`] against winner-updated measures without touching
/// the wire).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BaseMeasure {
    pub measure: u64,
    pub floor: u64,
    pub ceiling: Option<u64>,
}

impl BaseMeasure {
    fn slack_up(&self) -> Option<i128> {
        self.ceiling
            .map(|ceiling| i128::from(ceiling) - i128::from(self.measure))
    }

    fn slack_down(&self) -> i128 {
        i128::from(self.measure) - i128::from(self.floor)
    }
}

/// Why a pair fails to commute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictCause {
    /// A shared fact id, either mode: op effects are not independent.
    Fact { fid: [u8; 32] },

    /// A shared key-statement determinant.
    Key {
        statement: StatementId,
        key: [u8; 32],
    },

    /// A shared containment target group.
    Containment {
        statement: StatementId,
        key: [u8; 32],
    },

    /// A shared parent group whose worst-case interval endpoints
    /// violate a slack bound.
    CapacityInterval {
        statement: StatementId,
        key: [u8; 32],
    },

    /// A parent row's own move racing the other batch at the group.
    CapacityParent {
        statement: StatementId,
        key: [u8; 32],
    },

    /// A shared parent group the caller supplied no base measure for —
    /// conservatively priced as a conflict; re-judgment is always
    /// sound.
    CapacityMeasureMissing {
        statement: StatementId,
        key: [u8; 32],
    },
}

/// The loser's three outcomes at a lost slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoserDecision {
    /// Every F entry of the loser appears in the winner with the same
    /// mode: the winner already performed the loser's net effect;
    /// publish nothing.
    Subsumed,

    /// Full key disjointness: republish with a re-addressed header,
    /// verdict untouched (L7 licenses it).
    Disjoint,

    /// Anything else: rebuild the base and re-judge the recorded ops.
    Conflict(ConflictCause),
}

/// One shared parent group's verdict under the interval test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapacityCell {
    Commutes,
    IntervalExceeded,
    ParentRace,
    MeasureMissing,
}

/// The W matrix cell for one shared parent group: parent removal
/// tolerates only an inert other side; parent addition commutes with
/// child adds alone; child intervals must jointly respect both slack
/// bounds at their worst-case endpoints.
#[must_use]
pub fn capacity_cell(
    loser: &CapacityProfile,
    winner: &CapacityProfile,
    base: Option<&BaseMeasure>,
) -> CapacityCell {
    for (mover, other) in [(loser, winner), (winner, loser)] {
        if mover.parent_remove && !other.is_inert() {
            return CapacityCell::ParentRace;
        }
        if mover.parent_add && (other.min() < 0 || other.parent_add || other.parent_remove) {
            return CapacityCell::ParentRace;
        }
    }

    if loser.min() == 0 && loser.max() == 0 && winner.min() == 0 && winner.max() == 0 {
        return CapacityCell::Commutes;
    }

    let Some(base) = base else {
        return CapacityCell::MeasureMissing;
    };
    if let Some(slack_up) = base.slack_up()
        && loser.max() + winner.max() > slack_up
    {
        return CapacityCell::IntervalExceeded;
    }
    if loser.min() + winner.min() < -base.slack_down() {
        return CapacityCell::IntervalExceeded;
    }
    CapacityCell::Commutes
}

/// The pairwise decision. Inputs are the loser's own footprint and
/// ops plus the winner's ops; the winner's footprint and both sides'
/// capacity profiles are recomputed here.
pub fn intersect(
    vocabulary: &Vocabulary,
    loser_footprint: &[Entry],
    loser_ops: &[Op],
    winner_ops: &[Op],
    base: &BTreeMap<CapacityKey, BaseMeasure>,
) -> Result<LoserDecision, FootprintError> {
    let winner_footprint = footprint(vocabulary, winner_ops)?;

    if subsumed(loser_footprint, &winner_footprint) {
        return Ok(LoserDecision::Subsumed);
    }

    if let Some(cause) = shared_existential(loser_footprint, &winner_footprint) {
        return Ok(LoserDecision::Conflict(cause));
    }

    let loser_profiles = capacity_profiles(vocabulary, loser_ops)?;
    let winner_profiles = capacity_profiles(vocabulary, winner_ops)?;
    for (key, loser_profile) in &loser_profiles {
        let Some(winner_profile) = winner_profiles.get(key) else {
            continue;
        };
        let cell = capacity_cell(loser_profile, winner_profile, base.get(key));
        match cell {
            CapacityCell::Commutes => {}
            CapacityCell::IntervalExceeded => {
                return Ok(LoserDecision::Conflict(ConflictCause::CapacityInterval {
                    statement: key.statement,
                    key: key.key,
                }));
            }
            CapacityCell::ParentRace => {
                return Ok(LoserDecision::Conflict(ConflictCause::CapacityParent {
                    statement: key.statement,
                    key: key.key,
                }));
            }
            CapacityCell::MeasureMissing => {
                return Ok(LoserDecision::Conflict(
                    ConflictCause::CapacityMeasureMissing {
                        statement: key.statement,
                        key: key.key,
                    },
                ));
            }
        }
    }

    Ok(LoserDecision::Disjoint)
}

/// Every F entry of the loser appears in the winner with the same
/// mode. Both sections are sorted, F class first, so this is one
/// two-pointer walk over the F prefixes.
fn subsumed(loser: &[Entry], winner: &[Entry]) -> bool {
    let mut winner_facts = winner
        .iter()
        .filter_map(|entry| match entry {
            Entry::Fact { fid, mode } => Some((*fid, *mode)),
            _ => None,
        })
        .peekable();

    for entry in loser {
        let Entry::Fact { fid, mode } = entry else {
            break;
        };
        loop {
            match winner_facts.peek() {
                Some((wfid, wmode)) if wfid == fid => {
                    if wmode != mode {
                        return false;
                    }
                    break;
                }
                Some((wfid, _)) if *wfid < *fid => {
                    winner_facts.next();
                }
                _ => return false,
            }
        }
    }
    true
}

/// The first shared existential-class coordinate (F, K, or C), mode
/// ignored — the strict-disjointness scan. W coordinates are priced by
/// the interval test instead.
fn shared_existential(loser: &[Entry], winner: &[Entry]) -> Option<ConflictCause> {
    let mut l = 0;
    let mut w = 0;
    while l < loser.len() && w < winner.len() {
        let lk = loser[l].share_key();
        let wk = winner[w].share_key();
        match lk.cmp(&wk) {
            std::cmp::Ordering::Less => l += 1,
            std::cmp::Ordering::Greater => w += 1,
            std::cmp::Ordering::Equal => {
                match &loser[l] {
                    Entry::Fact { fid, .. } => {
                        return Some(ConflictCause::Fact { fid: *fid });
                    }
                    Entry::Key { statement, key } => {
                        return Some(ConflictCause::Key {
                            statement: *statement,
                            key: *key,
                        });
                    }
                    Entry::Containment { statement, key, .. } => {
                        return Some(ConflictCause::Containment {
                            statement: *statement,
                            key: *key,
                        });
                    }
                    Entry::Capacity { .. } => {}
                }
                l += 1;
            }
        }
    }
    None
}
