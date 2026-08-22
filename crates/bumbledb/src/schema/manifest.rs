//! The theory's manifest: name → id, as plain data
//! . The macro's id
//! (`Calendar::BUSY`, `Calendar::BUSY_PERSON`); the manifest gives a
//! struct straight off the descriptor, no serde, no derive machinery
//! (the dependency law: a downstream binding serializes it however it
//! likes; the engine never learns the wire format).

use super::{FieldId, RelationId, SchemaDescriptor, StatementId, StatementKind, ValueType};
use bumbledb_theory::Value;

/// Every name → id pairing of one theory, in declaration order — named
/// data, not ergonomics. A closed relation's handles ride as its
/// [`RowManifest`] list: the row id is the index, by the
/// declaration-order law. Closed relations carry their extension — the
/// vocabulary as data, so a foreign surface (render, future bindings)
/// sees every ground axiom without touching Rust. Statements ride in
/// materialized order with their canonical spellings, so a foreign host
/// can cite any statement id — a rejection's, a diagnostic's — without a
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Manifest {
    pub relations: Vec<RelationManifest>,

    pub statements: Vec<StatementManifest>,
}

/// One statement's identity, form tag, and canonical spelling
/// ([`super::render::render_declared`] — the one renderer, a bijection
/// on legal statements).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StatementManifest {
    pub id: StatementId,
    pub kind: StatementKind,
    pub spelling: String,
}

/// One relation's names and ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RelationManifest {
    pub name: Box<str>,
    pub id: RelationId,

    pub fields: Vec<FieldManifest>,

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

/// One field's name, id, and structural type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldManifest {
    pub name: Box<str>,
    pub id: FieldId,
    pub value_type: ValueType,
}

/// The manifest rendering as an extension trait: [`SchemaDescriptor`] is
/// theory data (hosted in `bumbledb-theory`), and the manifest needs the
/// engine-side renderer, so the method hangs off it here.
pub trait ManifestDescriptor {
    fn manifest(&self) -> Manifest;
}

impl ManifestDescriptor for SchemaDescriptor {
    /// # Panics
    fn manifest(&self) -> Manifest {
        let materialized = self.materialized_statements();
        let mirrors = super::validate::mirror_links(&materialized);
        Manifest {
            statements: materialized
                .iter()
                .enumerate()
                .map(|(idx, statement)| {
                    let id = StatementId(u16::try_from(idx).expect("statement count fits u16"));
                    StatementManifest {
                        id,
                        kind: statement.kind(),
                        spelling: super::render::render_materialized(
                            self,
                            &materialized,
                            &mirrors,
                            id,
                        ),
                    }
                })
                .collect(),
            relations: self
                .relations
                .iter()
                .enumerate()
                .map(|(rel_idx, relation)| {
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

                        fields: relation
                            .sealed_fields()
                            .enumerate()
                            .map(|(field_idx, slot)| FieldManifest {
                                name: slot.name().into(),
                                id: FieldId(
                                    u16::try_from(field_idx).expect("field count fits u16"),
                                ),
                                value_type: *slot.value_type(),
                            })
                            .collect(),
                        extension,
                    }
                })
                .collect(),
        }
    }
}
