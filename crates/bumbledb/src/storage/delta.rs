//! The write transaction delta core (PRD 06): a write transaction is an
//! in-memory net insert-set and delete-set of canonical fact bytes — last
//! disposition per fact wins — plus in-memory counters
//! (`docs/architecture/40-storage.md`).
//!
//! During accumulation, `insert`/`delete` are pure set arithmetic: encode is
//! the caller's job; membership is the delta's own disposition if present,
//! else an `M` probe against the borrowed read view. **Nothing touches an
//! LMDB data page until commit** (PRDs 07-08) — the LMDB write transaction
//! opens at commit, keeping the write-lock window to the commit step; an
//! abort (error or panic) just drops this struct and LMDB was never written.

use std::collections::BTreeMap;

use crate::arena::{Arena, ArenaSlice};
use crate::encoding::{decode_u64, fact_hash, field_bytes};
use crate::error::{Error, Result};
use crate::schema::{FieldId, Generation, RelationId, Schema};
use crate::storage::env::ReadTxn;
use crate::storage::keys::{self, KeyBuf, MAX_KEY};

/// The net effect recorded for one fact. Last disposition wins; whether it
/// actually applies is decided against base state at commit (PRD 07).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    Insert,
    Delete,
}

/// The accumulated write transaction.
pub struct WriteDelta<'s> {
    schema: &'s Schema,
    arena: Arena,
    /// `(relation, fact_hash) → (fact bytes, last disposition)`. Keyed by the
    /// full 32-byte blake3 of `fact_bytes` — hash equality *is* fact equality
    /// (collision axiom, `10-data-model.md`), and the `BTreeMap` gives the
    /// deterministic commit order PRD 07 requires.
    facts: BTreeMap<(RelationId, [u8; 32]), (ArenaSlice, Disposition)>,
    /// Serial next-values, lazily initialized from `Q` once per
    /// `(relation, field)` per transaction; a transaction sees its own
    /// allocations. The stored value is the *next* value to issue.
    serial_next: BTreeMap<(RelationId, FieldId), u64>,
    /// Net row-count change per relation, maintained alongside the
    /// changed-state reports (flushed to `S` by PRD 08).
    row_count_delta: BTreeMap<RelationId, i64>,
    /// Novel strings/bytes interned by this transaction: provisional ids
    /// assigned from the committed dictionary counter (the counter is
    /// in-memory-then-flush like every other counter; single-writer
    /// discipline makes provisional = final).
    pending_interns: BTreeMap<(u8, Box<[u8]>), u64>,
    /// The next dictionary id, lazily read once per transaction.
    dict_next: Option<u64>,
}

impl<'s> WriteDelta<'s> {
    #[must_use]
    pub fn new(schema: &'s Schema) -> Self {
        Self {
            schema,
            arena: Arena::new(),
            facts: BTreeMap::new(),
            serial_next: BTreeMap::new(),
            row_count_delta: BTreeMap::new(),
            pending_interns: BTreeMap::new(),
            dict_next: None,
        }
    }

    /// Interns a UTF-8 string for use in this transaction's facts: returns
    /// the committed id if present, else mints a provisional id flushed at
    /// commit. The `&str` boundary is the UTF-8 validation.
    ///
    /// # Errors
    ///
    /// `Lmdb` on a failed dictionary or counter read.
    pub fn intern_str(&mut self, view: &ReadTxn<'_>, value: &str) -> Result<u64> {
        self.intern(view, crate::storage::dict::TAG_STRING, value.as_bytes())
    }

    /// Interns a byte sequence; see [`Self::intern_str`].
    ///
    /// # Errors
    ///
    /// `Lmdb` on a failed dictionary or counter read.
    pub fn intern_bytes(&mut self, view: &ReadTxn<'_>, value: &[u8]) -> Result<u64> {
        self.intern(view, crate::storage::dict::TAG_BYTES, value)
    }

    fn intern(&mut self, view: &ReadTxn<'_>, tag: u8, raw: &[u8]) -> Result<u64> {
        if let Some(id) = crate::storage::dict::lookup_tagged(view, tag, raw)? {
            return Ok(id);
        }
        if let Some(id) = self.pending_interns.get(&(tag, Box::from(raw))) {
            return Ok(*id);
        }
        let next = match self.dict_next {
            Some(next) => next,
            None => view.dict_next_id()?,
        };
        assert!(
            next != crate::storage::dict::SENTINEL_ID,
            "dictionary id space exhausted (u64::MAX is the miss sentinel)"
        );
        self.pending_interns.insert((tag, Box::from(raw)), next);
        self.dict_next = Some(next + 1);
        Ok(next)
    }

