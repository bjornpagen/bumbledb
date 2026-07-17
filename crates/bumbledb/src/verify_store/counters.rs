//! The `S` pass: one cursor over `S | relation | stat`, reconciled against
//! the `F`-scan tallies — the stored row count must equal the scanned
//! cardinality, and the row-id high-water (the next id to assign) must
//! exceed every observed row id. A tallied relation with no stored counter
//! reads as zero (the commit path initializes lazily): rows on disk
//! convict the absent entry.

use std::collections::BTreeSet;

use crate::error::Result;
use crate::storage::keys::{self, StatKind};
use bumbledb_theory::schema::RelationId;

use super::{StoreFinding, Sweep, namespace};

const ROW_COUNT: u8 = StatKind::RowCount as u8;
const HIGH_WATER: u8 = StatKind::RowIdHighWater as u8;

pub(super) fn sweep(s: &mut Sweep<'_, '_>) -> Result<()> {
    let txn = s.txn;
    let mut seen: BTreeSet<(RelationId, u8)> = BTreeSet::new();
    for entry in namespace(s.data, txn, keys::NS_STAT)? {
        let (key, value) = entry?;
        let Some((rel, stat)) = keys::parse_stat_key(key) else {
            s.malformed(key, "S key length");
            continue;
        };
        if s.schema.relation_checked(rel).is_none() {
            s.malformed(key, "S key relation");
            continue;
        }
        let Ok(bytes) = <[u8; 8]>::try_from(value) else {
            s.malformed(key, "S counter value");
            continue;
        };
        let stored = u64::from_le_bytes(bytes);
        seen.insert((rel, stat));
        match stat {
            ROW_COUNT => {
                let counted = s.tallies.get(&rel).map_or(0, |t| t.rows);
                if stored != counted {
                    s.push(StoreFinding::RowCountDesync {
                        relation: rel,
                        stored,
                        counted,
                    });
                }
            }
            HIGH_WATER => {
                if let Some(tally) = s.tallies.get(&rel)
                    && stored <= tally.max_row_id
                {
                    s.push(StoreFinding::RowIdHighWaterLow {
                        relation: rel,
                        stored,
                        max_row_id: tally.max_row_id,
                    });
                }
            }
            _ => s.malformed(key, "S stat kind"),
        }
    }
    let absent: Vec<StoreFinding> = s
        .tallies
        .iter()
        .flat_map(|(&rel, tally)| {
            let count =
                (!seen.contains(&(rel, ROW_COUNT))).then_some(StoreFinding::RowCountDesync {
                    relation: rel,
                    stored: 0,
                    counted: tally.rows,
                });
            let water =
                (!seen.contains(&(rel, HIGH_WATER))).then_some(StoreFinding::RowIdHighWaterLow {
                    relation: rel,
                    stored: 0,
                    max_row_id: tally.max_row_id,
                });
            count.into_iter().chain(water)
        })
        .collect();
    s.findings.extend(absent);
    Ok(())
}
