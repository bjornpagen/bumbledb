//! `WriteTx` point reads (`docs/architecture/70-api.md` § `WriteTx`
//! point reads): `contains` / `get` / `get_dyn` read **committed state overlaid
//! with the pending delta** — the same final-state view the judgment phase
//! judges — so read-modify-write idioms (upsert, check-then-act guards)
//! are sound without exposing query machinery to the write path. These are
//! guard gets: no images, no plans, no snapshot.

use super::encode_dyn::shape_mismatch;
use super::{plumbing, Fact, Fresh, FreshKeyed, WriteTx};
use crate::encoding::{
    encode_bool, encode_i64, encode_interval_i64, encode_interval_u64, encode_u64,
};
use crate::error::{FactShapeError, Result};
use crate::ir::Value;
use crate::schema::{FieldId, RelationId, StatementId};
use crate::storage::delta::GuardOverlay;
use crate::storage::read;

impl<S> WriteTx<'_, S> {
    /// Whether `fact` is in the transaction's **final state** — reads
    /// observe the final-state view the judgment phase will judge
    /// (`docs/architecture/70-api.md`): the delta's own disposition when
    /// this transaction touched the fact, the committed `M` probe
    /// otherwise. The read-only sibling of [`WriteTx::insert`]/
    /// [`WriteTx::delete`]'s changed report; before commit it answers
    /// exactly what a post-commit read transaction would.
    ///
    /// Encodes through the transaction's read context — pending intern ids
    /// first, then the committed dictionary, **never minting**: a string
    /// or bytes value known to neither proves the fact absent everywhere,
    /// so the probe short-circuits to `false` with the dictionary
    /// untouched.
    ///
    /// # Errors
    ///
    /// `Lmdb` on the membership probe or dictionary reads.
    pub fn contains<'f, F: Fact<'f, Schema = S>>(&mut self, fact: &F) -> Result<bool> {
        self.with_scratch(|tx, bytes| {
            if !fact.encode_delete(tx, bytes)? {
                return Ok(false);
            }
            tx.delta.contains(&tx.view, F::RELATION, bytes)
        })
    }

    /// Point lookup of the full fact through the relation's fresh key —
    /// reads observe the final-state view the judgment phase will judge
    /// (`docs/architecture/70-api.md`): the delta's guard map first, the
    /// committed `U` → `F` path otherwise. Typed sugar for the dominant
    /// single-fresh-field case; every other key goes through
    /// [`WriteTx::get_dyn`].
    ///
    /// The returned fact is a **view at the transaction's lifetime**:
    /// variable-width fields borrow from the committed dictionary (mmap
    /// pages, stable for the transaction by LMDB `CoW`) or from this
    /// transaction's pending interns (the delta arena — read-your-writes
    /// included), whichever holds the value. No copy is made; a host that
    /// keeps a field past the transaction copies it explicitly
    /// (`to_owned()`).
    ///
    /// # Example — the blessed upsert idiom (`docs/architecture/70-api.md`)
    ///
    /// ```
    /// bumbledb::schema! {
    ///     pub Ledger;
    ///     relation Account { id: u64 as AccountId, fresh, balance: i64 }
    /// }
    ///
    /// fn add(db: &bumbledb::Db<Ledger>, id: AccountId, x: i64) -> bumbledb::Result<()> {
    ///     db.write(|tx| {
    ///         match tx.get::<Account>(id)? {
    ///             Some(old) => {
    ///                 tx.delete(&old)?;
    ///                 tx.insert(&Account { balance: old.balance + x, ..old })?;
    ///             }
    ///             None => {
    ///                 tx.insert(&Account { id, balance: x })?;
    ///             }
    ///         }
    ///         Ok(())
    ///     })
    /// }
    /// # let dir = std::env::temp_dir().join("bumbledb-doc-upsert");
    /// # let _ = std::fs::remove_dir_all(&dir);
    /// # std::fs::create_dir_all(&dir).unwrap();
    /// # let db = bumbledb::Db::create(&dir, Ledger).unwrap();
    /// # let id = db.write(|tx| tx.alloc::<AccountId>()).unwrap();
    /// # add(&db, id, 10).unwrap();
    /// # add(&db, id, 32).unwrap();
    /// # db.write(|tx| {
    /// #     assert_eq!(tx.get::<Account>(id)?.expect("upserted").balance, 42);
    /// #     Ok(())
    /// # }).unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// `Lmdb` on the guard probe, `Corruption` on undecodable stored
    /// bytes.
    pub fn get<'tx, F>(&'tx self, id: F::FreshKey) -> Result<Option<F>>
    where
        F: FreshKeyed<'tx, Schema = S>,
    {
        // The fresh field's guard is its canonical u64 encoding — the
        // one-field instance of the guard-byte format `get_dyn` spells
        // out value by value.
        let guard = encode_u64(id.fresh());
        let key = self.fresh_key_statement(F::RELATION, <F::FreshKey as Fresh>::FIELD);
        match self.fact_by_guard(F::RELATION, key, &guard)? {
            Some(bytes) => F::decode_write(self, bytes).map(Some),
            None => Ok(None),
        }
    }

    /// Point lookup of the full fact through any key statement of
    /// `relation` — reads observe the final-state view the judgment phase
    /// will judge (`docs/architecture/70-api.md`): the delta's guard map
    /// first, the committed `U` → `F` path otherwise. `key_values` are the
    /// key statement's projected fields in statement projection order,
    /// type-checked against the projection; the dynamic sibling of
    /// [`WriteTx::get`].
    ///
    /// String and bytes key values resolve through the transaction's read
    /// context — pending intern ids first, then the committed dictionary,
    /// never minting: a never-interned value proves no fact carries it, so
    /// the lookup answers `Ok(None)` with the dictionary untouched.
    ///
    /// # Errors
    ///
    /// `FactShape` when `relation` is unknown, `key` is not one of its
    /// `Functionality` statements, or `key_values` mismatch the projection
    /// in arity or type; `Lmdb`/`Corruption` from storage.
    pub fn get_dyn(
        &mut self,
        relation: RelationId,
        key: StatementId,
        key_values: &[Value],
    ) -> Result<Option<Vec<Value>>> {
        let Some(rel) = self.schema.relation_checked(relation) else {
            return Err(FactShapeError::UnknownRelation { relation }.into());
        };
        if !rel.keys().contains(&key) {
            return Err(FactShapeError::NotAKeyStatement {
                relation,
                statement: key,
            }
            .into());
        }
        let projection = self.schema.key_projection(key);
        if key_values.len() != projection.len() {
            return Err(FactShapeError::ArityMismatch {
                relation,
                expected: projection.len(),
                supplied: key_values.len(),
            }
            .into());
        }
        self.with_scratch(|tx, guard| {
            if !tx.encode_guard(relation, projection, key_values, guard)? {
                return Ok(None);
            }
            tx.fact_by_guard(relation, key, guard)?
                .map(|bytes| tx.decode_values(relation, bytes))
                .transpose()
        })
    }

    /// Encodes `key_values` into guard bytes — the concatenated canonical
    /// field encodings in statement projection order, byte-identical to
    /// what `keys::guard_bytes` slices out of a stored fact. `Ok(false)` =
    /// a string or bytes value was never interned: no fact can carry it.
    fn encode_guard(
        &self,
        relation: RelationId,
        projection: &[FieldId],
        key_values: &[Value],
        out: &mut Vec<u8>,
    ) -> Result<bool> {
        let rel = self.schema.relation(relation);
        for (value, &field) in key_values.iter().zip(projection) {
            if let Err(mismatch) = crate::schema::value_matches(value, &rel.field(field).value_type)
            {
                return Err(shape_mismatch(relation, field, mismatch).into());
            }
            match value {
                Value::AllenMask(_) => {
                    unreachable!("value_matches rejected mask values above: not a field type")
                }
                Value::Bool(v) => out.push(encode_bool(*v)),
                Value::Enum(ordinal) => out.push(*ordinal),
                Value::U64(v) => out.extend_from_slice(&encode_u64(*v)),
                Value::I64(v) => out.extend_from_slice(&encode_i64(*v)),
                Value::IntervalU64(start, end) => {
                    out.extend_from_slice(&encode_interval_u64(*start, *end));
                }
                Value::IntervalI64(start, end) => {
                    out.extend_from_slice(&encode_interval_i64(*start, *end));
                }
                Value::String(raw) => {
                    let text =
                        std::str::from_utf8(raw).expect("value_matches validated UTF-8 above");
                    match self.delta.resolve_str(&self.view, text)? {
                        Some(id) => out.extend_from_slice(&encode_u64(id)),
                        None => return Ok(false),
                    }
                }
                // Self-encoding: the padded canonical bytes, no dictionary.
                Value::FixedBytes(raw) => crate::encoding::encode_fixed_bytes(raw, out),
            }
        }
        Ok(true)
    }

    /// The shared lookup leg: delta guard map first (`Present` → the
    /// pending fact's bytes, `Absent` → known deleted), then the committed
    /// view — `U` get → `F` fetch.
    ///
    /// A **closed** relation resolves against its sealed extension instead
    /// — virtual storage, no `U` guards exist
    /// (`docs/architecture/50-storage.md` § virtual relations): the key's
    /// guard bytes are re-derived per row by the same slicing the commit
    /// path uses, and the scan is ≤256 rows, L1-resident — O(rows) is
    /// honest and tiny. No delta arm: writes are refused at entry.
    fn fact_by_guard(
        &self,
        relation: RelationId,
        key: StatementId,
        guard: &[u8],
    ) -> Result<Option<&[u8]>> {
        let rel = self.schema.relation(relation);
        if let Some(extension) = rel.extension() {
            let projection = self.schema.key_projection(key);
            let mut derived = Vec::with_capacity(guard.len());
            for row in extension {
                crate::storage::keys::guard_bytes(
                    rel.layout(),
                    projection,
                    &row.fact,
                    &mut derived,
                );
                if derived == guard {
                    return Ok(Some(&row.fact));
                }
            }
            return Ok(None);
        }
        match self.delta.guard_overlay(key, guard) {
            Some(GuardOverlay::Present(bytes)) => Ok(Some(bytes)),
            Some(GuardOverlay::Absent) => Ok(None),
            None => match read::guard_row(&self.view, relation, key, guard)? {
                Some(row) => read::fetch(&self.view, self.schema, relation, row).map(Some),
                None => Ok(None),
            },
        }
    }

    /// Decodes canonical fact bytes into owned values, resolving intern
    /// ids pending-first (a fact inserted this transaction carries
    /// provisional ids) — the dynamic sibling of [`Fact::decode_write`].
    fn decode_values(&self, relation: RelationId, fact: &[u8]) -> Result<Vec<Value>> {
        super::encode_dyn::decode_values(fact, self.schema.relation(relation).layout(), |id| {
            Ok(Box::from(
                plumbing::resolve_string_write(self, id)?.as_bytes(),
            ))
        })
    }

    /// The auto-materialized `Functionality` statement for one fresh
    /// field (schema validation guarantees exactly one exists).
    fn fresh_key_statement(&self, relation: RelationId, field: FieldId) -> StatementId {
        let rel = self.schema.relation(relation);
        *rel.keys()
            .iter()
            .find(|&&statement| self.schema.key_projection(statement) == [field])
            .expect("validated schema: every fresh field materializes its Functionality")
    }
}