    /// Records an insert. Returns whether the final state changes (an
    /// idempotent no-op if the fact is already present).
    ///
    /// Any serial field values the fact carries advance the in-memory
    /// high-water mark past them — explicit values are legal on the normal
    /// write path (`10-data-model.md`), and mixed explicit/generated
    /// allocation tracks the running maximum.
    ///
    /// # Errors
    ///
    /// `Lmdb` on a failed membership probe.
    pub fn insert(
        &mut self,
        view: &ReadTxn<'_>,
        rel: RelationId,
        fact_bytes: &[u8],
    ) -> Result<bool> {
        self.advance_serial_marks(view, rel, fact_bytes)?;
        let hash = fact_hash(fact_bytes);
        let changed = !self.present(view, rel, &hash, fact_bytes)?;
        if changed {
            let slice = self.arena.alloc(fact_bytes);
            self.facts.insert((rel, hash), (slice, Disposition::Insert));
            *self.row_count_delta.entry(rel).or_insert(0) += 1;
        }
        Ok(changed)
    }

    /// Records a delete. Returns whether the final state changes (an
    /// idempotent no-op if the fact is absent).
    ///
    /// # Errors
    ///
    /// `Lmdb` on a failed membership probe.
    pub fn delete(
        &mut self,
        view: &ReadTxn<'_>,
        rel: RelationId,
        fact_bytes: &[u8],
    ) -> Result<bool> {
        let hash = fact_hash(fact_bytes);
        let changed = self.present(view, rel, &hash, fact_bytes)?;
        if changed {
            let slice = self.arena.alloc(fact_bytes);
            self.facts.insert((rel, hash), (slice, Disposition::Delete));
            *self.row_count_delta.entry(rel).or_insert(0) -= 1;
        }
        Ok(changed)
    }

    /// Mints the next serial value for a `Serial`-generation field: reads
    /// `Q` once per `(relation, field)` per transaction, then increments in
    /// memory. Aborted transactions never touch the committed sequence.
    ///
    /// # Errors
    ///
    /// `SerialExhausted` when the sequence reaches `u64::MAX`; `Lmdb` on a
    /// failed `Q` read.
    ///
    /// # Panics
    ///
    /// On a programmer-invariant violation: `field` is not `Serial`
    /// generation (the typed write path makes this unwritable; the untyped
    /// path is the caller's responsibility to point at a serial field).
    pub fn alloc(&mut self, view: &ReadTxn<'_>, rel: RelationId, field: FieldId) -> Result<u64> {
        assert_eq!(
            self.schema.relation(rel).field(field).generation,
            Generation::Serial,
            "alloc on a non-serial field is a programmer error"
        );
        let next = match self.serial_next.get(&(rel, field)) {
            Some(next) => *next,
            None => read_serial_next(view, rel, field)?,
        };
        if next == u64::MAX {
            return Err(Error::SerialExhausted {
                relation: rel,
                field,
            });
        }
        self.serial_next.insert((rel, field), next + 1);
        Ok(next)
    }

    /// Iterates every recorded disposition in deterministic
    /// `(relation, fact_hash)` order with its fact bytes (reader: PRD 07's
    /// commit apply).
    pub(crate) fn entries(&self) -> impl Iterator<Item = (RelationId, &[u8], Disposition)> {
        self.facts
            .iter()
            .map(|((rel, _), (slice, disposition))| (*rel, self.arena.get(*slice), *disposition))
    }

