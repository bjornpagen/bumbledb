//! The `S` pass: one cursor over `S | relation | stat`, reconciled against
//! the `F`-scan tallies — the stored row count must equal the scanned
//! count, and the row-id high-water (the next id to assign) must
//! exceed every observed row id. A tallied relation with no stored counter
//! reads as zero (the commit path initializes lazily): rows on disk
//! convict the absent entry.
use std::collections::BTreeSet;

use crate::error::{CorruptionError, Result};
use crate::storage::catalog::CatalogRead;
use crate::storage::keys::{self, StatEntry, StatKind};
use bumbledb_theory::schema::RelationId;

use super::{StoreFinding, Sweep, for_namespace};

pub(super) fn sweep<C: CatalogRead + Copy>(s: &mut Sweep<'_, C>) -> Result<()> {
    let mut seen: BTreeSet<(RelationId, StatKind)> = BTreeSet::new();
    for_namespace(s.catalog, keys::Namespace::Stat, |key, value| {
        let Some((rel, stat)) = keys::parse_stat_key(key) else {
            s.malformed(key, "S key length");
            return Ok(());
        };
        if s.schema.relation_checked(rel).is_none() {
            s.malformed(key, "S key relation");
            return Ok(());
        }
        let Ok(bytes) = <[u8; 8]>::try_from(value) else {
            s.malformed(key, "S counter value");
            return Ok(());
        };
        let stored = u64::from_le_bytes(bytes);
        match stat {
            StatEntry::Known(kind) => {
                seen.insert((rel, kind));
                match kind {
                    StatKind::RowCount => {
                        let counted = s.tallies.get(&rel).map_or(0, |t| t.rows);
                        if stored != counted {
                            s.corrupt(CorruptionError::RowCountDesync {
                                relation: rel,
                                stored,
                                counted,
                            });
                        }
                    }
                    StatKind::RowIdHighWater => {
                        if s.schema.fresh_mint_field(rel).is_some() {
                            s.malformed(key, "S high-water on a fresh-keyed relation");
                        } else if let Some(tally) = s.tallies.get(&rel)
                            && stored <= tally.max_row_id
                        {
                            s.corrupt(CorruptionError::RowIdHighWaterLow {
                                relation: rel,
                                stored,
                                max_row_id: tally.max_row_id,
                            });
                        }
                    }
                }
            }
            StatEntry::Unknown(_) => s.malformed(key, "S stat kind"),
        }
        Ok(())
    })?;
    let absent: Vec<StoreFinding> = s
        .tallies
        .iter()
        .flat_map(|(&rel, tally)| {
            let count = (!seen.contains(&(rel, StatKind::RowCount))).then_some(
                StoreFinding::Corruption(CorruptionError::RowCountDesync {
                    relation: rel,
                    stored: 0,
                    counted: tally.rows,
                }),
            );

            let water = (!seen.contains(&(rel, StatKind::RowIdHighWater))
                && s.schema.fresh_mint_field(rel).is_none())
            .then_some(StoreFinding::Corruption(
                CorruptionError::RowIdHighWaterLow {
                    relation: rel,
                    stored: 0,
                    max_row_id: tally.max_row_id,
                },
            ));
            count.into_iter().chain(water)
        })
        .collect();
    s.findings.extend(absent);
    Ok(())
}
