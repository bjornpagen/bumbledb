//! Closed-relation read substrate: every sealed extension row decoded and
//! canonically re-encoded ONCE per handle, so closed reads (contains / get /
//! scan / typed decode) speak the same canonical wire as ordinary rows and
//! typed borrows have a stable owner for the handle's lifetime.
//!
//! Sealed rows are encoded at schema validation in the fixed-width sealed
//! codec (they refuse text columns by construction); this module is the one
//! bridge from that sealed form to canonical row bytes.

use std::collections::BTreeMap;

use crate::canonical::CanonicalRow;
use crate::error::Result;
use crate::ir::Value;
use crate::schema::Schema;
use crate::work::WorkContext;
use bumbledb_theory::schema::RelationId;

#[derive(Debug)]
pub(super) struct ClosedRow {
    /// The sealed row's canonical wire bytes (same codec as stored rows).
    pub(super) canonical: Box<[u8]>,
    /// The sealed row's decoded values, sealed field order.
    pub(super) values: Box<[Value]>,
}

/// Per-handle closed-relation rows. Empty for schemas without closed
/// relations; lookups on ordinary relations return `None`.
#[derive(Debug, Default)]
pub(super) struct ClosedRows {
    relations: BTreeMap<RelationId, Vec<ClosedRow>>,
}

impl ClosedRows {
    pub(super) fn build(schema: &Schema, work: &WorkContext) -> Result<Self> {
        let mut relations = BTreeMap::new();
        for (index, relation) in schema.relations().iter().enumerate() {
            let Some(extension) = relation.body().closed_rows() else {
                continue;
            };
            let id = RelationId(u32::try_from(index).expect("sealed relation ids fit u32"));
            let mut rows = Vec::with_capacity(extension.len());
            for sealed in extension {
                let values = super::get::decode_sealed_row(relation, &sealed.fact);
                let canonical =
                    CanonicalRow::encode(relation.fields(), &values, work).map_err(|error| {
                        crate::error::Error::from_store(crate::storage::store::StoreError::Changes(
                            crate::changes::ChangeError::Row(error),
                        ))
                    })?;
                rows.push(ClosedRow {
                    canonical: Box::from(canonical.as_bytes()),
                    values: values.into_boxed_slice(),
                });
            }
            relations.insert(id, rows);
        }
        Ok(Self { relations })
    }

    /// `Some(rows)` exactly for closed relations.
    pub(super) fn get(&self, relation: RelationId) -> Option<&[ClosedRow]> {
        self.relations.get(&relation).map(Vec::as_slice)
    }
}