    /// The schema this delta was accumulated against (reader: commit).
    pub(crate) fn schema(&self) -> &'s Schema {
        self.schema
    }

    /// Whether the delta records no dispositions at all (reader: PRD 08's
    /// skip-empty-commit rule; pending allocations and interns of an empty
    /// delta are deliberately dropped — none of them are observable).
    pub(crate) fn is_empty(&self) -> bool {
        self.facts.is_empty()
    }

    /// Serial next-values to flush to `Q` (reader: PRD 08 phase 4).
    pub(crate) fn serial_marks(&self) -> impl Iterator<Item = (RelationId, FieldId, u64)> + '_ {
        self.serial_next
            .iter()
            .map(|((rel, field), next)| (*rel, *field, *next))
    }

    /// Net row-count changes to fold into `S` (reader: PRD 08 phase 4).
    pub(crate) fn row_count_deltas(&self) -> impl Iterator<Item = (RelationId, i64)> + '_ {
        self.row_count_delta.iter().map(|(rel, d)| (*rel, *d))
    }

    /// Pending intern entries to flush to `_dict` (reader: PRD 08 phase 4).
    pub(crate) fn pending_interns(&self) -> impl Iterator<Item = (u8, &[u8], u64)> + '_ {
        self.pending_interns
            .iter()
            .map(|((tag, raw), id)| (*tag, raw.as_ref(), *id))
    }

    /// The dictionary next-id to flush, if this transaction minted any
    /// provisional ids (reader: PRD 08 phase 4).
    pub(crate) fn dict_next(&self) -> Option<u64> {
        self.dict_next
    }

    /// The recorded disposition for a fact, if any (last one wins).
    #[cfg(test)]
    #[must_use]
    pub fn disposition(&self, rel: RelationId, fact_bytes: &[u8]) -> Option<Disposition> {
        self.facts
            .get(&(rel, fact_hash(fact_bytes)))
            .map(|(_, disposition)| *disposition)
    }

    /// Effective membership: the delta's disposition if present, else an `M`
    /// probe against the read view (committed state).
    fn present(
        &self,
        view: &ReadTxn<'_>,
        rel: RelationId,
        hash: &[u8; 32],
        fact_bytes: &[u8],
    ) -> Result<bool> {
        if let Some((_, disposition)) = self.facts.get(&(rel, *hash)) {
            return Ok(*disposition == Disposition::Insert);
        }
        Ok(crate::storage::read::fact_row(view, rel, fact_bytes)?.is_some())
    }

    /// Advances serial marks past any serial-field values the fact carries
    /// (the layout knows the offsets; serial fields are always U64).
    fn advance_serial_marks(
        &mut self,
        view: &ReadTxn<'_>,
        rel: RelationId,
        fact_bytes: &[u8],
    ) -> Result<()> {
        let relation = self.schema.relation(rel);
        for (idx, field) in relation.fields().iter().enumerate() {
            if field.generation != Generation::Serial {
                continue;
            }
            let field_id =
                FieldId(u16::try_from(idx).expect("validated schema: field ids fit u16"));
            let raw = field_bytes(fact_bytes, relation.layout(), idx);
            let value = decode_u64(raw.try_into().expect("serial fields are 8 bytes"));
            let mark = match self.serial_next.get(&(rel, field_id)) {
                Some(mark) => *mark,
                None => read_serial_next(view, rel, field_id)?,
            };
            // `saturating_add`: an explicit u64::MAX is legal to insert; the
            // sequence is then exhausted for the generator (alloc errors).
            self.serial_next
                .insert((rel, field_id), mark.max(value.saturating_add(1)));
        }
        Ok(())
    }
}

