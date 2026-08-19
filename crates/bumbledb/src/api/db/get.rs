//! `WriteTx` point reads (`docs/architecture/70-api.md` § `WriteTx`
//! point reads): `contains` / `get` / `get_dyn` read **committed state overlaid
//! with the pending delta** — the same final-state view the judgment phase
//! judges — so read-modify-write idioms (upsert, check-then-act conditions)
//! are sound without exposing query machinery to the write path. These are
//! determinant gets: no images, no plans, no snapshot.

use super::encode_dyn::shape_mismatch;
use super::{Fact, Key, Probe, WriteTx};
use crate::encoding::encode_u64;
use crate::error::{DynIdError, FactShapeError, Mismatch, Result};
use crate::ir::Value;
use crate::schema::{KeyForm, KeyId, KeyStatement, Relation, RelationBody, Schema, StatementView};
use crate::storage::read;
use bumbledb_theory::schema::{FieldId, RelationId, StatementId};

/// Resolves a data-supplied `(relation, key statement)` pair to the
/// sealed key — the shared shape gate of both point-read surfaces
/// ([`WriteTx::get_dyn`] and [`super::ReadInstance::get_dyn`]): the id must
/// name a `Functionality` statement ON the queried relation, or the
/// mismatch is a typed error, never an index panic.
pub(super) fn key_statement_of(
    schema: &Schema,
    relation: RelationId,
    key: StatementId,
) -> Result<(KeyId, &KeyStatement)> {
    let Some(rel) = schema.relation_checked(relation) else {
        return Err(DynIdError::UnknownRelation { relation }.into());
    };
    let Some(StatementView::Key(key_id, statement)) = schema.statement_checked(key) else {
        return Err(DynIdError::NotAKeyStatement {
            relation,
            statement: key,
        }
        .into());
    };
    if statement.relation != relation || !rel.keys().contains(&key_id) {
        return Err(DynIdError::NotAKeyStatement {
            relation,
            statement: key,
        }
        .into());
    }
    Ok((key_id, statement))
}

/// Encodes `key_values` into determinant bytes — the concatenated
/// canonical field encodings in statement projection order,
/// byte-identical to what `keys::determinant_image` slices out of a
/// stored fact — under whichever string resolver the transaction kind
/// supplies (pending-first inside a write transaction, the committed
/// dictionary on a snapshot). `Ok(false)` = a string value was never
/// interned: no fact can carry it.
pub(super) fn encode_determinant_with(
    schema: &Schema,
    relation: RelationId,
    projection: &[FieldId],
    key_values: &[Value],
    out: &mut Vec<u8>,
    mut resolve_str: impl FnMut(&str) -> Result<Option<crate::encoding::InternId>>,
) -> Result<bool> {
    let rel = schema.relation(relation);
    if key_values.len() != projection.len() {
        return Err(FactShapeError::ArityMismatch {
            relation,
            mismatch: Mismatch {
                witnessed: key_values.len(),
                required: projection.len(),
            },
        }
        .into());
    }
    for (value, &field) in key_values.iter().zip(projection) {
        if let Err(mismatch) =
            bumbledb_theory::schema::value_matches(value, &rel.field(field).value_type)
        {
            return Err(shape_mismatch(relation, field, mismatch).into());
        }
        match value {
            Value::String(text) => match resolve_str(text)? {
                Some(id) => out.extend_from_slice(&encode_u64(id.raw())),
                None => return Ok(false),
            },
            // Every self-encoding value takes the one type-aware
            // literal encoder — a fixed-width interval position
            // contributes its 8-byte start, a general one its 16
            // bytes, exactly what `determinant_image` slices out of
            // a stored fact (String peeled above per the encoder's
            // contract; a mask value is unreachable — the check
            // rejected it: not a field type).
            _ => {
                crate::encoding::encode_literal(value, rel.field(field).value_type, out);
            }
        }
    }
    Ok(true)
}

