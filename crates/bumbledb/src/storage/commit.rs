//! Commit application (PRD 07): phases 1-2 of the canonical commit order —
//! all deletes, then all inserts — maintaining F/M/U/R and detecting unique
//! violations (`docs/architecture/40-storage.md`).
//!
//! Because every delete lands before any insert and the insert set is
//! deduplicated by construction, a `U` conflict during inserts is a genuine
//! unique violation; user operation order inside the transaction is
//! semantically irrelevant. PRD 08 extends this into the full commit
//! (FK validation, counters, tx id, LMDB commit).

use std::collections::{BTreeMap, BTreeSet};

use crate::encoding::field_bytes;
use crate::error::{CorruptionError, Error, FkViolation, Result};
use crate::schema::{ConstraintDescriptor, ConstraintId, Relation, RelationId, Schema};
use crate::storage::delta::{Disposition, WriteDelta};
use crate::storage::env::{Environment, WriteTxn};
use crate::storage::keys::{self, KeyBuf, StatKind, MAX_KEY};

/// One forward-FK probe PRD 08 must run: collected at insert time so
/// validation never re-derives key slices.
#[derive(Debug)]
pub struct FkProbe {
    /// The referencing side, for the violation error.
    pub source_relation: RelationId,
    pub source_constraint: ConstraintId,
    /// The `U` key components to probe.
    pub target_relation: RelationId,
    pub target_constraint: ConstraintId,
    pub guard: Vec<u8>,
    /// The offending fact, for the violation error.
    pub fact_bytes: Vec<u8>,
}

/// The applied-but-uncommitted state after phases 1-2, carrying the open
/// LMDB write transaction plus the bookkeeping PRD 08 consumes.
pub struct Applied<'env, 's> {
    /// The open, uncommitted LMDB write transaction.
    pub txn: WriteTxn<'env>,
    /// The source delta (its counters flush in PRD 08's phase 4).
    pub delta: WriteDelta<'s>,
    /// Whether any insert or delete actually applied (drives PRD 08's
    /// tx-id-advances-iff-state-changed rule).
    pub changed: bool,
    /// Per-relation next row id after this apply (flushed to `S` by PRD 08).
    pub row_id_next: BTreeMap<RelationId, u64>,
    /// Unique keys deleted in phase 1, restricted to FK-targeted
    /// constraints — the Restrict scan set.
    pub deleted_guards: BTreeSet<(RelationId, ConstraintId, Vec<u8>)>,
    /// Unique keys established in phase 2 for FK-targeted constraints —
    /// subtracted from `deleted_guards` before Restrict scanning.
    pub inserted_guards: BTreeSet<(RelationId, ConstraintId, Vec<u8>)>,
    /// Forward-FK probes for every inserted fact.
    pub fk_probes: Vec<FkProbe>,
}

/// Applies the delta to LMDB in canonical order: phase 1 all deletes, then
/// phase 2 all inserts. Opens the LMDB write transaction here — nothing
/// touched a data page before this call (PRD 06's lock-window rule).
///
/// # Errors
///
/// `UniqueViolation` when two live facts claim one unique key; `Lmdb` on
/// storage failure; `Corruption` on malformed base state. On any error the
/// transaction is dropped — nothing persists.
///
/// # Panics
///
/// Only on programmer-invariant violations (validated-schema id widths).
pub fn apply<'env, 's>(delta: WriteDelta<'s>, env: &'env Environment) -> Result<Applied<'env, 's>> {
    let txn = env.write_txn()?;
    let mut applier = Applier {
        txn,
        data: env.data(),
        changed: false,
        row_id_next: BTreeMap::new(),
        deleted_guards: BTreeSet::new(),
        inserted_guards: BTreeSet::new(),
        fk_probes: Vec::new(),
        key: [0; MAX_KEY],
        guard: Vec::new(),
    };

    // Phase 1: all deletes, then phase 2: all inserts — the canonical order
    // that makes user operation order semantically irrelevant.
    for (rel, fact_bytes, disposition) in delta.entries() {
        if disposition == Disposition::Delete {
            applier.delete_fact(delta.schema(), rel, fact_bytes)?;
        }
    }
    for (rel, fact_bytes, disposition) in delta.entries() {
        if disposition == Disposition::Insert {
            applier.insert_fact(delta.schema(), rel, fact_bytes)?;
        }
    }

    let Applier {
        txn,
        changed,
        row_id_next,
        deleted_guards,
        inserted_guards,
        fk_probes,
        ..
    } = applier;
    Ok(Applied {
        txn,
        delta,
        changed,
        row_id_next,
        deleted_guards,
        inserted_guards,
        fk_probes,
    })
}

