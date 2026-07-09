//! The `M` pass: one cursor over `M | relation | fact_hash`. Every entry's
//! row id must resolve to a live `F` fact whose blake3 matches the key —
//! the reverse direction of the `F` pass's membership check.

use std::ops::Bound;

use crate::encoding::fact_hash;
use crate::error::Result;
use crate::schema::RelationId;
use crate::storage::keys;

use super::{StoreFinding, Sweep};

pub(super) fn sweep(s: &mut Sweep<'_, '_>) -> Result<()> {
    let txn = s.txn;
    let (lo, hi) = ([keys::NS_MEMBERSHIP], [keys::NS_MEMBERSHIP + 1]);
    let bounds: (Bound<&[u8]>, Bound<&[u8]>) = (Bound::Included(&lo[..]), Bound::Excluded(&hi[..]));
    for entry in s.data.range(txn.raw(), &bounds)? {
        let (key, value) = entry?;
        if key.len() != keys::MEMBERSHIP_KEY_LEN {
            s.malformed(key, "M key length");
            continue;
        }
        let rel = RelationId(u32::from_be_bytes(
            key[1..5].try_into().expect("fixed-width slice"),
        ));
        if s.schema.relation_checked(rel).is_none() {
            s.malformed(key, "M key relation");
            continue;
        }
        let Ok(row_bytes) = <[u8; 8]>::try_from(value) else {
            s.malformed(key, "M row id");
            continue;
        };
        let row_id = u64::from_le_bytes(row_bytes);
        let resolves = s
            .fact(rel, row_id)?
            .is_some_and(|fact| fact_hash(fact) == key[5..]);
        if !resolves {
            s.push(StoreFinding::MembershipWithoutFact {
                relation: rel,
                row_id,
                membership_key: key.into(),
            });
        }
    }
    Ok(())
}