/// The fresh-row auto-key's committed probe target (the one id
/// allocator, ruled 2026-07-23, R16): the determinant IS the big-endian
/// row id — schema validation seals the auto-key's projection as the
/// one u64 fresh field, so the word is total.
pub(super) fn fresh_row_id(determinant: &[u8]) -> u64 {
    let Ok(word) = <[u8; 8]>::try_from(determinant) else {
        unreachable!("KeyForm::FreshRow determinant is one encoded u64");
    };
    u64::from_be_bytes(word)
}

/// The sealed point-read path: relation kind, then key form. One match
/// used by snapshot get, write-tx get, and the capacity parent probe.
pub(crate) enum PointRead {
    Closed,
    FreshRow { row_id: u64 },
    Determinant,
}

pub(crate) fn point_read(
    rel: &Relation,
    statement: &KeyStatement,
    determinant: &[u8],
) -> PointRead {
    match rel.body() {
        RelationBody::Closed { .. } => PointRead::Closed,
        RelationBody::Ordinary { .. } => match statement.form() {
            KeyForm::FreshRow { .. } => PointRead::FreshRow {
                row_id: fresh_row_id(determinant),
            },
            KeyForm::Scalar | KeyForm::Pointwise { .. } => PointRead::Determinant,
        },
    }
}

/// A **closed** relation's determinant lookup: virtual storage holds no
/// `U` determinants, so the key's determinant bytes re-derive per sealed
/// row by the same slicing the commit path uses — ≤256 rows, L1-resident
/// (`docs/architecture/50-storage.md` § virtual relations). Shared by
/// both transaction kinds (a closed relation reads identically
/// everywhere: no delta arm can exist — writes are refused at entry).
pub(super) fn closed_fact_by_determinant<'rel>(
    rel: &'rel Relation,
    statement: &KeyStatement,
    determinant: &[u8],
) -> Option<&'rel [u8]> {
    let extension = rel.body().closed_rows()?;
    let mut derived =
        crate::storage::keys::DeterminantImage::scratch_with_capacity(determinant.len());
    for row in extension {
        crate::storage::keys::determinant_image(
            rel.layout().encoded(&row.fact),
            &statement.projection,
            &mut derived,
        );
        if derived.as_bytes() == determinant {
            return Some(&row.fact);
        }
    }
    None
}