/// Reads the committed `Q` next-value for `(relation, field)`; a missing
/// entry means the sequence has never issued a value.
fn read_serial_next(view: &ReadTxn<'_>, rel: RelationId, field: FieldId) -> Result<u64> {
    let mut buf: KeyBuf = [0; MAX_KEY];
    let len = keys::serial_key(&mut buf, rel, field);
    match view.env().data().get(view.raw(), &buf[..len])? {
        Some(bytes) => Ok(u64::from_le_bytes(bytes.try_into().map_err(|_| {
            Error::Corruption(crate::error::CorruptionError::MalformedValue(
                "Q serial next",
            ))
        })?)),
        None => Ok(0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::encoding::{encode_fact, ValueRef};
    use crate::schema::{FieldDescriptor, RelationDescriptor, SchemaDescriptor, ValueType};
    use crate::storage::env::Environment;
    use crate::testutil::TempDir;

    /// R(id serial, amount i64).
    fn schema() -> Schema {
        SchemaDescriptor {
            relations: vec![RelationDescriptor {
                name: "R".into(),
                fields: vec![
                    FieldDescriptor {
                        name: "id".into(),
                        value_type: ValueType::U64,
                        generation: Generation::Serial,
                    },
                    FieldDescriptor {
                        name: "amount".into(),
                        value_type: ValueType::I64,
                        generation: Generation::None,
                    },
                ],
                constraints: vec![],
            }],
        }
        .validate()
        .expect("valid fixture")
    }

    const R: RelationId = RelationId(0);
    const ID: FieldId = FieldId(0);

    fn fact(schema: &Schema, id: u64, amount: i64) -> Vec<u8> {
        let mut bytes = Vec::new();
        encode_fact(
            &[ValueRef::U64(id), ValueRef::I64(amount)],
            schema.relation(R).layout(),
            &mut bytes,
        );
        bytes
    }

    fn data_snapshot(env: &Environment) -> Vec<(Vec<u8>, Vec<u8>)> {
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

    #[test]
    fn insert_then_delete_of_absent_fact_nets_noop_and_reports_true_true() {
        let dir = TempDir::new("delta-insert-delete");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        let f = fact(&schema, 1, 100);
        assert!(delta.insert(&view, R, &f).expect("insert"));
        assert!(delta.delete(&view, R, &f).expect("delete"));
        // Net disposition is Delete for a fact not in base: apply's base
        // check makes it a no-op (PRD 07).
        assert_eq!(delta.disposition(R, &f), Some(Disposition::Delete));
    }

    #[test]
    fn idempotent_double_insert_reports_true_false() {
        let dir = TempDir::new("delta-double-insert");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        let f = fact(&schema, 1, 100);
        assert!(delta.insert(&view, R, &f).expect("insert"));
        assert!(!delta.insert(&view, R, &f).expect("insert"));
    }

    #[test]
    fn disposition_last_wins_across_long_sequences() {
        let dir = TempDir::new("delta-last-wins");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        let f = fact(&schema, 1, 100);
        for _ in 0..7 {
            delta.insert(&view, R, &f).expect("insert");
            delta.delete(&view, R, &f).expect("delete");
        }
        delta.insert(&view, R, &f).expect("insert");
        assert_eq!(delta.disposition(R, &f), Some(Disposition::Insert));
        delta.delete(&view, R, &f).expect("delete");
        assert_eq!(delta.disposition(R, &f), Some(Disposition::Delete));
    }

    #[test]
    fn alloc_is_strictly_increasing_and_reads_q_once() {
        let dir = TempDir::new("delta-alloc");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        assert_eq!(delta.alloc(&view, R, ID).expect("alloc"), 0);
        assert_eq!(delta.alloc(&view, R, ID).expect("alloc"), 1);
        drop(view);

        // Bump the committed Q value behind the delta's back: the cached
        // in-memory next must win — Q is read once per (relation, field).
        {
            let mut wtxn = env.write_txn().expect("txn");
            let mut buf: KeyBuf = [0; MAX_KEY];
            let len = keys::serial_key(&mut buf, R, ID);
            env.data()
                .put(wtxn.raw_mut(), &buf[..len], 100u64.to_le_bytes().as_slice())
                .expect("put");
            wtxn.commit().expect("commit");
        }
        let view = env.read_txn().expect("txn");
        assert_eq!(delta.alloc(&view, R, ID).expect("alloc"), 2);

        // A fresh delta sees the committed value.
        let mut fresh = WriteDelta::new(&schema);
        assert_eq!(fresh.alloc(&view, R, ID).expect("alloc"), 100);
    }

    #[test]
    fn explicit_value_above_mark_advances_generated_successors() {
        let dir = TempDir::new("delta-explicit");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        assert!(delta
            .insert(&view, R, &fact(&schema, 50, 1))
            .expect("insert"));
        assert_eq!(delta.alloc(&view, R, ID).expect("alloc"), 51);
    }

    #[test]
    fn mixed_explicit_and_generated_allocation_tracks_running_maximum() {
        let dir = TempDir::new("delta-mixed");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        assert_eq!(delta.alloc(&view, R, ID).expect("alloc"), 0);
        delta
            .insert(&view, R, &fact(&schema, 10, 1))
            .expect("insert");
        assert_eq!(delta.alloc(&view, R, ID).expect("alloc"), 11);
        // An explicit value *below* the mark must not regress it.
        delta
            .insert(&view, R, &fact(&schema, 3, 2))
            .expect("insert");
        assert_eq!(delta.alloc(&view, R, ID).expect("alloc"), 12);
    }

    #[test]
    fn explicit_max_exhausts_the_generator() {
        let dir = TempDir::new("delta-exhausted");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        let view = env.read_txn().expect("txn");
        let mut delta = WriteDelta::new(&schema);
        delta
            .insert(&view, R, &fact(&schema, u64::MAX, 1))
            .expect("insert");
        let err = delta.alloc(&view, R, ID).unwrap_err();
        assert!(
            matches!(
                err,
                Error::SerialExhausted {
                    relation: R,
                    field: ID
                }
            ),
            "{err:?}"
        );
    }

    #[test]
    fn drop_leaves_lmdb_untouched() {
        let dir = TempDir::new("delta-drop");
        let schema = schema();
        let env = Environment::create(dir.path(), &schema).expect("create");
        let before = data_snapshot(&env);
        {
            let view = env.read_txn().expect("txn");
            let mut delta = WriteDelta::new(&schema);
            for i in 0i64..100 {
                delta
                    .insert(&view, R, &fact(&schema, i.cast_unsigned(), i))
                    .expect("insert");
            }
            delta.alloc(&view, R, ID).expect("alloc");
            delta
                .delete(&view, R, &fact(&schema, 5, 5))
                .expect("delete");
            // Abort = drop: nothing was ever written.
        }
        assert_eq!(before, data_snapshot(&env));
        assert!(before.is_empty());
    }
}
