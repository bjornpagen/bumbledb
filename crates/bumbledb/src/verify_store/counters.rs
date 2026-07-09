//! The `S` pass: one cursor over `S | relation | stat`, reconciled against
//! the `F`-scan tallies — the stored row count must equal the scanned
//! cardinality, and the row-id high-water (the next id to assign) must
//! exceed every observed row id. A tallied relation with no stored counter
//! reads as zero (the commit path initializes lazily): rows on disk
//! convict the absent entry.

use std::collections::BTreeSet;
use std::ops::Bound;

use crate::error::Result;
use crate::schema::RelationId;
use crate::storage::keys::{self, StatKind};

use super::{StoreFinding, Sweep};

const ROW_COUNT: u8 = StatKind::RowCount as u8;
const HIGH_WATER: u8 = StatKind::RowIdHighWater as u8;

pub(super) fn sweep(s: &mut Sweep<'_, '_>) -> Result<()> {
    let txn = s.txn;
    let (lo, hi) = ([keys::NS_STAT], [keys::NS_STAT + 1]);
    let bounds: (Bound<&[u8]>, Bound<&[u8]>) = (Bound::Included(&lo[..]), Bound::Excluded(&hi[..]));
    let mut seen: BTreeSet<(RelationId, u8)> = BTreeSet::new();
    for entry in s.data.range(txn.raw(), &bounds)? {
        let (key, value) = entry?;
        if key.len() != keys::STAT_KEY_LEN {
            s.malformed(key, "S key length");
            continue;
        }
        let rel = RelationId(u32::from_be_bytes(
            key[1..5].try_into().expect("fixed-width slice"),
        ));
        if s.schema.relation_checked(rel).is_none() {
            s.malformed(key, "S key relation");
            continue;
        }
        let Ok(bytes) = <[u8; 8]>::try_from(value) else {
            s.malformed(key, "S counter value");
            continue;
        };
        let stored = u64::from_le_bytes(bytes);
        seen.insert((rel, key[5]));
        match key[5] {
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
                if let Some(tally) = s.tallies.get(&rel) {
                    if stored <= tally.max_row_id {
                        s.push(StoreFinding::RowIdHighWaterLow {
                            relation: rel,
                            stored,
                            max_row_id: tally.max_row_id,
                        });
                    }
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
