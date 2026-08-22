//! `WriteTx` point reads: `contains` / `get` / `get_dyn` read **committed state overlaid
//! with the pending delta** — the same final-state view the judgment phase
//! judges — so read-modify-write idioms (upsert, check-then-act conditions)
//! are sound without exposing query machinery to the write path. These are
//! determinant gets: no images, no plans, no `ReadInstance`.

use super::collection::shape_mismatch;
use super::{Fact, Key, Probe, WriteTx};
use crate::encoding::encode_u64;
use crate::error::{DynIdError, FactShapeError, Mismatch, Result};
use crate::ir::Value;
use crate::schema::{KeyForm, KeyId, KeyStatement, Relation, RelationBody, Schema, StatementView};
use crate::storage::read;
use bumbledb_theory::schema::{FieldId, RelationId, StatementId};

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

            _ => {
                crate::encoding::encode_literal(value, rel.field(field).value_type, out);
            }
        }
    }
    Ok(true)
}

pub(super) fn fresh_row_id(determinant: &[u8]) -> u64 {
    let Ok(word) = <[u8; 8]>::try_from(determinant) else {
        unreachable!("KeyForm::FreshRow determinant is one encoded u64");
    };
    u64::from_be_bytes(word)
}

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

/// Shared by both transaction kinds (a closed relation reads identically
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
    /// otherwise. Before commit it answers exactly what a post-commit

    /// # Errors

    pub fn contains<'f, F: Fact<'f, Schema = S>>(&mut self, fact: &F) -> Result<bool> {
        self.mutation.contains(fact)
    }

    /// pages, stable for the transaction by LMDB `CoW`) or from this

    /// ```

    /// ```

    /// # Errors

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

    /// # Errors

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

    /// # Errors

    pub fn get_dyn_into(
        &mut self,
        relation: RelationId,
        key: StatementId,
        key_values: &[Value],
        out: &mut Vec<Value>,
    ) -> Result<bool> {
        self.mutation.get_dyn_into(relation, key, key_values, out)
    }

    /// # Errors

    pub fn contains_dyn(&mut self, rel: RelationId, values: &[Value]) -> Result<bool> {
        self.mutation.contains_dyn(rel, values)
    }
}