/// The commit outcome: whether logical state changed, and the resulting
/// storage generation (PRD 11's cache-eviction subscriber; PRD 28 wires it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CommitReport {
    pub changed: bool,
    pub new_generation: u64,
}

/// The full commit (PRD 08): apply (phases 1-2), FK validation against the
/// final state (phase 3), counter flush (phase 4), LMDB commit (phase 5).
/// Any error anywhere aborts — nothing persists.
///
/// # Errors
///
/// `UniqueViolation`/`ForeignKeyViolation` on constraint violations in the
/// final state; `Lmdb`/`Corruption` on storage failure.
///
/// # Panics
///
/// Only on programmer-invariant violations (validated-schema id widths,
/// well-formed R keys this same commit wrote).
pub fn commit(delta: WriteDelta<'_>, env: &Environment) -> Result<CommitReport> {
    // An all-no-op delta commits without touching LMDB at all: the tx id
    // does not advance and no cached image is invalidated. Pending serial
    // allocations and interns are dropped — none are observable.
    if delta.is_empty() {
        let rtxn = env.read_txn()?;
        return Ok(CommitReport {
            changed: false,
            new_generation: rtxn.generation()?,
        });
    }

    let applied = apply(delta, env)?;
    if !applied.changed {
        let generation = applied.txn.generation()?;
        applied.txn.abort(); // nothing was written; equivalent to commit
        return Ok(CommitReport {
            changed: false,
            new_generation: generation,
        });
    }
    let Applied {
        mut txn,
        delta,
        row_id_next,
        deleted_guards,
        inserted_guards,
        fk_probes,
        ..
    } = applied;
    let data = env.data();
    let mut key: KeyBuf = [0; MAX_KEY];

    // Phase 3a: forward FK validation — every inserted fact's targets must
    // resolve in the final state (the write txn reads its own writes).
    for probe in &fk_probes {
        let u_len = keys::unique_key(
            &mut key,
            probe.target_relation,
            probe.target_constraint,
            &probe.guard,
        );
        if data.get(txn.raw(), &key[..u_len])?.is_none() {
            return Err(Error::ForeignKeyViolation {
                relation: probe.source_relation,
                constraint: probe.source_constraint,
                violation: FkViolation::MissingTarget {
                    fact_bytes: probe.fact_bytes.clone().into_boxed_slice(),
                },
            });
        }
    }

    // Phase 3b: Restrict — every unique key deleted in phase 1 and not
    // re-established in phase 2 must have no remaining referrer. "No
    // committed state contains a dangling reference": deleting a target and
    // all its referrers in one transaction passes, as it should.
    for (rel, cid, guard) in deleted_guards.difference(&inserted_guards) {
        let p_len = keys::restrict_prefix(&mut key, *rel, *cid, guard);
        let mut iter = data.prefix_iter(txn.raw(), &key[..p_len])?;
        if let Some(entry) = iter.next() {
            let (surviving_key, ()) = entry.map(|(k, _)| (k, ()))?;
            // R | target_rel | constraint | guard | source_rel | source_row:
            // the referencing side is the last 12 bytes.
            let tail = &surviving_key[surviving_key.len() - 12..];
            let source_relation = RelationId(u32::from_be_bytes(
                tail[..4]
                    .try_into()
                    .expect("R keys carry a 4-byte source rel"),
            ));
            let source_row = u64::from_be_bytes(
                tail[4..]
                    .try_into()
                    .expect("R keys carry an 8-byte source row"),
            );
            // Fetch the referrer's fact bytes inside the still-open txn:
            // errors name facts, never storage row ids
            // (docs/architecture/10-data-model.md). Cold path — the fetch
            // costs one get on an aborting commit.
            drop(iter);
            let f_len = keys::fact_key(&mut key, source_relation, source_row);
            let fact_bytes: Box<[u8]> = data
                .get(txn.raw(), &key[..f_len])?
                .ok_or(Error::Corruption(CorruptionError::MissingFact {
                    relation: source_relation,
                    row_id: source_row,
                }))?
                .into();
            return Err(Error::ForeignKeyViolation {
                relation: *rel,
                constraint: *cid,
                violation: FkViolation::RemainingReference {
                    source_relation,
                    fact_bytes,
                },
            });
        }
    }

    // Phase 4: counters — row counts, row-id high-waters, serial sequences,
    // pending dictionary entries and the dictionary next-id.
    flush_counters(&mut txn, env, &delta, &row_id_next)?;

    // The storage tx id advances exactly once per state-changing commit.
    let new_generation = txn.generation()? + 1;
    txn.put_generation(new_generation)?;

    // Phase 5: LMDB commit (fsync per environment defaults).
    txn.commit()?;
    Ok(CommitReport {
        changed: true,
        new_generation,
    })
}

