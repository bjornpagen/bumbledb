//! The `M` pass: one cursor over `M | relation | fact_hash`. Every entry's
//! row id must resolve to a live `F` fact whose blake3 matches the key —
//! the reverse direction of the `F` pass's membership check.

use crate::encoding::fact_hash;
use crate::error::Result;
use crate::schema::RelationId;
use crate::storage::keys;

use super::{StoreFinding, Sweep, namespace};

pub(super) fn sweep(s: &mut Sweep<'_, '_>) -> Result<()> {
    let txn = s.txn;
    for entry in namespace(s.data, txn, keys::NS_MEMBERSHIP)? {
        let (key, value) = entry?;
        if key.len() != keys::MEMBERSHIP_KEY_LEN {
            s.malformed(key, "M key length");
            continue;
        }
        let rel = RelationId(u32::from_be_bytes(
            key[1..5].try_into().expect("fixed-width slice"),
        ));
        let Some(relation) = s.schema.relation_checked(rel) else {
            s.malformed(key, "M key relation");
            continue;
        };
        // Closed relations have no rows in the store: presence is the
        // finding (the F pass's exemption, mirrored).
        if relation.is_closed() {
            s.push(StoreFinding::ClosedRelationEntry {
                relation: rel,
                key: key.into(),
            });
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
