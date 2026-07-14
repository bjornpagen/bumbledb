use crate::error::{CorruptionError, Error, Result, Violation};
use crate::schema::{RelationId, StatementId};
use crate::storage::keys::{self, KeyBuf, MAX_KEY, StatKind};

use super::plan::FactOp;
use super::{Applier, crashpoint, decode_row_id, fact_by_row};

impl Applier<'_> {
    /// Phase-1 step: removes one fact's F/M/U/R entries, every key byte
    /// taken from the plan. The fact exists in base state by the delta's
    /// net-disposition invariant the plan was derived from — a missing
    /// `M` entry means storage disagrees with what the plan *proved*,
    /// unambiguously corruption (docs/architecture/50-storage.md).
    pub(super) fn delete_fact(&mut self, op: &FactOp<'_>) -> Result<()> {
        let rel = op.relation;
        let hash = crate::encoding::fact_hash(op.fact);
        let m_len = keys::membership_key(&mut self.key, rel, &hash);
        let Some(row_id_bytes) = self.data.get(self.txn.raw(), &self.key[..m_len])? else {
            return Err(Error::Corruption(CorruptionError::DispositionDesync {
                relation: rel,
            }));
        };
        let row_id = decode_row_id(row_id_bytes)?;
        self.data.delete(self.txn.raw_mut(), &self.key[..m_len])?;
        let f_len = keys::fact_key(&mut self.key, rel, row_id);
        // A live M entry MUST have its F row (and every U determinant below):
        // a miss is the M/F-disagreement corruption class, a hard error —
        // never silently scrubbed (docs/architecture/50-storage.md).
        if !self.data.delete(self.txn.raw_mut(), &self.key[..f_len])? {
            return Err(Error::Corruption(CorruptionError::MembershipDesync {
                relation: rel,
                row_id,
            }));
        }
        for determinant in &op.determinants {
            let u_len = keys::determinant_key(
                &mut self.key,
                rel,
                determinant.statement,
                determinant.determinant.as_bytes(),
            );
            if !self.data.delete(self.txn.raw_mut(), &self.key[..u_len])? {
                return Err(Error::Corruption(CorruptionError::MembershipDesync {
                    relation: rel,
                    row_id,
                }));
            }
        }
        // Outgoing R entries: the plan derived the same key bytes for the
        // insert-side puts, so the removal is byte-symmetric. Deleted
        // without verifying they existed — unlike F/M/U, a missing R
        // entry is not independently detectable here without re-deriving
        // every statement's edges; the class is deferred to the offline
        // sweeper, `Db::verify_store` (docs/architecture/50-storage.md,
        // R-delete verification).
        for edge in &op.edges {
            let r_len =
                keys::reverse_key(&mut self.key, edge.statement, &edge.key_bytes, rel, row_id);
            self.data.delete(self.txn.raw_mut(), &self.key[..r_len])?;
        }
        Ok(())
    }

    /// Phase-2 step: lands one fact's F/M/U/R entries, enforcing every key
    /// statement — scalar by put-conflict, pointwise by the
    /// ordered-neighbor probe the plan marked. A conflict RECORDS into the
    /// collector and the step continues (scan-complete: every determinant of
    /// every fact is judged, so the rejection carries the complete set of
    /// violated key statements; the transaction aborts after phase 2
    /// either way, so the skipped put persists nothing). The fact is
    /// absent from base state by the delta's net-disposition invariant
    /// the plan was derived from — a live `M` entry means storage
    /// disagrees with what the plan *proved*, unambiguously corruption
    /// (docs/architecture/50-storage.md).
    pub(super) fn insert_fact(&mut self, op: &FactOp<'_>) -> Result<()> {
        let rel = op.relation;
        let hash = crate::encoding::fact_hash(op.fact);
        let m_len = keys::membership_key(&mut self.key, rel, &hash);
        if self.data.get(self.txn.raw(), &self.key[..m_len])?.is_some() {
            return Err(Error::Corruption(CorruptionError::DispositionDesync {
                relation: rel,
            }));
        }
        let row_id = self.next_row_id(rel)?;
        self.data.put(
            self.txn.raw_mut(),
            &self.key[..m_len],
            row_id.to_le_bytes().as_slice(),
        )?;
        crashpoint!("mid-write-m");
        let f_len = keys::fact_key(&mut self.key, rel, row_id);
        self.data
            .put(self.txn.raw_mut(), &self.key[..f_len], op.fact)?;
        crashpoint!("mid-write-f");

        for determinant in &op.determinants {
            let u_len = keys::determinant_key(
                &mut self.key,
                rel,
                determinant.statement,
                determinant.determinant.as_bytes(),
            );
            // Every delete already landed and the insert set is
            // deduplicated, so an occupied determinant here is a genuine
            // violation of the final-state judgment. On a pointwise key
            // this exact-bytes conflict is the exact-duplicate-interval
            // case; the incumbent is named via its row_id (cold aborting
            // path — one extra get, docs/architecture/50-storage.md).
            // Recorded, put skipped (the incumbent keeps the determinant —
            // later conflicts against these exact bytes convict the same
            // statement), next determinant.
            if let Some(value) = self.data.get(self.txn.raw(), &self.key[..u_len])? {
                let incumbent = if determinant.pointwise {
                    let incumbent_row = decode_row_id(value)?;
                    Some(fact_by_row(self.data, self.txn.raw(), rel, incumbent_row)?.into())
                } else {
                    None
                };
                self.violations.push(Violation::Functionality {
                    statement: determinant.statement,
                    fact: op.fact.into(),
                    incumbent,
                });
                continue;
            }
            self.data.put(
                self.txn.raw_mut(),
                &self.key[..u_len],
                row_id.to_le_bytes().as_slice(),
            )?;
            crashpoint!("mid-write-u");
            if determinant.pointwise {
                // The exact put cannot detect overlap — only equality —
                // so a pointwise key additionally probes its ordered
                // neighbors within the scalar-prefix group.
                self.probe_neighbors(rel, determinant.statement, u_len, op.fact)?;
            }
        }
        for edge in &op.edges {
            let r_len =
                keys::reverse_key(&mut self.key, edge.statement, &edge.key_bytes, rel, row_id);
            self.data.put(self.txn.raw_mut(), &self.key[..r_len], &[])?;
            crashpoint!("mid-write-r");
        }
        Ok(())
    }

    /// The ordered-neighbor probe for a pointwise key: after the exact `U`
    /// put at `self.key[..u_len]`, checks the two adjacent determinant entries of
    /// the same scalar-prefix group for interval overlap. Two probes,
    /// O(log n), same write transaction — LMDB write txns read their own
    /// writes, so intra-delta overlaps are caught identically. An overlap
    /// records into the collector (the segment stays put — the commit
    /// aborts after phase 2, and one recorded conviction per group
    /// suffices: any later overlap in the group cites the same
    /// statement).
    ///
    /// Comparison directions, derived once from half-open semantics: a
    /// determinant is `prefix ‖ start ‖ end` with order-preserving 8-byte halves,
    /// so byte order within the group is `(start, end)` order, and byte
    /// comparison of halves is numeric comparison. Two half-open intervals
    /// `[s, e)` and `[x, y)` share a point iff `x < e && s < y`. The
    /// predecessor sorts strictly below the inserted key, so `ps <= s < e`
    /// and its test reduces to `pe > s`; the successor sorts strictly
    /// above, so `ne > ns >= s` and its test reduces to `ns < e`. Both
    /// comparisons are strict: `pe == s` and `ns == e` are adjacency,
    /// legal by half-open construction — no epsilon, no widening.
    fn probe_neighbors(
        &mut self,
        rel: RelationId,
        statement: StatementId,
        u_len: usize,
        fact_bytes: &[u8],
    ) -> Result<()> {
        let inserted = &self.key[..u_len];
        let prefix = &inserted[..u_len - 16];
        let start = &inserted[u_len - 16..u_len - 8];
        let end = &inserted[u_len - 8..u_len];

        let mut incumbent_row: Option<u64> = None;
        if let Some((pk, pv)) = self.data.get_lower_than(self.txn.raw(), inserted)?
            && pk.starts_with(prefix)
        {
            // Same statement, same determinant width: a prefix-sharing key
            // of any other length is corrupt data, a hard error.
            if pk.len() != u_len {
                return Err(Error::Corruption(CorruptionError::MalformedValue(
                    "U determinant key length",
                )));
            }
            // Predecessor `[ps, pe)`: violation iff `pe > s`.
            if &pk[u_len - 8..] > start {
                incumbent_row = Some(decode_row_id(pv)?);
            }
        }
        if incumbent_row.is_none()
            && let Some((nk, nv)) = self.data.get_greater_than(self.txn.raw(), inserted)?
            && nk.starts_with(prefix)
        {
            if nk.len() != u_len {
                return Err(Error::Corruption(CorruptionError::MalformedValue(
                    "U determinant key length",
                )));
            }
            // Successor `[ns, ne)`: violation iff `ns < e`.
            if &nk[u_len - 16..u_len - 8] < end {
                incumbent_row = Some(decode_row_id(nv)?);
            }
        }
        let Some(row) = incumbent_row else {
            return Ok(());
        };
        // Cold aborting path: name the incumbent by its fact bytes via
        // row_id → F get (errors carry facts, never row ids).
        let incumbent = fact_by_row(self.data, self.txn.raw(), rel, row)?;
        self.violations.push(Violation::Functionality {
            statement,
            fact: fact_bytes.into(),
            incumbent: Some(incumbent.into()),
        });
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
                    Some(bytes) => crate::storage::stored_u64(bytes, "S row-id high water")?,
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