/// Phase 4: folds row-count deltas into `S`, writes row-id high-waters,
/// serial next-values (`Q`), pending dictionary entries, and the
/// dictionary next-id.
fn flush_counters(
    txn: &mut WriteTxn<'_>,
    env: &Environment,
    delta: &WriteDelta<'_>,
    row_id_next: &BTreeMap<RelationId, u64>,
) -> Result<()> {
    let data = env.data();
    let mut key: KeyBuf = [0; MAX_KEY];
    for (rel, count_delta) in delta.row_count_deltas() {
        if count_delta == 0 {
            continue;
        }
        let len = keys::stat_key(&mut key, rel, StatKind::RowCount);
        let current =
            match data.get(txn.raw(), &key[..len])? {
                Some(bytes) => u64::from_le_bytes(bytes.try_into().map_err(|_| {
                    Error::Corruption(CorruptionError::MalformedValue("S row count"))
                })?),
                None => 0,
            };
        let updated = current
            .checked_add_signed(count_delta)
            .ok_or(Error::Corruption(CorruptionError::MalformedValue(
                "S row count underflow",
            )))?;
        data.put(txn.raw_mut(), &key[..len], updated.to_le_bytes().as_slice())?;
    }
    for (rel, next) in row_id_next {
        let len = keys::stat_key(&mut key, *rel, StatKind::RowIdHighWater);
        data.put(txn.raw_mut(), &key[..len], next.to_le_bytes().as_slice())?;
    }
    for (rel, field, next) in delta.serial_marks() {
        let len = keys::serial_key(&mut key, rel, field);
        data.put(txn.raw_mut(), &key[..len], next.to_le_bytes().as_slice())?;
    }
    for (tag, raw, id) in delta.pending_interns() {
        crate::storage::dict::put_pending(txn, tag, raw, id)?;
    }
    if let Some(dict_next) = delta.dict_next() {
        txn.put_dict_next_id(dict_next)?;
    }
    Ok(())
}

/// Working state threaded through phases 1-2.
struct Applier<'env> {
    txn: WriteTxn<'env>,
    data: heed::Database<heed::types::Bytes, heed::types::Bytes>,
    changed: bool,
    row_id_next: BTreeMap<RelationId, u64>,
    deleted_guards: BTreeSet<(RelationId, ConstraintId, Vec<u8>)>,
    inserted_guards: BTreeSet<(RelationId, ConstraintId, Vec<u8>)>,
    fk_probes: Vec<FkProbe>,
    key: KeyBuf,
    guard: Vec<u8>,
}

