use std::ops::Bound;

use crate::error::{Conflict, CorruptionError, Error, Result, Violation};
use crate::storage::catalog::{
    Bounds, CatalogMap, CatalogWrite, PutOutcome, ReadCursor, WriteCursor,
};
use crate::storage::keys;
use bumbledb_theory::schema::{RelationId, StatementId};

use super::plan::{DeleteOp, DeterminantOp, InsertOp, MarkWeight};
use super::{Applier, decode_row_id};

/// The three-state insert landing: a free row id may put; a refused
/// fresh-key conflict still walks remaining keys for a complete
/// rejection set. Put helpers take [`FreeRow`] by type.
#[derive(Clone, Copy)]
enum Landing {
    Free(FreeRow),
    /// Occupied fresh-row slot. The id is the third landing state the
    /// type carries; puts take [`FreeRow`] only.
    Refused(
        #[expect(
            dead_code,
            reason = "Refused carries the conflicting row; put helpers are typed over Free"
        )]
        u64,
    ),
}

/// A row id that may receive `F`/`M`/`U`/`R` puts.
#[derive(Clone, Copy)]
struct FreeRow(u64);

impl<C: CatalogWrite> Applier<'_, '_, C> {
    /// Phase-1 step: removes one fact's F/M/U/R entries, every key byte
    /// taken from the plan. The fact exists in base state by the delta's
    /// net-disposition invariant the plan was derived from — a missing
    /// `M` entry means storage disagrees with what the plan *proved*,
    /// unambiguously corruption (docs/architecture/50-storage.md).
    pub(super) fn delete_fact(&mut self, op: &DeleteOp<'_>) -> Result<()> {
        let rel = op.core.relation;
        let m_key = keys::membership_key(rel, op.core.fact_hash);
        // One cursor descent serves the `M` read AND its delete: the
        // inclusive single-key range positions on exactly the entry (a
        // greater key falls past the end bound and reads as the same
        // miss), the row id decodes to an owned word, and the delete
        // lands at the cursor — never a second root-to-leaf descent.
        let row_id = {
            let bounds = Bounds {
                start: Bound::Included(m_key.as_slice()),
                end: Bound::Included(m_key.as_slice()),
            };
            let mut cursor = self.catalog.range_mut(CatalogMap::Data, bounds)?;
            let Some(entry) = ReadCursor::next(&mut cursor)? else {
                return Err(Error::Corruption(CorruptionError::DispositionDesync {
                    relation: rel,
                }));
            };
            let row_id = decode_row_id(entry.value)?;
            WriteCursor::del_current(&mut cursor)?;
            row_id
        };
        let f_key = keys::fact_key(rel, row_id);
        // A live M entry MUST have its F row (and every U determinant below):
        // a miss is the M/F-disagreement corruption class, a hard error —
        // never silently scrubbed (docs/architecture/50-storage.md).
        if !self.catalog.delete(CatalogMap::Data, &f_key)? {
            return Err(Error::Corruption(CorruptionError::MembershipDesync {
                relation: rel,
                row_id,
            }));
        }
        for determinant in &op.core.determinants {
            let u_key = keys::determinant_key(
                &mut self.key,
                rel,
                determinant.statement(),
                determinant.determinant().as_bytes(),
            );
            if !self.catalog.delete(CatalogMap::Data, u_key)? {
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
        // R-delete verification). One loop: containments and capacities
        // are the same key-only `R` write.
        for edge in &op.r_keys {
            let statement = edge.statement_id(self.schema);
            let r_key = keys::reverse_key(&mut self.key, statement, &edge.key_bytes, rel, row_id);
            self.catalog.delete(CatalogMap::Data, r_key)?;
        }
        Ok(())
    }

    /// Phase-2 step: lands one fact's F/M/U/R entries, enforcing every key
    /// statement — scalar by put-conflict, pointwise by the
    /// ordered-neighbor probe the plan marked, and the fresh-row auto-key
    /// by the `F` put-conflict itself (the one id allocator, R16: the
    /// first fresh field's value IS the row id, so an occupied `F` key is
    /// exactly that key's functionality violation — the identical
    /// conflict mechanism the `U` phase uses). A conflict RECORDS into the
    /// collector and the step continues (scan-complete: every determinant of
    /// every fact is judged, so the rejection carries the complete set of
    /// violated key statements; the transaction aborts after phase 2
    /// either way, so skipped puts persist nothing). An `F`-conflicted
    /// fact skips its remaining *puts* (its row id has no free slot) but
    /// still walks every additional key in `op.determinants`.
    /// The fact is absent from base state by the delta's net-disposition
    /// invariant the plan was derived from — a live `M` entry means
    /// storage disagrees with what the plan *proved*, unambiguously
    /// corruption (docs/architecture/50-storage.md).
    pub(super) fn insert_fact(&mut self, op: &InsertOp<'_>) -> Result<()> {
        let landing = self.resolve_landing(op)?;
        if let Landing::Free(row) = landing {
            self.put_membership_and_fact(op, row)?;
        }
        for determinant in &op.core.determinants {
            self.judge_determinant(op, landing, determinant)?;
        }
        if let Landing::Free(row) = landing {
            self.put_r_keys(op, row)?;
        }
        Ok(())
    }

    /// One key statement of one insert: each arm mints its own
    /// violation. Occupied exact bytes skip the put (the incumbent
    /// keeps the determinant) and still walk remaining keys.
    fn judge_determinant(
        &mut self,
        op: &InsertOp<'_>,
        landing: Landing,
        determinant: &DeterminantOp,
    ) -> Result<()> {
        let rel = op.core.relation;
        let fact = op.core.fact;
        match determinant {
            DeterminantOp::Scalar {
                statement,
                determinant,
            } => {
                let u_len =
                    keys::determinant_key(&mut self.key, rel, *statement, determinant.as_bytes())
                        .len();
                if self
                    .catalog
                    .get(CatalogMap::Data, &self.key[..u_len])?
                    .is_some()
                {
                    self.violations.push(Violation::functionality(
                        self.schema.cite(*statement),
                        fact.into(),
                        Conflict::Scalar,
                    ));
                    return Ok(());
                }
                if let Landing::Free(row) = landing {
                    self.put_data(u_len, row.0.to_le_bytes().as_slice())?;
                }
            }
            DeterminantOp::Pointwise {
                statement,
                determinant,
                tail,
            } => {
                let u_len =
                    keys::determinant_key(&mut self.key, rel, *statement, determinant.as_bytes())
                        .len();
                if let Some(value) = self.catalog.get(CatalogMap::Data, &self.key[..u_len])? {
                    let incumbent_row = decode_row_id(value.as_ref())?;
                    let incumbent = self.stored_fact(rel, incumbent_row)?;
                    self.violations.push(Violation::functionality(
                        self.schema.cite(*statement),
                        fact.into(),
                        Conflict::Pointwise { incumbent },
                    ));
                    return Ok(());
                }
                if let Landing::Free(row) = landing {
                    self.put_data(u_len, row.0.to_le_bytes().as_slice())?;
                }
                self.probe_neighbors(rel, *statement, u_len, *tail, fact)?;
            }
        }
        Ok(())
    }

    fn resolve_landing(&mut self, op: &InsertOp<'_>) -> Result<Landing> {
        let rel = op.core.relation;
        match op.fresh_row {
            Some(fresh) => {
                if self.catalog.fetch_fact(rel, fresh.row_id)?.is_some() {
                    if self
                        .catalog
                        .membership_row(rel, op.core.fact_hash)?
                        .is_some()
                    {
                        return Err(Error::Corruption(CorruptionError::DispositionDesync {
                            relation: rel,
                        }));
                    }
                    self.violations.push(Violation::functionality(
                        self.schema.cite(fresh.statement),
                        op.core.fact.into(),
                        Conflict::Scalar,
                    ));
                    Ok(Landing::Refused(fresh.row_id))
                } else {
                    Ok(Landing::Free(FreeRow(fresh.row_id)))
                }
            }
            None => Ok(Landing::Free(FreeRow(self.next_row_id(rel)?))),
        }
    }

    fn put_membership_and_fact(&mut self, op: &InsertOp<'_>, row: FreeRow) -> Result<()> {
        let rel = op.core.relation;
        let m_key = keys::membership_key(rel, op.core.fact_hash);
        let row_bytes = row.0.to_le_bytes();
        match self
            .catalog
            .put_no_overwrite(CatalogMap::Data, &m_key, &row_bytes)?
        {
            PutOutcome::Inserted => {}
            PutOutcome::Occupied => {
                return Err(Error::Corruption(CorruptionError::DispositionDesync {
                    relation: rel,
                }));
            }
        }
        let f_key = keys::fact_key(rel, row.0);
        self.catalog.put(CatalogMap::Data, &f_key, op.core.fact)?;
        Ok(())
    }

    fn put_r_keys(&mut self, op: &InsertOp<'_>, row: FreeRow) -> Result<()> {
        let rel = op.core.relation;
        for edge in &op.r_keys {
            let statement = edge.statement_id(self.schema);
            let r_len =
                keys::reverse_key(&mut self.key, statement, &edge.key_bytes, rel, row.0).len();
            match edge.weight {
                MarkWeight::Weighted(weight) => {
                    self.put_data(r_len, weight.to_le_bytes().as_slice())?;
                }
                MarkWeight::Unit => self.put_data(r_len, &[])?,
            }
        }
        Ok(())
    }

    /// The one phase-2 put: every `M`/`F`/`U`/`R` landing funnels through
    /// here. PRD-C1 gravestone — the `MDB_APPEND` candidate died at this
    /// site (the retired C1 heed-flags packet, git history). The
    /// refutation is structural: this put stream is NEVER key-ordered —
    /// a fact's `M` key embeds the blake3 fact hash (hash order, never
    /// input order, so no caller-side sort can order it), its own `F`
    /// key (`F` < `M` in first-byte namespace order) immediately follows
    /// the byte-greater `M` put, and host-looped `Db::write` commits
    /// cannot restore append order across transactions besides
    /// (`MDB_APPEND` compares against the database's last key, not the
    /// transaction's). The armed twin (`PutFlags::APPEND` on this put)
    /// proved the failure loud and atomic, never silent corruption: LMDB
    /// rejected the first unordered put with `MDB_KEYEXIST` (surfaced
    /// typed as `Lmdb`), the transaction aborted whole, and the store
    /// read clean after. So append mode is unreachable for this key
    /// architecture — landing it would need input-order `M` keys, which
    /// the content-hash fact identity forbids by design.
    fn put_data(&mut self, len: usize, value: &[u8]) -> Result<()> {
        self.catalog
            .put(CatalogMap::Data, &self.key[..len], value)?;
        Ok(())
    }

    /// Canonical fact bytes by row id, copied for a violation payload.
    fn stored_fact(&self, rel: RelationId, row_id: u64) -> Result<Box<[u8]>> {
        let stored = self
            .catalog
            .fetch_fact(rel, row_id)?
            .ok_or(Error::Corruption(CorruptionError::MissingFact {
                relation: rel,
                row_id,
            }))?;
        crate::storage::read::check_width(self.schema, rel, row_id, stored.as_ref())?;
        Ok(Box::from(stored.as_ref()))
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
    /// determinant is `prefix ‖ interval-tail` with order-preserving
    /// words (16-byte `start ‖ end`, or the 8-byte fixed start whose end
    /// derives from the type's width — [`crate::encoding::interval_words`]), so byte
    /// order within the group is start order, and word comparison is
    /// numeric comparison. Two half-open intervals `[s, e)` and `[x, y)`
    /// share a point iff `x < e && s < y`. The predecessor sorts strictly
    /// below the inserted key, so `ps <= s < e` and its test reduces to
    /// `pe > s`; the successor sorts strictly above, so `ne > ns >= s`
    /// and its test reduces to `ns < e`. Both comparisons are strict:
    /// `pe == s` and `ns == e` are adjacency, legal by half-open
    /// construction — no epsilon, no widening.
    fn probe_neighbors(
        &mut self,
        rel: RelationId,
        statement: StatementId,
        u_len: usize,
        tail: crate::schema::ValueType,
        fact_bytes: &[u8],
    ) -> Result<()> {
        let inserted = &self.key[..u_len];
        let tail_bytes = tail.width();
        let prefix = &inserted[..u_len - tail_bytes];
        let (start, end) = crate::encoding::interval_words(tail, &inserted[u_len - tail_bytes..])
            .expect("the plan derived this determinant from a validated fact");

        let mut incumbent_row: Option<u64> = None;
        if let Some(pred) = self.catalog.lower(CatalogMap::Data, inserted)?
            && pred.key.starts_with(prefix)
        {
            // Same statement, same determinant width: a prefix-sharing key
            // of any other length is corrupt data, a hard error.
            if pred.key.len() != u_len {
                return Err(Error::Corruption(CorruptionError::MalformedValue(
                    "U determinant key length",
                )));
            }
            let (_, pe) = crate::encoding::interval_words(tail, &pred.key[u_len - tail_bytes..])
                .ok_or(Error::Corruption(CorruptionError::MalformedValue(
                    "U determinant tail",
                )))?;
            // Predecessor `[ps, pe)`: violation iff `pe > s`.
            if pe > start {
                incumbent_row = Some(decode_row_id(pred.value)?);
            }
        }
        if incumbent_row.is_none()
            && let Some(succ) = self.catalog.greater(CatalogMap::Data, inserted)?
            && succ.key.starts_with(prefix)
        {
            if succ.key.len() != u_len {
                return Err(Error::Corruption(CorruptionError::MalformedValue(
                    "U determinant key length",
                )));
            }
            let (ns, _) = crate::encoding::interval_words(tail, &succ.key[u_len - tail_bytes..])
                .ok_or(Error::Corruption(CorruptionError::MalformedValue(
                    "U determinant tail",
                )))?;
            // Successor `[ns, ne)`: violation iff `ns < e`.
            if ns < end {
                incumbent_row = Some(decode_row_id(succ.value)?);
            }
        }
        let Some(row) = incumbent_row else {
            return Ok(());
        };
        // Cold aborting path: name the incumbent by its fact bytes via
        // row_id → F get (errors carry facts, never row ids).
        let incumbent = self.stored_fact(rel, row)?;
        self.violations.push(Violation::functionality(
            self.schema.cite(statement),
            fact_bytes.into(),
            Conflict::Pointwise { incumbent },
        ));
        Ok(())
    }

    /// Assigns the next row id for `rel`, lazily initializing from the
    /// stored `S` high-water (the value is the next id to assign;
    /// missing = 0).
    fn next_row_id(&mut self, rel: RelationId) -> Result<u64> {
        let next = match self.row_id_next.entry(rel) {
            std::collections::btree_map::Entry::Occupied(entry) => entry.into_mut(),
            std::collections::btree_map::Entry::Vacant(entry) => {
                let stored = self.catalog.row_id_high_water(rel)?;
                entry.insert(stored)
            }
        };
        let row_id = *next;
        *next += 1;
        Ok(row_id)
    }
}