impl<S> WriteTx<'_, S> {
    /// Whether `fact` is in the transaction's **final state** — a point
    /// membership probe (`Result<bool>`), not a [`super::MutationReport`].
    /// Reads observe the final-state view the judgment phase will judge
    /// (`docs/architecture/70-api.md`): the delta's own disposition when
    /// this transaction touched the fact, the committed `M` probe
    /// otherwise. Before commit it answers exactly what a post-commit
    /// read transaction would.
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
        self.mutation.contains(fact)
    }

    /// Point lookup of the full fact through a typed key value ([`Key`]) —
    /// reads observe the final-state view the judgment phase will judge
    /// (`docs/architecture/70-api.md`): the delta's determinant map first, the
    /// committed `U` → `F` path otherwise. The key value's TYPE carries the
    /// relation and the key statement (`K::STATEMENT`, computed at `schema!`
    /// expansion), so which key FD a read goes through is never a runtime
    /// question. The committed-state sibling is [`super::ReadInstance::get`];
    /// data-supplied key statements go through [`WriteTx::get_dyn`].
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
    ///         match tx.get(id)? {
    ///             Some(old) => {
    ///                 tx.delete([&old])?;
    ///                 tx.insert([&Account { balance: old.balance + x, ..old }])?;
    ///             }
    ///             None => {
    ///                 tx.insert([&Account { id, balance: x }])?;
    ///             }
    ///         }
    ///         Ok(())
    ///     })?.expect("accepted");
    ///     Ok(())
    /// }
    /// # let dir = std::env::temp_dir().join("bumbledb-doc-upsert");
    /// # let _ = std::fs::remove_dir_all(&dir);
    /// # let db = bumbledb::Db::create(&dir, Ledger).unwrap().expect("accepted");
    /// # let id = db.write(|tx| Ok(tx.reserve::<AccountId>(1)?.start().expect("count 1"))).unwrap().unwrap().value;
    /// # add(&db, id, 10).unwrap();
    /// # add(&db, id, 32).unwrap();
    /// # db.write(|tx| {
    /// #     assert_eq!(tx.get(id)?.expect("upserted").balance, 42);
    /// #     Ok(())
    /// # }).unwrap();
    /// ```
    ///
    /// # Errors
    ///
    /// `FactShape` when a manual `Key` impl lies about its statement
    /// (typed, never a panic); `Lmdb` on the determinant probe,
    /// `Corruption` on undecodable stored bytes.
    #[expect(
        clippy::needless_pass_by_value,
        reason = "a key value is the read's input, spelled `tx.get(id)`: fresh \
                  newtypes are Copy and generated key structs are small — \
                  by-value keeps every call site free of `&` noise"
    )]
    pub fn get<'tx, K: Key<'tx, Schema = S>>(&'tx mut self, key: K) -> Result<Option<K::Fact>> {
        let relation = <K::Fact as Fact<'tx>>::RELATION;
        let (key_id, _) = key_statement_of(self.mutation.schema(), relation, K::STATEMENT)?;
        let mut key_bytes = std::mem::take(&mut self.mutation.scratch);
        key_bytes.clear();
        read::begin_determinant_key(&mut key_bytes, relation, K::STATEMENT);
        let filled = key.encode_determinant(self, &mut key_bytes);
        self.mutation.scratch = key_bytes;
        if matches!(filled?, Probe::ProvablyAbsent) {
            return Ok(None);
        }
        let this: &'tx Self = self;
        match this
            .mutation
            .fact_by_key(relation, key_id, &this.mutation.scratch)?
        {
            Some(bytes) => K::Fact::decode(this, bytes).map(Some),
            None => Ok(None),
        }
    }

    /// Point lookup of the full fact through any key statement of
    /// `relation` — reads observe the final-state view the judgment phase
    /// will judge (`docs/architecture/70-api.md`): the delta's determinant map
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
        let mut out = Vec::new();
        Ok(self
            .get_dyn_into(relation, key, key_values, &mut out)?
            .then_some(out))
    }

    /// [`WriteTx::get_dyn`] into a caller-provided buffer — the pooled
    /// point-read lane, the write-transaction sibling of
    /// [`super::ReadInstance::get_dyn_into`]: the values `Vec` is the
    /// caller's, its capacity retained across gets, so a warm keyed
    /// get's allocator traffic shrinks to the variable-width payload
    /// boxes alone. `Ok(true)` = hit, `out` holds the fact's fields in
    /// declaration order; `Ok(false)` = no fact, `out` empty.
    ///
    /// # Errors
    ///
    /// As [`WriteTx::get_dyn`].
    pub fn get_dyn_into(
        &mut self,
        relation: RelationId,
        key: StatementId,
        key_values: &[Value],
        out: &mut Vec<Value>,
    ) -> Result<bool> {
        self.mutation.get_dyn_into(relation, key, key_values, out)
    }

    /// Final-state membership of a dynamic fact — the dynamic sibling of
    /// [`WriteTx::contains`], completing the schema-generic write surface
    /// (`docs/architecture/70-api.md` § the dyn lane): one [`Value`] per
    /// field in declaration order, judged against the same base + pending
    /// delta view the commit judges. Never mints: a string value known to
    /// neither the delta nor the committed dictionary proves the fact
    /// absent everywhere. A **closed** relation answers from its sealed
    /// extension (virtual storage — no `M` rows exist).
    ///
    /// # Errors
    ///
    /// `FactShape` on an unknown relation id or an arity/type/UTF-8
    /// mismatch (typed, never a panic — the id-addressed surface is
    /// data); `Lmdb` on the membership probe or dictionary reads.
    pub fn contains_dyn(&mut self, rel: RelationId, values: &[Value]) -> Result<bool> {
        self.mutation.contains_dyn(rel, values)
    }
}