impl Applier<'_> {
    /// Phase-1 step: removes one fact's F/M/U/R entries if it exists in
    /// base state (else the delete is a no-op by construction).
    fn delete_fact(&mut self, schema: &Schema, rel: RelationId, fact_bytes: &[u8]) -> Result<()> {
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
    fn insert_fact(&mut self, schema: &Schema, rel: RelationId, fact_bytes: &[u8]) -> Result<()> {
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
            // R puts are unconditional; target validation is PRD 08's.
            self.data
                .put(self.txn.raw_mut(), &self.key[..r_len], [].as_slice())?;
            self.fk_probes.push(FkProbe {
                source_relation: rel,
                source_constraint: ConstraintId(
                    u16::try_from(con_idx).expect("validated schema: constraint ids fit u16"),
                ),
                target_relation: *target_relation,
                target_constraint: *target_constraint,
                guard: self.guard.clone(),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::{encode_fact, encode_u64, ValueRef};
    use crate::schema::{
        FieldDescriptor, FieldId, Generation, RelationDescriptor, Schema, SchemaDescriptor,
        ValueType,
    };
    use crate::storage::keys::StatKind;
    use crate::testutil::TempDir;

    /// Target(id serial) + Source(id serial, t u64 fk -> Target.id) +
    /// Keyed(x u64 unique, y i64).
    fn schema() -> Schema {
        SchemaDescriptor {
            relations: vec![
                RelationDescriptor {
                    name: "Target".into(),
                    fields: vec![FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        generation: Generation::Serial,
                    }],
                    constraints: vec![],
                },
                RelationDescriptor {
                    name: "Source".into(),
                    fields: vec![
                        FieldDescriptor {
                            name: "id".into(),
                            value_type: ValueType::U64,
                            generation: Generation::Serial,
                        },
                        FieldDescriptor {
                            name: "t".into(),
                            value_type: ValueType::U64,
                            generation: Generation::None,
                        },
                    ],
                    constraints: vec![ConstraintDescriptor::ForeignKey {
                        name: "source_target".into(),
                        fields: Box::new([FieldId(1)]),
                        target_relation: RelationId(0),
                        target_constraint: ConstraintId(0),
                    }],
                },
                RelationDescriptor {
                    name: "Keyed".into(),
                    fields: vec![
                        FieldDescriptor {
                            name: "x".into(),
                            value_type: ValueType::U64,
                            generation: Generation::None,
                        },
                        FieldDescriptor {
                            name: "y".into(),
                            value_type: ValueType::I64,
                            generation: Generation::None,
                        },
                    ],
                    constraints: vec![ConstraintDescriptor::Unique {
                        name: "x".into(),
                        fields: Box::new([FieldId(0)]),
                    }],
                },
            ],
        }
        .validate()
        .expect("valid fixture")
    }

    const TARGET: RelationId = RelationId(0);
    const SOURCE: RelationId = RelationId(1);
    const KEYED: RelationId = RelationId(2);
    const C0: ConstraintId = ConstraintId(0);

    fn target_fact(schema: &Schema, id: u64) -> Vec<u8> {
        let mut b = Vec::new();
        encode_fact(
            &[ValueRef::U64(id)],
            schema.relation(TARGET).layout(),
            &mut b,
        );
        b
    }

    fn source_fact(schema: &Schema, id: u64, t: u64) -> Vec<u8> {
        let mut b = Vec::new();
        encode_fact(
            &[ValueRef::U64(id), ValueRef::U64(t)],
            schema.relation(SOURCE).layout(),
            &mut b,
        );
        b
    }

    fn keyed_fact(schema: &Schema, x: u64, y: i64) -> Vec<u8> {
        let mut b = Vec::new();
        encode_fact(
            &[ValueRef::U64(x), ValueRef::I64(y)],
            schema.relation(KEYED).layout(),
            &mut b,
        );
        b
    }

    fn all_data_keys(txn: &WriteTxn<'_>, env: &Environment) -> BTreeSet<Vec<u8>> {
        env.data()
            .iter(txn.raw())
            .expect("iter")
            .map(|kv| kv.expect("kv").0.to_vec())
            .collect()
    }

    fn committed_data(env: &Environment) -> Vec<(Vec<u8>, Vec<u8>)> {
        let rtxn = env.read_txn().expect("txn");
        env.data()
            .iter(rtxn.raw())
            .expect("iter")
            .map(|kv| {
                let (k, v) = kv.expect("kv");
                (k.to_vec(), v.to_vec())
            })
            .collect()
    }

    fn key(write: impl FnOnce(&mut KeyBuf) -> usize) -> Vec<u8> {
        let mut buf: KeyBuf = [0; MAX_KEY];
        let len = write(&mut buf);
        buf[..len].to_vec()
    }

    #[test]
    fn insert_lands_exactly_the_expected_key_set() {
        let dir = TempDir::new("commit-insert-keys");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        let t = target_fact(&schema, 5);
        let s = source_fact(&schema, 9, 5);
        {
            let view = env.read_txn().expect("txn");
            let mut delta = WriteDelta::new(&schema);
            delta.insert(&view, TARGET, &t).expect("insert");
            delta.insert(&view, SOURCE, &s).expect("insert");
            drop(view);
            let applied = apply(delta, &env).expect("apply");

            let t_hash = crate::encoding::fact_hash(&t);
            let s_hash = crate::encoding::fact_hash(&s);
            let expected: BTreeSet<Vec<u8>> = [
                key(|b| keys::fact_key(b, TARGET, 0)),
                key(|b| keys::membership_key(b, TARGET, &t_hash)),
                key(|b| keys::unique_key(b, TARGET, C0, &encode_u64(5))),
                key(|b| keys::fact_key(b, SOURCE, 0)),
                key(|b| keys::membership_key(b, SOURCE, &s_hash)),
                key(|b| keys::unique_key(b, SOURCE, C0, &encode_u64(9))),
                key(|b| keys::restrict_key(b, TARGET, C0, &encode_u64(5), SOURCE, 0)),
            ]
            .into_iter()
            .collect();
            assert_eq!(all_data_keys(&applied.txn, &env), expected);

            // Bookkeeping: one forward probe for the FK, no deleted guards,
            // the inserted target guard recorded for the FK-targeted
            // constraint.
            assert_eq!(applied.fk_probes.len(), 1);
            assert_eq!(applied.fk_probes[0].guard, encode_u64(5));
            assert!(applied.deleted_guards.is_empty());
            assert!(applied
                .inserted_guards
                .contains(&(TARGET, C0, encode_u64(5).to_vec())));
            assert!(applied.changed);
            // Abort: drop the txn without committing.
        }
        assert!(committed_data(&env).is_empty());
    }

    #[test]
    fn deleting_a_fact_with_a_scrubbed_f_row_is_corruption() {
        // Craft the M/F disagreement: commit a fact, raw-delete its F row
        // behind the codec's back, then delta-delete it. The write path
        // must raise the hard corruption error, never silently scrub the
        // M entry (docs/architecture/40-storage.md).
        let dir = TempDir::new("commit-desync");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        let t5 = target_fact(&schema, 5);
        {
            let view = env.read_txn().expect("txn");
            let mut delta = WriteDelta::new(&schema);
            delta.insert(&view, TARGET, &t5).expect("insert");
            drop(view);
            apply(delta, &env)
                .expect("apply")
                .txn
                .commit()
                .expect("commit");
        }
        // Scrub the F row (row id 0) directly.
        {
            let mut wtxn = env.write_txn().expect("wtxn");
            let mut key: KeyBuf = [0; MAX_KEY];
            let f_len = keys::fact_key(&mut key, TARGET, 0);
            assert!(env
                .data()
                .delete(wtxn.raw_mut(), &key[..f_len])
                .expect("del"));
            wtxn.commit().expect("commit");
        }
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta.delete(&view, TARGET, &t5).expect("record delete");
        drop(view);
        let Err(err) = apply(delta, &env).map(|_| ()) else {
            panic!("apply must fail on a scrubbed F row");
        };
        assert!(matches!(
            err,
            Error::Corruption(CorruptionError::MembershipDesync {
                relation: TARGET,
                row_id: 0
            })
        ));
    }

    #[test]
    fn delete_removes_exactly_its_entries() {
        let dir = TempDir::new("commit-delete-keys");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        let t5 = target_fact(&schema, 5);
        let t6 = target_fact(&schema, 6);
        let s = source_fact(&schema, 9, 5);
        // Commit a base state: two targets, one source referencing t5.
        {
            let view = env.read_txn().expect("txn");
            let mut delta = WriteDelta::new(&schema);
            delta.insert(&view, TARGET, &t5).expect("insert");
            delta.insert(&view, TARGET, &t6).expect("insert");
            delta.insert(&view, SOURCE, &s).expect("insert");
            drop(view);
            apply(delta, &env)
                .expect("apply")
                .txn
                .commit()
                .expect("commit");
        }
        let before = committed_data(&env);

        // Delete the source fact: exactly its F/M/U/R entries disappear.
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta.delete(&view, SOURCE, &s).expect("delete");
        drop(view);
        let applied = apply(delta, &env).expect("apply");

        let s_hash = crate::encoding::fact_hash(&s);
        let removed: BTreeSet<Vec<u8>> = [
            key(|b| keys::fact_key(b, SOURCE, 0)),
            key(|b| keys::membership_key(b, SOURCE, &s_hash)),
            key(|b| keys::unique_key(b, SOURCE, C0, &encode_u64(9))),
            key(|b| keys::restrict_key(b, TARGET, C0, &encode_u64(5), SOURCE, 0)),
        ]
        .into_iter()
        .collect();
        let expected: BTreeSet<Vec<u8>> = before
            .iter()
            .map(|(k, _)| k.clone())
            .filter(|k| !removed.contains(k))
            .collect();
        assert_eq!(all_data_keys(&applied.txn, &env), expected);
        // Source's own serial unique is not FK-targeted; nothing to scan.
        assert!(applied.deleted_guards.is_empty());
    }

    #[test]
    fn deleting_an_fk_targeted_fact_records_its_guard() {
        let dir = TempDir::new("commit-deleted-guard");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        let t5 = target_fact(&schema, 5);
        {
            let view = env.read_txn().expect("txn");
            let mut delta = WriteDelta::new(&schema);
            delta.insert(&view, TARGET, &t5).expect("insert");
            drop(view);
            apply(delta, &env)
                .expect("apply")
                .txn
                .commit()
                .expect("commit");
        }
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta.delete(&view, TARGET, &t5).expect("delete");
        drop(view);
        let applied = apply(delta, &env).expect("apply");
        assert!(applied
            .deleted_guards
            .contains(&(TARGET, C0, encode_u64(5).to_vec())));
    }

    #[test]
    fn delete_plus_insert_of_same_unique_key_succeeds_in_either_user_order() {
        let dir = TempDir::new("commit-swap-order");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        let old = keyed_fact(&schema, 1, 10);
        {
            let view = env.read_txn().expect("txn");
            let mut delta = WriteDelta::new(&schema);
            delta.insert(&view, KEYED, &old).expect("insert");
            drop(view);
            apply(delta, &env)
                .expect("apply")
                .txn
                .commit()
                .expect("commit");
        }
        // The "wrong" user order: insert the replacement before deleting the
        // old fact. Commit-time semantics make order irrelevant.
        let new = keyed_fact(&schema, 1, 20);
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta.insert(&view, KEYED, &new).expect("insert");
        delta.delete(&view, KEYED, &old).expect("delete");
        drop(view);
        let applied = apply(delta, &env).expect("apply");
        // The guard key survives, now pointing at the new row.
        let u = key(|b| keys::unique_key(b, KEYED, C0, &encode_u64(1)));
        assert!(all_data_keys(&applied.txn, &env).contains(&u));
    }

    #[test]
    fn two_facts_claiming_one_unique_key_is_a_violation_and_base_stays_intact() {
        let dir = TempDir::new("commit-unique-violation");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        let before = committed_data(&env);

        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        let a = keyed_fact(&schema, 1, 10);
        let b = keyed_fact(&schema, 1, 20);
        delta.insert(&view, KEYED, &a).expect("insert");
        delta.insert(&view, KEYED, &b).expect("insert");
        drop(view);
        let Err(err) = apply(delta, &env) else {
            panic!("expected a unique violation");
        };
        assert!(
            matches!(
                err,
                Error::UniqueViolation {
                    relation: KEYED,
                    constraint: C0,
                    ..
                }
            ),
            "{err:?}"
        );
        assert_eq!(committed_data(&env), before);
    }

    #[test]
    fn rederived_guard_keys_match_independent_computation() {
        let dir = TempDir::new("commit-guard-derivation");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        let s = source_fact(&schema, 42, 7);
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta.insert(&view, SOURCE, &s).expect("insert");
        drop(view);
        let applied = apply(delta, &env).expect("apply");

        // The serial auto-unique guard is the canonical encoding of `id`,
        // and the FK guard is the canonical encoding of `t` — computed here
        // independently of `derive_guard`.
        let keys_present = all_data_keys(&applied.txn, &env);
        assert!(keys_present.contains(&key(|b| keys::unique_key(b, SOURCE, C0, &encode_u64(42)))));
        assert!(keys_present.contains(&key(|b| keys::restrict_key(
            b,
            TARGET,
            C0,
            &encode_u64(7),
            SOURCE,
            0
        ))));
        assert_eq!(applied.fk_probes[0].guard, encode_u64(7).to_vec());
    }
    // ---------- PRD 08: full commit ----------

    fn commit_facts(env: &Environment, schema: &Schema, facts: &[(RelationId, Vec<u8>)]) {
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(schema);
        for (rel, fact) in facts {
            delta.insert(&view, *rel, fact).expect("insert");
        }
        drop(view);
        commit(delta, env).expect("commit");
    }

    #[test]
    fn insert_referencing_same_delta_target_commits() {
        let dir = TempDir::new("commit8-order-irrelevance");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        // Referrer inserted before its target: order is irrelevant.
        delta
            .insert(&view, SOURCE, &source_fact(&schema, 1, 5))
            .expect("insert");
        delta
            .insert(&view, TARGET, &target_fact(&schema, 5))
            .expect("insert");
        drop(view);
        let report = commit(delta, &env).expect("commit succeeds");
        assert!(report.changed);
        assert_eq!(report.new_generation, 1);
    }

    #[test]
    fn insert_referencing_missing_target_aborts_with_fk_violation() {
        let dir = TempDir::new("commit8-missing-target");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        let before = committed_data(&env);
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        let orphan = source_fact(&schema, 1, 99);
        delta.insert(&view, SOURCE, &orphan).expect("insert");
        drop(view);
        let err = commit(delta, &env).unwrap_err();
        assert!(
            matches!(
                &err,
                Error::ForeignKeyViolation {
                    relation: SOURCE,
                    constraint: ConstraintId(1),
                    violation: FkViolation::MissingTarget { fact_bytes },
                } if **fact_bytes == orphan[..]
            ),
            "{err:?}"
        );
        assert_eq!(committed_data(&env), before);
    }

    #[test]
    fn deleting_a_referenced_target_alone_is_a_restrict_violation() {
        let dir = TempDir::new("commit8-restrict");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        commit_facts(
            &env,
            &schema,
            &[
                (TARGET, target_fact(&schema, 5)),
                (SOURCE, source_fact(&schema, 1, 5)),
            ],
        );
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta
            .delete(&view, TARGET, &target_fact(&schema, 5))
            .expect("delete");
        drop(view);
        let err = commit(delta, &env).unwrap_err();
        assert!(
            matches!(
                err,
                Error::ForeignKeyViolation {
                    relation: TARGET,
                    constraint: C0,
                    violation: FkViolation::RemainingReference {
                        source_relation: SOURCE,
                        ..
                    },
                }
            ),
            "{err:?}"
        );
    }

    #[test]
    fn deleting_target_and_all_referrers_in_one_delta_commits() {
        let dir = TempDir::new("commit8-cascade-by-hand");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        commit_facts(
            &env,
            &schema,
            &[
                (TARGET, target_fact(&schema, 5)),
                (SOURCE, source_fact(&schema, 1, 5)),
            ],
        );
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta
            .delete(&view, TARGET, &target_fact(&schema, 5))
            .expect("delete");
        delta
            .delete(&view, SOURCE, &source_fact(&schema, 1, 5))
            .expect("delete");
        drop(view);
        commit(delta, &env).expect("deleting target and referrers together passes");
    }

    #[test]
    fn delete_and_reinsert_of_referenced_unique_key_commits() {
        let dir = TempDir::new("commit8-reestablish");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        commit_facts(
            &env,
            &schema,
            &[
                (TARGET, target_fact(&schema, 5)),
                (SOURCE, source_fact(&schema, 1, 5)),
            ],
        );
        // Replace the target fact wholesale, re-supplying its unique key —
        // Restrict sees the re-established key and passes.
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta
            .delete(&view, TARGET, &target_fact(&schema, 5))
            .expect("delete");
        delta
            .insert(&view, TARGET, &target_fact(&schema, 5))
            .expect("insert");
        drop(view);
        // Net effect: delete then insert of the same fact is a no-op delta
        // (last disposition wins → Insert of a base-present fact).
        let report = commit(delta, &env).expect("commit");
        assert!(!report.changed);
    }

    #[test]
    fn tx_id_advances_once_per_state_changing_commit_only() {
        let dir = TempDir::new("commit8-tx-id");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        let f = target_fact(&schema, 5);
        commit_facts(&env, &schema, &[(TARGET, f.clone())]);
        {
            let rtxn = env.read_txn().expect("txn");
            assert_eq!(rtxn.generation().expect("generation"), 1);
        }

        // All-no-op delta: re-inserting an existing fact records nothing.
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        assert!(!delta.insert(&view, TARGET, &f).expect("insert"));
        drop(view);
        let report = commit(delta, &env).expect("commit");
        assert!(!report.changed);
        assert_eq!(report.new_generation, 1);
        {
            let rtxn = env.read_txn().expect("txn");
            assert_eq!(rtxn.generation().expect("generation"), 1);
        }

        // A second state-changing commit bumps exactly once.
        commit_facts(&env, &schema, &[(TARGET, target_fact(&schema, 6))]);
        let rtxn = env.read_txn().expect("txn");
        assert_eq!(rtxn.generation().expect("generation"), 2);
    }

    #[test]
    fn counters_after_reopen_match_a_recount_of_f_entries() {
        let dir = TempDir::new("commit8-reopen-counters");
        let schema = schema();
        {
            let env = Environment::create(dir.path(), &schema).expect("create");
            commit_facts(
                &env,
                &schema,
                &[
                    (TARGET, target_fact(&schema, 1)),
                    (TARGET, target_fact(&schema, 2)),
                    (TARGET, target_fact(&schema, 3)),
                ],
            );
            // Mixed insert/delete commit.
            let view = env.read_txn().expect("txn");
            let mut delta = WriteDelta::new(&schema);
            delta
                .delete(&view, TARGET, &target_fact(&schema, 2))
                .expect("delete");
            delta
                .insert(&view, TARGET, &target_fact(&schema, 4))
                .expect("insert");
            drop(view);
            commit(delta, &env).expect("commit");
        }

        // Reopen: the flushed counters are the only test that can catch a
        // never-persisted high-water.
        let env = Environment::open(dir.path(), &schema).expect("open");
        let rtxn = env.read_txn().expect("txn");
        let mut key: KeyBuf = [0; MAX_KEY];
        let len = keys::stat_key(&mut key, TARGET, StatKind::RowCount);
        let count = u64::from_le_bytes(
            env.data()
                .get(rtxn.raw(), &key[..len])
                .expect("get")
                .expect("row count present")
                .try_into()
                .expect("u64"),
        );
        let prefix_len = keys::fact_prefix(&mut key, TARGET);
        let scanned = env
            .data()
            .prefix_iter(rtxn.raw(), &key[..prefix_len])
            .expect("iter")
            .count() as u64;
        assert_eq!(count, scanned);
        assert_eq!(count, 3); // 3 inserted + 1 inserted - 1 deleted

        // The high-water also survived: row ids 0..=3 were assigned, so the
        // stored next id is 4.
        let hw_len = keys::stat_key(&mut key, TARGET, StatKind::RowIdHighWater);
        let high_water = u64::from_le_bytes(
            env.data()
                .get(rtxn.raw(), &key[..hw_len])
                .expect("get")
                .expect("high water present")
                .try_into()
                .expect("u64"),
        );
        assert_eq!(high_water, 4);
    }

    #[test]
    fn serials_allocated_in_an_aborted_txn_are_reissued() {
        let dir = TempDir::new("commit8-serial-abort");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        {
            let view = env.read_txn().expect("txn");
            let mut delta = WriteDelta::new(&schema);
            assert_eq!(delta.alloc(&view, TARGET, FieldId(0)).expect("alloc"), 0);
            assert_eq!(delta.alloc(&view, TARGET, FieldId(0)).expect("alloc"), 1);
            // Abort: drop the delta without committing.
        }
        // The committed sequence is untouched: the next transaction
        // re-issues the same values.
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        let id = delta.alloc(&view, TARGET, FieldId(0)).expect("alloc");
        assert_eq!(id, 0);
        delta
            .insert(&view, TARGET, &target_fact(&schema, id))
            .expect("insert");
        drop(view);
        commit(delta, &env).expect("commit");

        // After a *committed* allocation, the sequence advances past it.
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        assert_eq!(delta.alloc(&view, TARGET, FieldId(0)).expect("alloc"), 1);
    }

    #[test]
    fn pending_interns_flush_at_commit_and_advance_the_counter() {
        let dir = TempDir::new("commit8-pending-interns");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        let id = delta.intern_str(&view, "holder-name").expect("intern");
        assert_eq!(delta.intern_str(&view, "holder-name").expect("intern"), id);
        // The delta must record a state change for the commit to flush; a
        // fact carrying the fresh id plays that role.
        delta
            .insert(&view, TARGET, &target_fact(&schema, 7))
            .expect("insert");
        drop(view);
        commit(delta, &env).expect("commit");

        let rtxn = env.read_txn().expect("txn");
        assert_eq!(
            crate::storage::dict::lookup_str(&rtxn, "holder-name").expect("lookup"),
            Some(id)
        );
        assert_eq!(
            crate::storage::dict::resolve(&rtxn, id, crate::storage::dict::TAG_STRING)
                .expect("resolve"),
            b"holder-name"
        );
        drop(rtxn);
        // A later direct intern continues past the flushed counter.
        let mut wtxn = env.write_txn().expect("txn");
        let next = crate::storage::dict::intern_str(&mut wtxn, "other").expect("intern");
        assert_eq!(next, id + 1);
    }
}
