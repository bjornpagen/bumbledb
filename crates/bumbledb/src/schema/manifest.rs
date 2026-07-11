//! The theory's manifest: name → id, as plain data
//! (`docs/architecture/70-api.md` § the manifest). The macro's id
//! constants give the *Rust* host its numbers at compile time
//! (`Calendar::BUSY`, `Calendar::BUSY_PERSON`); the manifest gives a
//! *foreign* host the same numbers as a runtime value — a plain Rust
//! struct straight off the descriptor, no serde, no derive machinery
//! (the dependency law: a downstream binding serializes it however it
//! likes; the engine never learns the wire format).

use super::{FieldId, RelationId, SchemaDescriptor, ValueType};
use crate::value::Value;

/// Every name → id pairing of one theory, in declaration order — named
/// data, not ergonomics. Enum variants ride inside each field's
/// [`ValueType::Enum`] variant list: the ordinal is the index, by the
/// declaration-order law. Closed relations carry their extension — the
/// vocabulary as data, so a foreign surface (render, future bindings)
/// sees every ground axiom without touching Rust.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    /// One entry per relation; `RelationId` = the index, stated
    /// explicitly on each entry so a reader never re-derives it.
    pub relations: Vec<RelationManifest>,
}

/// One relation's names and ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationManifest {
    pub name: Box<str>,
    pub id: RelationId,
    /// One entry per field, in declaration order; `FieldId` = the index.
    pub fields: Vec<FieldManifest>,
    /// A closed relation's ground axioms in declaration order (`None` =
    /// ordinary): handle → id → intrinsic values, plain data off the
    /// descriptor.
    pub extension: Option<Vec<RowManifest>>,
}

/// One ground axiom as manifest data: the handle, its declaration-order
/// row id, and each intrinsic (column, value) pair in field-declaration
/// order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowManifest {
    pub handle: Box<str>,
    pub id: u64,
    pub values: Vec<(Box<str>, Value)>,
}

/// One field's name, id, and structural type (an enum field's variant
/// list carries the variant-name → ordinal mapping as its indices).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldManifest {
    pub name: Box<str>,
    pub id: FieldId,
    pub value_type: ValueType,
}

impl SchemaDescriptor {
    /// Renders the manifest off the descriptor — the ids are the
    /// declaration-order indices, made explicit.
    ///
    /// # Panics
    ///
    /// When a relation or field ordinal exceeds the id space (`u32`/`u16`)
    /// — impossible for a descriptor the declaration boundary admits.
    #[must_use]
    pub fn manifest(&self) -> Manifest {
        Manifest {
            relations: self
                .relations
                .iter()
                .enumerate()
                .map(|(rel_idx, relation)| {
                    // A closed relation's sealed field list opens with the
                    // synthetic (`id`, U64) field, so the manifest reports
                    // the ids the sealed schema answers to (handle-name
                    // entries are the emission PRD's).
                    let synthetic = relation.extension.is_some().then(|| FieldManifest {
                        name: "id".into(),
                        id: FieldId(0),
                        value_type: super::ValueType::U64,
                    });
                    let offset = usize::from(synthetic.is_some());
                    // The extension table: handle → declaration-order id →
                    // (column, value) pairs — the vocabulary as data.
                    let extension = relation.extension.as_ref().map(|rows| {
                        rows.iter()
                            .enumerate()
                            .map(|(row_idx, row)| RowManifest {
                                handle: row.handle.clone(),
                                id: u64::try_from(row_idx).expect("row count fits u64"),
                                values: relation
                                    .fields
                                    .iter()
                                    .map(|field| field.name.clone())
                                    .zip(row.values.iter().cloned())
                                    .collect(),
                            })
                            .collect()
                    });
                    RelationManifest {
                        name: relation.name.clone(),
                        id: RelationId(u32::try_from(rel_idx).expect("relation count fits u32")),
                        fields: synthetic
                            .into_iter()
                            .chain(
                                relation
                                    .fields
                                    .iter()
                                    .enumerate()
                                    .map(|(field_idx, field)| FieldManifest {
                                        name: field.name.clone(),
                                        id: FieldId(
                                            u16::try_from(field_idx + offset)
                                                .expect("field count fits u16"),
                                        ),
                                        value_type: field.value_type.clone(),
                                    }),
                            )
                            .collect(),
                        extension,
                    }
                })
                .collect(),
        }
    }
}
