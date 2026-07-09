use crate::error::{CorruptionError, Error, Result};
use crate::schema::{RelationId, Resolved, Schema, StatementDescriptor, StatementId};
use crate::storage::keys::{self, KeyBuf, StatKind, MAX_KEY};

use super::{judgment, Applier};

impl Applier<'_> {
    /// Phase-1 step: removes one fact's F/M/U entries. The fact exists in
    /// base state by the delta's net-disposition invariant — a missing `M`
    /// entry means storage disagrees with what the delta proved at op
    /// time, unambiguously corruption (docs/architecture/50-storage.md).
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
            return Err(Error::Corruption(CorruptionError::DispositionDesync {
                relation: rel,
            }));
        };
        let row_id = decode_row_id(row_id_bytes)?;
        self.data.delete(self.txn.raw_mut(), &self.key[..m_len])?;
        let f_len = keys::fact_key(&mut self.key, rel, row_id);
        // A live M entry MUST have its F row (and every U guard below):
        // a miss is the M/F-disagreement corruption class, a hard error —
        // never silently scrubbed (docs/architecture/50-storage.md).
        if !self.data.delete(self.txn.raw_mut(), &self.key[..f_len])? {
            return Err(Error::Corruption(CorruptionError::MembershipDesync {
                relation: rel,
                row_id,
            }));
        }

        // Guard keys are re-derived by slicing projected fields out of
        // fact_bytes — never a scan; interval fields slice as their whole
        // 16 bytes (`keys::guard_bytes`).
        for &sid in relation.keys() {
            keys::guard_bytes(
                relation.layout(),
                schema.statement(sid).key_projection(),
                fact_bytes,
                &mut self.guard,
            );
            let u_len = keys::guard_key(&mut self.key, rel, sid, &self.guard);
            if !self.data.delete(self.txn.raw_mut(), &self.key[..u_len])? {
                return Err(Error::Corruption(CorruptionError::MembershipDesync {
                    relation: rel,
                    row_id,
                }));
            }
            if !schema.dependents(sid).is_empty() {
                self.deleted_guards.insert((sid, self.guard.clone()));
            }
        }
        // Outgoing R entries: the same key derivation as the insert-side
        // puts, so the removal is byte-symmetric. Deleted without
        // verifying they existed — unlike F/M/U, a missing R entry is not
        // independently detectable here without re-deriving every
        // statement's edges; the class is deferred to the offline
        // sweeper, `Db::verify_store` (docs/architecture/50-storage.md,
        // R-delete verification).
        for &sid in relation.outgoing() {
            if let Some(r_len) = self.reverse_key_for(schema, rel, sid, fact_bytes, row_id) {
                self.data.delete(self.txn.raw_mut(), &self.key[..r_len])?;
            }
        }
        Ok(())
    }

    /// Phase-2 step: lands one fact's F/M/U entries, enforcing every key
    /// statement — scalar by put-conflict, pointwise by the
    /// ordered-neighbor probe. The fact is absent from base state by the
    /// delta's net-disposition invariant — a live `M` entry means storage
    /// disagrees with what the delta proved at op time, unambiguously
    /// corruption (docs/architecture/50-storage.md).
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
        let f_len = keys::fact_key(&mut self.key, rel, row_id);
        self.data
            .put(self.txn.raw_mut(), &self.key[..f_len], fact_bytes)?;

        for &sid in relation.keys() {
            let statement = schema.statement(sid);
            let Resolved::Functionality { interval_position } = &statement.resolved else {
                unreachable!("validated schema: relation keys resolve as Functionality")
            };
            let pointwise = interval_position.is_some();
            keys::guard_bytes(
                relation.layout(),
                statement.key_projection(),
                fact_bytes,
                &mut self.guard,
            );
            let u_len = keys::guard_key(&mut self.key, rel, sid, &self.guard);
            // Every delete already landed and the insert set is
            // deduplicated, so an occupied guard here is a genuine
            // violation of the final-state judgment. On a pointwise key
            // this exact-bytes conflict is the exact-duplicate-interval
            // case; the incumbent is named via its row_id (cold aborting
            // path — one extra get, docs/architecture/50-storage.md).
            if let Some(value) = self.data.get(self.txn.raw(), &self.key[..u_len])? {
                let incumbent = if pointwise {
                    let incumbent_row = decode_row_id(value)?;
                    Some(self.fetch_fact(rel, incumbent_row)?)
                } else {
                    None
                };
                return Err(Error::FunctionalityViolation {
                    statement: sid,
                    fact: fact_bytes.into(),
                    incumbent,
                });
            }
            self.data.put(
                self.txn.raw_mut(),
                &self.key[..u_len],
                row_id.to_le_bytes().as_slice(),
            )?;
            if pointwise {
                // The exact put cannot detect overlap — only equality —
                // so a pointwise key additionally probes its ordered
                // neighbors within the scalar-prefix group.
                self.probe_neighbors(rel, sid, u_len, fact_bytes)?;
            }
            if !schema.dependents(sid).is_empty() {
                self.inserted_guards.insert((sid, self.guard.clone()));
            }
        }
        // One R entry per outgoing containment statement whose source
        // selection this fact satisfies — conditional containments write
        // reverse edges only for facts inside their σ
        // (docs/architecture/50-storage.md § key layout).
        for &sid in relation.outgoing() {
            if let Some(r_len) = self.reverse_key_for(schema, rel, sid, fact_bytes, row_id) {
                self.data.put(self.txn.raw_mut(), &self.key[..r_len], &[])?;
            }
        }
        Ok(())
    }

    /// Derives one outgoing statement's `R` key into `self.key` — the
    /// source fact's projection laid down in the target key's guard order
    /// (`keys::permuted_guard_bytes`), statement-scoped. Returns the key
    /// length, or `None` when the fact is outside the statement's source
    /// selection (no reverse edge exists for it, by design). The same
    /// derivation serves the insert-phase put and the delete-phase
    /// removal, which is what makes them byte-symmetric.
    fn reverse_key_for(
        &mut self,
        schema: &Schema,
        rel: RelationId,
        sid: StatementId,
        fact_bytes: &[u8],
        row_id: u64,
    ) -> Option<usize> {
        let relation = schema.relation(rel);
        let statement = schema.statement(sid);
        let StatementDescriptor::Containment { source, .. } = &statement.descriptor else {
            unreachable!("validated schema: outgoing ids name Containment statements")
        };
        let Resolved::Containment {
            key_permutation, ..
        } = &statement.resolved
        else {
            unreachable!("validated schema: Containment resolves as Containment")
        };
        if !judgment::satisfies(
            &self.selections.containment(sid).source,
            relation.layout(),
            fact_bytes,
        ) {
            return None;
        }
        // Scratch reuse: the key-statement loop is done with `self.guard`.
        keys::permuted_guard_bytes(
            relation.layout(),
            &source.projection,
            key_permutation,
            fact_bytes,
            &mut self.guard,
        );
        Some(keys::reverse_key(
            &mut self.key,
            sid,
            &self.guard,
            rel,
            row_id,
        ))
    }

    /// The ordered-neighbor probe for a pointwise key: after the exact `U`
    /// put at `self.key[..u_len]`, checks the two adjacent guard entries of
    /// the same scalar-prefix group for interval overlap. Two probes,
    /// O(log n), same write transaction — LMDB write txns read their own
    /// writes, so intra-delta overlaps are caught identically.
    ///
    /// Comparison directions, derived once from half-open semantics: a
    /// guard is `prefix ‖ start ‖ end` with order-preserving 8-byte halves,
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
        statement: crate::schema::StatementId,
        u_len: usize,
        fact_bytes: &[u8],
    ) -> Result<()> {
        let inserted = &self.key[..u_len];
        let prefix = &inserted[..u_len - 16];
        let start = &inserted[u_len - 16..u_len - 8];
        let end = &inserted[u_len - 8..u_len];

        let mut incumbent_row: Option<u64> = None;
        if let Some((pk, pv)) = self.data.get_lower_than(self.txn.raw(), inserted)? {
            if pk.starts_with(prefix) {
                // Same statement, same guard width: a prefix-sharing key
                // of any other length is corrupt data, a hard error.
                if pk.len() != u_len {
                    return Err(Error::Corruption(CorruptionError::MalformedValue(
                        "U guard key length",
                    )));
                }
                // Predecessor `[ps, pe)`: violation iff `pe > s`.
                if &pk[u_len - 8..] > start {
                    incumbent_row = Some(decode_row_id(pv)?);
                }
            }
        }
        if incumbent_row.is_none() {
            if let Some((nk, nv)) = self.data.get_greater_than(self.txn.raw(), inserted)? {
                if nk.starts_with(prefix) {
                    if nk.len() != u_len {
                        return Err(Error::Corruption(CorruptionError::MalformedValue(
                            "U guard key length",
                        )));
                    }
                    // Successor `[ns, ne)`: violation iff `ns < e`.
                    if &nk[u_len - 16..u_len - 8] < end {
                        incumbent_row = Some(decode_row_id(nv)?);
                    }
                }
            }
        }
        let Some(row) = incumbent_row else {
            return Ok(());
        };
        // Cold aborting path: name the incumbent by its fact bytes via
        // row_id → F get (errors carry facts, never row ids).
        let incumbent = self.fetch_fact(rel, row)?;
        Err(Error::FunctionalityViolation {
            statement,
            fact: fact_bytes.into(),
            incumbent: Some(incumbent),
        })
    }

    /// Fetches a fact's canonical bytes by row id — the incumbent lookup
    /// of an aborting violation (cold path; one sanctioned extra get).
    fn fetch_fact(&self, rel: RelationId, row_id: u64) -> Result<Box<[u8]>> {
        // Own scratch: `self.key` still holds the caller's guard key.
        let mut key: KeyBuf = [0; MAX_KEY];
        let f_len = keys::fact_key(&mut key, rel, row_id);
        Ok(self
            .data
            .get(self.txn.raw(), &key[..f_len])?
            .ok_or(Error::Corruption(CorruptionError::MissingFact {
                relation: rel,
                row_id,
            }))?
            .into())
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

pub(super) fn decode_row_id(bytes: &[u8]) -> Result<u64> {
    Ok(u64::from_le_bytes(bytes.try_into().map_err(|_| {
        Error::Corruption(CorruptionError::MalformedValue("M row id"))
    })?))
}
