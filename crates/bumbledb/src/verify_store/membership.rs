//! The `M` pass: one cursor over `M | relation | fact_hash`. Every entry's
//! row id must resolve to a live `F` fact whose blake3 matches the key —
//! the reverse direction of the `F` pass's membership check.

use crate::encoding::fact_hash;
use crate::error::{CorruptionError, Result};
use crate::storage::catalog::CatalogRead;
use crate::storage::keys;

use super::{Sweep, for_namespace};

pub(super) fn sweep<C: CatalogRead + Copy>(s: &mut Sweep<'_, C>) -> Result<()> {
    for_namespace(s.catalog, keys::Namespace::Membership, |key, value| {
        let Some((rel, hash)) = keys::parse_membership_key(key) else {
            s.malformed(key, "M key length");
            return Ok(());
        };
        let Some(relation) = s.schema.relation_checked(rel) else {
            s.malformed(key, "M key relation");
            return Ok(());
        };

        if relation.body().closed_rows().is_some() {
            s.corrupt(CorruptionError::ClosedRelationEntry {
                relation: rel,
                key: key.into(),
            });
            return Ok(());
        }
        let Ok(row_bytes) = <[u8; 8]>::try_from(value) else {
            s.malformed(key, "M row id");
            return Ok(());
        };
        let row_id = u64::from_le_bytes(row_bytes);
        let resolves = s
            .fact(rel, row_id)?
            .is_some_and(|fact| fact_hash(fact.as_ref()) == *hash);
        if !resolves {
            s.corrupt(CorruptionError::MembershipWithoutFact {
                relation: rel,
                row_id,
                membership_key: key.into(),
            });
        }
        Ok(())
    })
}
