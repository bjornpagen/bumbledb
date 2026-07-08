use crate::encoding::field_bytes;
use crate::error::{CorruptionError, Error, Result};
use crate::schema::{ConstraintDescriptor, ConstraintId, Relation, RelationId, Schema};
use crate::storage::keys::{self, KeyBuf, StatKind, MAX_KEY};

use super::{Applier, FkProbe};

impl Applier<'_> {
    /// Phase-1 step: removes one fact's F/M/U/R entries if it exists in
    /// base state (else the delete is a no-op by construction).
    pub(super) fn delete_fact(
        &mut self,
        schema: &Schema,
        rel: RelationId,
        fact_bytes: &[u8],
    ) -> Result<()> {
        let relation = schema.relation(rel);
        let hash = crate::encoding::fact_hash(fact_bytes);
        let m_len = keys::membership_key(&mut self.key, rel, &hash);
        let Some(row_id_bytes) = self.data.get(self.txn.raw(), &self.key[..m_len])? else {
            return Ok(());
        };
        let row_id = decode_row_id(row_id_bytes)?;
        self.data.delete(self.txn.raw_mut(), &self.key[..m_len])?;
        let f_len = keys::fact_key(&mut self.key, rel, row_id);
        // A live M entry MUST have its F row (and every U guard below):
        // a miss is the M/F-disagreement corruption class, a hard error —
        // never silently scrubbed (docs/architecture/40-storage.md).
        if !self.data.delete(self.txn.raw_mut(), &self.key[..f_len])? {
            return Err(Error::Corruption(CorruptionError::MembershipDesync {
                relation: rel,
                row_id,
            }));
        }

        // Guard keys are re-derived by slicing constrained fields out of
        // fact_bytes — never a scan.
        for &cid in relation.unique_constraints() {
            derive_guard(
                relation,
                relation.constraint(cid).fields(),
                fact_bytes,
                &mut self.guard,
            );
            let u_len = keys::unique_key(&mut self.key, rel, cid, &self.guard);
            if !self.data.delete(self.txn.raw_mut(), &self.key[..u_len])? {
                return Err(Error::Corruption(CorruptionError::MembershipDesync {
                    relation: rel,
                    row_id,
                }));
            }
            if relation.fk_targeted().contains(&cid) {
                self.deleted_guards.insert((rel, cid, self.guard.clone()));
            }
        }
        for constraint in relation.constraints() {
            let ConstraintDescriptor::ForeignKey {
                target_relation,
                target_constraint,
                fields,
                ..
            } = constraint
            else {
                continue;
            };
            derive_guard(relation, fields, fact_bytes, &mut self.guard);
            let r_len = keys::restrict_key(
                &mut self.key,
                *target_relation,
                *target_constraint,
                &self.guard,
                rel,
                row_id,
            );
            self.data.delete(self.txn.raw_mut(), &self.key[..r_len])?;
        }
        self.changed = true;
        Ok(())
    }

    /// Phase-2 step: lands one fact's F/M/U/R entries if it is not in base
    /// state (else the insert is a no-op).
    pub(super) fn insert_fact(
        &mut self,
        schema: &Schema,
        rel: RelationId,
        fact_bytes: &[u8],
    ) -> Result<()> {
        let relation = schema.relation(rel);
        let hash = crate::encoding::fact_hash(fact_bytes);
        let m_len = keys::membership_key(&mut self.key, rel, &hash);
        if self.data.get(self.txn.raw(), &self.key[..m_len])?.is_some() {
            return Ok(());
        }
        let row_id = self.next_row_id(rel)?;
        self.data.put(
            self.txn.raw_mut(),
            &self.key[..m_len],
            row_id.to_le_bytes().as_slice(),
        )?;
        let f_len = keys::fact_key(&mut self.key, rel, row_id);
        self.data
            .put(self.txn.raw_mut(), &self.key[..f_len], fact_bytes)?;

        for &cid in relation.unique_constraints() {
            derive_guard(
                relation,
                relation.constraint(cid).fields(),
                fact_bytes,
                &mut self.guard,
            );
            let u_len = keys::unique_key(&mut self.key, rel, cid, &self.guard);
            // Every delete already landed and the insert set is
            // deduplicated, so an occupied guard here is a genuine
            // violation of the commit-time invariant.
            if self.data.get(self.txn.raw(), &self.key[..u_len])?.is_some() {
                return Err(Error::UniqueViolation {
                    relation: rel,
                    constraint: cid,
                    fact_bytes: fact_bytes.into(),
                });
            }
            self.data.put(
                self.txn.raw_mut(),
                &self.key[..u_len],
                row_id.to_le_bytes().as_slice(),
            )?;
            if relation.fk_targeted().contains(&cid) {
                self.inserted_guards.insert((rel, cid, self.guard.clone()));
            }
        }
        for (con_idx, constraint) in relation.constraints().iter().enumerate() {
            let ConstraintDescriptor::ForeignKey {
                target_relation,
                target_constraint,
                fields,
                ..
            } = constraint
            else {
                continue;
            };
            derive_guard(relation, fields, fact_bytes, &mut self.guard);
            let r_len = keys::restrict_key(
                &mut self.key,
                *target_relation,
                *target_constraint,
                &self.guard,
                rel,
                row_id,
            );
            // R puts are unconditional; target validation is the 40-storage doc's.
            self.data
                .put(self.txn.raw_mut(), &self.key[..r_len], [].as_slice())?;
            self.fk_probes
                .entry((*target_relation, *target_constraint, self.guard.clone()))
                .or_insert_with(|| FkProbe {
                    source_relation: rel,
                    source_constraint: ConstraintId(
                        u16::try_from(con_idx).expect("validated schema: constraint ids fit u16"),
                    ),
                    fact_bytes: fact_bytes.to_vec(),
                });
        }
        self.changed = true;
        Ok(())
    }

    /// Assigns the next row id for `rel`, lazily initializing from the
    /// stored `S` high-water (the value is the next id to assign;
    /// missing = 0).
    fn next_row_id(&mut self, rel: RelationId) -> Result<u64> {
        let next = match self.row_id_next.entry(rel) {
            std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::btree_map::Entry::Vacant(entry) => {
                // Own scratch: `self.key` still holds the caller's pending
                // membership key.
                let mut key: KeyBuf = [0; MAX_KEY];
                let len = keys::stat_key(&mut key, rel, StatKind::RowIdHighWater);
                let stored = match self.data.get(self.txn.raw(), &key[..len])? {
                    Some(bytes) => u64::from_le_bytes(bytes.try_into().map_err(|_| {
                        Error::Corruption(CorruptionError::MalformedValue("S row-id high water"))
                    })?),
                    None => 0,
                };
                entry.insert(stored)
            }
        };
        let row_id = *next;
        *next += 1;
        Ok(row_id)
    }
}

/// Concatenates the canonical encodings of the given constraint fields, in
/// field order, into `out` — the guard key body for both `U` and `R` keys.
fn derive_guard(
    relation: &Relation,
    fields: &[crate::schema::FieldId],
    fact_bytes: &[u8],
    out: &mut Vec<u8>,
) {
    out.clear();
    for &field in fields {
        out.extend_from_slice(field_bytes(
            fact_bytes,
            relation.layout(),
            usize::from(field.0),
        ));
    }
}

fn decode_row_id(bytes: &[u8]) -> Result<u64> {
    Ok(u64::from_le_bytes(bytes.try_into().map_err(|_| {
        Error::Corruption(CorruptionError::MalformedValue("M row id"))
    })?))
}
