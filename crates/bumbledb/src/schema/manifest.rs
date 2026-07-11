//! The theory's manifest: name → id, as plain data
//! (`docs/architecture/70-api.md` § the manifest). The macro's id
//! constants give the *Rust* host its numbers at compile time
//! (`Calendar::BUSY`, `Calendar::BUSY_PERSON`); the manifest gives a
//! *foreign* host the same numbers as a runtime value — a plain Rust
//! struct straight off the descriptor, no serde, no derive machinery
//! (the dependency law: a downstream binding serializes it however it
//! likes; the engine never learns the wire format).

use super::{FieldId, RelationId, SchemaDescriptor, ValueType};

/// Every name → id pairing of one theory, in declaration order — named
/// data, not ergonomics. Enum variants ride inside each field's
/// [`ValueType::Enum`] variant list: the ordinal is the index, by the
/// declaration-order law.
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
                    }
                })
                .collect(),
        }
    }
}
